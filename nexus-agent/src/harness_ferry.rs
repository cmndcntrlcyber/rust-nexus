//! Ferry handler — maps incoming `HarnessTask` proto messages to local
//! task execution and returns `HarnessTaskResult`.
//!
//! This module lives in the binary (not the library) because it depends
//! on `execution::TaskExecutor` which is binary-internal. The library
//! exposes `harness_bridge` for the `AttackTechnique` path instead.
//!
//! v1.6 (WS2 Phase 2c): collects per-task telemetry and submits
//! aggregated snapshots to the GML adjustment layer via the
//! `SubmitTelemetrySnapshot` RPC at each window boundary.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nexus_a2a::pb;
use nexus_a2a::{A2aClient, HarnessFerryHandler};
use nexus_common::messages::TaskData;
use nexus_mesh::telemetry::{NodeFeatures, TelemetryAggregator, Direction};
use tokio::sync::Mutex;
use tonic_14::Status;
use tracing::{debug, info, warn};

use crate::execution::TaskExecutor;

/// Default telemetry window duration (1 hour, matching GML spec).
const TELEMETRY_WINDOW: Duration = Duration::from_secs(3600);

/// Agent-side ferry: receives harness tool invocations over gRPC and
/// executes them locally via the existing `TaskExecutor`.
pub struct AgentFerryHandler {
    executor: TaskExecutor,
    /// Maximum concurrent ferry tasks (0 = unlimited).
    max_concurrent: u32,
    /// Currently executing tasks (for rate-limiting).
    active_tasks: AtomicU64,
    /// Telemetry aggregator for GML (WS2 Phase 2c).
    telemetry: Arc<Mutex<TelemetryAggregator>>,
    /// Local peer ID for telemetry attribution.
    local_peer_id: String,
    /// Optional A2A client for telemetry submission.
    telemetry_client: Arc<Mutex<Option<A2aClient>>>,
}

