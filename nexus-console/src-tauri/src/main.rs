// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(missing_docs)]

//! `nexus-console` — Tauri v2 operator console entry point.

use nexus_console::commands;
use nexus_console::state::ConsoleState;
use tracing_subscriber::EnvFilter;

fn main() {
    init_tracing();

    tauri::Builder::default()
        .manage(ConsoleState::new())
        .invoke_handler(tauri::generate_handler![
            commands::connect_c2,
            commands::disconnect_c2,
            commands::connection_summary,
            commands::list_agents,
            commands::open_shell_session,
            commands::send_shell_bytes,
            commands::resize_shell,
            commands::close_shell_session,
            // v1.4.4 — audit log viewer (Phase 1.4.4).
            commands::audit_log_tail,
            commands::audit_log_filter,
            commands::audit_log_verify,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();
}
