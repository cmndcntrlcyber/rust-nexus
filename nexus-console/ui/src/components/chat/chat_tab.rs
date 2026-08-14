//! Root chat tab component: mode selector + message history + input bar.

use leptos::prelude::*;

use super::chat_state::ChatStore;
use super::input_bar::InputBar;
use super::message_list::MessageList;
use super::mode_selector::ModeSelector;
use super::streaming_indicator::StreamingIndicator;
use super::types::{ChatChunkPayload, ChatMessage, ChatMode, MessageRole};

use crate::tauri_api;

#[component]
pub fn ChatTab() -> impl IntoView {
    let store = RwSignal::new(ChatStore::new());
    let active_mode = Signal::derive(move || store.get().active_mode);

    let set_mode = Callback::new(move |mode: ChatMode| {
        store.update(|s| s.active_mode = mode);
    });

    // Listen for streaming chat chunks from Tauri backend
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let store = store;
            let _unlisten = tauri_api::listen::<ChatChunkPayload, _>(
                "chat-chunk",
                move |payload| {
                    store.update(|s| {
                        let state = s.active_mut();
                        if payload.done {
                            let content =
                                std::mem::take(&mut state.stream_buffer);
                            let (model_used, latency_ms) =
                                if let Some(meta) = &payload.meta {
                                    (
                                        Some(meta.model_used.clone()),
                                        Some(meta.latency_ms),
                                    )
                                } else {
                                    (None, None)
                                };
                            if !content.is_empty() {
                                state.messages.push(ChatMessage {
                                    id: format!(
                                        "sys-{}",
                                        js_sys::Date::now() as u64
                                    ),
                                    role: MessageRole::System,
                                    content,
                                    timestamp_ms: js_sys::Date::now(),
                                    model_used,
                                    latency_ms,
                                });
                            }
                            state.is_streaming = false;
                        } else {
                            state.stream_buffer.push_str(&payload.text);
                        }
                    });
                },
            )
            .await;
        });
    });

    view! {
        <div class="chat-tab">
            <ModeSelector active=active_mode on_select=set_mode />
            <div class="chat-content">
                <MessageList store=store />
                <StreamingIndicator store=store />
                <InputBar store=store />
            </div>
        </div>
    }
}
