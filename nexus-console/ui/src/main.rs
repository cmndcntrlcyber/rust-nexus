//! nexus-console UI entrypoint.

#![warn(missing_docs)]

use leptos::prelude::*;

mod components;
mod tauri_api;
mod types;
mod xterm;

use components::agent_list::AgentList;
use components::connect_dialog::ConnectDialog;
use components::status_bar::StatusBar;
use components::terminal::Terminal;
use types::ConnectionInfo;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (connection, set_connection) = signal::<Option<ConnectionInfo>>(None);
    let (selected_agent, set_selected_agent) = signal::<Option<String>>(None);
    let (session_id, set_session_id) = signal::<Option<u64>>(None);
    let (agent_count, set_agent_count) = signal(0usize);

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
                <AgentList
                    selected=selected_agent
                    set_selected=set_selected_agent
                    set_agent_count=set_agent_count
                />
                <Terminal
                    selected=selected_agent
                    set_session_id_signal=set_session_id
                />
                <StatusBar
                    connection=connection
                    session_id=session_id
                    agent_count=agent_count
                />
            </div>
        </Show>
    }
}
