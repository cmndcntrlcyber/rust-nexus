//! Swarm coordination handler — aggregates `SwarmCoordinate` signals
//! from distributed nexus-harness instances via `BroadcastSwarmState`
//! bidi streaming RPC (WS1 Phase 1c).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info};

use crate::pb;

/// Per-round vote tally for a consensus round.
#[derive(Debug, Default)]
struct RoundState {
    votes: Vec<pb::SwarmCoordinate>,
}

/// Server-side aggregator for distributed swarm consensus.
///
/// Multiple nexus-harness instances exchange `SwarmCoordinate` signals
/// through the nexus-a2a control plane. The coordinator maintains
/// per-round vote tallies and broadcasts merged state to all subscribers.
pub struct SwarmCoordinator {
    rounds: Mutex<HashMap<String, RoundState>>,
    broadcast_tx: broadcast::Sender<pb::SwarmCoordinate>,
}

impl SwarmCoordinator {
    /// Create a new coordinator with the given broadcast channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            rounds: Mutex::new(HashMap::new()),
            broadcast_tx: tx,
        }
    }

    /// Record an incoming vote and broadcast it to all subscribers.
    pub async fn record_vote(&self, coord: pb::SwarmCoordinate) {
        let round_id = coord.round_id.clone();

        {
            let mut rounds = self.rounds.lock().await;
            let state = rounds.entry(round_id.clone()).or_default();
            state.votes.push(coord.clone());
            debug!(
                round = %round_id,
                triad = %coord.triad_id,
                total_votes = state.votes.len(),
                "swarm coordinator: vote recorded"
            );
        }

        let _ = self.broadcast_tx.send(coord);
    }

    /// Subscribe to the broadcast channel for outgoing votes.
    pub fn subscribe(&self) -> broadcast::Receiver<pb::SwarmCoordinate> {
        self.broadcast_tx.subscribe()
    }

    /// Get vote count for a round.
    pub async fn round_vote_count(&self, round_id: &str) -> usize {
        let rounds = self.rounds.lock().await;
        rounds.get(round_id).map_or(0, |s| s.votes.len())
    }

    /// Prune completed rounds older than the given threshold.
    pub async fn prune_rounds(&self, keep_latest: usize) {
        let mut rounds = self.rounds.lock().await;
        if rounds.len() > keep_latest {
            let excess = rounds.len() - keep_latest;
            let keys: Vec<String> = rounds.keys().take(excess).cloned().collect();
            for key in keys {
                rounds.remove(&key);
            }
            info!(pruned = excess, remaining = rounds.len(), "swarm coordinator: pruned old rounds");
        }
    }
}

impl Default for SwarmCoordinator {
    fn default() -> Self {
        Self::new(256)
    }
}
