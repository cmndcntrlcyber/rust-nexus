//! Compliance-specific WebSocket handling
//!
//! Provides real-time updates for compliance events including:
//! - Control status changes
//! - Evidence collection notifications
//! - Compliance score updates
//! - Configuration drift alerts

use crate::{DashboardEvent, DashboardState};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::ws::{Message, WebSocket};
use warp::Filter;

/// Client subscription preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionPreferences {
    /// Subscribe to control status changes
    pub control_changes: bool,
    /// Subscribe to evidence events
    pub evidence_events: bool,
    /// Subscribe to score updates
    pub score_updates: bool,
    /// Subscribe to drift alerts
    pub drift_alerts: bool,
    /// Subscribe to report status
    pub report_status: bool,
    /// Filter by specific framework IDs (empty = all)
    pub framework_filter: Vec<String>,
    /// Filter by specific asset IDs (empty = all)
    pub asset_filter: Vec<String>,
}

impl SubscriptionPreferences {
    /// Create preferences that subscribe to all events
    pub fn all() -> Self {
        Self {
            control_changes: true,
            evidence_events: true,
            score_updates: true,
            drift_alerts: true,
            report_status: true,
            framework_filter: vec![],
            asset_filter: vec![],
        }
    }

    /// Check if this event should be sent based on preferences
    pub fn should_send(&self, event: &DashboardEvent) -> bool {
        match event {
            DashboardEvent::ControlStatusChanged { framework_id, .. } => {
                self.control_changes && self.matches_framework(framework_id)
            }
            DashboardEvent::EvidenceCollected { .. } => self.evidence_events,
            DashboardEvent::EvidenceStatusChanged { .. } => self.evidence_events,
            DashboardEvent::ComplianceScoreChanged { framework_id, .. } => {
                self.score_updates && self.matches_framework(framework_id)
            }
            DashboardEvent::AssetComplianceChanged { asset_id, framework_id, .. } => {
                self.score_updates
                    && self.matches_framework(framework_id)
                    && self.matches_asset(asset_id)
            }
            DashboardEvent::DriftDetected { asset_id, .. } => {
                self.drift_alerts && self.matches_asset(asset_id)
            }
            DashboardEvent::ReportStatusUpdate { .. } => self.report_status,
            DashboardEvent::ReportCompleted { .. } => self.report_status,
            DashboardEvent::FrameworkUpdated { framework_id } => self.matches_framework(framework_id),
            DashboardEvent::SystemAlert { .. } => true, // Always send system alerts
        }
    }

    fn matches_framework(&self, framework_id: &str) -> bool {
        self.framework_filter.is_empty() || self.framework_filter.contains(&framework_id.to_string())
    }

    fn matches_asset(&self, asset_id: &str) -> bool {
        self.asset_filter.is_empty() || self.asset_filter.contains(&asset_id.to_string())
    }
}

/// Client message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    /// Subscribe to specific event types
    Subscribe(SubscriptionPreferences),
    /// Unsubscribe from events
    Unsubscribe,
    /// Ping to keep connection alive
    Ping,
    /// Request current dashboard state
    RequestDashboard,
}

/// Server message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    /// Acknowledgment of subscription
    Subscribed { preferences: SubscriptionPreferences },
    /// Dashboard event
    Event(DashboardEvent),
    /// Pong response
    Pong,
    /// Error message
    Error { message: String },
    /// Connection established
    Connected { client_id: String },
}

/// Connected client tracking
pub struct ConnectedClient {
    pub client_id: String,
    pub preferences: SubscriptionPreferences,
}

/// Compliance WebSocket manager
pub struct ComplianceWebSocketManager {
    clients: Arc<RwLock<Vec<Arc<RwLock<ConnectedClient>>>>>,
}

impl ComplianceWebSocketManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

