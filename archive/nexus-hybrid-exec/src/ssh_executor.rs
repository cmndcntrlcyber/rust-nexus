//! SSH-based execution implementation.
//!
//! Real implementation is gated behind the `ssh` Cargo feature, which
//! pulls in `russh` + `russh-keys`.  Without the feature the executor
//! returns a clear `TaskExecutionError` so the `HybridExecutor` fallback
//! chain tries the next method.

use crate::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_executor_construction() {
        let config = HybridExecConfig::default();
        let executor = SshExecutor::new(config);
        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_ssh_execute_without_feature_returns_error() {
        let config = HybridExecConfig::default();
        let executor = SshExecutor::new(config).unwrap();
        let request = ExecutionRequest {
            target_endpoint: "127.0.0.1".to_string(),
            execution_method: ExecutionProtocol::Ssh,
            command: "whoami".to_string(),
            parameters: std::collections::HashMap::new(),
            timeout: None,
            credentials: None,
            fallback_methods: vec![],
        };
        #[cfg(not(feature = "ssh"))]
        {
            let result = executor.execute(&request).await;
            assert!(result.is_err());
        }
    }
}

pub struct SshExecutor {
    config: HybridExecConfig,
}

impl SshExecutor {
    pub fn new(config: HybridExecConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        #[cfg(feature = "ssh")]
        {
            self.execute_real(request).await
        }
        #[cfg(not(feature = "ssh"))]
        {
            let _ = request;
            Err(NexusError::TaskExecutionError(
                "SSH executor requires the 'ssh' Cargo feature".into(),
            ))
        }
    }

    #[cfg(feature = "ssh")]
    async fn execute_real(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        use russh::client;
        use russh_keys::key;
        use std::sync::Arc;

        let start = std::time::Instant::now();
        let timeout_secs = request.timeout.unwrap_or(self.config.default_timeout);

        // Resolve credentials.
        let creds = match &request.credentials {
            Some(ExecutionCredentials::Ssh(c)) => c.clone(),
            _ => self
                .config
                .credentials_store
                .ssh_credentials
                .get(&request.target_endpoint)
                .cloned()
                .ok_or_else(|| {
                    NexusError::TaskExecutionError(format!(
                        "no SSH credentials for {}",
                        request.target_endpoint
                    ))
                })?,
        };

        let host = request.target_endpoint.clone();
        let port = creds.port;

        struct NullHandler;
        #[async_trait::async_trait]
        impl client::Handler for NullHandler {
            type Error = russh::Error;
            async fn check_server_key(
                &mut self,
                _key: &key::PublicKey,
            ) -> std::result::Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let cfg = Arc::new(client::Config {
            connection_timeout: Some(std::time::Duration::from_secs(timeout_secs)),
            ..<client::Config as Default>::default()
        });

        let mut session = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            client::connect(cfg, (host.as_str(), port), NullHandler),
        )
        .await
        .map_err(|_| NexusError::NetworkError(format!("SSH connect to {}:{} timed out", host, port)))?
        .map_err(|e| NexusError::NetworkError(format!("SSH connect: {}", e)))?;

        // Authenticate.
        let authed = if let Some(ref password) = creds.password {
            session
                .authenticate_password(creds.username.clone(), password.clone())
                .await
                .map_err(|e| NexusError::NetworkError(format!("SSH auth: {}", e)))?
        } else if let Some(ref key_str) = creds.private_key {
            let keypair = russh_keys::decode_secret_key(key_str, None)
                .map_err(|e| NexusError::NetworkError(format!("SSH key decode: {}", e)))?;
            session
                .authenticate_publickey(creds.username.clone(), Arc::new(keypair))
                .await
                .map_err(|e| NexusError::NetworkError(format!("SSH auth: {}", e)))?
        } else {
            return Err(NexusError::TaskExecutionError(
                "SSH credentials require password or private_key".into(),
            ));
        };

        if !authed {
            return Err(NexusError::TaskExecutionError(format!(
                "SSH authentication failed for {}@{}",
                creds.username, host
            )));
        }

        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|e| NexusError::NetworkError(format!("SSH open session: {}", e)))?;

        channel
            .exec(true, request.command.as_bytes())
            .await
            .map_err(|e| NexusError::NetworkError(format!("SSH exec: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code: Option<i32> = None;

        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Data { ref data }) => {
                    stdout.extend_from_slice(data);
                }
                Some(russh::ChannelMsg::ExtendedData { ref data, ext: 1 }) => {
                    stderr.extend_from_slice(data);
                }
                Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                    exit_code = Some(exit_status as i32);
                }
                Some(russh::ChannelMsg::Eof) | None => break,
                _ => {}
            }
        }

        let _ = session.disconnect(russh::Disconnect::ByApplication, "", "en").await;

        let stdout_str = String::from_utf8_lossy(&stdout).into_owned();
        let stderr_str = String::from_utf8_lossy(&stderr).into_owned();
        let success = exit_code.unwrap_or(0) == 0;

        Ok(ExecutionResult {
            success,
            output: stdout_str,
            error: if stderr_str.is_empty() { None } else { Some(stderr_str) },
            exit_code,
            execution_method: ExecutionProtocol::Ssh,
            target_endpoint: request.target_endpoint.clone(),
            duration: start.elapsed(),
            timestamp: chrono::Utc::now(),
        })
    }
}
