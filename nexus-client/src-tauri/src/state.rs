use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

use nexus_common::tasks::TaskResult;
use crate::grpc_client::AgentInfo;

/// Main application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Option<ClientConfig>,
    pub connection_status: ConnectionStatus,
    pub agents: HashMap<String, AgentSession>,
    pub task_history: Vec<TaskHistoryEntry>,
    pub bof_library: HashMap<String, BofEntry>,
    pub notifications: Vec<NotificationEntry>,
    pub chat_messages: Vec<ChatMessage>,
}

impl AppState {
    pub async fn new() -> Self {
        Self {
            config: None,
            connection_status: ConnectionStatus::Disconnected,
            agents: HashMap::new(),
            task_history: Vec::new(),
            bof_library: HashMap::new(),
            notifications: Vec::new(),
            chat_messages: Vec::new(),
        }
    }

    pub fn add_agent(&mut self, agent: AgentSession) {
        self.agents.insert(agent.id.clone(), agent);
    }

    pub fn remove_agent(&mut self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    pub fn update_agent_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = status;
            agent.last_seen = Utc::now();
        }
    }

    pub fn add_task_history(&mut self, entry: TaskHistoryEntry) {
        self.task_history.push(entry);

        // Keep only the last 1000 entries
        if self.task_history.len() > 1000 {
            self.task_history.drain(0..self.task_history.len() - 1000);
        }
    }

    pub fn add_notification(&mut self, notification: NotificationEntry) {
        self.notifications.push(notification);

        // Keep only the last 100 notifications
        if self.notifications.len() > 100 {
            self.notifications.drain(0..self.notifications.len() - 100);
        }
    }

    pub fn add_chat_message(&mut self, message: ChatMessage) {
        self.chat_messages.push(message);
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server_endpoint: String,
    pub server_port: u16,
    pub use_tls: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_cert_path: Option<String>,
    pub username: String,
    pub team_name: String,
    pub auto_connect: bool,
    pub websocket_endpoint: Option<String>,
    pub update_interval_ms: u64,
    pub max_concurrent_tasks: u32,
    pub log_level: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_endpoint: "127.0.0.1".to_string(),
            server_port: 8443,
            use_tls: true,
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            username: "operator".to_string(),
            team_name: "red_team".to_string(),
            auto_connect: false,
            websocket_endpoint: None,
            update_interval_ms: 5000,
            max_concurrent_tasks: 10,
            log_level: "info".to_string(),
        }
    }
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Agent session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_info: AgentInfo,
    pub status: AgentStatus,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub task_queue: Vec<String>,
    pub active_tasks: HashMap<String, ActiveTask>,
    pub file_browser_path: String,
    pub shell_history: Vec<String>,
    pub notes: String,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Inactive,
    Executing,
    Error(String),
    Disconnected,
}

/// Active task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTask {
    pub task_id: String,
    pub task_type: String,
    pub started_at: DateTime<Utc>,
    pub timeout: Option<DateTime<Utc>>,
    pub parameters: HashMap<String, String>,
}

/// Task history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryEntry {
    pub id: String,
    pub agent_id: String,
    pub task_type: String,
    pub command: String,
    pub executed_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
    pub success: bool,
    pub duration_ms: Option<u64>,
}

/// BOF library entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub file_path: String,
    pub entry_point: String,
    pub parameters: Vec<BofParameter>,
    pub added_at: DateTime<Utc>,
    pub usage_count: u32,
}

/// BOF parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Notification entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: String,
    pub level: NotificationLevel,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub read: bool,
    pub source: String,
}

/// Notification levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
    Critical,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub username: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Chat,
    System,
    Command,
    Error,
    Success,
}

/// Create system tray
pub fn create_system_tray() -> SystemTray {
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide Window");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let separator = SystemTrayMenuItem::Separator;

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(separator)
        .add_item(quit);

    SystemTray::new().with_menu(tray_menu)
}

/// Handle system tray events
pub fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick {
            position: _,
            size: _,
            ..
        } => {
            info!("System tray received a left click");
            let window = app.get_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
        }
        SystemTrayEvent::RightClick {
            position: _,
            size: _,
            ..
        } => {
            info!("System tray received a right click");
        }
        SystemTrayEvent::DoubleClick {
            position: _,
            size: _,
            ..
        } => {
            info!("System tray received a double click");
            let window = app.get_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "hide" => {
                    let window = app.get_window("main").unwrap();
                    window.hide().unwrap();
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// File transfer progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferProgress {
    pub transfer_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: Option<u64>,
    pub status: TransferStatus,
}

/// Transfer status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Starting,
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// Domain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    pub domain: String,
    pub status: DomainStatus,
    pub certificate_valid: bool,
    pub certificate_expiry: Option<DateTime<Utc>>,
    pub last_health_check: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
}

/// Domain status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainStatus {
    Active,
    Inactive,
    Error(String),
    Rotating,
}
