//! nexus-console UI entrypoint.

#![warn(missing_docs)]

use leptos::prelude::*;
use wasm_bindgen::JsCast;

mod components;
mod tauri_api;
mod types;
mod xterm;

use components::agent_list::AgentList;
use components::chat::chat_tab::ChatTab;
use components::connect_dialog::ConnectDialog;
use components::dashboard_tab::DashboardTab;
use components::kali_tab::KaliTab;
use components::mesh_tab::MeshTab;
use components::service_tab::ServiceTab;
use components::status_bar::StatusBar;
use components::tab_bar::{TabBar, TabKind, TunnelServiceInfo};
use components::terminal::Terminal;
use components::transfer_tab::TransferTab;
use types::ConnectionInfo;

fn main() {
    console_error_panic_hook::set_once();
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("root")
        .expect("#root not found")
        .unchecked_into::<web_sys::HtmlElement>();
    leptos::mount::mount_to(root, App).forget();
}

#[component]
fn App() -> impl IntoView {
    let (connection, set_connection) = signal::<Option<ConnectionInfo>>(None);
    let (selected_agent, set_selected_agent) = signal::<Option<String>>(None);
    let (session_id, set_session_id) = signal::<Option<u64>>(None);
    let (agent_count, set_agent_count) = signal(0usize);

    // Tab architecture signals.
    let (active_tab, set_active_tab) = signal(TabKind::Chat);
    let (_sidebar_collapsed, _set_sidebar_collapsed) = signal(false);
    let (dynamic_tabs, set_dynamic_tabs) = signal::<Vec<TabKind>>(Vec::new());

    // v4.4: tunnel service discovery.
    let (tunnel_services, set_tunnel_services) = signal::<Vec<TunnelServiceInfo>>(Vec::new());

    // Load tunnel services from Tauri backend on mount.
    {
        let set_tunnel_services = set_tunnel_services.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(services) = tauri_api::get_tunnel_services().await {
                set_tunnel_services.set(services);
            }
        });
    }

    // Derived signals for new fixed tab URLs (tunnel URL with localhost fallback).
    let tunnel_url_for = move |suffix: &str, fallback: &str| -> String {
        let services = tunnel_services.get();
        services
            .iter()
            .find(|s| s.suffix == suffix)
            .map(|s| s.url.clone())
            .unwrap_or_else(|| fallback.to_string())
    };

    // Status bar services: services without a fixed tab mapping (Empire, Registry, API).
    let status_bar_services = Signal::derive(move || {
        tunnel_services
            .get()
            .into_iter()
            .filter(|s| s.tab_mapping.is_none())
            .collect::<Vec<_>>()
    });

    let on_close_dynamic = Callback::new(move |tab: TabKind| {
        let mut current = dynamic_tabs.get();
        current.retain(|t| t != &tab);
        set_dynamic_tabs.set(current);
    });

    let add_dynamic_tab = Callback::new(move |tab: TabKind| {
        let mut current = dynamic_tabs.get();
        if !current.contains(&tab) {
            current.push(tab);
            set_dynamic_tabs.set(current);
        }
    });

    view! {
        <Show
            when=move || connection.get().is_some()
            fallback=move || {
                view! {
                    <div class="root pre-connect">
                        <ConnectDialog set_connection=set_connection />
                    </div>
                }
            }
        >
            <div class="root connected">
                <TabBar
                    active_tab=active_tab
                    set_active_tab=set_active_tab
                    dynamic_tabs=dynamic_tabs
                    on_close_dynamic=on_close_dynamic
                    tunnel_services=tunnel_services
                />
                {move || {
                    active_tab.get().shows_sidebar().then(|| {
                        view! {
                            <AgentList
                                selected=selected_agent
                                set_selected=set_selected_agent
                                set_agent_count=set_agent_count
                            />
                        }
                    })
                }}
                <main class="tab-content-area">
                    {move || {
                        match active_tab.get() {
                            TabKind::Dashboard => view! { <DashboardTab /> }.into_any(),
                            TabKind::Transfer => view! { <TransferTab /> }.into_any(),
                            TabKind::Mesh => view! { <MeshTab /> }.into_any(),
                            TabKind::Kali => view! { <KaliTab /> }.into_any(),
                            TabKind::Chat => view! { <ChatTab /> }.into_any(),

                            // v4.4: new fixed tabs with tunnel URL + localhost fallback.
                            TabKind::Workbench => {
                                let url = tunnel_url_for("-workbench", "http://localhost:3020");
                                view! { <ServiceTab url=url label="ATT&CK Workbench".into() /> }.into_any()
                            }
                            TabKind::Portainer => {
                                let url = tunnel_url_for("-mgmt", "http://localhost:9443");
                                view! { <ServiceTab url=url label="Portainer".into() /> }.into_any()
                            }
                            TabKind::Wiki => {
                                let url = tunnel_url_for("-wiki", "http://localhost:3010");
                                view! { <ServiceTab url=url label="Docmost Wiki".into() /> }.into_any()
                            }
                            TabKind::VsCode => {
                                let url = tunnel_url_for("-vscode", "http://localhost:6902");
                                view! { <ServiceTab url=url label="VS Code Desktop".into() /> }.into_any()
                            }
                            TabKind::Reports => {
                                let url = tunnel_url_for("-reports", "http://localhost:7777");
                                view! { <ServiceTab url=url label="SysReptor Reports".into() /> }.into_any()
                            }
                            TabKind::Kasm => {
                                let url = tunnel_url_for("-kasm", "https://localhost:8443");
                                view! { <ServiceTab url=url label="Kasm Portal".into() /> }.into_any()
                            }

                            TabKind::Shell { .. } => view! {
                                <Terminal
                                    selected=selected_agent
                                    set_session_id_signal=set_session_id
                                />
                            }.into_any(),
                            TabKind::AuditLog => view! {
                                <div class="tab-placeholder">
                                    <p>"Audit Log"</p>
                                </div>
                            }.into_any(),
                            // v4.4: dynamic tunnel service tab (Empire, Registry).
                            TabKind::TunnelService { url, label, .. } => {
                                view! { <ServiceTab url=url label=label /> }.into_any()
                            }
                        }
                    }}
                </main>
                <StatusBar
                    connection=connection
                    session_id=session_id
                    agent_count=agent_count
                    status_bar_services=status_bar_services
                    set_active_tab=set_active_tab
                    add_dynamic_tab=add_dynamic_tab
                />
            </div>
        </Show>
    }
}
