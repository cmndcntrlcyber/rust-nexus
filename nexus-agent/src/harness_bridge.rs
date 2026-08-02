//! Bridge between `AttackTechnique` trait and the ferry protocol.
//!
//! Translates `HarnessTask` (tool_name / json_arguments) into
//! `TechniqueParams` (command / parameters) for `AttackTechnique::execute`,
//! and maps `TechniqueResult` back to `HarnessTaskResult`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use nexus_a2a::pb;
use nexus_a2a::HarnessFerryHandler;
use nexus_common::technique::{AttackTechnique, ExecutionContext, TechniqueParams};
use tonic_14::Status;
use tracing::{debug, warn};

/// Ferry handler that dispatches to registered `AttackTechnique`
/// implementations by matching `HarnessTask.tool_name` against
/// technique IDs and task type strings.
pub struct TechniqueBridge {
    techniques: Vec<Arc<dyn AttackTechnique>>,
    execution_context: Arc<ExecutionContext>,
}

impl TechniqueBridge {
    pub fn new(
        techniques: Vec<Arc<dyn AttackTechnique>>,
        execution_context: Arc<ExecutionContext>,
    ) -> Self {
        Self {
            techniques,
            execution_context,
        }
    }

    fn find_technique(&self, tool_name: &str) -> Option<&Arc<dyn AttackTechnique>> {
        self.techniques.iter().find(|t| {
            t.technique_id() == tool_name
                || t.task_types().iter().any(|tt| tt == tool_name)
        })
    }
}

#[async_trait::async_trait]
impl HarnessFerryHandler for TechniqueBridge {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, Status> {
        let technique = self.find_technique(&task.tool_name).ok_or_else(|| {
            Status::not_found(format!(
                "no AttackTechnique registered for '{}'",
                task.tool_name
            ))
        })?;

        let parameters: HashMap<String, String> =
            serde_json::from_str(&task.json_arguments).map_err(|e| {
                Status::invalid_argument(format!("invalid json_arguments: {e}"))
            })?;

        let command = parameters
            .get("command")
            .cloned()
            .unwrap_or_default();

        let params = TechniqueParams {
            command,
            parameters,
            timeout: None,
        };

        if let Err(e) = technique.validate(&params) {
            return Err(Status::invalid_argument(format!(
                "technique validation failed: {e}"
            )));
        }

        debug!(
            task_id = %task.task_id,
            technique = %technique.technique_id(),
            "bridge: dispatching to AttackTechnique"
        );

        let start = Instant::now();
        match technique.execute(&self.execution_context, params).await {
            Ok(result) => {
                let duration = start.elapsed();
                Ok(pb::HarnessTaskResult {
                    task_id: task.task_id,
                    output: result.output,
                    is_error: !result.success,
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
                    "bridge: technique execution failed"
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
