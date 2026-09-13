// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(missing_docs)]

//! `nexus-console` — Tauri v2 operator console entry point.

mod tray;

use tauri::Manager;

use nexus_console::commands;
use nexus_console::state::ConsoleState;
use tracing_subscriber::EnvFilter;

fn main() {
    init_tracing();

    tauri::Builder::default()
        .manage(ConsoleState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_startup_config,
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
            // WS2.4 — tab management.
            commands::switch_tab,
            // WS1 Phase 1d — dual-auth query.
            commands::is_dual_authenticated,
            // WS9 Phase 9e — topology stream.
            commands::start_topology_stream,
            commands::stop_topology_stream,
            // v4.4 — tunnel badge connector.
            commands::load_tunnel_config,
            commands::get_tunnel_services,
            commands::open_tunnel_url,
        ])
        .setup(|app| {
            tray::setup_tray(app)?;

            // v4.4: auto-load tunnel config from RTPI_SLUG + RTPI_DOMAIN.
            if let (Ok(slug), Ok(domain)) = (
                std::env::var("RTPI_SLUG"),
                std::env::var("RTPI_DOMAIN"),
            ) {
                let config = commands::build_tunnel_config(&slug, &domain);
                tracing::info!(slug = %slug, domain = %domain, "auto-loaded tunnel config from env");
                let state: tauri::State<'_, ConsoleState> = app.state();
                tauri::async_runtime::block_on(async {
                    state.set_tunnel_config(config).await;
                });
            }

            Ok(())
        })
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
