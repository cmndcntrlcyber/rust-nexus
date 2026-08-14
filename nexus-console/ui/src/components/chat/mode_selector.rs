//! Sub-tab navigation for chat modes.

use leptos::prelude::*;

use super::types::ChatMode;

const MODES: &[(ChatMode, &str, &str)] = &[
    (ChatMode::OrgLlm, "Org LLM", "General-purpose AI assistant"),
    (ChatMode::Operations, "Ops", "Engagement lifecycle management"),
    (ChatMode::Harness, "Harness", "Direct skill invocation"),
    (ChatMode::Coworkers, "Coworkers", "Team chat (coming soon)"),
];

#[component]
pub fn ModeSelector(
    active: Signal<ChatMode>,
    on_select: Callback<ChatMode>,
) -> impl IntoView {
    view! {
        <div class="mode-selector">
            {MODES.iter().map(|(mode, label, tooltip)| {
                let mode = *mode;
                let is_active = move || active.get() == mode;
                let disabled = mode.is_stub();
                view! {
                    <button
                        class="mode-btn"
                        class:active=is_active
                        class:disabled=disabled
                        title=*tooltip
                        on:click=move |_| {
                            if !disabled {
                                on_select.run(mode);
                            }
                        }
                    >
                        {*label}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
