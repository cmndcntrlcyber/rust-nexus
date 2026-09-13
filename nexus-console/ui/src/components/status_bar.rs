//! Bottom status bar with tunnel service badges.

use leptos::prelude::*;

use crate::components::tab_bar::{TabKind, TunnelServiceInfo};
use crate::types::ConnectionInfo;

#[component]
pub fn StatusBar(
    /// Active connection.
    connection: ReadSignal<Option<ConnectionInfo>>,
    /// Live session id.
    session_id: ReadSignal<Option<u64>>,
    /// Agent count.
    agent_count: ReadSignal<usize>,
    /// Tunnel services without fixed tabs (Empire, Registry, API).
    status_bar_services: Signal<Vec<TunnelServiceInfo>>,
    /// Setter to open a dynamic tunnel service tab.
    set_active_tab: WriteSignal<TabKind>,
    /// Setter to add a dynamic tab.
    add_dynamic_tab: Callback<TabKind>,
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

            // v4.4: tunnel service badges for services without fixed tabs.
            {move || {
                let services = status_bar_services.get();
                if services.is_empty() {
                    None
                } else {
                    Some(view! {
                        <span class="status-bar-services">
                            <span>"·"</span>
                            {services.into_iter().map(|svc| {
                                let label = svc.label.clone();
                                let url = svc.url.clone();
                                let suffix = svc.suffix.clone();
                                let embeddable = svc.embeddable;
                                let badge_title = format!("{} \u{2014} {}", label, url);
                                view! {
                                    <span
                                        class="service-badge"
                                        title=badge_title
                                    >
                                        <span class="badge-dot active"></span>
                                        {label.clone()}
                                        {if embeddable {
                                            let tab_label = label.clone();
                                            let tab_url = url.clone();
                                            let tab_suffix = suffix.clone();
                                            Some(view! {
                                                <button
                                                    class="open-in-tab-btn"
                                                    title="Open in tab"
                                                    on:click=move |_| {
                                                        let tab = TabKind::TunnelService {
                                                            suffix: tab_suffix.clone(),
                                                            label: tab_label.clone(),
                                                            url: tab_url.clone(),
                                                        };
                                                        add_dynamic_tab.run(tab.clone());
                                                        set_active_tab.set(tab);
                                                    }
                                                >
                                                    "\u{21d7}"
                                                </button>
                                            })
                                        } else {
                                            None
                                        }}
                                    </span>
                                }
                            }).collect::<Vec<_>>()}
                        </span>
                    })
                }
            }}

            <span style="margin-left: auto;">
                {move || connection.get().map(|info| {
                    if info.insecure_network {
                        view! { <span class="insecure">"INSECURE \u{2014} non-loopback"</span> }
                            .into_any()
                    } else {
                        view! { <span class="secure">"loopback"</span> }.into_any()
                    }
                })}
            </span>
        </footer>
    }
}
