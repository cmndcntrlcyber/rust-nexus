use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tonic::Status;
use tonic::Request;

use crate::state::ClientConfig;

// Import generated gRPC client
use nexus_infra::proto::{
    nexus_c2_client::NexusC2Client, BofRequest, FileChunk, TaskRequest,
    Task as ProtoTask, BofArgument,
};

/// gRPC client manager for connecting to nexus-server
pub struct GrpcClientManager {
    channel: Channel,
    config: ClientConfig,
}

impl GrpcClientManager {
    /// Create a new gRPC client manager
    pub async fn new(config: &ClientConfig) -> Result<Self> {
        let endpoint = if config.use_tls {
            format!("https://{}:{}", config.server_endpoint, config.server_port)
        } else {
            format!("http://{}:{}", config.server_endpoint, config.server_port)
        };

        info!("Connecting to gRPC server at: {}", endpoint);

        let mut channel_builder = Channel::from_shared(endpoint)?
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        // Configure TLS if enabled
        if config.use_tls {
            let mut tls_config = ClientTlsConfig::new();

            // Set server name for TLS verification
            if !config.server_endpoint.parse::<std::net::IpAddr>().is_ok() {
                tls_config = tls_config.domain_name(&config.server_endpoint);
            }

            // Configure client certificate if provided
            if let (Some(cert_path), Some(key_path)) = (&config.cert_path, &config.key_path) {
                let cert_pem = tokio::fs::read(cert_path).await?;
                let key_pem = tokio::fs::read(key_path).await?;
                let client_identity = Identity::from_pem(cert_pem, key_pem);
                tls_config = tls_config.identity(client_identity);
            }

            // Configure CA certificate if provided
            if let Some(ca_cert_path) = &config.ca_cert_path {
                let ca_cert_pem = tokio::fs::read(ca_cert_path).await?;
                let ca_certificate = Certificate::from_pem(ca_cert_pem);
                tls_config = tls_config.ca_certificate(ca_certificate);
            }

            channel_builder = channel_builder.tls_config(tls_config)?;
        }

        let channel = channel_builder.connect().await?;

        Ok(Self {
            channel,
            config: config.clone(),
        })
    }

    /// Test the connection to the gRPC server
    pub async fn test_connection(&self) -> Result<()> {
        debug!("Testing gRPC connection");

        // Create gRPC client
        let mut client = NexusC2Client::new(self.channel.clone());

        // Try to get server info by attempting to list agents (which should return empty list but prove connectivity)
        let request = Request::new(TaskRequest {
            agent_id: "health-check".to_string(),
            max_tasks: 0,
        });

        match client.get_tasks(request).await {
            Ok(_) => {
                info!("gRPC connection test successful");
                Ok(())
            }
            Err(e) => {
                warn!("gRPC connection test failed: {}", e);
                Err(anyhow::anyhow!("Connection test failed: {}", e))
            }
        }
    }

    /// List agents connected to the server
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        debug!("Listing agents from server");

        // Create gRPC client
        let mut client = NexusC2Client::new(self.channel.clone());

        // Note: The current proto doesn't have a direct list_agents method,
        // so we'll need to implement this by getting agent info for known agent IDs
        // For now, we'll return empty list until server-side agent tracking is implemented

