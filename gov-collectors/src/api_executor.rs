//! HTTP API-based execution implementation

use crate::*;

pub struct ApiExecutor {
    config: HybridExecConfig,
}

impl ApiExecutor {
    pub fn new(config: HybridExecConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn execute(&self, _request: &ExecutionRequest) -> Result<ExecutionResult> {
        // Stub implementation
        Ok(ExecutionResult {
            success: false,
            output: "API execution not yet implemented".to_string(),
            error: Some("Not implemented".to_string()),
            exit_code: Some(-1),
            execution_method: ExecutionProtocol::Api,
            target_endpoint: "stub".to_string(),
            duration: std::time::Duration::from_secs(0),
            timestamp: chrono::Utc::now(),
        })
    }
}
