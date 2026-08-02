//! Telemetry aggregation for gossipsub mesh activity (WS2 — GML foundations).
//!
//! Collects per-peer message / byte counters in temporal windows and
//! flushes them as [`TelemetrySnapshot`]s for downstream consumption
//! by the GML adjustment layer in `nexus-a2a`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Per-window counters for gossipsub activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    /// Unix timestamp of the window start.
    pub window_start: u64,
    /// Unix timestamp of the window end.
    pub window_end: u64,
    /// Per-node feature vectors keyed by peer ID string.
    pub node_features: HashMap<String, NodeFeatures>,
    /// Inter-node edge features captured during this window.
    pub edge_features: Vec<EdgeFeature>,
}

/// Aggregate feature vector for a single peer within one telemetry window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFeatures {
    /// Peer identity (libp2p PeerId string).
    pub peer_id: String,
    /// Total gossipsub messages handled.
    pub message_count: u64,
    /// Total bytes sent to other peers.
    pub bytes_sent: u64,
    /// Total bytes received from other peers.
    pub bytes_recv: u64,
    /// Number of tasks dispatched via this peer.
    pub task_count: u64,
    /// Error rate (errors / total_messages) in \[0.0, 1.0\].
    pub error_rate: f64,

    // ── Kernel context aggregates (from WS1.5) ──────────────────
    /// Total syscalls observed across all tasks for this peer.
    pub total_syscalls: u64,
    /// Total memory allocated across all tasks (bytes).
    pub total_memory_allocated: u64,
    /// Total child processes spawned.
    pub total_processes_spawned: u64,
    /// Total files read/written/created.
    pub total_files_touched: u64,
    /// Total network connections opened.
    pub total_network_connections: u64,
    /// Number of sandbox containment violations.
    pub sandbox_violations: u64,
}

impl NodeFeatures {
    fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            message_count: 0,
            bytes_sent: 0,
            bytes_recv: 0,
            task_count: 0,
            error_rate: 0.0,
            total_syscalls: 0,
            total_memory_allocated: 0,
            total_processes_spawned: 0,
            total_files_touched: 0,
            total_network_connections: 0,
            sandbox_violations: 0,
        }
    }
}

/// Edge feature between two peers in one telemetry window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFeature {
    /// Source peer ID.
    pub source: String,
    /// Target peer ID.
    pub target: String,
    /// Category of the edge (e.g. "gossipsub", "task_dispatch").
    pub edge_type: String,
    /// Average bytes per second over the window.
    pub byte_rate: f64,
    /// Average messages per second over the window.
    pub message_rate: f64,
}

/// Direction of a gossipsub message for telemetry recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Message sent to peer(s).
    Sent,
    /// Message received from peer(s).
    Received,
}

/// Aggregates gossipsub events into temporal windows.
///
/// Call [`record_message`] on each gossipsub send/receive. The
/// aggregator accumulates per-peer counters. When [`should_flush`]
/// returns `true`, call [`flush`] to produce a [`TelemetrySnapshot`]
/// and reset the window.
pub struct TelemetryAggregator {
    window_duration: Duration,
    window_start: Instant,
    window_start_unix: u64,
    counters: HashMap<String, NodeFeatures>,
    errors: HashMap<String, u64>,
    totals: HashMap<String, u64>,
}

impl TelemetryAggregator {
    /// Create a new aggregator with the specified window duration.
    pub fn new(window_duration: Duration) -> Self {
        Self {
            window_duration,
            window_start: Instant::now(),
            window_start_unix: nexus_common::current_timestamp(),
            counters: HashMap::new(),
            errors: HashMap::new(),
            totals: HashMap::new(),
        }
    }

    /// Record a single gossipsub message for telemetry.
    pub fn record_message(&mut self, peer_id: &str, bytes: u64, direction: Direction) {
        let entry = self
            .counters
            .entry(peer_id.to_string())
            .or_insert_with(|| NodeFeatures::new(peer_id.to_string()));

        entry.message_count += 1;
        match direction {
            Direction::Sent => entry.bytes_sent += bytes,
            Direction::Received => entry.bytes_recv += bytes,
        }

        *self.totals.entry(peer_id.to_string()).or_insert(0) += 1;
    }

    /// Record a task dispatch for a peer.
    pub fn record_task(&mut self, peer_id: &str) {
        let entry = self
            .counters
            .entry(peer_id.to_string())
            .or_insert_with(|| NodeFeatures::new(peer_id.to_string()));
        entry.task_count += 1;
    }

