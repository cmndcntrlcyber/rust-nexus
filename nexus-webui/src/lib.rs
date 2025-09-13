//! Nexus Web UI Module
//!
//! Provides a modern web-based management interface for the rust-nexus C2 framework.
//! Integrates tauri-executor's web interface technology with rust-nexus's enterprise features.

use std::sync::Arc;
use warp::Filter;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use log::{info, warn, error, debug};

pub mod handlers;
pub mod websocket;
pub mod static_files;
pub mod templates;

use nexus_common::*;
// Temporarily commented out due to nexus-infra compilation issues
// use nexus_infra::{grpc_client::GrpcClient, domain_manager::DomainManager};

// Placeholder types for integration testing
pub struct GrpcClient;
pub struct DomainManager;

impl GrpcClient {
    pub fn new() -> Self { Self }
}

impl DomainManager {
    pub fn new() -> Self { Self }
}

/// Web UI server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebUIConfig {
    pub bind_address: String,
    pub port: u16,
    pub enable_websockets: bool,
    pub static_files_path: Option<String>,
    pub grpc_endpoint: String,
    pub cors_origins: Vec<String>,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            enable_websockets: true,
            static_files_path: None,
            grpc_endpoint: "https://127.0.0.1:8443".to_string(),
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// Web UI server state
#[derive(Clone)]
pub struct WebUIState {
    pub config: WebUIConfig,
    pub grpc_client: Arc<GrpcClient>,
    pub domain_manager: Arc<DomainManager>,
    pub agent_connections: Arc<RwLock<std::collections::HashMap<String, AgentConnection>>>,
    pub broadcast_tx: broadcast::Sender<WebUIEvent>,
}

/// Agent connection information for web UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConnection {
    pub agent_id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub status: AgentStatus,
    pub capabilities: Vec<String>,
}

/// Agent status for web UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Inactive,
    Executing,
    Error(String),
}

/// Web UI events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebUIEvent {
    AgentConnected(AgentConnection),
    AgentDisconnected(String),
    AgentStatusUpdate { agent_id: String, status: AgentStatus },
    TaskResult { agent_id: String, task_id: String, result: String },
    SystemAlert { level: String, message: String },
    DomainRotation { old_domain: String, new_domain: String },
}

/// Main Web UI server
pub struct WebUIServer {
    state: WebUIState,
}

impl WebUIServer {
    /// Create a new Web UI server
    pub fn new(
        config: WebUIConfig,
        grpc_client: Arc<GrpcClient>,
        domain_manager: Arc<DomainManager>,
    ) -> Result<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);

        let state = WebUIState {
            config,
            grpc_client,
            domain_manager,
            agent_connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
            broadcast_tx,
        };

        Ok(Self { state })
    }

    /// Start the web UI server
    pub async fn start(self) -> Result<()> {
        let state = self.state.clone();

        info!("Starting Nexus Web UI server on {}:{}", state.config.bind_address, state.config.port);

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&serde_json::json!({
                    "status": "healthy",
                    "service": "nexus-webui",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            });

        // API routes
        let api_routes = self.build_api_routes().await;

        // WebSocket routes (always enabled for now)
        let ws_routes = self.build_websocket_routes().await;

        // Static files routes
        let static_routes = self.build_static_routes().await;

        // CORS configuration
        let cors = warp::cors()
            .allow_origins(state.config.cors_origins.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec!["content-type", "authorization"])
            .build();

        // Combine all routes
        let routes = health
            .or(api_routes)
            .or(ws_routes)
            .or(static_routes)
            .with(cors)
            .with(warp::log("nexus-webui"));

        // Start the server
        let addr = format!("{}:{}", state.config.bind_address, state.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| NexusError::ConfigurationError(format!("Invalid bind address: {}", e)))?;

        warp::serve(routes)
            .run(addr)
            .await;

        Ok(())
    }

    /// Build API routes
    async fn build_api_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let state = self.state.clone();

        // Agent management routes
        let agents_list = warp::path!("api" / "agents")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(handlers::list_agents);

        let agent_details = warp::path!("api" / "agents" / String)
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(handlers::get_agent_details);

        let execute_task = warp::path!("api" / "agents" / String / "tasks")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(handlers::execute_task);

        // Infrastructure management routes
        let domains_list = warp::path!("api" / "domains")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(handlers::list_domains);

        let rotate_domain = warp::path!("api" / "domains" / "rotate")
            .and(warp::post())
            .and(with_state(state.clone()))
            .and_then(handlers::rotate_domain);

        // System information routes
        let system_info = warp::path!("api" / "system")
            .and(warp::get())
            .and(with_state(state))
            .and_then(handlers::get_system_info);

        agents_list
            .or(agent_details)
            .or(execute_task)
            .or(domains_list)
            .or(rotate_domain)
            .or(system_info)
            .boxed()
    }

    /// Build WebSocket routes for real-time updates
    async fn build_websocket_routes(&self) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        let state = self.state.clone();

        warp::path("ws")
            .and(warp::ws())
            .and(with_state(state))
            .and_then(websocket::handle_websocket)
            .boxed()
    }

    /// Build static file routes
    async fn build_static_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        // Serve embedded static files (filesystem serving temporarily disabled for type compatibility)
        static_files::embedded_files()
    }

    /// Broadcast an event to all connected WebSocket clients
    pub async fn broadcast_event(&self, event: WebUIEvent) {
        if let Err(e) = self.state.broadcast_tx.send(event) {
            warn!("Failed to broadcast event: {}", e);
        }
    }

    /// Update agent connection status
    pub async fn update_agent(&self, agent: AgentConnection) {
        let mut connections = self.state.agent_connections.write().await;
        let agent_id = agent.agent_id.clone();
        connections.insert(agent_id.clone(), agent.clone());
        drop(connections);

        // Broadcast the update
        self.broadcast_event(WebUIEvent::AgentConnected(agent)).await;
    }

    /// Remove disconnected agent
    pub async fn remove_agent(&self, agent_id: &str) {
        let mut connections = self.state.agent_connections.write().await;
        connections.remove(agent_id);
        drop(connections);

        // Broadcast the disconnection
        self.broadcast_event(WebUIEvent::AgentDisconnected(agent_id.to_string())).await;
    }
}

/// Helper function to inject state into warp filters
fn with_state(
    state: WebUIState,
) -> impl Filter<Extract = (WebUIState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

/// Task execution request from web UI
#[derive(Debug, Deserialize)]
pub struct TaskExecutionRequest {
    pub task_type: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub timeout: Option<u64>,
    pub priority: Option<u32>,
}

/// Task execution response for web UI
#[derive(Debug, Serialize)]
pub struct TaskExecutionResponse {
    pub task_id: String,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webui_config_default() {
        let config = WebUIConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert!(config.enable_websockets);
    }

    #[test]
    fn test_agent_connection_serialization() {
        let agent = AgentConnection {
            agent_id: "test-agent".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            platform: "windows".to_string(),
            last_seen: chrono::Utc::now(),
            status: AgentStatus::Active,
            capabilities: vec!["shell".to_string(), "file_ops".to_string()],
        };

        let serialized = serde_json::to_string(&agent).unwrap();
        let deserialized: AgentConnection = serde_json::from_str(&serialized).unwrap();

        assert_eq!(agent.agent_id, deserialized.agent_id);
        assert_eq!(agent.hostname, deserialized.hostname);
    }
}