        // This would be enhanced once we have a proper agent registry on the server
        info!("Agent listing via gRPC - waiting for server-side agent registry");
        Ok(vec![])
    }

    /// Execute command on specific agent
    pub async fn execute_command(&self, agent_id: &str, command: &str) -> Result<String> {
        info!("Executing command on agent {}: {}", agent_id, command);

        // Create gRPC client
        let mut client = NexusC2Client::new(self.channel.clone());

        // Create a task for command execution
        let task_id = uuid::Uuid::new_v4().to_string();
        let mut parameters = std::collections::HashMap::new();
        parameters.insert("command".to_string(), command.to_string());

        let task = ProtoTask {
            task_id: task_id.clone(),
            task_type: 1, // TaskType::TASK_TYPE_SHELL_COMMAND
            command: command.to_string(),
            parameters,
            created_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            scheduled_for: None,
            timeout_seconds: 300, // 5 minutes timeout
            priority: 100,
            max_retries: 3,
        };

        // Note: Since the current architecture uses task queuing rather than direct execution,
        // we would need to add the task to the server's task queue for the agent
        // For now, we'll return the task ID as the execution identifier

        info!("Created command execution task: {} for agent: {}", task_id, agent_id);
        Ok(task_id)
    }

    /// Upload file to agent
    pub async fn upload_file(
        &self,
        agent_id: &str,
        local_path: &str,
        remote_path: &str,
    ) -> Result<String> {
        info!("Uploading file from {} to agent {} at {}", local_path, agent_id, remote_path);

        // Create gRPC client
        let mut client = NexusC2Client::new(self.channel.clone());

        // Read file data
        let file_data = tokio::fs::read(local_path).await
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", local_path, e))?;

        let file_size = file_data.len() as u64;
        let chunk_size = 64 * 1024; // 64KB chunks
        let filename = std::path::Path::new(remote_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("uploaded_file"))
            .to_string_lossy()
            .to_string();

        // Calculate SHA256 checksum
        let checksum = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&file_data);
            format!("{:x}", hasher.finalize())
        };

        // Create stream of file chunks - move values into closure to fix lifetime issues
        let chunks: Vec<FileChunk> = file_data
            .chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk)| {
                let offset = i * chunk_size;
                FileChunk {
                    filename: filename.clone(),
                    data: chunk.to_vec(),
                    offset: offset as u64,
                    total_size: file_size,
                    checksum: if offset + chunk.len() >= file_data.len() {
                        checksum.clone()
                    } else {
                        String::new()
                    },
                }
            })
            .collect();

        let chunk_stream = tokio_stream::iter(chunks);
        let request = Request::new(chunk_stream);

        // Upload file via streaming gRPC
        match client.upload_file(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    info!("File upload successful: {}", response.message);
                    Ok(response.file_id)
                } else {
                    Err(anyhow::anyhow!("File upload failed: {}", response.message))
                }
            }
            Err(e) => {
                warn!("File upload failed: {}", e);
                Err(anyhow::anyhow!("File upload error: {}", e))
            }
        }
    }

    /// Download file from agent
    pub async fn download_file(
        &self,
        agent_id: &str,
        remote_path: &str,
        local_path: &str,
    ) -> Result<String> {
        info!("Downloading file from agent {} at {} to {}", agent_id, remote_path, local_path);

        // TODO: Implement actual gRPC streaming file download
        // This would use server streaming for file transfer

        let transfer_id = uuid::Uuid::new_v4().to_string();
        Ok(transfer_id)
    }

    /// Execute BOF on agent
    pub async fn execute_bof(
        &self,
        agent_id: &str,
        bof_data: &[u8],
        entry_point: &str,
        arguments: &[String],
    ) -> Result<String> {
        info!("Executing BOF on agent {} with entry point: {}", agent_id, entry_point);

        // Create gRPC client
        let mut client = NexusC2Client::new(self.channel.clone());

        // Convert string arguments to BOF arguments
        let bof_arguments: Vec<BofArgument> = arguments
            .iter()
            .map(|arg| BofArgument {
                r#type: 3, // BofArgumentType::BOF_ARGUMENT_TYPE_STRING
                value: arg.as_bytes().to_vec(),
            })
            .collect();

        // Create BOF request
        let request = Request::new(BofRequest {
            agent_id: agent_id.to_string(),
            bof_data: bof_data.to_vec(),
            function_name: entry_point.to_string(),
            arguments: bof_arguments,
            options: std::collections::HashMap::new(),
        });

        // Execute BOF via gRPC
        match client.execute_bof(request).await {
            Ok(response) => {
                let response = response.into_inner();
                info!("BOF execution response: {} - {}", response.success, response.message);

                if response.success {
                    Ok(format!("BOF execution successful: {}", response.message))
                } else {
                    Err(anyhow::anyhow!("BOF execution failed: {}", response.message))
                }
            }
            Err(e) => {
                warn!("BOF execution failed: {}", e);
                Err(anyhow::anyhow!("BOF execution error: {}", e))
            }
        }
    }

    /// List files in agent directory
    pub async fn list_files(&self, agent_id: &str, path: &str) -> Result<Vec<FileInfo>> {
        debug!("Listing files on agent {} at path: {}", agent_id, path);

        // TODO: Implement actual gRPC call to list files

        Ok(vec![])
    }

    /// Get agent system information
    pub async fn get_agent_info(&self, agent_id: &str) -> Result<AgentInfo> {
        debug!("Getting agent info for: {}", agent_id);

        // TODO: Implement actual gRPC call to get agent information

        // Return mock data for now
        Ok(AgentInfo {
            id: agent_id.to_string(),
            hostname: "mock-host".to_string(),
            username: "mock-user".to_string(),
            domain: "mock-domain".to_string(),
            os: "Windows 10".to_string(),
            arch: "x86_64".to_string(),
            pid: 1234,
            elevated: false,
        })
    }

    /// Rotate domains (infrastructure management)
    pub async fn rotate_domain(&self) -> Result<String> {
        info!("Requesting domain rotation");

        // TODO: Implement actual gRPC call to rotate domains

        Ok("Domain rotation initiated".to_string())
    }

    /// Get domain status
    pub async fn get_domains(&self) -> Result<Vec<DomainStatus>> {
        debug!("Getting domain status");

        // TODO: Implement actual gRPC call to get domain information

        Ok(vec![])
    }

    /// Get certificate information
    pub async fn get_certificates(&self) -> Result<Vec<CertificateStatus>> {
        debug!("Getting certificate status");

        // TODO: Implement actual gRPC call to get certificate information

        Ok(vec![])
    }

    /// Subscribe to agent events (for real-time updates)
    pub async fn subscribe_to_events(&self) -> Result<tonic::Streaming<ServerEvent>> {
        info!("Subscribing to server events");

        // TODO: Implement actual gRPC streaming subscription
        // This would return a stream of server events for real-time updates

        Err(anyhow::anyhow!("Event subscription not yet implemented"))
    }
}