    /// Record an error for a peer (updates error_rate on flush).
    pub fn record_error(&mut self, peer_id: &str) {
        *self.errors.entry(peer_id.to_string()).or_insert(0) += 1;
        *self.totals.entry(peer_id.to_string()).or_insert(0) += 1;
    }

    /// Record kernel context aggregates for a peer (from WS1.5).
    pub fn record_kernel_context(
        &mut self,
        peer_id: &str,
        ctx: &nexus_common::KernelContext,
    ) {
        let entry = self
            .counters
            .entry(peer_id.to_string())
            .or_insert_with(|| NodeFeatures::new(peer_id.to_string()));

        entry.total_syscalls += ctx.syscall_count;
        entry.total_memory_allocated += ctx.memory_allocated_bytes;
        entry.total_processes_spawned += ctx.processes_spawned as u64;
        entry.total_files_touched += ctx.files_touched as u64;
        entry.total_network_connections += ctx.network_connections as u64;

        if ctx.sandbox_verdict != nexus_common::kernel_context::SandboxVerdict::Nominal {
            entry.sandbox_violations += 1;
        }
    }

    /// Returns `true` if the current window duration has elapsed.
    pub fn should_flush(&self) -> bool {
        self.window_start.elapsed() >= self.window_duration
    }

    /// Flush the current window, returning a snapshot and resetting
    /// all counters for the next window.
    pub fn flush(&mut self) -> TelemetrySnapshot {
        let now_unix = nexus_common::current_timestamp();
        let elapsed_secs = self.window_start.elapsed().as_secs_f64().max(0.001);

        // Compute error rates.
        for (peer_id, features) in self.counters.iter_mut() {
            let errs = self.errors.get(peer_id).copied().unwrap_or(0) as f64;
            let total = self.totals.get(peer_id).copied().unwrap_or(1).max(1) as f64;
            features.error_rate = errs / total;
        }

        // Build edge features from the counters (self-to-peer edges).
        let edge_features: Vec<EdgeFeature> = self
            .counters
            .values()
            .filter(|f| f.bytes_sent > 0 || f.bytes_recv > 0)
            .map(|f| EdgeFeature {
                source: "local".to_string(),
                target: f.peer_id.clone(),
                edge_type: "gossipsub".to_string(),
                byte_rate: (f.bytes_sent + f.bytes_recv) as f64 / elapsed_secs,
                message_rate: f.message_count as f64 / elapsed_secs,
            })
            .collect();

        let snapshot = TelemetrySnapshot {
            window_start: self.window_start_unix,
            window_end: now_unix,
            node_features: std::mem::take(&mut self.counters),
            edge_features,
        };

        // Reset for the next window.
        self.window_start = Instant::now();
        self.window_start_unix = now_unix;
        self.errors.clear();
        self.totals.clear();

        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregator_records_and_flushes() {
        let mut agg = TelemetryAggregator::new(Duration::from_millis(10));

        agg.record_message("peer-1", 100, Direction::Sent);
        agg.record_message("peer-1", 200, Direction::Received);
        agg.record_message("peer-2", 50, Direction::Sent);
        agg.record_task("peer-1");
        agg.record_error("peer-2");

        let snap = agg.flush();

        assert_eq!(snap.node_features.len(), 2);

        let p1 = &snap.node_features["peer-1"];
        assert_eq!(p1.message_count, 2);
        assert_eq!(p1.bytes_sent, 100);
        assert_eq!(p1.bytes_recv, 200);
        assert_eq!(p1.task_count, 1);
        assert!((p1.error_rate - 0.0).abs() < f64::EPSILON);

        let p2 = &snap.node_features["peer-2"];
        assert_eq!(p2.message_count, 1);
        assert_eq!(p2.bytes_sent, 50);
        // error_rate = 1 error / 2 total (1 message + 1 error)
        assert!((p2.error_rate - 0.5).abs() < f64::EPSILON);

        // After flush, counters are reset.
        assert!(agg.counters.is_empty());
    }

    #[test]
    fn should_flush_respects_window() {
        let agg = TelemetryAggregator::new(Duration::from_secs(3600));
        assert!(!agg.should_flush());

        let agg2 = TelemetryAggregator::new(Duration::from_millis(0));
        assert!(agg2.should_flush());
    }
}
