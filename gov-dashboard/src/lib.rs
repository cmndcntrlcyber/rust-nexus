//! Gov-Dashboard Module
//!
//! Provides a modern web-based compliance monitoring dashboard for the gov-nexus platform.
//! Supports real-time compliance tracking, framework management, and audit reporting.

use std::sync::Arc;
use warp::Filter;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use log::{info, warn, error, debug};

pub mod compliance_routes;
pub mod compliance_websocket;
pub mod handlers;
pub mod models;
pub mod websocket;
pub mod static_files;
pub mod templates;

pub use compliance_routes::*;
pub use compliance_websocket::*;
pub use models::*;

use gov_common::*;
// Temporarily commented out due to nexus-infra compilation issues
// use gov_infra::{grpc_client::GrpcClient, domain_manager::DomainManager};

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

// ============================================================================
// Compliance Dashboard Configuration and State
// ============================================================================

/// Dashboard server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Enable WebSocket real-time updates
    pub enable_websockets: bool,
    /// Static files directory path
    pub static_files_path: Option<String>,
    /// API endpoint for gov-api service
    pub api_endpoint: String,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Default page size for pagination
    pub default_page_size: usize,
    /// Enable audit logging
    pub enable_audit_log: bool,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            enable_websockets: true,
            static_files_path: None,
            api_endpoint: "http://127.0.0.1:3000".to_string(),
            cors_origins: vec!["*".to_string()],
            default_page_size: 25,
            enable_audit_log: true,
            session_timeout_secs: 3600,
        }
    }
}

/// Dashboard server state for compliance monitoring
#[derive(Clone)]
pub struct DashboardState {
    /// Server configuration
    pub config: DashboardConfig,
    /// Cached frameworks
    pub frameworks: Arc<RwLock<Vec<Framework>>>,
    /// Cached controls by framework ID
    pub controls: Arc<RwLock<std::collections::HashMap<String, Vec<Control>>>>,
    /// Cached assets
    pub assets: Arc<RwLock<Vec<Asset>>>,
    /// Evidence by control ID
    pub evidence: Arc<RwLock<std::collections::HashMap<String, Vec<Evidence>>>>,
    /// Compliance scores by framework ID
    pub compliance_scores: Arc<RwLock<std::collections::HashMap<String, ComplianceScore>>>,
    /// Report generation jobs
    pub report_jobs: Arc<RwLock<Vec<ReportJob>>>,
    /// Broadcast channel for real-time events
    pub broadcast_tx: broadcast::Sender<DashboardEvent>,
}

impl DashboardState {
    /// Create a new dashboard state
    pub fn new(config: DashboardConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Self {
            config,
            frameworks: Arc::new(RwLock::new(Vec::new())),
            controls: Arc::new(RwLock::new(std::collections::HashMap::new())),
            assets: Arc::new(RwLock::new(Vec::new())),
            evidence: Arc::new(RwLock::new(std::collections::HashMap::new())),
            compliance_scores: Arc::new(RwLock::new(std::collections::HashMap::new())),
            report_jobs: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
        }
    }

    /// Get a subscriber for dashboard events
    pub fn subscribe(&self) -> broadcast::Receiver<DashboardEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: DashboardEvent) {
        if let Err(e) = self.broadcast_tx.send(event) {
            warn!("Failed to broadcast dashboard event: {}", e);
        }
    }
}

/// Dashboard events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DashboardEvent {
    /// Framework data updated
    FrameworkUpdated { framework_id: String },
    /// Control status changed
    ControlStatusChanged {
        control_id: String,
        framework_id: String,
        old_status: ControlStatus,
        new_status: ControlStatus,
    },
    /// New evidence collected
    EvidenceCollected {
        evidence_id: String,
        control_id: String,
    },
    /// Evidence status changed
    EvidenceStatusChanged {
        evidence_id: String,
        new_status: EvidenceStatus,
    },
    /// Compliance score changed
    ComplianceScoreChanged {
        framework_id: String,
        new_score: f64,
        trend: ScoreTrend,
    },
    /// Asset compliance status changed
    AssetComplianceChanged {
        asset_id: String,
        framework_id: String,
        compliant: bool,
    },
    /// Report generation status update
    ReportStatusUpdate {
        job_id: String,
        status: ReportStatus,
        progress: Option<u8>,
    },
    /// Report completed
    ReportCompleted {
        job_id: String,
        file_path: String,
    },
    /// System alert
    SystemAlert {
        level: AlertLevel,
        message: String,
    },
    /// Configuration drift detected
    DriftDetected {
        asset_id: String,
        control_id: String,
        description: String,
    },
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "Info"),
            AlertLevel::Warning => write!(f, "Warning"),
            AlertLevel::Error => write!(f, "Error"),
            AlertLevel::Critical => write!(f, "Critical"),
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

    // ============================================================================
    // Compliance Dashboard Tests
    // ============================================================================

    #[test]
    fn test_dashboard_config_default() {
        let config = DashboardConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert!(config.enable_websockets);
        assert_eq!(config.default_page_size, 25);
        assert!(config.enable_audit_log);
        assert_eq!(config.session_timeout_secs, 3600);
    }

    #[test]
    fn test_dashboard_state_creation() {
        let config = DashboardConfig::default();
        let state = DashboardState::new(config.clone());

        assert_eq!(state.config.port, config.port);
    }

    #[test]
    fn test_dashboard_event_serialization() {
        let event = DashboardEvent::ComplianceScoreChanged {
            framework_id: "nist-csf".to_string(),
            new_score: 85.5,
            trend: ScoreTrend::Improving,
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("ComplianceScoreChanged"));
        assert!(serialized.contains("nist-csf"));
    }

    #[test]
    fn test_dashboard_event_control_status() {
        let event = DashboardEvent::ControlStatusChanged {
            control_id: "AC-1".to_string(),
            framework_id: "nist-800-53".to_string(),
            old_status: ControlStatus::Pending,
            new_status: ControlStatus::Pass,
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("ControlStatusChanged"));
        assert!(serialized.contains("AC-1"));
    }

    #[test]
    fn test_dashboard_event_drift_detected() {
        let event = DashboardEvent::DriftDetected {
            asset_id: "server-001".to_string(),
            control_id: "CM-6".to_string(),
            description: "Configuration changed from baseline".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("DriftDetected"));
        assert!(serialized.contains("Configuration changed"));
    }

    #[test]
    fn test_alert_level_display() {
        assert_eq!(AlertLevel::Info.to_string(), "Info");
        assert_eq!(AlertLevel::Warning.to_string(), "Warning");
        assert_eq!(AlertLevel::Error.to_string(), "Error");
        assert_eq!(AlertLevel::Critical.to_string(), "Critical");
    }

    #[test]
    fn test_dashboard_broadcast() {
        let config = DashboardConfig::default();
        let state = DashboardState::new(config);

        let mut receiver = state.subscribe();

        state.broadcast(DashboardEvent::FrameworkUpdated {
            framework_id: "test-framework".to_string(),
        });

        // Check that event was received (non-blocking check)
        let result = receiver.try_recv();
        assert!(result.is_ok());
    }
}
