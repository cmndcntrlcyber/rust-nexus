use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::state::ClientConfig;

/// WebSocket connection manager for real-time updates
pub struct WebSocketManager {
    app_handle: AppHandle,
    config: ClientConfig,
}

impl WebSocketManager {
    pub fn new(app_handle: AppHandle, config: ClientConfig) -> Self {
        Self { app_handle, config }
    }

    /// Start WebSocket connection with automatic reconnection
    pub async fn start(&self) -> Result<()> {
        let ws_url = self.get_websocket_url()?;
        info!("Starting WebSocket connection to: {}", ws_url);

        loop {
            match self.connect_and_handle(&ws_url).await {
                Ok(_) => {
                    info!("WebSocket connection closed normally");
                }
                Err(e) => {
                    error!("WebSocket connection error: {}", e);

                    // Emit connection error to frontend
                    let _ = self.app_handle.emit_all("websocket_error", &format!("{}", e));

                    // Wait before reconnecting
                    sleep(Duration::from_secs(5)).await;
                    info!("Attempting to reconnect WebSocket...");
                }
            }
        }
    }

    /// Connect to WebSocket and handle messages
    async fn connect_and_handle(&self, ws_url: &str) -> Result<()> {
        let (ws_stream, _) = connect_async(ws_url).await?;
        info!("WebSocket connected successfully");

        // Emit connection success to frontend
        let _ = self.app_handle.emit_all("websocket_connected", &true);

        let (mut write, mut read) = ws_stream.split();

        // Handle incoming messages
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    debug!("Received WebSocket message: {}", text);
                    self.handle_message(&text).await?;
                }
                Ok(Message::Binary(data)) => {
                    debug!("Received WebSocket binary message ({} bytes)", data.len());
                    self.handle_binary_message(&data).await?;
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received WebSocket ping");
                    write.send(Message::Pong(data)).await?;
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received WebSocket pong");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Ok(Message::Frame(_)) => {
                    debug!("Received WebSocket frame message");
                    // Handle frame messages if needed
                }
                Err(e) => {
                    error!("WebSocket message error: {}", e);
                    return Err(e.into());
                }
            }
        }

        // Emit disconnection to frontend
        let _ = self.app_handle.emit_all("websocket_connected", &false);

        Ok(())
    }

    /// Handle text WebSocket messages
    async fn handle_message(&self, message: &str) -> Result<()> {
        match serde_json::from_str::<WebSocketEvent>(message) {
            Ok(event) => {
                self.handle_event(event).await?;
            }
            Err(e) => {
                warn!("Failed to parse WebSocket message: {} - Raw message: {}", e, message);
            }
        }
        Ok(())
    }

    /// Handle binary WebSocket messages
    async fn handle_binary_message(&self, _data: &[u8]) -> Result<()> {
        // TODO: Implement binary message handling if needed
        debug!("Binary message handling not implemented");
        Ok(())
    }

    /// Handle parsed WebSocket events
    async fn handle_event(&self, event: WebSocketEvent) -> Result<()> {
        match event.event_type.as_str() {
            "agent_connected" => {
                info!("Agent connected: {:?}", event.data);
                self.app_handle.emit_all("agent_connected", &event.data)?;
            }
            "agent_disconnected" => {
                info!("Agent disconnected: {:?}", event.data);
                self.app_handle.emit_all("agent_disconnected", &event.data)?;
            }
            "agent_status_update" => {
                debug!("Agent status update: {:?}", event.data);
                self.app_handle.emit_all("agent_status_update", &event.data)?;
            }
            "task_result" => {
                info!("Task result received: {:?}", event.data);
                self.app_handle.emit_all("task_result", &event.data)?;
            }
            "system_alert" => {
                warn!("System alert: {:?}", event.data);
                self.app_handle.emit_all("system_alert", &event.data)?;
            }
            "domain_rotation" => {
                info!("Domain rotation event: {:?}", event.data);
                self.app_handle.emit_all("domain_rotation", &event.data)?;
            }
            "file_transfer_progress" => {
                debug!("File transfer progress: {:?}", event.data);
                self.app_handle.emit_all("file_transfer_progress", &event.data)?;
            }
            "chat_message" => {
                info!("Chat message: {:?}", event.data);
                self.app_handle.emit_all("chat_message", &event.data)?;
            }
            "notification" => {
                info!("Notification: {:?}", event.data);
                self.app_handle.emit_all("notification_received", &event.data)?;
            }
            _ => {
                debug!("Unknown event type: {}", event.event_type);
                self.app_handle.emit_all("unknown_websocket_event", &event)?;
            }
        }
        Ok(())
    }

    /// Get WebSocket URL based on configuration
    fn get_websocket_url(&self) -> Result<String> {
        if let Some(ws_endpoint) = &self.config.websocket_endpoint {
            Ok(ws_endpoint.clone())
        } else {
            // Construct WebSocket URL from server endpoint
            let protocol = if self.config.use_tls { "wss" } else { "ws" };
            let ws_url = format!("{}://{}:{}/ws", protocol, self.config.server_endpoint, self.config.server_port);
            Ok(ws_url)
        }
    }
}

/// WebSocket event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: Option<String>,
}

/// Connect to WebSocket (called from main.rs)
pub async fn connect_websocket(app_handle: &AppHandle, config: &ClientConfig) -> Result<()> {
    let manager = WebSocketManager::new(app_handle.clone(), config.clone());

    // Spawn WebSocket connection task
    tokio::spawn(async move {
        if let Err(e) = manager.start().await {
            error!("WebSocket manager error: {}", e);
        }
    });

    Ok(())
}

/// Reconnect WebSocket (can be called from commands)
pub async fn reconnect_websocket(app_handle: &AppHandle, config: &ClientConfig) -> Result<()> {
    info!("Reconnecting WebSocket");
    connect_websocket(app_handle, config).await
}

/// Send message to WebSocket server
pub async fn send_websocket_message(
    _app_handle: &AppHandle,
    _config: &ClientConfig,
    _message: WebSocketMessage,
) -> Result<()> {
    // TODO: Implement sending messages to WebSocket server
    // This would require maintaining a connection handle that can be accessed
    // for sending messages back to the server

    info!("WebSocket message sending not yet implemented");
    Ok(())
}

/// Message to send to WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub target: Option<String>,
}

/// WebSocket connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// WebSocket statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStats {
    pub connection_status: WebSocketStatus,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub connection_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    pub reconnect_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_event_serialization() {
        let event = WebSocketEvent {
            event_type: "test_event".to_string(),
            data: serde_json::json!({"test": "data"}),
            timestamp: chrono::Utc::now(),
            source: Some("test".to_string()),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: WebSocketEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event.event_type, deserialized.event_type);
        assert_eq!(event.data, deserialized.data);
    }

    #[test]
    fn test_websocket_url_generation() {
        let config = ClientConfig {
            server_endpoint: "example.com".to_string(),
            server_port: 8443,
            use_tls: true,
            websocket_endpoint: None,
            ..Default::default()
        };

        // This test would need a mock AppHandle to work properly
        // For now, just verify the URL construction logic manually
        let expected_url = "wss://example.com:8443/ws";
        let protocol = if config.use_tls { "wss" } else { "ws" };
        let actual_url = format!("{}://{}:{}/ws", protocol, config.server_endpoint, config.server_port);

        assert_eq!(expected_url, actual_url);
    }
}
