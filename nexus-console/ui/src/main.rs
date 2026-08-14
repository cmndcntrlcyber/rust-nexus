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
use components::status_bar::StatusBar;
use components::tab_bar::{TabBar, TabKind};
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

    let on_close_dynamic = Callback::new(move |tab: TabKind| {
        let mut current = dynamic_tabs.get();
        current.retain(|t| t != &tab);
        set_dynamic_tabs.set(current);
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
                        }
                    }}
                </main>
                <StatusBar
                    connection=connection
                    session_id=session_id
                    agent_count=agent_count
                />
            </div>
        </Show>
    }
}
