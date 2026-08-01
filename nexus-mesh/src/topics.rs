//! Gossipsub topic naming conventions.
//!
//! Stable strings; bumping the format here is a breaking change across the
//! mesh.
//!
//! - `nexus/op/{op_id}` — broadcast within an operation
//! - `nexus/agent/{hex}/inbox` — direct to a specific agent
//! - `nexus/server/inbox` — to the C2
//! - `nexus/heartbeat` — global presence

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

/// Per-harness task inbox. `peer_id` is the 32-byte BLAKE3 peer id.
#[must_use]
pub fn harness_task_inbox(peer_id: &[u8; 32]) -> IdentTopic {
    IdentTopic::new(format!("nexus/harness/{}/task", hex_lower(peer_id)))
}

/// Harness results aggregation topic.
#[must_use]
pub fn harness_result_inbox() -> IdentTopic {
    IdentTopic::new("nexus/harness/results")
}

/// Telemetry snapshot broadcast.
#[must_use]
pub fn telemetry_snapshot() -> IdentTopic {
    IdentTopic::new("nexus/telemetry/snapshot")
}

/// Swarm coordination broadcast.
#[must_use]
pub fn swarm_coordinate() -> IdentTopic {
    IdentTopic::new("nexus/swarm/coordinate")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    C2,
    Agent,
    Operator,
    Harness,
}

impl Role {
    #[must_use]
    pub fn subscriptions(&self, local_peer_id: &[u8; 32]) -> Vec<IdentTopic> {
        match self {
            Role::C2 => vec![server_inbox(), heartbeat(), swarm_coordinate()],
            Role::Agent => vec![
                agent_inbox(local_peer_id),
                harness_task_inbox(local_peer_id),
                heartbeat(),
            ],
            Role::Operator => vec![heartbeat(), swarm_coordinate()],
            Role::Harness => vec![
                harness_result_inbox(),
                heartbeat(),
                telemetry_snapshot(),
                swarm_coordinate(),
            ],
        }
    }
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

    #[test]
    fn harness_topic_names_are_stable() {
        let peer = [0xABu8; 32];
        assert_eq!(
            harness_task_inbox(&peer).to_string(),
            format!("nexus/harness/{}/task", "ab".repeat(32))
        );
        assert_eq!(
            harness_result_inbox().to_string(),
            "nexus/harness/results"
        );
        assert_eq!(telemetry_snapshot().to_string(), "nexus/telemetry/snapshot");
        assert_eq!(swarm_coordinate().to_string(), "nexus/swarm/coordinate");
    }

    #[test]
    fn harness_role_subscriptions() {
        let peer = [0x01u8; 32];
        let subs = Role::Harness.subscriptions(&peer);
        let names: Vec<String> = subs.iter().map(|t| t.to_string()).collect();
        assert_eq!(
            names,
            vec![
                "nexus/harness/results",
                "nexus/heartbeat",
                "nexus/telemetry/snapshot",
                "nexus/swarm/coordinate",
            ]
        );
    }

    #[test]
    fn agent_role_includes_harness_task() {
        let peer = [0xCDu8; 32];
        let subs = Role::Agent.subscriptions(&peer);
        let names: Vec<String> = subs.iter().map(|t| t.to_string()).collect();
        assert!(names.contains(&format!("nexus/harness/{}/task", "cd".repeat(32))));
    }
}
