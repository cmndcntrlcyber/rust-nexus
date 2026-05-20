//! Left-pane agent list.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen_futures::JsFuture;

use crate::tauri_api;
use crate::types::AgentInfo;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[component]
pub fn AgentList(
    /// Current selection.
    selected: ReadSignal<Option<String>>,
    /// Setter for selection.
    set_selected: WriteSignal<Option<String>>,
    /// Reactive count for status bar.
    set_agent_count: WriteSignal<usize>,
) -> impl IntoView {
    let (agents, set_agents) = signal::<Vec<AgentInfo>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    Effect::new(move |_| {
        spawn_local(async move {
            loop {
                match tauri_api::invoke_no_args::<Vec<AgentInfo>>("list_agents").await {
                    Ok(list) => {
                        set_agent_count.set(list.len());
                        set_agents.set(list);
                        set_error.set(None);
                    }
                    Err(err) => set_error.set(Some(err)),
                }
                sleep(POLL_INTERVAL).await;
            }
        });
    });

    view! {
        <aside class="agent-list">
            <h2>"Agents"</h2>
            <p class="agent-count">{move || format!("{} registered", agents.with(Vec::len))}</p>
            <ul>
                <For
                    each=move || agents.get()
                    key=|a| a.peer_id.clone()
                    children=move |agent| {
                        let agent_for_click = agent.clone();
                        let peer = agent.peer_id.clone();
                        let selected = selected;
                        let is_selected = move || selected.get().as_deref() == Some(&peer);
                        view! {
                            <li
                                class:selected=is_selected
                                on:click=move |_| {
                                    set_selected.set(Some(agent_for_click.peer_id.clone()));
                                }
                            >
                                <span class="tag">{agent.tag.clone()}</span>
                                <span class="os">{agent.os.clone()}</span>
                                <span class="peer">{shorten(&agent.peer_id)}</span>
                            </li>
                        }
                    }
                />
            </ul>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
        </aside>
    }
}

fn shorten(hex: &str) -> String {
    if hex.len() > 10 {
        format!("{}…{}", &hex[..6], &hex[hex.len() - 4..])
    } else {
        hex.to_string()
    }
}

async fn sleep(d: Duration) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                i32::try_from(d.as_millis()).unwrap_or(i32::MAX),
            );
        }
    });
    let _ = JsFuture::from(promise).await;
}
