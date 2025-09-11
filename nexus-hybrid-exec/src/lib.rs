//! Nexus Hybrid Execution Module
//!
//! Integrates tauri-executor's cross-platform execution capabilities with rust-nexus's
//! enterprise C2 framework, providing multiple execution protocols and fallback methods.

use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;

pub mod api_executor;
pub mod powershell_executor;
pub mod ssh_executor;
pub mod wmi_executor;

use nexus_common;

/// Hybrid execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridExecConfig {
    pub default_timeout: u64,
    pub max_concurrent_jobs: u32,
    pub retry_attempts: u32,
    pub fallback_enabled: bool,
    pub execution_protocols: Vec<ExecutionProtocol>,
    pub credentials_store: CredentialsStore,
}

impl Default for HybridExecConfig {
    fn default() -> Self {
        Self {
            default_timeout: 60,
            max_concurrent_jobs: 5,
            retry_attempts: 3,
            fallback_enabled: true,
            execution_protocols: vec![
                ExecutionProtocol::Grpc,
                ExecutionProtocol::Ssh,
                ExecutionProtocol::Api,
                ExecutionProtocol::PowerShell,
            ],
            credentials_store: CredentialsStore::default(),
        }
    }
}

/// Available execution protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionProtocol {
    Local,
    Grpc,
    Ssh,
    Wmi,
    Api,
    PowerShell,
    WebSocket,
}

/// Credentials storage for different execution methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsStore {
    pub ssh_credentials: HashMap<String, SshCredentials>,
    pub wmi_credentials: HashMap<String, WmiCredentials>,
    pub api_credentials: HashMap<String, ApiCredentials>,
}

impl Default for CredentialsStore {
    fn default() -> Self {
        Self {
            ssh_credentials: HashMap::new(),
            wmi_credentials: HashMap::new(),
            api_credentials: HashMap::new(),
        }
    }
}

/// SSH connection credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshCredentials {
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub port: u16,
}

/// WMI connection credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmiCredentials {
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

/// API connection credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCredentials {
    pub api_key: String,
    pub endpoint: String,
    pub port: u16,
    pub use_tls: bool,
}

/// Execution request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub target_endpoint: String,
    pub execution_method: ExecutionProtocol,
    pub command: String,
    pub parameters: HashMap<String, String>,
    pub timeout: Option<u64>,
    pub credentials: Option<ExecutionCredentials>,
    pub fallback_methods: Vec<ExecutionProtocol>,
}

/// Unified credentials for execution requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExecutionCredentials {
    Ssh(SshCredentials),
    Wmi(WmiCredentials),
    Api(ApiCredentials),
    None,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub execution_method: ExecutionProtocol,
    pub target_endpoint: String,
    pub duration: std::time::Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Main hybrid execution engine
pub struct HybridExecutor {
    config: HybridExecConfig,
    ssh_executor: ssh_executor::SshExecutor,
    #[cfg(target_os = "windows")]
    wmi_executor: wmi_executor::WmiExecutor,
    api_executor: api_executor::ApiExecutor,
    powershell_executor: powershell_executor::PowerShellExecutor,
}

impl HybridExecutor {
    /// Create a new hybrid executor
    pub fn new(config: HybridExecConfig) -> nexus_common::Result<Self> {
        Ok(Self {
            ssh_executor: ssh_executor::SshExecutor::new(config.clone())?,
            #[cfg(target_os = "windows")]
            wmi_executor: wmi_executor::WmiExecutor::new(config.clone())?,
            api_executor: api_executor::ApiExecutor::new(config.clone())?,
            powershell_executor: powershell_executor::PowerShellExecutor::new(config.clone())?,
            config,
        })
    }

