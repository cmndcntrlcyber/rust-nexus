//! Engagement phase indicator for the status bar.

use leptos::prelude::*;

use super::hud_state::HudState;

#[component]
pub fn EngagementStatus(hud: RwSignal<HudState>) -> impl IntoView {
    let engagement = move || hud.get().engagement.clone();

    view! {
        {move || engagement().map(|eng| view! {
            <span class="engagement-indicator">
                <span class="eng-name">{eng.name}</span>
                <span class="eng-phase">{format!("Phase {}/8", eng.current_phase)}</span>
                <span class="eng-findings">{format!("{} findings", eng.findings_count)}</span>
                <span class="eng-agents">{format!("{} agents", eng.active_agents)}</span>
            </span>
        })}
    }
}
