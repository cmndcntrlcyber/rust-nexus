//! Tab bar component for the operator console UI.
//!
//! Renders a horizontal bar of 5 fixed tab buttons (Dashboard, Transfer,
//! Mesh, Kali, Chat) plus optional dynamic tabs (Shell sessions,
//! AuditLog). The active tab receives an accent underline via the
//! `active` CSS class.

use leptos::prelude::*;

/// The kind of content a tab holds.
#[derive(Clone, PartialEq)]
pub enum TabKind {
    /// System dashboard overview.
    Dashboard,
    /// File transfer pane.
    Transfer,
    /// Mesh network visualizer.
    Mesh,
    /// Kali Linux tooling pane.
    Kali,
    /// Chat / command pane (renders Terminal temporarily).
    Chat,
    /// An interactive shell session targeting a specific agent.
    Shell {
        /// The peer id of the agent this shell connects to.
        agent_peer_id: String,
    },
    /// The audit-log viewer pane.
    AuditLog,
}

impl TabKind {
    /// Human-readable label for the tab button.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Transfer => "Transfer",
            Self::Mesh => "Mesh",
            Self::Kali => "Kali",
            Self::Chat => "Chat",
            Self::Shell { .. } => "Shell",
            Self::AuditLog => "Audit Log",
        }
    }

    /// Numeric index for the Tauri `switch_tab` command.
    pub fn tab_index(&self) -> u64 {
        match self {
            Self::Dashboard => 0,
            Self::Transfer => 1,
            Self::Mesh => 2,
            Self::Kali => 3,
            Self::Chat => 4,
            Self::Shell { .. } => 5,
            Self::AuditLog => 6,
        }
    }

    /// Whether the agent sidebar should be visible for this tab.
    pub fn shows_sidebar(&self) -> bool {
        matches!(
            self,
            Self::Transfer | Self::Mesh | Self::Chat | Self::Shell { .. }
        )
    }

    /// Stable string key for `<For>` item keying.
    pub fn key(&self) -> String {
        match self {
            Self::Dashboard => "dashboard".into(),
            Self::Transfer => "transfer".into(),
            Self::Mesh => "mesh".into(),
            Self::Kali => "kali".into(),
            Self::Chat => "chat".into(),
            Self::Shell { agent_peer_id } => format!("shell-{agent_peer_id}"),
            Self::AuditLog => "audit-log".into(),
        }
    }

    /// Display label including dynamic details (e.g. shortened peer id
    /// for Shell tabs).
    pub fn display_label(&self) -> String {
        match self {
            Self::Shell { agent_peer_id } => {
                let short = if agent_peer_id.len() > 10 {
                    format!(
                        "{}..{}",
                        &agent_peer_id[..6],
                        &agent_peer_id[agent_peer_id.len() - 4..]
                    )
                } else {
                    agent_peer_id.clone()
                };
                format!("Shell {short}")
            }
            other => other.label().to_string(),
        }
    }
}

/// Horizontal tab bar rendered at the top of the connected layout.
///
/// Renders 5 fixed tab buttons plus any dynamic tabs passed via the
/// `dynamic_tabs` signal. The active tab receives the `active` CSS
/// class. Dynamic tabs include a close button.
#[component]
pub fn TabBar(
    /// Signal providing the currently active tab.
    active_tab: ReadSignal<TabKind>,
    /// Setter invoked when a tab is clicked.
    set_active_tab: WriteSignal<TabKind>,
    /// Dynamic tabs beyond the 5 fixed tabs.
    dynamic_tabs: ReadSignal<Vec<TabKind>>,
    /// Callback invoked when a dynamic tab's close button is clicked.
    on_close_dynamic: Callback<TabKind>,
) -> impl IntoView {
    let fixed = vec![
        TabKind::Dashboard,
        TabKind::Transfer,
        TabKind::Mesh,
        TabKind::Kali,
        TabKind::Chat,
    ];

    view! {
        <nav class="tab-bar">
            {fixed
                .into_iter()
                .map(|tab| {
                    let tab_for_click = tab.clone();
                    let tab_for_class = tab.clone();
                    let label = tab.label();
                    view! {
                        <button
                            class="tab-button"
                            class:active=move || active_tab.get() == tab_for_class
                            on:click=move |_| set_active_tab.set(tab_for_click.clone())
                        >
                            {label}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
            <For
                each=move || dynamic_tabs.get()
                key=|tab| tab.key()
                children=move |tab| {
                    let tab_for_click = tab.clone();
                    let tab_for_class = tab.clone();
                    let tab_for_close = tab.clone();
                    let label = tab.display_label();
                    view! {
                        <button
                            class="tab-button"
                            class:active=move || active_tab.get() == tab_for_class
                            on:click=move |_| set_active_tab.set(tab_for_click.clone())
                        >
                            {label}
                            <span
                                class="close-btn"
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_close_dynamic.run(tab_for_close.clone());
                                }
                            >
                                "\u{00d7}"
                            </span>
                        </button>
                    }
                }
            />
        </nav>
    }
}
