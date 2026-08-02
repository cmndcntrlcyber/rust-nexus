//! Situational awareness — live view of connected agents, their
//! capabilities, task load, and health (WS1 Phase 1c).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::debug;

/// Snapshot of a connected agent's state.
#[derive(Debug, Clone)]
pub struct AgentSnapshot {
    /// 32-byte BLAKE3 peer id (hex-encoded for display).
    pub peer_id_hex: String,
    /// OS label.
    pub os: String,
    /// Agent build version.
    pub version: String,
    /// Operator-assigned tag.
    pub tag: String,
    /// Unix seconds at last heartbeat/activity.
    pub last_seen_unix: u64,
    /// Number of tasks currently executing on this agent.
    pub active_task_count: u32,
    /// Total tasks executed since registration.
    pub total_tasks_executed: u64,
    /// Error count since registration.
    pub total_errors: u64,
    /// Nexus-harness skill names this agent can execute (from installed tools).
    pub harness_skills: Vec<String>,
    /// AttackTechnique IDs registered on this agent.
    pub technique_ids: Vec<String>,
}

/// Maintains a live view of all connected agents for the operator console
/// and swarm coordination.
pub struct SituationalAwareness {
    agents: Mutex<HashMap<String, AgentSnapshot>>,
}

impl SituationalAwareness {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
        }
    }

    /// Register or update an agent's snapshot.
    pub async fn upsert_agent(&self, snapshot: AgentSnapshot) {
        let peer_id = snapshot.peer_id_hex.clone();
        debug!(peer = %peer_id, "situational awareness: agent upsert");
        self.agents.lock().await.insert(peer_id, snapshot);
    }

    /// Remove an agent (disconnected).
    pub async fn remove_agent(&self, peer_id_hex: &str) {
        debug!(peer = %peer_id_hex, "situational awareness: agent removed");
        self.agents.lock().await.remove(peer_id_hex);
    }

    /// Update heartbeat timestamp.
    pub async fn heartbeat(&self, peer_id_hex: &str, unix_ts: u64) {
        if let Some(agent) = self.agents.lock().await.get_mut(peer_id_hex) {
            agent.last_seen_unix = unix_ts;
        }
    }

    /// Increment active task count (task dispatched to agent).
    pub async fn task_started(&self, peer_id_hex: &str) {
        if let Some(agent) = self.agents.lock().await.get_mut(peer_id_hex) {
            agent.active_task_count += 1;
            agent.total_tasks_executed += 1;
        }
    }

    /// Decrement active task count (task completed or failed).
    pub async fn task_finished(&self, peer_id_hex: &str, error: bool) {
        if let Some(agent) = self.agents.lock().await.get_mut(peer_id_hex) {
            agent.active_task_count = agent.active_task_count.saturating_sub(1);
            if error {
                agent.total_errors += 1;
            }
        }
    }

    /// Snapshot of all currently connected agents.
    pub async fn all_agents(&self) -> Vec<AgentSnapshot> {
        self.agents.lock().await.values().cloned().collect()
    }

    /// Count of connected agents.
    pub async fn agent_count(&self) -> usize {
        self.agents.lock().await.len()
    }

    /// Find agents that can execute a specific harness skill.
    pub async fn agents_with_skill(&self, skill_name: &str) -> Vec<AgentSnapshot> {
        self.agents
            .lock()
            .await
            .values()
            .filter(|a| a.harness_skills.iter().any(|s| s == skill_name))
            .cloned()
            .collect()
    }

    /// Find agents that implement a specific AttackTechnique.
    pub async fn agents_with_technique(&self, technique_id: &str) -> Vec<AgentSnapshot> {
        self.agents
            .lock()
            .await
            .values()
            .filter(|a| a.technique_ids.iter().any(|t| t == technique_id))
            .cloned()
            .collect()
    }
}

impl Default for SituationalAwareness {
    fn default() -> Self {
        Self::new()
    }
}
