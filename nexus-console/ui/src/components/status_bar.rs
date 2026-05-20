//! Bottom status bar.

use leptos::prelude::*;

use crate::types::ConnectionInfo;

#[component]
pub fn StatusBar(
    /// Active connection.
    connection: ReadSignal<Option<ConnectionInfo>>,
    /// Live session id.
    session_id: ReadSignal<Option<u64>>,
    /// Agent count.
    agent_count: ReadSignal<usize>,
) -> impl IntoView {
    view! {
        <footer class="status-bar">
            <span>"nexus-console " {env!("CARGO_PKG_VERSION")}</span>
            <span>"·"</span>
            <span>
                {move || match connection.get() {
                    Some(info) => format!(
                        "Connected: {} ({}) @ {}",
                        info.server_name, info.server_version, info.addr
                    ),
                    None => "Disconnected".to_string(),
                }}
            </span>
            <span>"·"</span>
            <span>{move || format!("{} agents", agent_count.get())}</span>
            <span>"·"</span>
            <span>
                {move || match session_id.get() {
                    Some(id) => format!("Session #{id}"),
                    None => "No session".to_string(),
                }}
            </span>
            <span style="margin-left: auto;">
                {move || connection.get().map(|info| {
                    if info.insecure_network {
                        view! { <span class="insecure">"INSECURE — non-loopback"</span> }
                            .into_any()
                    } else {
                        view! { <span class="secure">"loopback"</span> }.into_any()
                    }
                })}
            </span>
        </footer>
    }
}
