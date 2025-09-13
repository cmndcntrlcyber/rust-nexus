use anyhow::Result;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::grpc_client::GrpcClientManager;
use crate::state::{
    AgentSession, AppState, BofEntry, ChatMessage, ClientConfig, ConnectionStatus,
    DomainInfo, FileTransferProgress, NotificationEntry, NotificationLevel, TaskHistoryEntry,
};

// Type alias for the app state
type AppStateType = Arc<RwLock<AppState>>;

/// Configuration Commands

#[tauri::command]
pub async fn load_config(state: State<'_, AppStateType>) -> Result<ClientConfig, String> {
    debug!("Loading configuration");

    let config_path = "nexus-client-config.json";

    if Path::new(config_path).exists() {
        match tokio::fs::read_to_string(config_path).await {
            Ok(config_str) => {
                match serde_json::from_str::<ClientConfig>(&config_str) {
                    Ok(config) => {
                        let mut state_guard = state.write().await;
                        state_guard.config = Some(config.clone());
                        Ok(config)
                    }
                    Err(e) => {
                        error!("Failed to parse config: {}", e);
                        Err(format!("Failed to parse configuration: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to read config file: {}", e);
                Err(format!("Failed to read configuration file: {}", e))
            }
        }
    } else {
        info!("Config file not found, using defaults");
        let default_config = ClientConfig::default();
        let mut state_guard = state.write().await;
        state_guard.config = Some(default_config.clone());
        Ok(default_config)
    }
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, AppStateType>,
    config: ClientConfig,
) -> Result<(), String> {
    debug!("Saving configuration");

    match serde_json::to_string_pretty(&config) {
        Ok(config_str) => {
            match tokio::fs::write("nexus-client-config.json", config_str).await {
                Ok(_) => {
                    let mut state_guard = state.write().await;
                    state_guard.config = Some(config);
                    info!("Configuration saved successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to write config file: {}", e);
                    Err(format!("Failed to save configuration: {}", e))
                }
            }
        }
        Err(e) => {
            error!("Failed to serialize config: {}", e);
            Err(format!("Failed to serialize configuration: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppStateType>) -> Result<Option<ClientConfig>, String> {
    let state_guard = state.read().await;
    Ok(state_guard.config.clone())
}

/// Connection Commands

#[tauri::command]
pub async fn connect_to_server(
    state: State<'_, AppStateType>,
    app_handle: AppHandle,
) -> Result<(), String> {
    info!("Attempting to connect to server");

    let config = {
        let state_guard = state.read().await;
        state_guard.config.clone()
    };

    if let Some(config) = config {
        // Update connection status to connecting
        {
            let mut state_guard = state.write().await;
            state_guard.connection_status = ConnectionStatus::Connecting;
        }

        // Emit connection status update
        app_handle
            .emit_all("connection_status_changed", &ConnectionStatus::Connecting)
            .map_err(|e| format!("Failed to emit connection status: {}", e))?;

        // Initialize gRPC client
        match GrpcClientManager::new(&config).await {
            Ok(client) => {
                // Test connection
                match client.test_connection().await {
                    Ok(_) => {
                        let mut state_guard = state.write().await;
                        state_guard.connection_status = ConnectionStatus::Connected;

                        // Emit success
                        app_handle
                            .emit_all("connection_status_changed", &ConnectionStatus::Connected)
                            .map_err(|e| format!("Failed to emit connection status: {}", e))?;

                        info!("Successfully connected to server");
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Connection test failed: {}", e);
                        error!("{}", error_msg);

                        let mut state_guard = state.write().await;
                        state_guard.connection_status = ConnectionStatus::Error(error_msg.clone());

                        app_handle
                            .emit_all("connection_status_changed", &ConnectionStatus::Error(error_msg.clone()))
                            .map_err(|e| format!("Failed to emit connection status: {}", e))?;

                        Err(error_msg)
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create gRPC client: {}", e);
                error!("{}", error_msg);

                let mut state_guard = state.write().await;
                state_guard.connection_status = ConnectionStatus::Error(error_msg.clone());

                app_handle
                    .emit_all("connection_status_changed", &ConnectionStatus::Error(error_msg.clone()))
                    .map_err(|e| format!("Failed to emit connection status: {}", e))?;

                Err(error_msg)
            }
        }
    } else {
        let error_msg = "No configuration found. Please configure connection settings first.".to_string();
        error!("{}", error_msg);
        Err(error_msg)
    }
}

#[tauri::command]
pub async fn disconnect_from_server(
    state: State<'_, AppStateType>,
    app_handle: AppHandle,
) -> Result<(), String> {
    info!("Disconnecting from server");

    let mut state_guard = state.write().await;
    state_guard.connection_status = ConnectionStatus::Disconnected;
    state_guard.agents.clear();

    app_handle
        .emit_all("connection_status_changed", &ConnectionStatus::Disconnected)
        .map_err(|e| format!("Failed to emit connection status: {}", e))?;

    app_handle
        .emit_all("agents_updated", &Vec::<AgentSession>::new())
        .map_err(|e| format!("Failed to emit agents update: {}", e))?;

    info!("Disconnected from server");
    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(state: State<'_, AppStateType>) -> Result<ConnectionStatus, String> {
    let state_guard = state.read().await;
    Ok(state_guard.connection_status.clone())
}

/// Agent Management Commands

#[tauri::command]
pub async fn list_agents(state: State<'_, AppStateType>) -> Result<Vec<AgentSession>, String> {
    let state_guard = state.read().await;
    let agents: Vec<AgentSession> = state_guard.agents.values().cloned().collect();
    Ok(agents)
}

#[tauri::command]
pub async fn get_agent_details(
    state: State<'_, AppStateType>,
    agent_id: String,
) -> Result<Option<AgentSession>, String> {
    let state_guard = state.read().await;
    Ok(state_guard.agents.get(&agent_id).cloned())
}

#[tauri::command]
pub async fn interact_with_agent(
    state: State<'_, AppStateType>,
    agent_id: String,
) -> Result<(), String> {
    debug!("Starting interaction with agent: {}", agent_id);

    let state_guard = state.read().await;
    if state_guard.agents.contains_key(&agent_id) {
        // This would typically open an interaction session
        // For now, we just log the interaction
        info!("Interactive session started with agent: {}", agent_id);
        Ok(())
    } else {
        Err(format!("Agent {} not found", agent_id))
    }
}

#[tauri::command]
pub async fn execute_command(
    state: State<'_, AppStateType>,
    app_handle: AppHandle,
    agent_id: String,
    command: String,
) -> Result<String, String> {
    info!("Executing command on agent {}: {}", agent_id, command);

    let task_id = Uuid::new_v4().to_string();

    // Create task history entry
    let task_entry = TaskHistoryEntry {
        id: task_id.clone(),
        agent_id: agent_id.clone(),
        task_type: "shell_command".to_string(),
        command: command.clone(),
        executed_at: chrono::Utc::now(),
        completed_at: None,
        result: None,
        success: false,
        duration_ms: None,
    };

    // Add to task history
    {
        let mut state_guard = state.write().await;
        state_guard.add_task_history(task_entry);
    }

    // Emit task started event
    app_handle
        .emit_all("task_started", &task_id)
        .map_err(|e| format!("Failed to emit task started event: {}", e))?;

    // TODO: Implement actual gRPC command execution
    // For now, simulate command execution
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let result = format!("Command '{}' executed successfully on agent {}", command, agent_id);

    // Update task history with result
    {
        let mut state_guard = state.write().await;
        if let Some(task) = state_guard.task_history.iter_mut().find(|t| t.id == task_id) {
            task.completed_at = Some(chrono::Utc::now());
            task.success = true;
            task.duration_ms = Some(500);
        }
    }

    // Emit task completed event
    app_handle
        .emit_all("task_completed", &task_id)
        .map_err(|e| format!("Failed to emit task completed event: {}", e))?;

    Ok(result)
}

/// File Management Commands

#[tauri::command]
pub async fn list_agent_files(
    _state: State<'_, AppStateType>,
    agent_id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    debug!("Listing files for agent {} at path: {}", agent_id, path);

    // TODO: Implement actual file listing via gRPC
    // For now, return mock data
    let mock_files = vec![
        FileEntry {
            name: "document.txt".to_string(),
            path: format!("{}/document.txt", path),
            size: 1024,
            is_directory: false,
            modified: chrono::Utc::now(),
            permissions: "rw-r--r--".to_string(),
        },
        FileEntry {
            name: "subfolder".to_string(),
            path: format!("{}/subfolder", path),
            size: 0,
            is_directory: true,
            modified: chrono::Utc::now(),
            permissions: "drwxr-xr-x".to_string(),
        },
    ];

    Ok(mock_files)
}

#[tauri::command]
pub async fn upload_file_to_agent(
    _state: State<'_, AppStateType>,
    app_handle: AppHandle,
    agent_id: String,
    local_path: String,
    remote_path: String,
) -> Result<String, String> {
    info!("Uploading file from {} to agent {} at {}", local_path, agent_id, remote_path);

    let transfer_id = Uuid::new_v4().to_string();
    let transfer_id_clone = transfer_id.clone();

    // Simulate file upload progress
    tokio::spawn(async move {
        for progress in (0..=100).step_by(10) {
            let progress_data = FileTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: Path::new(&local_path).file_name()
                    .unwrap_or_default().to_string_lossy().to_string(),
                total_bytes: 10240,
                transferred_bytes: (10240 * progress / 100),
                percentage: progress as f64,
                speed_bytes_per_sec: 1024,
                eta_seconds: Some(((100 - progress) / 10) as u64),
                status: crate::state::TransferStatus::InProgress,
            };

            let _ = app_handle.emit_all("file_transfer_progress", &progress_data);
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        let completed_progress = FileTransferProgress {
            transfer_id: transfer_id.clone(),
            file_name: Path::new(&local_path).file_name()
                .unwrap_or_default().to_string_lossy().to_string(),
            total_bytes: 10240,
            transferred_bytes: 10240,
            percentage: 100.0,
            speed_bytes_per_sec: 1024,
            eta_seconds: None,
            status: crate::state::TransferStatus::Completed,
        };

        let _ = app_handle.emit_all("file_transfer_progress", &completed_progress);
    });

    Ok(transfer_id_clone)
}

#[tauri::command]
pub async fn download_file_from_agent(
    _state: State<'_, AppStateType>,
    app_handle: AppHandle,
    agent_id: String,
    remote_path: String,
    local_path: String,
) -> Result<String, String> {
    info!("Downloading file from agent {} at {} to {}", agent_id, remote_path, local_path);

    let transfer_id = Uuid::new_v4().to_string();
    let transfer_id_clone = transfer_id.clone();

    // Simulate file download progress (similar to upload)
    tokio::spawn(async move {
        for progress in (0..=100).step_by(15) {
            let progress_data = FileTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: Path::new(&remote_path).file_name()
                    .unwrap_or_default().to_string_lossy().to_string(),
                total_bytes: 5120,
                transferred_bytes: (5120 * progress / 100),
                percentage: progress as f64,
                speed_bytes_per_sec: 2048,
                eta_seconds: Some(((100 - progress) / 15) as u64),
                status: crate::state::TransferStatus::InProgress,
            };

            let _ = app_handle.emit_all("file_transfer_progress", &progress_data);
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    });

    Ok(transfer_id_clone)
}

/// Task Management Commands

#[tauri::command]
pub async fn execute_task(
    state: State<'_, AppStateType>,
    agent_id: String,
    task_type: String,
    parameters: HashMap<String, String>,
) -> Result<String, String> {
    info!("Executing task {} on agent {} with parameters: {:?}", task_type, agent_id, parameters);

    let task_id = Uuid::new_v4().to_string();

    // Create and add task to history
    let task_entry = TaskHistoryEntry {
        id: task_id.clone(),
        agent_id: agent_id.clone(),
        task_type: task_type.clone(),
        command: format!("{:?}", parameters),
        executed_at: chrono::Utc::now(),
        completed_at: None,
        result: None,
        success: false,
        duration_ms: None,
    };

    let mut state_guard = state.write().await;
    state_guard.add_task_history(task_entry);

    Ok(task_id)
}

#[tauri::command]
pub async fn get_task_results(
    state: State<'_, AppStateType>,
    task_id: String,
) -> Result<Option<TaskHistoryEntry>, String> {
    let state_guard = state.read().await;
    let task = state_guard.task_history.iter().find(|t| t.id == task_id).cloned();
    Ok(task)
}

#[tauri::command]
pub async fn list_task_history(
    state: State<'_, AppStateType>,
    agent_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<TaskHistoryEntry>, String> {
    let state_guard = state.read().await;
    let mut tasks: Vec<TaskHistoryEntry> = state_guard.task_history.iter()
        .filter(|t| agent_id.as_ref().map_or(true, |id| &t.agent_id == id))
        .cloned()
        .collect();

    tasks.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));

    if let Some(limit) = limit {
        tasks.truncate(limit);
    }

    Ok(tasks)
}

/// BOF Management Commands

#[tauri::command]
pub async fn list_available_bofs(state: State<'_, AppStateType>) -> Result<Vec<BofEntry>, String> {
    let state_guard = state.read().await;
    let bofs: Vec<BofEntry> = state_guard.bof_library.values().cloned().collect();
    Ok(bofs)
}

#[tauri::command]
pub async fn execute_bof(
    _state: State<'_, AppStateType>,
    agent_id: String,
    bof_id: String,
    arguments: Vec<String>,
) -> Result<String, String> {
    info!("Executing BOF {} on agent {} with args: {:?}", bof_id, agent_id, arguments);

    // TODO: Implement actual BOF execution via gRPC
    let task_id = Uuid::new_v4().to_string();
    Ok(task_id)
}

#[tauri::command]
pub async fn upload_bof(
    state: State<'_, AppStateType>,
    file_path: String,
    metadata: BofMetadata,
) -> Result<String, String> {
    info!("Uploading BOF from: {}", file_path);

    let bof_id = Uuid::new_v4().to_string();
    let bof_entry = BofEntry {
        id: bof_id.clone(),
        name: metadata.name,
        description: metadata.description,
        author: metadata.author,
        version: metadata.version,
        file_path: file_path.clone(),
        entry_point: metadata.entry_point,
        parameters: metadata.parameters,
        added_at: chrono::Utc::now(),
        usage_count: 0,
    };

    let mut state_guard = state.write().await;
    state_guard.bof_library.insert(bof_id.clone(), bof_entry);

    Ok(bof_id)
}

/// Infrastructure Commands

#[tauri::command]
pub async fn get_domains(_state: State<'_, AppStateType>) -> Result<Vec<DomainInfo>, String> {
    // TODO: Implement actual domain retrieval via gRPC
    let mock_domains = vec![
        DomainInfo {
            domain: "c2.example.com".to_string(),
            status: crate::state::DomainStatus::Active,
            certificate_valid: true,
            certificate_expiry: Some(chrono::Utc::now() + chrono::Duration::days(30)),
            last_health_check: chrono::Utc::now(),
            response_time_ms: Some(150),
        },
    ];

    Ok(mock_domains)
}

#[tauri::command]
pub async fn rotate_domain(_state: State<'_, AppStateType>) -> Result<String, String> {
    info!("Rotating domain");
    // TODO: Implement actual domain rotation via gRPC
    Ok("Domain rotation initiated".to_string())
}

#[tauri::command]
pub async fn get_certificates(_state: State<'_, AppStateType>) -> Result<Vec<CertificateInfo>, String> {
    // TODO: Implement certificate information retrieval
    Ok(vec![])
}

/// Certificate Management Commands

#[tauri::command]
pub async fn upload_client_certificate(
    _state: State<'_, AppStateType>,
    file_path: String,
) -> Result<String, String> {
    info!("Uploading client certificate from: {}", file_path);

    // Validate file exists
    if !Path::new(&file_path).exists() {
        return Err("Certificate file not found".to_string());
    }

    // Create certs directory if it doesn't exist
    let certs_dir = Path::new("./certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(certs_dir)
            .map_err(|e| format!("Failed to create certs directory: {}", e))?;
    }

    // Copy file to certs/client.crt
    let dest_path = certs_dir.join("client.crt");
    tokio::fs::copy(&file_path, &dest_path).await
        .map_err(|e| format!("Failed to copy certificate file: {}", e))?;

    info!("Client certificate uploaded successfully to: {:?}", dest_path);
    Ok("Client certificate uploaded successfully".to_string())
}

#[tauri::command]
pub async fn upload_client_key(
    _state: State<'_, AppStateType>,
    file_path: String,
) -> Result<String, String> {
    info!("Uploading client key from: {}", file_path);

    // Validate file exists
    if !Path::new(&file_path).exists() {
        return Err("Key file not found".to_string());
    }

    // Create certs directory if it doesn't exist
    let certs_dir = Path::new("./certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(certs_dir)
            .map_err(|e| format!("Failed to create certs directory: {}", e))?;
    }

    // Copy file to certs/client.key
    let dest_path = certs_dir.join("client.key");
    tokio::fs::copy(&file_path, &dest_path).await
        .map_err(|e| format!("Failed to copy key file: {}", e))?;

    info!("Client key uploaded successfully to: {:?}", dest_path);
    Ok("Client key uploaded successfully".to_string())
}

#[tauri::command]
pub async fn upload_ca_certificate(
    _state: State<'_, AppStateType>,
    file_path: String,
) -> Result<String, String> {
    info!("Uploading CA certificate from: {}", file_path);

    // Validate file exists
    if !Path::new(&file_path).exists() {
        return Err("CA certificate file not found".to_string());
    }

    // Create certs directory if it doesn't exist
    let certs_dir = Path::new("./certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(certs_dir)
            .map_err(|e| format!("Failed to create certs directory: {}", e))?;
    }

    // Copy file to certs/ca.crt
    let dest_path = certs_dir.join("ca.crt");
    tokio::fs::copy(&file_path, &dest_path).await
        .map_err(|e| format!("Failed to copy CA certificate file: {}", e))?;

    info!("CA certificate uploaded successfully to: {:?}", dest_path);
    Ok("CA certificate uploaded successfully".to_string())
}

#[tauri::command]
pub async fn validate_certificate_files(_state: State<'_, AppStateType>) -> Result<CertificateValidation, String> {
    let certs_dir = Path::new("./certs");

    let client_cert_path = certs_dir.join("client.crt");
    let client_key_path = certs_dir.join("client.key");
    let ca_cert_path = certs_dir.join("ca.crt");

    let mut validation = CertificateValidation {
        client_cert_valid: false,
        client_key_valid: false,
        ca_cert_valid: false,
        client_cert_info: None,
        ca_cert_info: None,
        errors: Vec::new(),
    };

    // Validate client certificate
    if client_cert_path.exists() {
        match validate_certificate_file(&client_cert_path).await {
            Ok(info) => {
                validation.client_cert_valid = true;
                validation.client_cert_info = Some(info);
            }
            Err(e) => validation.errors.push(format!("Client certificate: {}", e)),
        }
    } else {
        validation.errors.push("Client certificate file not found".to_string());
    }

    // Validate client key
    if client_key_path.exists() {
        match validate_key_file(&client_key_path).await {
            Ok(_) => validation.client_key_valid = true,
            Err(e) => validation.errors.push(format!("Client key: {}", e)),
        }
    } else {
        validation.errors.push("Client key file not found".to_string());
    }

    // Validate CA certificate
    if ca_cert_path.exists() {
        match validate_certificate_file(&ca_cert_path).await {
            Ok(info) => {
                validation.ca_cert_valid = true;
                validation.ca_cert_info = Some(info);
            }
            Err(e) => validation.errors.push(format!("CA certificate: {}", e)),
        }
    } else {
        validation.errors.push("CA certificate file not found".to_string());
    }

    Ok(validation)
}

#[tauri::command]
pub async fn get_certificate_info(
    _state: State<'_, AppStateType>,
    cert_path: String,
) -> Result<CertificateInfo, String> {
    validate_certificate_file(&Path::new(&cert_path)).await
}

/// Config File Management Commands

#[tauri::command]
pub async fn load_config_from_file(
    state: State<'_, AppStateType>,
    file_path: String,
) -> Result<ClientConfig, String> {
    info!("Loading config from file: {}", file_path);

    if !Path::new(&file_path).exists() {
        return Err("Configuration file not found".to_string());
    }

    match tokio::fs::read_to_string(&file_path).await {
        Ok(config_str) => {
            match serde_json::from_str::<ClientConfig>(&config_str) {
                Ok(config) => {
                    // Validate the configuration
                    match validate_config_internal(&config).await {
                        Ok(_) => {
                            let mut state_guard = state.write().await;
                            state_guard.config = Some(config.clone());
                            info!("Configuration loaded successfully from: {}", file_path);
                            Ok(config)
                        }
                        Err(e) => Err(format!("Configuration validation failed: {}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to parse configuration: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to read configuration file: {}", e)),
    }
}

#[tauri::command]
pub async fn validate_config(
    _state: State<'_, AppStateType>,
    config: ClientConfig,
) -> Result<ConfigValidation, String> {
    validate_config_internal(&config).await
}

#[tauri::command]
pub async fn connect_with_config(
    state: State<'_, AppStateType>,
    app_handle: AppHandle,
    config: ClientConfig,
) -> Result<(), String> {
    info!("Connecting to server with provided configuration");

    // Update state with new config
    {
        let mut state_guard = state.write().await;
        state_guard.config = Some(config.clone());
        state_guard.connection_status = ConnectionStatus::Connecting;
    }

    // Emit connection status update
    app_handle
        .emit_all("connection_status_changed", &ConnectionStatus::Connecting)
        .map_err(|e| format!("Failed to emit connection status: {}", e))?;

    // Initialize gRPC client with new config
    match GrpcClientManager::new(&config).await {
        Ok(client) => {
            // Test connection
            match client.test_connection().await {
                Ok(_) => {
                    let mut state_guard = state.write().await;
                    state_guard.connection_status = ConnectionStatus::Connected;

                    // Emit success
                    app_handle
                        .emit_all("connection_status_changed", &ConnectionStatus::Connected)
                        .map_err(|e| format!("Failed to emit connection status: {}", e))?;

                    info!("Successfully connected to server with provided configuration");
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Connection test failed: {}", e);
                    error!("{}", error_msg);

                    let mut state_guard = state.write().await;
                    state_guard.connection_status = ConnectionStatus::Error(error_msg.clone());

                    app_handle
                        .emit_all("connection_status_changed", &ConnectionStatus::Error(error_msg.clone()))
                        .map_err(|e| format!("Failed to emit connection status: {}", e))?;

                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create gRPC client: {}", e);
            error!("{}", error_msg);

            let mut state_guard = state.write().await;
            state_guard.connection_status = ConnectionStatus::Error(error_msg.clone());

            app_handle
                .emit_all("connection_status_changed", &ConnectionStatus::Error(error_msg.clone()))
                .map_err(|e| format!("Failed to emit connection status: {}", e))?;

            Err(error_msg)
        }
    }
}

/// Notification Commands

#[tauri::command]
pub async fn show_notification(
    state: State<'_, AppStateType>,
    app_handle: AppHandle,
    title: String,
    message: String,
    level: String,
) -> Result<(), String> {
    let notification_level = match level.as_str() {
        "info" => NotificationLevel::Info,
        "warning" => NotificationLevel::Warning,
        "error" => NotificationLevel::Error,
        "success" => NotificationLevel::Success,
        "critical" => NotificationLevel::Critical,
        _ => NotificationLevel::Info,
    };

    let notification = NotificationEntry {
        id: Uuid::new_v4().to_string(),
        level: notification_level,
        title: title.clone(),
        message: message.clone(),
        timestamp: chrono::Utc::now(),
        read: false,
        source: "client".to_string(),
    };

    let mut state_guard = state.write().await;
    state_guard.add_notification(notification.clone());
    drop(state_guard);

    // Show system notification
    tauri::api::notification::Notification::new(&app_handle.config().tauri.bundle.identifier)
        .title(&title)
        .body(&message)
        .show()
        .map_err(|e| format!("Failed to show notification: {}", e))?;

    // Emit to frontend
    app_handle
        .emit_all("notification_received", &notification)
        .map_err(|e| format!("Failed to emit notification: {}", e))?;

    Ok(())
}

/// System Commands

#[tauri::command]
pub async fn get_system_info(_state: State<'_, AppStateType>) -> Result<SystemInfo, String> {
    let system_info = SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        uptime: chrono::Utc::now(),
    };

    Ok(system_info)
}

#[tauri::command]
pub async fn export_session_data(
    state: State<'_, AppStateType>,
    file_path: String,
) -> Result<(), String> {
    info!("Exporting session data to: {}", file_path);

    let state_guard = state.read().await;
    let export_data = SessionExportData {
        agents: state_guard.agents.clone(),
        task_history: state_guard.task_history.clone(),
        chat_messages: state_guard.chat_messages.clone(),
        exported_at: chrono::Utc::now(),
    };

    let json_data = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize session data: {}", e))?;

    tokio::fs::write(&file_path, json_data).await
        .map_err(|e| format!("Failed to write session data: {}", e))?;

    info!("Session data exported successfully to: {}", file_path);
    Ok(())
}

#[tauri::command]
pub async fn import_session_data(
    state: State<'_, AppStateType>,
    file_path: String,
) -> Result<(), String> {
    info!("Importing session data from: {}", file_path);

    let json_data = tokio::fs::read_to_string(&file_path).await
        .map_err(|e| format!("Failed to read session data: {}", e))?;

    let import_data: SessionExportData = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to parse session data: {}", e))?;

    let mut state_guard = state.write().await;

    // Import agents
    for (agent_id, agent) in import_data.agents {
        state_guard.agents.insert(agent_id, agent);
    }

    // Import task history
    state_guard.task_history.extend(import_data.task_history);

    // Import chat messages
    state_guard.chat_messages.extend(import_data.chat_messages);

    info!("Session data imported successfully from: {}", file_path);
    Ok(())
}

// Supporting structures

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub permissions: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BofMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub entry_point: String,
    pub parameters: Vec<crate::state::BofParameter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub domain: String,
    pub issuer: String,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: chrono::DateTime<chrono::Utc>,
    pub is_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub uptime: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionExportData {
    pub agents: HashMap<String, AgentSession>,
    pub task_history: Vec<TaskHistoryEntry>,
    pub chat_messages: Vec<ChatMessage>,
    pub exported_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateValidation {
    pub client_cert_valid: bool,
    pub client_key_valid: bool,
    pub ca_cert_valid: bool,
    pub client_cert_info: Option<CertificateInfo>,
    pub ca_cert_info: Option<CertificateInfo>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// Certificate validation helper functions
async fn validate_certificate_file(cert_path: &Path) -> Result<CertificateInfo, String> {
    // Basic file existence check
    if !cert_path.exists() {
        return Err("Certificate file not found".to_string());
    }

    // Read the certificate file
    let cert_content = tokio::fs::read_to_string(cert_path).await
        .map_err(|e| format!("Failed to read certificate file: {}", e))?;

    // Basic PEM format validation
    if !cert_content.contains("-----BEGIN CERTIFICATE-----") || !cert_content.contains("-----END CERTIFICATE-----") {
        return Err("Invalid certificate format - must be PEM format".to_string());
    }

    // For now, return mock certificate info since we don't have a full X.509 parser
    // In a real implementation, you would parse the certificate using a library like rustls or openssl
    Ok(CertificateInfo {
        domain: "example.com".to_string(),
        issuer: "Certificate Authority".to_string(),
        valid_from: chrono::Utc::now() - chrono::Duration::days(30),
        valid_to: chrono::Utc::now() + chrono::Duration::days(365),
        is_valid: true,
    })
}

async fn validate_key_file(key_path: &Path) -> Result<(), String> {
    // Basic file existence check
    if !key_path.exists() {
        return Err("Key file not found".to_string());
    }

    // Read the key file
    let key_content = tokio::fs::read_to_string(key_path).await
        .map_err(|e| format!("Failed to read key file: {}", e))?;

    // Basic PEM format validation for private keys
    if (!key_content.contains("-----BEGIN PRIVATE KEY-----") && !key_content.contains("-----BEGIN RSA PRIVATE KEY-----")) ||
       (!key_content.contains("-----END PRIVATE KEY-----") && !key_content.contains("-----END RSA PRIVATE KEY-----")) {
        return Err("Invalid key format - must be PEM format".to_string());
    }

    Ok(())
}

async fn validate_config_internal(config: &ClientConfig) -> Result<ConfigValidation, String> {
    let mut validation = ConfigValidation {
        is_valid: true,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    // Validate server endpoint
    if config.server_endpoint.is_empty() {
        validation.is_valid = false;
        validation.errors.push("Server endpoint cannot be empty".to_string());
    }

    // Validate server port
    if config.server_port == 0 || config.server_port > 65535 {
        validation.is_valid = false;
        validation.errors.push("Server port must be between 1 and 65535".to_string());
    }

    // Validate TLS configuration
    if config.use_tls {
        if config.cert_path.as_ref().map_or(true, |s| s.is_empty()) {
            validation.warnings.push("TLS enabled but no certificate path specified".to_string());
        }
        if config.key_path.as_ref().map_or(true, |s| s.is_empty()) {
            validation.warnings.push("TLS enabled but no key path specified".to_string());
        }
        if config.ca_cert_path.as_ref().map_or(true, |s| s.is_empty()) {
            validation.warnings.push("TLS enabled but no CA certificate path specified".to_string());
        }
    }

    // Validate username
    if config.username.is_empty() {
        validation.warnings.push("Username is empty".to_string());
    }

    // Validate team name
    if config.team_name.is_empty() {
        validation.warnings.push("Team name is empty".to_string());
    }

    // Validate WebSocket endpoint if provided
    if let Some(ws_endpoint) = &config.websocket_endpoint {
        if !ws_endpoint.is_empty() {
            if !ws_endpoint.starts_with("ws://") && !ws_endpoint.starts_with("wss://") {
                validation.errors.push("WebSocket endpoint must start with ws:// or wss://".to_string());
                validation.is_valid = false;
            }
        }
    }

    // Validate update interval
    if config.update_interval_ms < 1000 {
        validation.warnings.push("Update interval less than 1 second may cause performance issues".to_string());
    }

    // Validate max concurrent tasks
    if config.max_concurrent_tasks == 0 {
        validation.warnings.push("Max concurrent tasks is 0 - no tasks will be executed".to_string());
    } else if config.max_concurrent_tasks > 100 {
        validation.warnings.push("Very high max concurrent tasks may cause performance issues".to_string());
    }

    Ok(validation)
}
