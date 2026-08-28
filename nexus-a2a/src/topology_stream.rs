//! `TopologyStream` — assembles `MeshTopologySnapshot` from live agent
//! and GML state. Used by `StreamMeshTopology` to push real-time mesh
//! views to the nexus-console Mesh tab.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::debug;

use crate::gml::GmlAdjustmentLayer;
use crate::pb;
use crate::situational_awareness::SituationalAwareness;

/// Broadcasts `MeshTopologySnapshot` frames at a configurable interval.
pub struct TopologyStream {
    sa: Arc<SituationalAwareness>,
    gml: Arc<Mutex<GmlAdjustmentLayer>>,
    tx: broadcast::Sender<pb::MeshTopologySnapshot>,
    sequence: std::sync::atomic::AtomicU64,
}

impl TopologyStream {
    /// Create a new publisher. `capacity` is the broadcast channel depth.
    pub fn new(
        sa: Arc<SituationalAwareness>,
        gml: Arc<Mutex<GmlAdjustmentLayer>>,
        capacity: usize,
    ) -> (Arc<Self>, broadcast::Receiver<pb::MeshTopologySnapshot>) {
        let (tx, rx) = broadcast::channel(capacity);
        let this = Arc::new(Self {
            sa,
            gml,
            tx,
            sequence: std::sync::atomic::AtomicU64::new(0),
        });
        (this, rx)
    }

    /// Subscribe to the snapshot stream.
    pub fn subscribe(&self) -> broadcast::Receiver<pb::MeshTopologySnapshot> {
        self.tx.subscribe()
    }

    /// Run the publish loop at the given interval until cancelled.
    pub async fn run(self: &Arc<Self>, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let snapshot = self.build_snapshot().await;
            if self.tx.send(snapshot).is_err() {
                debug!("topology_stream: no subscribers, skipping");
            }
        }
    }

    /// Build one snapshot from current SA + GML state.
    pub async fn build_snapshot_public(&self) -> pb::MeshTopologySnapshot {
        self.build_snapshot().await
    }

    async fn build_snapshot(&self) -> pb::MeshTopologySnapshot {
        let agents_raw = self.sa.all_agents().await;
        let barometer_score = {
            let g = self.gml.lock().unwrap();
            g.barometer()
        };

        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let agents: Vec<pb::AgentNodeProto> = agents_raw
            .iter()
            .map(|a| pb::AgentNodeProto {
                agent_id: a.peer_id_hex.clone(),
                peer_id: a.peer_id_hex.clone(),
                health: if a.total_errors > 0 && a.active_task_count == 0 {
                    pb::AgentHealthProto::AgentHealthDegraded as i32
                } else {
                    pb::AgentHealthProto::AgentHealthHealthy as i32
                },
                current_tasks: a.active_task_count,
                skills: a.harness_skills.clone(),
                techniques: a.technique_ids.clone(),
                role: a.tag.clone(),
                connected_at_unix: 0,
                last_heartbeat_unix: a.last_seen_unix,
            })
            .collect();

        let barometer = Some(pb::NoiseBarometerProto {
            global_score: barometer_score as f32,
            per_agent: std::collections::HashMap::new(),
            controller_state: String::new(),
            throttle_floor: 0.3,
        });

        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        pb::MeshTopologySnapshot {
            agents,
            edges: Vec::new(),
            flows: Vec::new(),
            barometer,
            snapshot_timestamp_unix: now_unix,
            sequence_number: seq,
        }
    }
}
