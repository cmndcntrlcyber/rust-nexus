//! Pre-connect form with retry, cancel, and address validation.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::tauri_api;
use crate::types::{ConnectResponse, ConnectionInfo};

#[derive(Serialize)]
struct ConnectArgs<'a> {
    addr: &'a str,
    #[serde(rename = "insecureNetwork")]
    insecure_network: bool,
}

/// WASM-compatible sleep (no extra crate needed).
async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });
    let _ = JsFuture::from(promise).await;
}

/// Connection dialog.
#[component]
pub fn ConnectDialog(
    /// Setter called once connected.
    set_connection: WriteSignal<Option<ConnectionInfo>>,
) -> impl IntoView {
    let (addr, set_addr) = signal("http://127.0.0.1:50052".to_string());
    let (insecure, set_insecure) = signal(false);
    let (status, set_status) = signal::<Option<String>>(None);
    let (busy, set_busy) = signal(false);
    let (cancel_token, set_cancel_token) = signal::<Option<Rc<Cell<bool>>>>(None);

    // Pre-fill address from NEXUS_SERVER_ADDR if the deploy script set it.
    {
        let set_addr = set_addr;
        Effect::new(move |_| {
            spawn_local(async move {
                #[derive(serde::Deserialize)]
                struct StartupConfig {
                    server_addr: Option<String>,
                }
                if let Ok(cfg) =
                    tauri_api::invoke::<_, StartupConfig>("get_startup_config", &()).await
                {
                    if let Some(a) = cfg.server_addr {
                        set_addr.set(a);
                    }
                }
            });
        });
    }

    let on_submit = move |_| {
        if busy.get() {
            return;
        }
        let a = addr.get();
        let i = insecure.get();
        set_busy.set(true);

        let cancelled = Rc::new(Cell::new(false));
        set_cancel_token.set(Some(cancelled.clone()));

        spawn_local(async move {
            let max_retries: u32 = 3;
            let mut attempt: u32 = 0;
            let mut backoff_ms: u32 = 1000;

            loop {
                if cancelled.get() {
                    return;
                }

                attempt += 1;

                if attempt == 1 {
                    set_status.set(Some("connecting\u{2026}".to_string()));
                } else {
                    set_status.set(Some(format!(
                        "retry {}/{} \u{2014} connecting\u{2026}",
                        attempt - 1,
                        max_retries
                    )));
                }

                let args = ConnectArgs {
                    addr: &a,
                    insecure_network: i,
                };
                match tauri_api::invoke::<_, ConnectResponse>("connect_c2", &args).await {
                    Ok(resp) => {
                        set_status.set(Some(format!(
                            "connected to {} ({})",
                            resp.server_name, resp.server_version
                        )));
                        set_connection.set(Some(resp.into()));
                        return;
                    }
                    Err(err) => {
                        if cancelled.get() {
                            return;
                        }

                        let is_preflight = err.contains("invalid address")
                            || err.contains("missing scheme")
                            || err.contains("missing port")
                            || err.contains("missing hostname")
                            || err.contains("requires mTLS");

                        if is_preflight || attempt > max_retries {
                            set_status.set(Some(format!("error: {err}")));
                            set_busy.set(false);
                            return;
                        }

                        set_status.set(Some(format!(
                            "attempt {} failed: {err} \u{2014} retrying in {}s\u{2026}",
                            attempt,
                            backoff_ms / 1000
                        )));

                        let delay = backoff_ms;
                        backoff_ms = (backoff_ms * 2).min(30_000);
                        sleep_ms(delay).await;
                    }
                }
            }
        });
    };

    let on_cancel = move |_| {
        if let Some(token) = cancel_token.get() {
            token.set(true);
        }
        set_busy.set(false);
        set_status.set(Some("cancelled".to_string()));
    };

    view! {
        <div class="connect-dialog">
            <h1>"nexus-console"</h1>
            <p class="subtitle">"Connect to a rust-nexus C2 server (A2A endpoint)"</p>
            <label>
                "C2 A2A address"
                <input
                    type="text"
                    prop:value=move || addr.get()
                    on:input=move |ev| {
                        let val = ev.target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|e| e.value())
                            .unwrap_or_default();
                        set_addr.set(val);
                    }
                    placeholder="http://127.0.0.1:50052"
                />
            </label>
            {move || {
                let a = addr.get();
                if !a.is_empty() && !a.starts_with("http://") && !a.starts_with("https://") {
                    view! {
                        <p class="input-hint warning">"Address must start with http:// or https://"</p>
                    }.into_any()
                } else if a.contains("-a2a.") || a.contains("-api.") {
                    view! {
                        <p class="input-hint warning">
                            "This looks like a tunnel URL. The C2 gRPC endpoint is typically a direct address (e.g. https://c2.example.com:50052)"
                        </p>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
            <label class="checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || insecure.get()
                    on:change=move |ev| {
                        let checked = ev.target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|e| e.checked())
                            .unwrap_or(false);
                        set_insecure.set(checked);
                    }
                />
                "Allow non-loopback address"
            </label>
            <div class="connect-buttons">
                <button on:click=on_submit disabled=move || busy.get()>
                    {move || if busy.get() { "Connecting\u{2026}" } else { "Connect" }}
                </button>
                {move || {
                    if busy.get() {
                        view! {
                            <button class="cancel-btn" on:click=on_cancel>
                                "Cancel"
                            </button>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>
            <p class="status">{move || status.get().unwrap_or_default()}</p>
        </div>
    }
}
