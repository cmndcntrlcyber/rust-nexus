//! LitterBox deployment automation
//!
//! Handles automated deployment of LitterBox sandbox instances
//! using Docker and infrastructure integration.

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{DetectionError, Result};

/// LitterBox deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Docker image to use
    pub image: String,
    /// Container name
    pub container_name: String,
    /// Port mappings (host:container)
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Environment variables
    pub environment: Vec<EnvVar>,
    /// Network mode
    pub network_mode: Option<String>,
    /// Restart policy
    pub restart_policy: RestartPolicy,
    /// Resource limits
    pub resources: ResourceLimits,
}

/// Port mapping for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host: u16,
    pub container: u16,
    pub protocol: String,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// Environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Container restart policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
    UnlessStopped,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::UnlessStopped
    }
}

/// Resource limits for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit (e.g., "2g", "512m")
    pub memory: Option<String>,
    /// CPU limit (e.g., "2.0", "0.5")
    pub cpus: Option<String>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory: Some("4g".to_string()),
            cpus: Some("2.0".to_string()),
        }
    }
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            image: "blacksnufkin/litterbox:latest".to_string(),
            container_name: "litterbox-sandbox".to_string(),
            ports: vec![
                PortMapping {
                    host: 8000,
                    container: 8000,
                    protocol: "tcp".to_string(),
                },
            ],
            volumes: vec![
                VolumeMount {
                    host_path: "/var/lib/litterbox/samples".to_string(),
                    container_path: "/app/samples".to_string(),
                    read_only: false,
                },
                VolumeMount {
                    host_path: "/var/lib/litterbox/results".to_string(),
                    container_path: "/app/results".to_string(),
                    read_only: false,
                },
            ],
            environment: vec![
                EnvVar {
                    name: "LITTERBOX_API_KEY".to_string(),
                    value: "".to_string(), // Should be set by user
                },
            ],
            network_mode: None,
            restart_policy: RestartPolicy::UnlessStopped,
            resources: ResourceLimits::default(),
        }
    }
}

/// Deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentStatus {
    /// Not deployed
    NotDeployed,
    /// Container is starting
    Starting,
    /// Container is running
    Running,
    /// Container is stopped
    Stopped,
    /// Container failed
    Failed,
    /// Unknown status
    Unknown,
}

/// LitterBox deployer
pub struct LitterBoxDeployer {
    config: DeploymentConfig,
}

impl LitterBoxDeployer {
    /// Create a new deployer with config
    pub fn new(config: DeploymentConfig) -> Self {
        Self { config }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(DeploymentConfig::default())
    }

    /// Check if Docker is available
    pub async fn check_docker(&self) -> Result<bool> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Docker not available: {}", e)))?;

