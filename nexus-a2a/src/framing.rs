//! Shell-session control-frame framing.
//!
//! Multiplexes two payload types over the same A2A `Part`:
//!
//! - **Raw bytes** travel in `Part::file`. Operator → agent: PTY stdin.
//!   Agent → operator: PTY stdout chunks.
//! - **Control frames** travel in `Part::text` as JSON. The `kind` field
//!   discriminates between `shell-open`, `shell-resize`, and `shell-exit`.

use serde::{Deserialize, Serialize};

use crate::pb;

/// String value of the `kind` field for shell-open control frames.
pub const SHELL_OPEN_KIND: &str = "shell-open";
/// String value of the `kind` field for shell-resize control frames.
pub const SHELL_RESIZE_KIND: &str = "shell-resize";
/// String value of the `kind` field for shell-exit control frames.
pub const SHELL_EXIT_KIND: &str = "shell-exit";
/// String value of the `kind` field for agent-register control frames (v1.2).
pub const AGENT_REGISTER_KIND: &str = "agent-register";

/// Control-frame payloads carried inside a `Part::text` slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ShellControl {
    /// First frame of a session. Sent operator → agent.
    ShellOpen {
        /// Initial terminal columns.
        cols: u16,
        /// Initial terminal rows.
        rows: u16,
        /// Optional shell override.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shell: Option<String>,
        /// Optional target-agent selector (hex peer id).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_agent_id: Option<String>,
    },
    /// Operator-initiated terminal resize.
    ShellResize {
        /// New columns.
        cols: u16,
        /// New rows.
        rows: u16,
    },
    /// Agent → operator: shell process exited.
    ShellExit {
        /// Exit code, or `None` when there isn't one (signal, kill).
        code: Option<i32>,
    },
    /// v1.2: first frame an agent sends when dialing the C2 in A2A mode.
    /// Identifies the agent for routing — the C2 stores the bidi back-channel
    /// in `AgentChannels` keyed by `peer_id_hex` so operator sessions can be
    /// proxied to this agent.
    AgentRegister {
        /// Hex-encoded 32-byte peer id (see [`nexus_common::PeerId`]).
        peer_id_hex: String,
        /// OS label (e.g. "linux", "windows", "macos") for operator display.
        os: String,
        /// Agent build version, free-form.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        /// Optional human-readable tag for operator-side display.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
    },
}

impl ShellControl {
    /// Encode as the `text` payload of a [`pb::Part`].
    pub fn to_text_part(&self) -> Result<pb::Part, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(pb::Part {
            part: Some(pb::part::Part::Text(json)),
        })
    }

    /// Parse a control frame out of a [`pb::Part`].
    pub fn try_from_part(part: &pb::Part) -> Result<Option<Self>, serde_json::Error> {
        match part.part.as_ref() {
            Some(pb::part::Part::Text(text)) if !text.is_empty() => {
                let control = serde_json::from_str(text)?;
                Ok(Some(control))
            }
            _ => Ok(None),
        }
    }
}

/// Build a streaming response carrying a single `Part::file`.
#[must_use]
pub fn bytes_response(role: &str, task_id: &str, bytes: Vec<u8>) -> pb::StreamResponse {
    pb::StreamResponse {
        payload: Some(pb::stream_response::Payload::Message(pb::Message {
            message_id: String::new(),
            role: role.to_string(),
            parts: vec![pb::Part {
                part: Some(pb::part::Part::File(bytes)),
            }],
            task_id: task_id.to_string(),
        })),
    }
}

/// Build a streaming response carrying a control frame.
pub fn control_response(
    role: &str,
    task_id: &str,
    control: &ShellControl,
) -> Result<pb::StreamResponse, serde_json::Error> {
    let part = control.to_text_part()?;
    Ok(pb::StreamResponse {
        payload: Some(pb::stream_response::Payload::Message(pb::Message {
            message_id: String::new(),
            role: role.to_string(),
            parts: vec![part],
            task_id: task_id.to_string(),
        })),
    })
}

/// Build an inbound message carrying raw bytes.
#[must_use]
pub fn bytes_request(task_id: &str, bytes: Vec<u8>) -> pb::Message {
    pb::Message {
        message_id: String::new(),
        role: "user".to_string(),
        parts: vec![pb::Part {
            part: Some(pb::part::Part::File(bytes)),
        }],
        task_id: task_id.to_string(),
    }
}

/// Build an inbound message carrying a control frame.
pub fn control_request(
    task_id: &str,
    control: &ShellControl,
) -> Result<pb::Message, serde_json::Error> {
    Ok(pb::Message {
        message_id: String::new(),
        role: "user".to_string(),
        parts: vec![control.to_text_part()?],
        task_id: task_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_open_round_trip() {
        let frame = ShellControl::ShellOpen {
            cols: 80,
            rows: 24,
            shell: Some("powershell".into()),
            target_agent_id: None,
        };
        let part = frame.to_text_part().expect("encode");
        let back = ShellControl::try_from_part(&part).expect("decode");
        assert_eq!(back, Some(frame));
    }

    #[test]
    fn shell_open_omits_null_fields() {
        let frame = ShellControl::ShellOpen {
            cols: 80,
            rows: 24,
            shell: None,
            target_agent_id: None,
        };
        let json = serde_json::to_string(&frame).expect("ser");
        assert_eq!(json, r#"{"kind":"shell-open","cols":80,"rows":24}"#);
    }

    #[test]
    fn shell_exit_round_trip() {
        let frame = ShellControl::ShellExit { code: Some(0) };
        let part = frame.to_text_part().expect("encode");
        let back = ShellControl::try_from_part(&part).expect("decode");
        assert_eq!(back, Some(frame));
    }

    #[test]
    fn try_from_part_returns_none_for_bytes() {
        let part = pb::Part {
            part: Some(pb::part::Part::File(b"raw bytes".to_vec())),
        };
        assert!(ShellControl::try_from_part(&part).expect("ok").is_none());
    }
}