impl AgentFerryHandler {
    pub fn new() -> Self {
        Self {
            executor: TaskExecutor::new(),
            max_concurrent: 0,
            active_tasks: AtomicU64::new(0),
            telemetry: Arc::new(Mutex::new(TelemetryAggregator::new(TELEMETRY_WINDOW))),
            local_peer_id: "local".into(),
            telemetry_client: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the maximum concurrent ferry tasks for rate-limiting.
    pub fn with_rate_limit(mut self, max_concurrent: u32) -> Self {
        self.max_concurrent = max_concurrent;
        self
    }

    /// Set the local peer ID for telemetry attribution.
    pub fn with_peer_id(mut self, peer_id: String) -> Self {
        self.local_peer_id = peer_id;
        self
    }

    /// Set the A2A client for telemetry submission to the GML layer.
    pub fn with_telemetry_client(mut self, client: A2aClient) -> Self {
        self.telemetry_client = Arc::new(Mutex::new(Some(client)));
        self
    }

    /// Try to flush and submit telemetry if the window has elapsed.
    async fn maybe_submit_telemetry(&self) {
        let should_flush = {
            let agg = self.telemetry.lock().await;
            agg.should_flush()
        };

        if !should_flush {
            return;
        }

        let snapshot = {
            let mut agg = self.telemetry.lock().await;
            agg.flush()
        };

        let mut client_guard = self.telemetry_client.lock().await;
        let Some(client) = client_guard.as_mut() else {
            debug!("ferry: telemetry window flushed but no client configured — snapshot dropped");
            return;
        };

        let proto = snapshot_to_proto(&snapshot);
        match client.submit_telemetry_snapshot(proto).await {
            Ok(()) => {
                debug!(
                    window_start = snapshot.window_start,
                    window_end = snapshot.window_end,
                    peers = snapshot.node_features.len(),
                    "ferry: telemetry snapshot submitted"
                );
            }
            Err(e) => {
                warn!(error = %e, "ferry: failed to submit telemetry snapshot");
            }
        }
    }
}

#[async_trait::async_trait]
impl HarnessFerryHandler for AgentFerryHandler {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, Status> {
        // Rate-limiting: reject if at capacity.
        if self.max_concurrent > 0 {
            let active = self.active_tasks.load(Ordering::Relaxed);
            if active >= self.max_concurrent as u64 {
                warn!(
                    task_id = %task.task_id,
                    active,
                    limit = self.max_concurrent,
                    "ferry: rate-limited — rejecting task"
                );
                // Record error in telemetry.
                self.telemetry.lock().await.record_error(&self.local_peer_id);
                return Ok(pb::HarnessTaskResult {
                    task_id: task.task_id,
                    output: "rate-limited: too many concurrent ferry tasks".into(),
                    is_error: true,
                    execution_duration_ms: 0,
                    bytes_sent: 0,
                    bytes_recv: 0,
                    commands_run: 0,
                    kernel_context: None,
                });
            }
        }

        self.active_tasks.fetch_add(1, Ordering::Relaxed);

        debug!(
            task_id = %task.task_id,
            tool = %task.tool_name,
            "ferry: received harness task"
        );

        // Record task dispatch in telemetry.
        {
            let mut agg = self.telemetry.lock().await;
            agg.record_task(&self.local_peer_id);
            agg.record_message(
                &self.local_peer_id,
                task.json_arguments.len() as u64,
                Direction::Received,
            );
        }

        let task_data = harness_task_to_task_data(&task);
        let start = Instant::now();

        let result = match self.executor.execute_task(task_data).await {
            Ok(output) => {
                let duration = start.elapsed();
                let output_len = output.len() as u64;
                info!(
                    task_id = %task.task_id,
                    duration_ms = duration.as_millis() as u64,
                    "ferry: task completed"
                );
                // Record outbound bytes in telemetry.
                self.telemetry.lock().await.record_message(
                    &self.local_peer_id,
                    output_len,
                    Direction::Sent,
                );
                Ok(pb::HarnessTaskResult {
                    task_id: task.task_id,
                    output,
                    is_error: false,
                    execution_duration_ms: duration.as_millis() as u64,
                    bytes_sent: 0,
                    bytes_recv: 0,
                    commands_run: 1,
                    kernel_context: None,
                })
            }
            Err(e) => {
                let duration = start.elapsed();
                warn!(
                    task_id = %task.task_id,
                    error = %e,
                    "ferry: task failed"
                );
                // Record error in telemetry.
                self.telemetry.lock().await.record_error(&self.local_peer_id);
                Ok(pb::HarnessTaskResult {
                    task_id: task.task_id,
                    output: e.to_string(),
                    is_error: true,
                    execution_duration_ms: duration.as_millis() as u64,
                    bytes_sent: 0,
                    bytes_recv: 0,
                    commands_run: 1,
                    kernel_context: None,
                })
            }
        };

        self.active_tasks.fetch_sub(1, Ordering::Relaxed);

        // Check if telemetry window should be flushed and submitted.
        self.maybe_submit_telemetry().await;

        result
    }
}

/// Translate a proto `HarnessTask` into the `TaskData` struct that
/// `TaskExecutor::execute_task` expects.
fn harness_task_to_task_data(task: &pb::HarnessTask) -> TaskData {
    let mut parameters: HashMap<String, String> = serde_json::from_str(&task.json_arguments)
        .unwrap_or_default();

    let command = parameters.remove("command").unwrap_or_default();

    TaskData {
        task_id: task.task_id.clone(),
        task_type: task.tool_name.clone(),
        command,
        parameters,
        timeout: None,
        priority: 100,
    }
}

/// Convert a domain `TelemetrySnapshot` into the proto representation.
fn snapshot_to_proto(
    snapshot: &nexus_mesh::telemetry::TelemetrySnapshot,
) -> pb::TelemetrySnapshotProto {
    let node_features = snapshot
        .node_features
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                pb::NodeFeaturesProto {
                    peer_id: v.peer_id.clone(),
                    message_count: v.message_count,
                    bytes_sent: v.bytes_sent,
                    bytes_recv: v.bytes_recv,
                    task_count: v.task_count,
                    error_rate: v.error_rate,
                    total_syscalls: v.total_syscalls,
                    total_memory_allocated: v.total_memory_allocated,
                    total_processes_spawned: v.total_processes_spawned,
                    total_files_touched: v.total_files_touched,
                    total_network_connections: v.total_network_connections,
                    sandbox_violations: v.sandbox_violations,
                },
            )
        })
        .collect();

    pb::TelemetrySnapshotProto {
        window_start: snapshot.window_start,
        window_end: snapshot.window_end,
        node_features,
    }
}
