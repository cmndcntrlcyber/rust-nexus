//! HITL approval queue panel.

use leptos::prelude::*;

use super::hud_state::HudState;
use crate::tauri_api;

#[component]
pub fn ApprovalQueue(
    hud: RwSignal<HudState>,
    on_close: Callback<()>,
) -> impl IntoView {
    let pending = move || hud.get().pending_approvals.clone();

    view! {
        <div class="approval-queue slide-over">
            <div class="queue-header">
                <h3>"Pending Approvals"</h3>
                <span class="count">{move || format!("{} pending", pending().len())}</span>
                <button class="close-btn" on:click=move |_| on_close.run(())>"×"</button>
            </div>
            <div class="queue-list">
                {move || pending().into_iter().map(|req| {
                    let risk_class = match req.risk_level.as_str() {
                        "critical" => "risk-critical",
                        "high" => "risk-high",
                        "medium" => "risk-medium",
                        _ => "risk-low",
                    };
                    let skill = req.skill_name.clone();
                    let target = req.target.clone();
                    let agent = req.requesting_agent.clone();
                    let desc = req.description.clone();
                    let techniques = req.techniques.join(", ");
                    let approval_id = req.approval_id.clone();
                    let approval_id_deny = approval_id.clone();

                    view! {
                        <div class=format!("approval-card {risk_class}")>
                            <div class="card-header">
                                <span class="risk-badge">{req.risk_level.clone()}</span>
                                <span class="skill-name">{skill}</span>
                            </div>
                            <div class="card-body">
                                <div class="field"><label>"Target"</label><span>{target}</span></div>
                                <div class="field"><label>"Agent"</label><span>{agent}</span></div>
                                <div class="field"><label>"Description"</label><p>{desc}</p></div>
                                {(!techniques.is_empty()).then(|| view! {
                                    <div class="field"><label>"Techniques"</label><span>{techniques}</span></div>
                                })}
                            </div>
                            <div class="card-actions">
                                <button class="approve-btn" on:click={
                                    let id = approval_id.clone();
                                    move |_| {
                                        let id = id.clone();
                                        leptos::task::spawn_local(async move {
                                            let _ = tauri_api::invoke::<_, ()>(
                                                "submit_approval",
                                                &serde_json::json!({
                                                    "approvalId": id,
                                                    "approved": true,
                                                    "reason": "",
                                                }),
                                            ).await;
                                        });
                                    }
                                }>"Approve"</button>
                                <button class="deny-btn" on:click={
                                    let id = approval_id_deny;
                                    move |_| {
                                        let id = id.clone();
                                        leptos::task::spawn_local(async move {
                                            let _ = tauri_api::invoke::<_, ()>(
                                                "submit_approval",
                                                &serde_json::json!({
                                                    "approvalId": id,
                                                    "approved": false,
                                                    "reason": "Operator denied",
                                                }),
                                            ).await;
                                        });
                                    }
                                }>"Deny"</button>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