// Supporting data structures
// These would typically be generated from protobuf definitions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub domain: String,
    pub os: String,
    pub arch: String,
    pub pid: u32,
    pub elevated: bool,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub permissions: String,
}

#[derive(Debug, Clone)]
pub struct DomainStatus {
    pub domain: String,
    pub active: bool,
    pub certificate_valid: bool,
    pub last_health_check: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CertificateStatus {
    pub domain: String,
    pub issuer: String,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: chrono::DateTime<chrono::Utc>,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct ServerEvent {
    pub event_type: String,
    pub agent_id: Option<String>,
    pub data: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Helper functions for handling gRPC errors
pub fn handle_grpc_error(status: Status) -> anyhow::Error {
    match status.code() {
        tonic::Code::Unauthenticated => {
            anyhow::anyhow!("Authentication failed: {}", status.message())
        }
        tonic::Code::PermissionDenied => {
            anyhow::anyhow!("Permission denied: {}", status.message())
        }
        tonic::Code::NotFound => {
            anyhow::anyhow!("Resource not found: {}", status.message())
        }
        tonic::Code::Unavailable => {
            anyhow::anyhow!("Server unavailable: {}", status.message())
        }
        tonic::Code::DeadlineExceeded => {
            anyhow::anyhow!("Request timeout: {}", status.message())
        }
        _ => {
            anyhow::anyhow!("gRPC error [{}]: {}", status.code(), status.message())
        }
    }
}
