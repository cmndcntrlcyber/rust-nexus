//! Mid-task steering controls for live agent intervention.

use leptos::prelude::*;

use crate::tauri_api;

#[component]
pub fn SteeringPanel(agent_id: String) -> impl IntoView {
    let (steer_result, set_steer_result) = signal::<Option<String>>(None);
    let (redirect_input, set_redirect_input) = signal(String::new());

    let do_steer = {
        let agent_id = agent_id.clone();
        move |action: &'static str| {
            let agent_id = agent_id.clone();
            let instruction = redirect_input.get_untracked();
            leptos::task::spawn_local(async move {
                let args = serde_json::json!({
                    "agentId": agent_id,
                    "taskId": "",
                    "action": action,
                    "instruction": instruction,
                });
                match tauri_api::invoke::<_, serde_json::Value>("steer_agent", &args).await {
                    Ok(resp) => {
                        let msg = resp
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("OK")
                            .to_string();
                        set_steer_result.set(Some(msg));
                    }
                    Err(e) => set_steer_result.set(Some(format!("Error: {e}"))),
                }
            });
        }
    };

    let on_redirect_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement =
            wasm_bindgen::JsCast::unchecked_into(target);
        set_redirect_input.set(textarea.value());
    };

    view! {
        <div class="steering-panel">
            <h4>"Agent Control"</h4>
            <div class="agent-id">{agent_id}</div>

            <div class="steer-actions">
                <button class="steer-btn pause"
                    on:click={let s = do_steer.clone(); move |_| s("pause")}
                >"⏸ Pause"</button>
                <button class="steer-btn resume"
                    on:click={let s = do_steer.clone(); move |_| s("resume")}
                >"▶ Resume"</button>
                <button class="steer-btn kill"
                    on:click={let s = do_steer.clone(); move |_| s("kill")}
                >"⏹ Kill"</button>
            </div>

            <div class="steer-redirect">
                <textarea
                    placeholder="Redirect instruction..."
                    prop:value=redirect_input
                    on:input=on_redirect_input
                />
                <button class="steer-btn redirect"
                    on:click={let s = do_steer.clone(); move |_| s("redirect")}
                >"↻ Redirect"</button>
            </div>

            {move || steer_result.get().map(|msg| view! {
                <div class="steer-result">{msg}</div>
            })}
        </div>
    }
}
