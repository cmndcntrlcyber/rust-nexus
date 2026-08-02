//! GML adjustment layer (WS2 — GML foundations).
//!
//! Ingests [`TelemetrySnapshot`]s from the mesh telemetry aggregator
//! and computes an anomaly barometer using z-score analysis. When the
//! barometer exceeds a configurable threshold, the layer issues rate
//! adjustments to constrain misbehaving agents.
//!
//! In shadow mode (the default at first deployment), the layer logs
//! its decisions but does not actually throttle.
//!
//! Implements hysteresis (v2.1.1 R3): once throttling engages at
//! `engage_threshold` (default 0.7), it remains active until the
//! barometer drops below `disengage_threshold` (default 0.4).

use nexus_mesh::telemetry::TelemetrySnapshot;
use std::collections::HashMap;

/// GML adjustment layer -- ingests telemetry, computes anomaly barometer.
pub struct GmlAdjustmentLayer {
    /// Current anomaly barometer: 0.0 = nominal, 1.0 = maximum alert.
    barometer: f64,
    /// When true, log decisions but do not issue rate adjustments.
    shadow_mode: bool,
    /// Per-agent attribution scores: higher = more anomalous.
    agent_attributions: HashMap<String, f64>,

    // ── Hysteresis thresholds (v2.1.1 R3) ───────────────────────
    /// Barometer level at which throttling engages.
    engage_threshold: f64,
    /// Barometer level at which throttling disengages.
    disengage_threshold: f64,
    /// Whether throttling is currently active.
    throttling_active: bool,
    /// Operator-signed kill switch: disables all throttling when active.
    kill_switch_active: bool,

    // ── Internal running statistics for z-score ─────────────────
    /// Running mean of per-window aggregate message counts.
    running_mean: f64,
    /// Running variance (Welford's online algorithm).
    running_m2: f64,
    /// Number of snapshots ingested.
    snapshot_count: u64,
}

impl GmlAdjustmentLayer {
    /// Create a new GML adjustment layer with default thresholds.
    pub fn new() -> Self {
        Self {
            barometer: 0.0,
            shadow_mode: true,
            agent_attributions: HashMap::new(),
            engage_threshold: 0.7,
            disengage_threshold: 0.4,
            throttling_active: false,
            kill_switch_active: false,
            running_mean: 0.0,
            running_m2: 0.0,
            snapshot_count: 0,
        }
    }

    /// Ingest a telemetry snapshot and update the anomaly barometer.
    ///
    /// Uses Welford's online algorithm to maintain running mean and
    /// standard deviation of aggregate message counts. The barometer
    /// is set to `sigmoid(z_score)` clamped to \[0.0, 1.0\].
    pub fn ingest_snapshot(&mut self, snapshot: &TelemetrySnapshot) {
        // Compute aggregate message count across all peers.
        let total_messages: u64 = snapshot
            .node_features
            .values()
            .map(|f| f.message_count)
            .sum();
        let x = total_messages as f64;

        // Welford's online update.
        self.snapshot_count += 1;
        let n = self.snapshot_count as f64;
        let delta = x - self.running_mean;
        self.running_mean += delta / n;
        let delta2 = x - self.running_mean;
        self.running_m2 += delta * delta2;

        // Compute z-score.
        let variance = if self.snapshot_count > 1 {
            self.running_m2 / (n - 1.0)
        } else {
            0.0
        };
        let stddev = variance.sqrt();

        let z_score = if stddev > f64::EPSILON {
            (x - self.running_mean) / stddev
        } else {
            0.0
        };

        // Map z-score to [0, 1] via sigmoid.
        self.barometer = sigmoid(z_score).clamp(0.0, 1.0);

        // Update per-agent attributions.
        self.agent_attributions.clear();
        if !snapshot.node_features.is_empty() {
            let mean_per_agent = x / snapshot.node_features.len() as f64;
            for (peer_id, features) in &snapshot.node_features {
                let agent_z = if stddev > f64::EPSILON {
                    (features.message_count as f64 - mean_per_agent) / stddev
                } else {
                    0.0
                };
                self.agent_attributions
                    .insert(peer_id.clone(), sigmoid(agent_z).clamp(0.0, 1.0));
            }
        }

        // Update hysteresis state.
        if self.barometer >= self.engage_threshold {
            if !self.throttling_active {
                tracing::info!(
                    barometer = self.barometer,
                    "GML: engaging throttling (barometer >= engage_threshold)"
                );
            }
            self.throttling_active = true;
        } else if self.barometer <= self.disengage_threshold {
            if self.throttling_active {
                tracing::info!(
                    barometer = self.barometer,
                    "GML: disengaging throttling (barometer <= disengage_threshold)"
                );
            }
            self.throttling_active = false;
        }
        // Between disengage and engage: maintain current state (hysteresis).
    }

    /// Return the current anomaly barometer value (0.0 = nominal, 1.0 = max alert).
    pub fn barometer(&self) -> f64 {
        self.barometer
    }

    /// Iterate over the per-agent attribution scores.
    pub fn agent_attributions_iter(&self) -> impl Iterator<Item = (&String, &f64)> {
        self.agent_attributions.iter()
    }

