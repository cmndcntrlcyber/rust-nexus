//! Pre-connect form.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use wasm_bindgen::JsCast;

use crate::tauri_api;
use crate::types::{ConnectResponse, ConnectionInfo};

#[derive(Serialize)]
struct ConnectArgs<'a> {
    addr: &'a str,
    #[serde(rename = "insecureNetwork")]
    insecure_network: bool,
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

    let on_submit = move |_| {
        if busy.get() {
            return;
        }
        let a = addr.get();
        let i = insecure.get();
        set_busy.set(true);
        set_status.set(Some("connecting…".to_string()));

        spawn_local(async move {
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
                }
                Err(err) => {
                    set_status.set(Some(format!("error: {err}")));
                    set_busy.set(false);
                }
            }
        });
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
            <button on:click=on_submit disabled=move || busy.get()>
                {move || if busy.get() { "Connecting…" } else { "Connect" }}
            </button>
            <p class="status">{move || status.get().unwrap_or_default()}</p>
        </div>
    }
}
