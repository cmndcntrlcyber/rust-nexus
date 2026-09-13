//! Tab bar component for the operator console UI.
//!
//! Renders a horizontal bar of 11 fixed tab buttons (Dashboard, Transfer,
//! Mesh, Kali, Chat, Workbench, Portainer, Wiki, VS Code, Reports, Kasm)
//! plus optional dynamic tabs (Shell sessions, AuditLog, TunnelService).
//! The active tab receives an accent underline via the `active` CSS class.

use leptos::prelude::*;

/// The kind of content a tab holds.
#[derive(Clone, PartialEq)]
pub enum TabKind {
    // ── Existing fixed tabs (indices 0-4) ──
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

    // ── New fixed tabs (indices 5-10) ──
    /// ATT&CK Workbench iframe.
    Workbench,
    /// Portainer container management iframe.
    Portainer,
    /// Docmost wiki iframe.
    Wiki,
    /// KasmVNC VS Code desktop iframe.
    VsCode,
    /// SysReptor reporting iframe.
    Reports,
    /// Kasm Workspace Portal iframe.
    Kasm,

    // ── Dynamic tabs ──
    /// An interactive shell session targeting a specific agent.
    Shell {
        /// The peer id of the agent this shell connects to.
        agent_peer_id: String,
    },
    /// The audit-log viewer pane.
    AuditLog,
    /// A dynamically-opened tunnel service iframe tab (Empire, Registry).
    TunnelService {
        /// Tunnel subdomain suffix.
        suffix: String,
        /// Human-readable label.
        label: String,
        /// Full tunnel URL.
        url: String,
    },
}

impl TabKind {
    /// Human-readable label for the tab button.
    pub fn label(&self) -> &str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Transfer => "Transfer",
            Self::Mesh => "Mesh",
            Self::Kali => "Kali",
            Self::Chat => "Chat",
            Self::Workbench => "Workbench",
            Self::Portainer => "Portainer",
            Self::Wiki => "Wiki",
            Self::VsCode => "VS Code",
            Self::Reports => "Reports",
            Self::Kasm => "Kasm",
            Self::Shell { .. } => "Shell",
            Self::AuditLog => "Audit Log",
            Self::TunnelService { label, .. } => label,
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
            Self::Workbench => 5,
            Self::Portainer => 6,
            Self::Wiki => 7,
            Self::VsCode => 8,
            Self::Reports => 9,
            Self::Kasm => 10,
            Self::Shell { .. } => 11,
            Self::AuditLog => 12,
            Self::TunnelService { .. } => 13,
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
            Self::Workbench => "workbench".into(),
            Self::Portainer => "portainer".into(),
            Self::Wiki => "wiki".into(),
            Self::VsCode => "vscode".into(),
            Self::Reports => "reports".into(),
            Self::Kasm => "kasm".into(),
            Self::Shell { agent_peer_id } => format!("shell-{agent_peer_id}"),
            Self::AuditLog => "audit-log".into(),
            Self::TunnelService { suffix, .. } => format!("tunnel{suffix}"),
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

    /// Tunnel subdomain suffix mapped to this tab, if any.
    pub fn tunnel_suffix(&self) -> Option<&'static str> {
        match self {
            Self::Dashboard => Some("-admin"),
            Self::Kali => Some("-kali"),
            Self::Workbench => Some("-workbench"),
            Self::Portainer => Some("-mgmt"),
            Self::Wiki => Some("-wiki"),
            Self::VsCode => Some("-vscode"),
            Self::Reports => Some("-reports"),
            Self::Kasm => Some("-kasm"),
            _ => None,
        }
    }
}

/// Horizontal tab bar rendered at the top of the connected layout.
///
/// Renders 11 fixed tab buttons plus any dynamic tabs passed via the
/// `dynamic_tabs` signal. The active tab receives the `active` CSS
/// class. Dynamic tabs include a close button. Fixed tabs with a
/// tunnel mapping show a green badge dot.
#[component]
pub fn TabBar(
    /// Signal providing the currently active tab.
    active_tab: ReadSignal<TabKind>,
    /// Setter invoked when a tab is clicked.
    set_active_tab: WriteSignal<TabKind>,
    /// Dynamic tabs beyond the 11 fixed tabs.
    dynamic_tabs: ReadSignal<Vec<TabKind>>,
    /// Callback invoked when a dynamic tab's close button is clicked.
    on_close_dynamic: Callback<TabKind>,
    /// Tunnel services (empty when RTPI_SLUG/RTPI_DOMAIN not set).
    tunnel_services: ReadSignal<Vec<TunnelServiceInfo>>,
) -> impl IntoView {
    let fixed = vec![
        TabKind::Dashboard,
        TabKind::Transfer,
        TabKind::Mesh,
        TabKind::Kali,
        TabKind::Chat,
        TabKind::Workbench,
        TabKind::Portainer,
        TabKind::Wiki,
        TabKind::VsCode,
        TabKind::Reports,
        TabKind::Kasm,
    ];

    view! {
        <nav class="tab-bar">
            {fixed
                .into_iter()
                .map(|tab| {
                    let tab_for_click = tab.clone();
                    let tab_for_class = tab.clone();
                    let label = tab.label().to_string();
                    let suffix = tab.tunnel_suffix().map(String::from);
                    view! {
                        <button
                            class="tab-button"
                            class:active=move || active_tab.get() == tab_for_class
                            on:click=move |_| set_active_tab.set(tab_for_click.clone())
                        >
                            {label}
                            {move || {
                                let suffix = suffix.clone();
                                suffix.and_then(|sfx| {
                                    let services = tunnel_services.get();
                                    services.iter().find(|s| s.suffix == sfx).map(|s| {
                                        let url = s.url.clone();
                                        let label = s.label.clone();
                                        view! { <TunnelBadge url=url label=label /> }
                                    })
                                })
                            }}
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

/// Minimal tunnel service info used by the UI (deserialized from Tauri).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct TunnelServiceInfo {
    /// Tunnel subdomain suffix (e.g. "-admin").
    pub suffix: String,
    /// Human-readable label.
    pub label: String,
    /// Computed URL.
    pub url: String,
    /// Fixed tab mapping key, if any.
    pub tab_mapping: Option<String>,
    /// Whether the service can be embedded in an iframe.
    pub embeddable: bool,
}

/// Small green dot rendered next to a tab label when the tab has a
/// tunnel service mapping. Clicking the dot opens the URL in the
/// system browser.
#[component]
fn TunnelBadge(url: String, label: String) -> impl IntoView {
    let url_for_click = url.clone();
    view! {
        <span
            class="tunnel-badge"
            title=format!("{label} \u{2014} {url}")
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                let url = url_for_click.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = crate::tauri_api::open_tunnel_url(&url).await;
                });
            }
        >
            <span class="badge-dot active"></span>
        </span>
    }
}