    /// Execute a command using the specified method with automatic fallback
    pub async fn execute(
        &self,
        request: ExecutionRequest,
    ) -> nexus_common::Result<ExecutionResult> {
        info!(
            "Executing command on {} via {:?}",
            request.target_endpoint, request.execution_method
        );

        let start_time = std::time::Instant::now();
        let mut last_error = None;

        // Try primary execution method
        match self
            .execute_with_method(&request, &request.execution_method)
            .await
        {
            Ok(mut result) => {
                result.duration = start_time.elapsed();
                return Ok(result);
            }
            Err(e) => {
                warn!("Primary execution method failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try fallback methods if enabled
        if self.config.fallback_enabled {
            for fallback_method in &request.fallback_methods {
                info!("Trying fallback method: {:?}", fallback_method);

                match self.execute_with_method(&request, fallback_method).await {
                    Ok(mut result) => {
                        result.duration = start_time.elapsed();
                        result.execution_method = fallback_method.clone();
                        return Ok(result);
                    }
                    Err(e) => {
                        warn!("Fallback method {:?} failed: {}", fallback_method, e);
                        last_error = Some(e);
                    }
                }
            }
        }

        // All methods failed
        Err(last_error.unwrap_or_else(|| {
            nexus_common::NexusError::TaskExecutionError("All execution methods failed".to_string())
        }))
    }

    /// Execute with a specific method
    async fn execute_with_method(
        &self,
        request: &ExecutionRequest,
        method: &ExecutionProtocol,
    ) -> nexus_common::Result<ExecutionResult> {
        match method {
            ExecutionProtocol::Local => self.execute_local(request).await,
            ExecutionProtocol::Ssh => self.ssh_executor.execute(request).await,
            #[cfg(target_os = "windows")]
            ExecutionProtocol::Wmi => self.wmi_executor.execute(request).await,
            #[cfg(not(target_os = "windows"))]
            ExecutionProtocol::Wmi => Err(nexus_common::NexusError::TaskExecutionError(
                "WMI execution not available on this platform".to_string(),
            )),
            ExecutionProtocol::Api => self.api_executor.execute(request).await,
            ExecutionProtocol::PowerShell => self.powershell_executor.execute(request).await,
            ExecutionProtocol::Grpc => {
                // TODO: Integrate with existing gRPC infrastructure
                Err(nexus_common::NexusError::TaskExecutionError(
                    "gRPC execution not yet implemented in hybrid executor".to_string(),
                ))
            }
            ExecutionProtocol::WebSocket => {
                // TODO: Implement WebSocket execution
                Err(nexus_common::NexusError::TaskExecutionError(
                    "WebSocket execution not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Execute command locally
    async fn execute_local(
        &self,
        request: &ExecutionRequest,
    ) -> nexus_common::Result<ExecutionResult> {
        let start_time = std::time::Instant::now();

        #[cfg(target_os = "windows")]
        let mut cmd = Command::new("cmd");
        #[cfg(target_os = "windows")]
        cmd.args(&["/C", &request.command]);

        #[cfg(not(target_os = "windows"))]
        let mut cmd = Command::new("sh");
        #[cfg(not(target_os = "windows"))]
        cmd.args(&["-c", &request.command]);

        // Set timeout
        let timeout_duration =
            std::time::Duration::from_secs(request.timeout.unwrap_or(self.config.default_timeout));

        let output = tokio::time::timeout(timeout_duration, cmd.output())
            .await
            .map_err(|_| {
                nexus_common::NexusError::TaskExecutionError(
                    "Command execution timeout".to_string(),
                )
            })?
            .map_err(|e| {
                nexus_common::NexusError::TaskExecutionError(format!(
                    "Command execution failed: {}",
                    e
                ))
            })?;

        Ok(ExecutionResult {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error: if !output.stderr.is_empty() {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            } else {
                None
            },
            exit_code: output.status.code(),
            execution_method: ExecutionProtocol::Local,
            target_endpoint: "localhost".to_string(),
            duration: start_time.elapsed(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Get supported execution methods for the current platform
    pub fn get_supported_methods(&self) -> Vec<ExecutionProtocol> {
        let methods = vec![
            ExecutionProtocol::Local,
            ExecutionProtocol::Ssh,
            ExecutionProtocol::Api,
            ExecutionProtocol::PowerShell,
        ];

        #[cfg(target_os = "windows")]
        methods.push(ExecutionProtocol::Wmi);

        methods
    }

    /// Test connectivity to a target endpoint
    pub async fn test_connectivity(
        &self,
        endpoint: &str,
        method: &ExecutionProtocol,
    ) -> nexus_common::Result<bool> {
        let test_request = ExecutionRequest {
            target_endpoint: endpoint.to_string(),
            execution_method: method.clone(),
            command: self.get_test_command(),
            parameters: HashMap::new(),
            timeout: Some(10), // Quick timeout for connectivity test
            credentials: None,
            fallback_methods: vec![],
        };

        match self.execute_with_method(&test_request, method).await {
            Ok(result) => Ok(result.success),
            Err(_) => Ok(false),
        }
    }

    /// Get platform-appropriate test command
    fn get_test_command(&self) -> String {
        #[cfg(target_os = "windows")]
        return "echo test".to_string();

        #[cfg(not(target_os = "windows"))]
        return "echo test".to_string();
    }

    /// Create execution task for nexus-agent integration
    pub fn create_execution_task(
        &self,
        endpoint: String,
        command: String,
        method: ExecutionProtocol,
        credentials: Option<ExecutionCredentials>,
    ) -> nexus_common::TaskData {
        let mut parameters = HashMap::new();
        parameters.insert("endpoint".to_string(), endpoint);
        parameters.insert("command".to_string(), command);
        parameters.insert("method".to_string(), format!("{:?}", method));

        if let Some(creds) = credentials {
            parameters.insert(
                "credentials".to_string(),
                serde_json::to_string(&creds).unwrap_or_default(),
            );
        }

        nexus_common::TaskData {
            task_id: uuid::Uuid::new_v4().to_string(),
            task_type: "hybrid_execution".to_string(),
            command: "execute_hybrid".to_string(),
            parameters,
            timeout: Some(self.config.default_timeout),
            priority: 150,
        }
    }

    /// Get system information from all connected endpoints
    pub async fn get_system_info(&self) -> SystemInfoCollection {
        SystemInfoCollection {
            localhost: self.get_local_system_info().await,
            supported_methods: self.get_supported_methods(),
            concurrent_jobs: self.config.max_concurrent_jobs,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get local system information
    async fn get_local_system_info(&self) -> LocalSystemInfo {
        LocalSystemInfo {
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            uptime: self.get_system_uptime().await.unwrap_or(0),
            free_memory: self.get_free_memory().await.unwrap_or(0),
            total_memory: self.get_total_memory().await.unwrap_or(0),
        }
    }

    /// Get system uptime in seconds
    async fn get_system_uptime(&self) -> nexus_common::Result<u64> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("cmd")
                .args(&["/C", "systeminfo | findstr \"System Boot Time\""])
                .output()
                .await?;

            // Parse Windows systeminfo output (simplified)
            Ok(3600) // Placeholder - would need proper parsing
        }

        #[cfg(not(target_os = "windows"))]
        {
            let output = Command::new("cat").args(&["/proc/uptime"]).output().await?;

            let uptime_str = String::from_utf8_lossy(&output.stdout);
            let uptime_float: f64 = uptime_str
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);

            Ok(uptime_float as u64)
        }
    }

    /// Get free memory in MB
    async fn get_free_memory(&self) -> nexus_common::Result<u64> {
        // Simplified implementation - would need platform-specific logic
        Ok(1024) // Placeholder
    }

    /// Get total memory in MB
    async fn get_total_memory(&self) -> nexus_common::Result<u64> {
        // Simplified implementation - would need platform-specific logic
        Ok(8192) // Placeholder
    }
}

/// System information collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfoCollection {
    pub localhost: LocalSystemInfo,
    pub supported_methods: Vec<ExecutionProtocol>,
    pub concurrent_jobs: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Local system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSystemInfo {
    pub platform: String,
    pub arch: String,
    pub hostname: String,
    pub uptime: u64,
    pub free_memory: u64,
    pub total_memory: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridExecConfig::default();
        assert_eq!(config.default_timeout, 60);
        assert_eq!(config.max_concurrent_jobs, 5);
        assert!(config.fallback_enabled);
        assert!(!config.execution_protocols.is_empty());
    }

    #[test]
    fn test_execution_request_serialization() {
        let request = ExecutionRequest {
            target_endpoint: "192.168.1.100".to_string(),
            execution_method: ExecutionProtocol::Ssh,
            command: "whoami".to_string(),
            parameters: HashMap::new(),
            timeout: Some(30),
            credentials: None,
            fallback_methods: vec![ExecutionProtocol::Api],
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ExecutionRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request.target_endpoint, deserialized.target_endpoint);
        assert_eq!(request.execution_method, deserialized.execution_method);
        assert_eq!(request.command, deserialized.command);
    }

    #[tokio::test]
    async fn test_local_execution() {
        let config = HybridExecConfig::default();
        let executor = HybridExecutor::new(config).unwrap();

        let request = ExecutionRequest {
            target_endpoint: "localhost".to_string(),
            execution_method: ExecutionProtocol::Local,
            command: "echo test".to_string(),
            parameters: HashMap::new(),
            timeout: Some(10),
            credentials: None,
            fallback_methods: vec![],
        };

        let result = executor.execute_local(&request).await;
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(execution_result.output.contains("test"));
    }

    #[test]
    fn test_supported_methods() {
        let config = HybridExecConfig::default();
        let executor = HybridExecutor::new(config).unwrap();

        let methods = executor.get_supported_methods();
        assert!(methods.contains(&ExecutionProtocol::Local));
        assert!(methods.contains(&ExecutionProtocol::Ssh));
        assert!(methods.contains(&ExecutionProtocol::Api));
    }
}
