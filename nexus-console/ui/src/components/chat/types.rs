//! Chat-specific types mirroring the v1.7 proto messages.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ChatMode {
    OrgLlm,
    Operations,
    Harness,
    Coworkers,
}

impl ChatMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OrgLlm => "Org LLM",
            Self::Operations => "Ops",
            Self::Harness => "Harness",
            Self::Coworkers => "Coworkers",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::OrgLlm => "General-purpose AI assistant",
            Self::Operations => "Engagement lifecycle management",
            Self::Harness => "Direct skill invocation",
            Self::Coworkers => "Team chat (coming soon)",
        }
    }

    pub fn wire_name(&self) -> &'static str {
        match self {
            Self::OrgLlm => "org_llm",
            Self::Operations => "operations",
            Self::Harness => "harness",
            Self::Coworkers => "coworkers",
        }
    }

    pub fn is_stub(&self) -> bool {
        matches!(self, Self::Coworkers)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum MessageRole {
    Operator,
    System,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp_ms: f64,
    pub model_used: Option<String>,
    pub latency_ms: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ChatChunkPayload {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub session_id: String,
    pub meta: Option<ChatChunkMetaPayload>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ChatChunkMetaPayload {
    #[serde(default)]
    pub model_used: String,
    #[serde(default)]
    pub latency_ms: f32,
}
