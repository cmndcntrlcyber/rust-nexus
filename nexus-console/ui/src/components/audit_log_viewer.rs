//! v1.4.4 audit log viewer (Phase 1.4.4).
//!
//! Read-only viewer that calls the v1.4.3 `StreamAuditRecords` A2A
//! RPC via three Tauri commands:
//!
//! - `audit_log_tail(count)` — fetch up to N most-recent records.
//! - `audit_log_filter({actor, action, since_unix}, count)` — same with
//!   filter knobs.
//! - `audit_log_verify(records)` — recompute the hash chain client-side
//!   (BLAKE3) and report `Some(index)` on the first break.
//!
//! The component renders a table with timestamp / actor / action /
//! resource / record_hash columns, filter inputs, and a "Verify"
//! button. No live tailing yet (v1.5 — needs a Tauri event channel for
//! per-record push).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::tauri_api;

/// One audit record (mirrors the Tauri-side `AuditRecord` struct).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditRecord {
    pub timestamp_unix: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub prev_hash: String,
    pub record_hash: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct FilterArgs {
    filter: AuditFilter,
    count: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
struct AuditFilter {
    actor: String,
    action: String,
    since_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
struct TailArgs {
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyArgs {
    records: Vec<AuditRecord>,
}

#[component]
pub fn AuditLogViewer() -> impl IntoView {
    let (records, set_records) = signal::<Vec<AuditRecord>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (verify_status, set_verify_status) = signal::<Option<String>>(None);
    let (actor_filter, set_actor_filter) = signal::<String>(String::new());
    let (action_filter, set_action_filter) = signal::<String>(String::new());
    let (since_filter, set_since_filter) = signal::<u64>(0);
    let (busy, set_busy) = signal::<bool>(false);

    // -- Refresh handler: tail or filter depending on whether any
    //    filter input is non-empty.
    let on_refresh = move |_| {
        let actor = actor_filter.get();
        let action = action_filter.get();
        let since = since_filter.get();
        set_busy.set(true);
        set_verify_status.set(None);
        spawn_local(async move {
            let result: Result<Vec<AuditRecord>, String> =
                if actor.is_empty() && action.is_empty() && since == 0 {
                    tauri_api::invoke("audit_log_tail", &TailArgs { count: 200 }).await
                } else {
                    tauri_api::invoke(
                        "audit_log_filter",
                        &FilterArgs {
                            filter: AuditFilter {
                                actor,
                                action,
                                since_unix: since,
                            },
                            count: 200,
                        },
                    )
                    .await
                };
            match result {
                Ok(list) => {
                    set_records.set(list);
                    set_error.set(None);
                }
                Err(err) => set_error.set(Some(err)),
            }
            set_busy.set(false);
        });
    };

    // -- Verify handler: ship the in-memory records back to the
    //    Tauri side and ask it to recompute the chain.
    let on_verify = move |_| {
        let snapshot = records.get();
        if snapshot.is_empty() {
            set_verify_status.set(Some("no records to verify".to_string()));
            return;
        }
        set_busy.set(true);
        spawn_local(async move {
            let result: Result<Option<usize>, String> = tauri_api::invoke(
                "audit_log_verify",
                &VerifyArgs { records: snapshot },
            )
            .await;
            match result {
                Ok(None) => set_verify_status.set(Some("✓ chain intact".to_string())),
                Ok(Some(idx)) => {
                    set_verify_status.set(Some(format!("✗ chain broken at record {idx}")))
                }
                Err(err) => set_verify_status.set(Some(format!("verify error: {err}"))),
            }
            set_busy.set(false);
        });
    };

    view! {
        <section class="audit-log-viewer">
            <h2>"Audit log"</h2>
            <div class="filter-row">
                <label>
                    "Actor"
                    <input
                        type="text"
                        prop:value=move || actor_filter.get()
                        on:input=move |ev| set_actor_filter.set(event_target_value(&ev))
                    />
                </label>
                <label>
                    "Action"
                    <input
                        type="text"
                        prop:value=move || action_filter.get()
                        on:input=move |ev| set_action_filter.set(event_target_value(&ev))
                    />
                </label>
                <label>
                    "Since (unix)"
                    <input
                        type="number"
                        min="0"
                        prop:value=move || since_filter.get().to_string()
                        on:input=move |ev| {
                            let s = event_target_value(&ev);
                            set_since_filter.set(s.parse().unwrap_or(0));
                        }
                    />
                </label>
                <button
                    on:click=on_refresh
                    disabled=move || busy.get()
                >
                    "Refresh"
                </button>
                <button
                    on:click=on_verify
                    disabled=move || busy.get() || records.with(Vec::is_empty)
                >
                    "Verify integrity"
                </button>
            </div>
            {move || verify_status.get().map(|s| view! { <p class="verify-status">{s}</p> })}
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
            <p class="record-count">
                {move || format!("{} records", records.with(Vec::len))}
            </p>
            <table class="audit-table">
                <thead>
                    <tr>
                        <th>"Timestamp"</th>
                        <th>"Actor"</th>
                        <th>"Action"</th>
                        <th>"Resource"</th>
                        <th>"Record hash"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || records.get()
                        key=|r| r.record_hash.clone()
                        children=move |r| {
                            view! {
                                <tr>
                                    <td>{r.timestamp_unix.to_string()}</td>
                                    <td>{r.actor.clone()}</td>
                                    <td>{r.action.clone()}</td>
                                    <td>{r.resource.clone()}</td>
                                    <td class="hash">{shorten_hash(&r.record_hash)}</td>
                                </tr>
                            }
                        }
                    />
                </tbody>
            </table>
        </section>
    }
}

fn shorten_hash(hex: &str) -> String {
    if hex.len() > 12 {
        format!("{}…{}", &hex[..6], &hex[hex.len() - 6..])
    } else {
        hex.to_string()
    }
}
