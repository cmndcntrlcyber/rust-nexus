//! Gossipsub topic naming conventions.
//!
//! Stable strings; bumping the format here is a breaking change across the
//! mesh.
//!
//! - `nexus/op/{op_id}` — broadcast within an operation
//! - `nexus/agent/{hex}/inbox` — direct to a specific agent
//! - `nexus/server/inbox` — to the C2
//! - `nexus/heartbeat` — global presence
//!
//! v1.3 adds [`Role`] — used by `MeshNode::subscribe_role` (Phase 1.3.3)
//! so the C2 server, agents, and operators each subscribe to the
//! topics they need.

use libp2p::gossipsub::IdentTopic;

/// Broadcast topic for an operation (e.g. a campaign id).
#[must_use]
pub fn op_broadcast(op_id: &str) -> IdentTopic {
    IdentTopic::new(format!("nexus/op/{op_id}"))
}

/// Per-agent inbox. `peer_id` is the 32-byte BLAKE3 peer id (NOT the libp2p
/// PeerId).
#[must_use]
pub fn agent_inbox(peer_id: &[u8; 32]) -> IdentTopic {
    IdentTopic::new(format!("nexus/agent/{}/inbox", hex_lower(peer_id)))
}

/// C2 server's inbox.
#[must_use]
pub fn server_inbox() -> IdentTopic {
    IdentTopic::new("nexus/server/inbox")
}

/// Global presence heartbeat.
#[must_use]
pub fn heartbeat() -> IdentTopic {
    IdentTopic::new("nexus/heartbeat")
}

/// v1.3 — mesh participant role tag (Phase 1.3.3).
///
/// `MeshNode::subscribe_role(role)` subscribes to the appropriate
/// inbound topics for the role and refuses to subscribe to topics
/// reserved for other roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// C2 server endpoint. Subscribes to `server_inbox()` + `heartbeat()`.
    C2,
    /// Endpoint agent. Subscribes to its own `agent_inbox(peer_id)` +
    /// `heartbeat()`. Publishes shell-session output to `server_inbox()`.
    Agent,
    /// Operator. Publishes shell-session control to `server_inbox()`;
    /// subscribes to per-session response topics dispatched by the C2.
    Operator,
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_names_are_stable() {
        assert_eq!(server_inbox().to_string(), "nexus/server/inbox");
        assert_eq!(heartbeat().to_string(), "nexus/heartbeat");
        assert_eq!(op_broadcast("op-42").to_string(), "nexus/op/op-42");
        let peer = [0xABu8; 32];
        assert_eq!(
            agent_inbox(&peer).to_string(),
            format!("nexus/agent/{}/inbox", "ab".repeat(32))
        );
    }
}
