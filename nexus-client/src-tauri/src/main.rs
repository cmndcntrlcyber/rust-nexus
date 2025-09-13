// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

mod commands;
mod grpc_client;
mod state;
mod websocket;

use commands::*;
use state::AppState;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Starting Nexus Client Application");

    let app_state = AppState::new().await;
    let app_state_arc = Arc::new(RwLock::new(app_state));

    tauri::Builder::default()
        .manage(app_state_arc.clone())
        .setup(move |app| {
            let app_handle = app.handle();
            let state = app_state_arc.clone();

            tokio::spawn(async move {
                // Initialize WebSocket connection if configured
                let state_guard = state.read().await;
                if let Some(config) = &state_guard.config {
                    if let Err(e) = websocket::connect_websocket(&app_handle, config).await {
                        error!("Failed to establish WebSocket connection: {}", e);
                    }
                }
            });

            Ok(())
        })
        .system_tray(state::create_system_tray())
        .on_system_tray_event(state::handle_system_tray_event)
        .invoke_handler(tauri::generate_handler![
            // Configuration commands
            load_config,
            save_config,
            get_config,

            // Connection commands
            connect_to_server,
            disconnect_from_server,
            get_connection_status,

            // Agent management commands
            list_agents,
            get_agent_details,
            interact_with_agent,
            execute_command,

            // File management commands
            list_agent_files,
            upload_file_to_agent,
            download_file_from_agent,

            // Task management commands
            execute_task,
            get_task_results,
            list_task_history,

            // BOF management commands
            list_available_bofs,
            execute_bof,
            upload_bof,

            // Infrastructure commands
            get_domains,
            rotate_domain,
            get_certificates,

            // Certificate management commands
            upload_client_certificate,
            upload_client_key,
            upload_ca_certificate,
            validate_certificate_files,
            get_certificate_info,

            // Config file management commands
            load_config_from_file,
            validate_config,
            connect_with_config,

            // Notification commands
            show_notification,

            // System commands
            get_system_info,
            export_session_data,
            import_session_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
