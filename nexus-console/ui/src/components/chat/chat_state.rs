//! Per-mode reactive state for chat sessions.

use std::collections::HashMap;

use super::types::{ChatMessage, ChatMode};

#[derive(Clone, Debug)]
pub struct ChatModeState {
    pub mode: ChatMode,
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
    pub stream_buffer: String,
    pub input_draft: String,
}

impl ChatModeState {
    pub fn new(mode: ChatMode) -> Self {
        let session_id = format!("{:?}-{}", mode, js_sys::Date::now() as u64);
        Self {
            mode,
            session_id,
            messages: Vec::new(),
            is_streaming: false,
            stream_buffer: String::new(),
            input_draft: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatStore {
    pub modes: HashMap<ChatMode, ChatModeState>,
    pub active_mode: ChatMode,
}

impl ChatStore {
    pub fn new() -> Self {
        let mut modes = HashMap::new();
        for mode in [
            ChatMode::OrgLlm,
            ChatMode::Operations,
            ChatMode::Harness,
            ChatMode::Coworkers,
        ] {
            modes.insert(mode, ChatModeState::new(mode));
        }
        Self {
            modes,
            active_mode: ChatMode::Harness,
        }
    }

    pub fn active(&self) -> &ChatModeState {
        &self.modes[&self.active_mode]
    }

    pub fn active_mut(&mut self) -> &mut ChatModeState {
        self.modes.get_mut(&self.active_mode).unwrap()
    }
}