    /// Whether the layer recommends throttling right now.
    ///
    /// Returns `false` when:
    /// - the kill switch is active (operator override), OR
    /// - shadow mode is enabled, OR
    /// - the hysteresis state says throttling is not active.
    pub fn should_throttle(&self) -> bool {
        if self.kill_switch_active {
            return false;
        }
        if self.shadow_mode {
            if self.throttling_active {
                tracing::debug!(
                    barometer = self.barometer,
                    "GML: shadow mode — would throttle but suppressing"
                );
            }
            return false;
        }
        self.throttling_active
    }

    /// Return the recommended rate adjustment for an agent.
    ///
    /// Returns a multiplier in \[0.1, 1.0\]: 1.0 = full speed, 0.1 =
    /// maximum throttle (floor at 10% to avoid complete starvation).
    pub fn rate_adjustment(&self, agent_id: &str) -> f64 {
        if !self.should_throttle() {
            return 1.0;
        }
        let attribution = self
            .agent_attributions
            .get(agent_id)
            .copied()
            .unwrap_or(0.0);
        // Higher attribution = more anomalous = lower rate.
        // Map [0, 1] attribution to [1.0, 0.1] rate.
        let rate = 1.0 - (attribution * 0.9);
        rate.clamp(0.1, 1.0)
    }

    /// Activate the operator-signed kill switch, disabling all throttling.
    pub fn activate_kill_switch(&mut self) {
        tracing::warn!("GML: kill switch activated — all throttling disabled");
        self.kill_switch_active = true;
        self.throttling_active = false;
    }

    /// Enable or disable shadow mode.
    pub fn set_shadow_mode(&mut self, enabled: bool) {
        tracing::info!(shadow_mode = enabled, "GML: shadow mode updated");
        self.shadow_mode = enabled;
    }

    /// Return whether shadow mode is active.
    pub fn is_shadow_mode(&self) -> bool {
        self.shadow_mode
    }

    /// Return whether the kill switch is active.
    pub fn is_kill_switch_active(&self) -> bool {
        self.kill_switch_active
    }
}

impl Default for GmlAdjustmentLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Sigmoid function mapping any real value to (0, 1).
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_mesh::telemetry::NodeFeatures;
    use std::collections::HashMap;

    fn make_snapshot(peer_counts: &[(&str, u64)]) -> TelemetrySnapshot {
        let mut node_features = HashMap::new();
        for (peer, count) in peer_counts {
            node_features.insert(
                peer.to_string(),
                NodeFeatures {
                    peer_id: peer.to_string(),
                    message_count: *count,
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
                },
            );
        }
        TelemetrySnapshot {
            window_start: 1000,
            window_end: 1060,
            node_features,
            edge_features: vec![],
        }
    }

    #[test]
    fn initial_barometer_is_zero() {
        let gml = GmlAdjustmentLayer::new();
        assert!((gml.barometer() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn shadow_mode_prevents_throttling() {
        let mut gml = GmlAdjustmentLayer::new();
        // Force throttling_active.
        gml.throttling_active = true;
        assert!(gml.shadow_mode);
        assert!(!gml.should_throttle());
    }

    #[test]
    fn kill_switch_prevents_throttling() {
        let mut gml = GmlAdjustmentLayer::new();
        gml.set_shadow_mode(false);
        gml.throttling_active = true;
        gml.activate_kill_switch();
        assert!(!gml.should_throttle());
    }

    #[test]
    fn rate_adjustment_is_full_when_not_throttling() {
        let gml = GmlAdjustmentLayer::new();
        assert!((gml.rate_adjustment("any-agent") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ingest_updates_barometer() {
        let mut gml = GmlAdjustmentLayer::new();

        // Ingest several nominal snapshots to establish a baseline.
        for _ in 0..10 {
            let snap = make_snapshot(&[("peer-a", 100), ("peer-b", 100)]);
            gml.ingest_snapshot(&snap);
        }

        // Barometer should be around 0.5 (nominal baseline).
        assert!(
            gml.barometer() > 0.3 && gml.barometer() < 0.7,
            "expected near-nominal barometer, got {}",
            gml.barometer()
        );
    }

    #[test]
    fn hysteresis_engage_disengage() {
        let mut gml = GmlAdjustmentLayer::new();
        gml.set_shadow_mode(false);

        // Directly set barometer above engage threshold.
        gml.barometer = 0.8;
        // Simulate the hysteresis check that happens in ingest.
        if gml.barometer >= gml.engage_threshold {
            gml.throttling_active = true;
        }
        assert!(gml.should_throttle());

        // Drop barometer to between thresholds — should still throttle (hysteresis).
        gml.barometer = 0.5;
        assert!(gml.should_throttle());

        // Drop below disengage.
        gml.barometer = 0.3;
        gml.throttling_active = false; // would happen in ingest_snapshot
        assert!(!gml.should_throttle());
    }

    #[test]
    fn sigmoid_bounds() {
        assert!((sigmoid(0.0) - 0.5).abs() < f64::EPSILON);
        assert!(sigmoid(100.0) > 0.99);
        assert!(sigmoid(-100.0) < 0.01);
    }
}