        Ok(output.status.success())
    }

    /// Get current deployment status
    pub async fn get_status(&self) -> Result<DeploymentStatus> {
        let output = Command::new("docker")
            .args(["inspect", "-f", "{{.State.Status}}", &self.config.container_name])
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to check status: {}", e)))?;

        if !output.status.success() {
            return Ok(DeploymentStatus::NotDeployed);
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        match status.as_str() {
            "running" => Ok(DeploymentStatus::Running),
            "created" | "restarting" => Ok(DeploymentStatus::Starting),
            "exited" | "dead" => Ok(DeploymentStatus::Stopped),
            "paused" => Ok(DeploymentStatus::Stopped),
            _ => Ok(DeploymentStatus::Unknown),
        }
    }

    /// Deploy LitterBox container
    pub async fn deploy(&self) -> Result<()> {
        // Check if already running
        if self.get_status().await? == DeploymentStatus::Running {
            return Ok(());
        }

        // Pull the image
        self.pull_image().await?;

        // Create and start container
        self.create_container().await?;
        self.start_container().await?;

        // Wait for container to be ready
        self.wait_for_ready().await?;

        Ok(())
    }

    /// Pull the Docker image
    async fn pull_image(&self) -> Result<()> {
        let output = Command::new("docker")
            .args(["pull", &self.config.image])
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to pull image: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectionError::LitterBoxError(format!(
                "Failed to pull image: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Create the container
    async fn create_container(&self) -> Result<()> {
        // Remove existing container if any
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.config.container_name])
            .output()
            .await;

        // Build docker run arguments
        let mut args = vec![
            "create".to_string(),
            "--name".to_string(),
            self.config.container_name.clone(),
        ];

        // Add port mappings
        for port in &self.config.ports {
            args.push("-p".to_string());
            args.push(format!(
                "{}:{}/{}",
                port.host, port.container, port.protocol
            ));
        }

        // Add volume mounts
        for vol in &self.config.volumes {
            args.push("-v".to_string());
            let mount = if vol.read_only {
                format!("{}:{}:ro", vol.host_path, vol.container_path)
            } else {
                format!("{}:{}", vol.host_path, vol.container_path)
            };
            args.push(mount);
        }

        // Add environment variables
        for env in &self.config.environment {
            if !env.value.is_empty() {
                args.push("-e".to_string());
                args.push(format!("{}={}", env.name, env.value));
            }
        }

        // Add resource limits
        if let Some(ref memory) = self.config.resources.memory {
            args.push("--memory".to_string());
            args.push(memory.clone());
        }
        if let Some(ref cpus) = self.config.resources.cpus {
            args.push("--cpus".to_string());
            args.push(cpus.clone());
        }

        // Add restart policy
        args.push("--restart".to_string());
        args.push(match self.config.restart_policy {
            RestartPolicy::No => "no".to_string(),
            RestartPolicy::Always => "always".to_string(),
            RestartPolicy::OnFailure => "on-failure".to_string(),
            RestartPolicy::UnlessStopped => "unless-stopped".to_string(),
        });

        // Add network mode
        if let Some(ref network) = self.config.network_mode {
            args.push("--network".to_string());
            args.push(network.clone());
        }

        // Add image name
        args.push(self.config.image.clone());

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to create container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectionError::LitterBoxError(format!(
                "Failed to create container: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Start the container
    async fn start_container(&self) -> Result<()> {
        let output = Command::new("docker")
            .args(["start", &self.config.container_name])
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to start container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectionError::LitterBoxError(format!(
                "Failed to start container: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Wait for container to be ready
    async fn wait_for_ready(&self) -> Result<()> {
        let max_attempts = 30;
        let delay = std::time::Duration::from_secs(2);

        for attempt in 0..max_attempts {
            if self.get_status().await? == DeploymentStatus::Running {
                // Try to connect to the API
                let port = self.config.ports.first().map(|p| p.host).unwrap_or(8000);
                let url = format!("http://localhost:{}/health", port);

                if let Ok(response) = reqwest::get(&url).await {
                    if response.status().is_success() {
                        return Ok(());
                    }
                }
            }

            if attempt < max_attempts - 1 {
                tokio::time::sleep(delay).await;
            }
        }

        Err(DetectionError::LitterBoxError(
            "Container failed to become ready".to_string(),
        ))
    }

    /// Stop the container
    pub async fn stop(&self) -> Result<()> {
        let output = Command::new("docker")
            .args(["stop", &self.config.container_name])
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to stop container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectionError::LitterBoxError(format!(
                "Failed to stop container: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Remove the container
    pub async fn remove(&self) -> Result<()> {
        // Stop first if running
        let _ = self.stop().await;

        let output = Command::new("docker")
            .args(["rm", "-f", &self.config.container_name])
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to remove container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectionError::LitterBoxError(format!(
                "Failed to remove container: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Get container logs
    pub async fn get_logs(&self, tail: Option<usize>) -> Result<String> {
        let mut args = vec!["logs".to_string()];

        if let Some(n) = tail {
            args.push("--tail".to_string());
            args.push(n.to_string());
        }

        args.push(self.config.container_name.clone());

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to get logs: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("{}{}", stdout, stderr))
    }

    /// Get the API URL for the deployed instance
    pub fn get_api_url(&self) -> String {
        let port = self.config.ports.first().map(|p| p.host).unwrap_or(8000);
        format!("http://localhost:{}", port)
    }

    /// Get configuration reference
    pub fn config(&self) -> &DeploymentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeploymentConfig::default();
        assert_eq!(config.container_name, "litterbox-sandbox");
        assert!(!config.ports.is_empty());
    }

    #[test]
    fn test_deployer_creation() {
        let deployer = LitterBoxDeployer::with_defaults();
        assert_eq!(deployer.get_api_url(), "http://localhost:8000");
    }

    #[test]
    fn test_restart_policy_default() {
        let policy = RestartPolicy::default();
        assert!(matches!(policy, RestartPolicy::UnlessStopped));
    }
}
