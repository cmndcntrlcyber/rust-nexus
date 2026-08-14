//! Scrollable message history with auto-scroll.

use leptos::prelude::*;

use super::chat_state::ChatStore;
use super::message_bubble::MessageBubble;

#[component]
pub fn MessageList(store: RwSignal<ChatStore>) -> impl IntoView {
    let messages = move || store.get().active().messages.clone();

    view! {
        <div class="message-list">
            {move || {
                let msgs = messages();
                if msgs.is_empty() {
                    view! {
                        <div class="empty-chat">
                            <p>"Start a conversation..."</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="messages-container">
                            {msgs.into_iter().map(|msg| {
                                view! { <MessageBubble message=msg /> }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
