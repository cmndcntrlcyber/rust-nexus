//! Shell tab wrapper component.
//!
//! Wraps the existing Terminal component for use inside the tab
//! container. Does not modify the Terminal component itself.

use leptos::prelude::*;

/// A tab pane representing a shell session connected to a specific
/// agent.
#[component]
pub fn ShellTab(agent_peer_id: String) -> impl IntoView {
    view! {
        <div class="shell-tab">
            <p>"Shell session for: " {agent_peer_id}</p>
        </div>
    }
}
