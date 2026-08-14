//! Individual chat message rendering.

use leptos::prelude::*;

use super::types::{ChatMessage, MessageRole};

#[component]
pub fn MessageBubble(message: ChatMessage) -> impl IntoView {
    let is_operator = message.role == MessageRole::Operator;
    let role_label = if is_operator { "You" } else { "Nexus" };

    view! {
        <div class="message-bubble" class:operator=is_operator class:system=!is_operator>
            <div class="message-header">
                <span class="role-label">{role_label}</span>
            </div>
            <div class="message-content">
                <pre class="message-text">{message.content.clone()}</pre>
            </div>
            {message.model_used.clone().map(|model| {
                let latency = message.latency_ms.map(|ms| format!("{ms:.0}ms")).unwrap_or_default();
                view! {
                    <div class="message-meta">
                        <span class="model">{model}</span>
                        <span class="latency">{latency}</span>
                    </div>
                }
            })}
        </div>
    }
}
