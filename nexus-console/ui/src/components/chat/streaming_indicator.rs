//! Animated streaming indicator shown while LLM response is in-flight.

use leptos::prelude::*;

use super::chat_state::ChatStore;

#[component]
pub fn StreamingIndicator(store: RwSignal<ChatStore>) -> impl IntoView {
    let is_streaming = move || store.get().active().is_streaming;
    let buffer_text = move || store.get().active().stream_buffer.clone();

    view! {
        {move || {
            if is_streaming() {
                let text = buffer_text();
                Some(view! {
                    <div class="streaming-indicator">
                        {if !text.is_empty() {
                            Some(view! {
                                <div class="stream-preview">
                                    <span class="role-label">"Nexus"</span>
                                    <pre class="stream-text">{text}</pre>
                                </div>
                            })
                        } else {
                            None
                        }}
                        <div class="typing-dots">
                            <span class="dot" />
                            <span class="dot" />
                            <span class="dot" />
                        </div>
                    </div>
                })
            } else {
                None
            }
        }}
    }
}