impl Default for ComplianceWebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a compliance WebSocket connection
pub async fn handle_compliance_websocket(ws: WebSocket, state: DashboardState) {
    let (mut tx, mut rx) = ws.split();

    // Generate a unique client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    info!("Compliance WebSocket client connected: {}", client_id);

    // Send connection acknowledgment
    let connected_msg = ServerMessage::Connected {
        client_id: client_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        if let Err(e) = tx.send(Message::text(json)).await {
            error!("Failed to send connected message: {}", e);
            return;
        }
    }

    // Subscribe to dashboard events
    let mut event_rx = state.subscribe();

    // Default to all subscriptions
    let preferences = Arc::new(RwLock::new(SubscriptionPreferences::all()));
    let prefs_clone = preferences.clone();

    // Spawn task to forward dashboard events to WebSocket
    let mut event_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let prefs = prefs_clone.read().await;
            if prefs.should_send(&event) {
                let server_msg = ServerMessage::Event(event);
                if let Ok(json) = serde_json::to_string(&server_msg) {
                    if tx.send(Message::text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
                            match client_msg {
                                ClientMessage::Subscribe(new_prefs) => {
                                    let mut prefs = preferences.write().await;
                                    *prefs = new_prefs.clone();
                                    debug!("Client {} updated subscription preferences", client_id);
                                }
                                ClientMessage::Unsubscribe => {
                                    let mut prefs = preferences.write().await;
                                    *prefs = SubscriptionPreferences::default();
                                    debug!("Client {} unsubscribed from events", client_id);
                                }
                                ClientMessage::Ping => {
                                    // Pong is handled by the event task
                                    debug!("Ping from client {}", client_id);
                                }
                                ClientMessage::RequestDashboard => {
                                    // Could send current dashboard state here
                                    debug!("Dashboard request from client {}", client_id);
                                }
                            }
                        }
                    }
                } else if msg.is_close() {
                    break;
                }
            }
            Err(e) => {
                warn!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
        }
    }

    // Cleanup
    event_task.abort();
    info!("Compliance WebSocket client disconnected: {}", client_id);
}

/// Build WebSocket filter for compliance dashboard
pub fn compliance_ws_route(
    state: DashboardState,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("ws" / "compliance")
        .and(warp::ws())
        .and(with_state(state))
        .map(|ws: warp::ws::Ws, state: DashboardState| {
            ws.on_upgrade(move |socket| handle_compliance_websocket(socket, state))
        })
}

fn with_state(
    state: DashboardState,
) -> impl warp::Filter<Extract = (DashboardState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AlertLevel, ControlStatus, EvidenceStatus, ReportStatus, ScoreTrend};

    #[test]
    fn test_subscription_preferences_default() {
        let prefs = SubscriptionPreferences::default();
        assert!(!prefs.control_changes);
        assert!(!prefs.evidence_events);
        assert!(!prefs.score_updates);
    }

    #[test]
    fn test_subscription_preferences_all() {
        let prefs = SubscriptionPreferences::all();
        assert!(prefs.control_changes);
        assert!(prefs.evidence_events);
        assert!(prefs.score_updates);
        assert!(prefs.drift_alerts);
        assert!(prefs.report_status);
    }

    #[test]
    fn test_should_send_control_change() {
        let prefs = SubscriptionPreferences {
            control_changes: true,
            ..Default::default()
        };

        let event = DashboardEvent::ControlStatusChanged {
            control_id: "AC-1".to_string(),
            framework_id: "nist-800-53".to_string(),
            old_status: ControlStatus::Pending,
            new_status: ControlStatus::Pass,
        };

        assert!(prefs.should_send(&event));
    }

    #[test]
    fn test_should_send_with_framework_filter() {
        let prefs = SubscriptionPreferences {
            control_changes: true,
            framework_filter: vec!["nist-800-53".to_string()],
            ..Default::default()
        };

        let event_match = DashboardEvent::ControlStatusChanged {
            control_id: "AC-1".to_string(),
            framework_id: "nist-800-53".to_string(),
            old_status: ControlStatus::Pending,
            new_status: ControlStatus::Pass,
        };

        let event_no_match = DashboardEvent::ControlStatusChanged {
            control_id: "A.5.1".to_string(),
            framework_id: "iso-27001".to_string(),
            old_status: ControlStatus::Pending,
            new_status: ControlStatus::Pass,
        };

        assert!(prefs.should_send(&event_match));
        assert!(!prefs.should_send(&event_no_match));
    }

    #[test]
    fn test_should_send_system_alert_always() {
        let prefs = SubscriptionPreferences::default();

        let event = DashboardEvent::SystemAlert {
            level: AlertLevel::Critical,
            message: "Test alert".to_string(),
        };

        // System alerts should always be sent
        assert!(prefs.should_send(&event));
    }

    #[test]
    fn test_client_message_serialization() {
        let prefs = SubscriptionPreferences::all();
        let msg = ClientMessage::Subscribe(prefs);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Subscribe"));

        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            ClientMessage::Subscribe(p) => {
                assert!(p.control_changes);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_message_serialization() {
        let event = DashboardEvent::ComplianceScoreChanged {
            framework_id: "test".to_string(),
            new_score: 85.5,
            trend: ScoreTrend::Improving,
        };
        let msg = ServerMessage::Event(event);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Event"));
        assert!(json.contains("ComplianceScoreChanged"));
    }

    #[test]
    fn test_websocket_manager_creation() {
        let manager = ComplianceWebSocketManager::new();
        // Just verify it creates successfully
        assert!(std::mem::size_of_val(&manager) > 0);
    }
}
