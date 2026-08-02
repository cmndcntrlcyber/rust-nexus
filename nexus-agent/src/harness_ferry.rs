//! Ferry handler — maps incoming `HarnessTask` proto messages to local
//! task execution and returns `HarnessTaskResult`.
//!
//! This module lives in the binary (not the library) because it depends
//! on `execution::TaskExecutor` which is binary-internal. The library
//! exposes `harness_bridge` for the `AttackTechnique` path instead.

use std::collections::HashMap;
use std::time::Instant;

use nexus_a2a::pb;
use nexus_a2a::HarnessFerryHandler;
use nexus_common::messages::TaskData;
use tonic_14::Status;
use tracing::{debug, warn};

use crate::execution::TaskExecutor;

/// Agent-side ferry: receives harness tool invocations over gRPC and
/// executes them locally via the existing `TaskExecutor`.
pub struct AgentFerryHandler {
    executor: TaskExecutor,
}

impl AgentFerryHandler {
    pub fn new() -> Self {
        Self {
            executor: TaskExecutor::new(),
        }
    }
}

#[async_trait::async_trait]
impl HarnessFerryHandler for AgentFerryHandler {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, Status> {
        debug!(
            task_id = %task.task_id,
            tool = %task.tool_name,
            "ferry: received harness task"
        );

        let task_data = harness_task_to_task_data(&task);
        let start = Instant::now();

        match self.executor.execute_task(task_data).await {
            Ok(output) => {
                let duration = start.elapsed();
                debug!(
                    task_id = %task.task_id,
                    duration_ms = duration.as_millis() as u64,
                    "ferry: task completed"
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
        }
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
