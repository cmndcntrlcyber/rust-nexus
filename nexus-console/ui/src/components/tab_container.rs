//! Tab container component for the operator console UI.

use leptos::prelude::*;

/// The kind of content a tab holds.
#[derive(Clone, PartialEq)]
pub enum TabKind {
    /// An interactive shell session targeting a specific agent.
    Shell {
        /// The peer id of the agent this shell connects to.
        agent_peer_id: String,
    },
    /// The audit-log viewer pane.
    AuditLog,
}

/// Descriptor for a single tab in the container.
#[derive(Clone)]
pub struct TabDescriptor {
    /// Unique tab identifier.
    pub id: u64,
    /// Human-readable label shown on the tab bar.
    pub label: String,
    /// What kind of content this tab renders.
    pub kind: TabKind,
}

/// A tabbed container that renders a horizontal tab bar with clickable
/// labels and delegates content rendering to the caller.
///
/// # Props
/// - `tabs` -- signal providing the current list of tab descriptors.
/// - `active_tab` -- signal providing the id of the currently-selected
///   tab, or `None` when no tab is active.
/// - `on_select` -- callback invoked when the user clicks a tab label.
#[component]
pub fn TabContainer(
    tabs: Signal<Vec<TabDescriptor>>,
    active_tab: Signal<Option<u64>>,
    on_select: Callback<u64>,
) -> impl IntoView {
    view! {
        <div class="tab-container">
            <div class="tab-bar">
                <For
                    each=move || tabs.get()
                    key=|tab| tab.id
                    let(tab)
                >
                    {
                        let tab_id = tab.id;
                        let label = tab.label.clone();
                        let is_active = move || {
                            active_tab.get().map(|id| id == tab_id).unwrap_or(false)
                        };
                        view! {
                            <button
                                class="tab-label"
                                class:active=is_active
                                on:click=move |_| on_select.run(tab_id)
                            >
                                {label}
                            </button>
                        }
                    }
                </For>
            </div>
            <div class="tab-content">
                {move || {
                    if tabs.get().is_empty() {
                        Some(view! { <p class="no-tabs">"No tabs open"</p> }.into_any())
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}
