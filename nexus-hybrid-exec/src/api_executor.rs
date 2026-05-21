//! HTTP API-based execution implementation.
//!
//! When the `api` Cargo feature is enabled the executor makes a real
//! POST to `<endpoint>/execute` with a JSON body and parses the JSON
//! response.  Without the feature the executor returns a clear error
//! so callers fall back to other methods.

use crate::*;

pub struct ApiExecutor {
    config: HybridExecConfig,
    #[cfg(feature = "api")]
    client: reqwest::Client,
}

impl ApiExecutor {
    pub fn new(config: HybridExecConfig) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "api")]
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(config.default_timeout))
                .build()
                .map_err(|e| {
                    NexusError::NetworkError(format!("reqwest client init: {}", e))
                })?,
            config,
        })
    }

    pub async fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        #[cfg(feature = "api")]
        {
            self.execute_real(request).await
        }
        #[cfg(not(feature = "api"))]
        {
            let _ = request;
            Err(NexusError::TaskExecutionError(
                "API executor requires the 'api' Cargo feature".into(),
            ))
        }
    }

    #[cfg(feature = "api")]
    async fn execute_real(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        use serde_json::json;
        let start = std::time::Instant::now();

        let creds = request.credentials.as_ref();
        let api_key = match creds {
            Some(ExecutionCredentials::Api(c)) => c.api_key.clone(),
            _ => {
                self.config
                    .credentials_store
                    .api_credentials
                    .get(&request.target_endpoint)
                    .map(|c| c.api_key.clone())
                    .unwrap_or_default()
            }
        };

        let url = format!("https://{}/execute", request.target_endpoint);
        let body = json!({
            "command": request.command,
            "parameters": request.parameters,
            "timeout": request.timeout,
        });

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(request.timeout.unwrap_or(self.config.default_timeout)),
            self.client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| NexusError::TaskExecutionError("API request timed out".into()))?
        .map_err(|e| NexusError::NetworkError(format!("API request: {}", e)))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| NexusError::NetworkError(format!("API response body: {}", e)))?;

        let (output, error, exit_code) = if status.is_success() {
            // Try to parse { "output": "...", "exit_code": 0 }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let out = v["output"].as_str().unwrap_or(&text).to_string();
                let code = v["exit_code"].as_i64().map(|n| n as i32).or(Some(0));
                (out, None, code)
            } else {
                (text, None, Some(0))
            }
        } else {
            (String::new(), Some(format!("HTTP {}: {}", status, text)), Some(status.as_u16() as i32))
        };

        Ok(ExecutionResult {
            success: status.is_success(),
            output,
            error,
            exit_code,
            execution_method: ExecutionProtocol::Api,
            target_endpoint: request.target_endpoint.clone(),
            duration: start.elapsed(),
            timestamp: chrono::Utc::now(),
        })
    }
}
