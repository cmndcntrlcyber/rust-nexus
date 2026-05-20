//! `RegistryLister` — adapts the overlay's `GrpcServer` agent registry to
//! the A2A `AgentLister` trait.
//!
//! Per D-V1.1-I, the A2A `peer_id` is derived from the overlay's UUID-based
//! `agent_id` via `BLAKE3(uuid.bytes())`. The mapping is one-way and stable.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use nexus_a2a::{AgentLister, RegisteredAgentInfo};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::grpc_server::AgentSession;

/// Live read-only view of the C2's agent registry, presented as an
/// [`AgentLister`] for the A2A operator-facing service.
#[derive(Clone)]
pub struct RegistryLister {
    agents: Arc<RwLock<HashMap<String, AgentSession>>>,
}

impl RegistryLister {
    /// Construct from the same `Arc<RwLock<...>>` that the overlay's
    /// `GrpcServer` uses.
    #[must_use]
    pub fn new(agents: Arc<RwLock<HashMap<String, AgentSession>>>) -> Self {
        Self { agents }
    }
}

#[async_trait]
impl AgentLister for RegistryLister {
    async fn list(&self) -> Vec<RegisteredAgentInfo> {
        let guard = self.agents.read().await;
        guard
            .values()
            .map(|session| {
                let info = &session.registration_info;
                RegisteredAgentInfo {
                    peer_id: peer_id_from_uuid_str(&session.agent_id),
                    os: os_label(&info.os_type),
                    version: info.os_version.clone(),
                    tag: info.hostname.clone(),
                    last_seen_unix: session.last_heartbeat.timestamp().max(0) as u64,
                }
            })
            .collect()
    }
}

/// Map the overlay's UUID-shaped `agent_id` to a stable 32-byte BLAKE3
/// digest. If the string isn't a valid UUID (some test fixtures may use
/// arbitrary ids), digest the bytes directly so we still produce a stable
/// id.
#[must_use]
pub fn peer_id_from_uuid_str(agent_id: &str) -> [u8; 32] {
    let bytes = Uuid::parse_str(agent_id)
        .ok()
        .map(|u| u.as_bytes().to_vec())
        .unwrap_or_else(|| agent_id.as_bytes().to_vec());
    let mut out = [0u8; 32];
    out.copy_from_slice(blake3::hash(&bytes).as_bytes());
    out
}

/// Lowercase the overlay's `os_type` (typically "Windows" / "Linux" /
/// "Darwin") so it matches the A2A schema (`"windows"`, `"linux"`,
/// `"macos"`, `"other"`).
fn os_label(s: &str) -> String {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "windows" | "linux" => lower,
        "darwin" | "macos" | "mac" | "osx" => "macos".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_is_stable_for_uuid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let a = peer_id_from_uuid_str(id);
        let b = peer_id_from_uuid_str(id);
        assert_eq!(a, b);
    }

    #[test]
    fn peer_id_falls_back_for_non_uuid() {
        let id = "non-uuid-string-12345";
        let a = peer_id_from_uuid_str(id);
        let b = peer_id_from_uuid_str(id);
        assert_eq!(a, b);
    }

    #[test]
    fn different_inputs_produce_different_peer_ids() {
        let a = peer_id_from_uuid_str("550e8400-e29b-41d4-a716-446655440000");
        let b = peer_id_from_uuid_str("00000000-0000-0000-0000-000000000000");
        assert_ne!(a, b);
    }

    #[test]
    fn os_label_normalises() {
        assert_eq!(os_label("Windows"), "windows");
        assert_eq!(os_label("Linux"), "linux");
        assert_eq!(os_label("Darwin"), "macos");
        assert_eq!(os_label("FreeBSD"), "freebsd");
    }
}
