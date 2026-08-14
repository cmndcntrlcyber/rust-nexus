//! Chat input bar with send button.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::chat_state::ChatStore;
use super::types::{ChatMessage, MessageRole};
use crate::tauri_api;

#[component]
pub fn InputBar(store: RwSignal<ChatStore>) -> impl IntoView {
    let (input, set_input) = signal(String::new());
    let is_streaming = move || store.get().active().is_streaming;

    let send = move || {
        let content = input.get();
        if content.trim().is_empty() {
            return;
        }

        let mode = store.get_untracked().active_mode;
        let session_id = store.with_untracked(|s| s.active().session_id.clone());

        store.update(|s| {
            let state = s.active_mut();
            state.messages.push(ChatMessage {
                id: format!("op-{}", js_sys::Date::now() as u64),
                role: MessageRole::Operator,
                content: content.clone(),
                timestamp_ms: js_sys::Date::now(),
                model_used: None,
                latency_ms: None,
            });
            state.is_streaming = true;
        });

        let mode_str = mode.wire_name().to_string();
        let content_clone = content.clone();
        leptos::task::spawn_local(async move {
            let args = serde_json::json!({
                "mode": mode_str,
                "content": content_clone,
                "sessionId": session_id,
            });
            let _ = tauri_api::invoke::<_, String>("send_chat_message", &args).await;
        });

        set_input.set(String::new());
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && ev.ctrl_key() {
            ev.prevent_default();
            send();
        }
    };

    let on_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.unchecked_into();
        set_input.set(textarea.value());
    };

    view! {
        <div class="input-bar">
            <textarea
                class="chat-input"
                placeholder="Type a message or /command... (Ctrl+Enter to send)"
                prop:value=input
                prop:disabled=is_streaming
                on:input=on_input
                on:keydown=on_keydown
            />
            <button
                class="send-btn"
                prop:disabled=is_streaming
                on:click=move |_| send()
            >
                "Send"
            </button>
        </div>
    }
}
