//! Nexus C2 gRPC Server
//!
//! Main binary for the Nexus C2 framework's gRPC server.
//! Provides enterprise-grade C2 capabilities with advanced infrastructure management.

use anyhow::{Context, Result};
use clap::{Arg, ArgMatches, Command};
use log::{error, info, warn};
use nexus_infra::{
    CertManager, GrpcServer, InfraError, InfraResult, NexusConfig, OriginCertConfig,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();

    // Parse command line arguments
    let matches = create_cli().get_matches();

    // Load configuration
    let config_path = matches
        .get_one::<String>("config")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("nexus.toml"));

    let config = load_config(&config_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_path.display()
        )
    })?;

    info!("Nexus C2 Server starting...");
    info!("Configuration loaded from: {}", config_path.display());

    // Handle different commands
    match matches.subcommand() {
        Some(("start", sub_matches)) => {
            start_server(config, sub_matches).await?;
        }
        Some(("validate", _)) => {
            validate_configuration(config)?;
        }
        Some(("init", sub_matches)) => {
            init_infrastructure(config, sub_matches).await?;
        }
        _ => {
            // Default action is to start the server
            start_server(config, &matches).await?;
        }
    }

    Ok(())
}

/// Create the CLI interface
fn create_cli() -> Command {
    Command::new("nexus-server")
        .version("0.1.0")
        .author("Nexus Team")
        .about("Nexus C2 gRPC Server - Enterprise Command & Control Framework")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("nexus.toml"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity level"),
        )
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .value_name("ADDRESS")
                .help("Override server bind address"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Override server port")
                .value_parser(clap::value_parser!(u16)),
        )
        .subcommand(
            Command::new("start")
                .about("Start the gRPC server (default action)")
                .arg(
                    Arg::new("daemon")
                        .short('d')
                        .long("daemon")
                        .action(clap::ArgAction::SetTrue)
                        .help("Run as daemon in background"),
                ),
        )
        .subcommand(Command::new("validate").about("Validate configuration and exit"))
        .subcommand(
            Command::new("init")
                .about("Initialize infrastructure (domains and certificates)")
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force initialization even if already configured"),
                ),
        )
}

/// Initialize logging based on verbosity level
fn init_logging() {
    let log_level = match std::env::var("RUST_LOG") {
        Ok(_) => return, // Let user's RUST_LOG take precedence
        Err(_) => "nexus_server=info,nexus_infra=info",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_secs()
        .init();
}

/// Load configuration from file
fn load_config(config_path: &PathBuf) -> Result<NexusConfig> {
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Configuration file does not exist: {}. Create one using the template in config/examples/",
            config_path.display()
        ));
    }

    let config = NexusConfig::from_file(config_path)
        .map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    info!("Configuration validated successfully");
    Ok(config)
}

/// Start the gRPC server
async fn start_server(mut config: NexusConfig, matches: &ArgMatches) -> Result<()> {
    // Override configuration from CLI arguments
    if let Some(bind_address) = matches.get_one::<String>("bind") {
        config.grpc_server.bind_address = bind_address.clone();
    }

    if let Some(port) = matches.get_one::<u16>("port") {
        config.grpc_server.port = *port;
    }

    info!(
        "Starting Nexus C2 Server on {}:{}",
        config.grpc_server.bind_address, config.grpc_server.port
    );

    // Initialize certificate manager
    let cert_manager = initialize_cert_manager(&config.origin_cert)
        .await
        .with_context(|| "Failed to initialize certificate manager")?;

    // Create gRPC server
    let server = GrpcServer::new(config.grpc_server.clone(), cert_manager);

    // Start the server
    server
        .start()
        .await
        .with_context(|| "Failed to start gRPC server")?;

    info!("✅ Nexus C2 Server is running");
    info!(
        "🌐 Listening on: https://{}:{}",
        config.grpc_server.bind_address, config.grpc_server.port
    );

    // Start cleanup task for inactive agents
    let server_clone = Arc::new(server);
    let cleanup_server = server_clone.clone();
    let cleanup_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            let removed = cleanup_server.cleanup_inactive_agents(30).await; // 30 minutes timeout
            if removed > 0 {
                info!("Cleaned up {} inactive agents", removed);
            }
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown().await;

    info!("🛑 Shutdown signal received, stopping server...");

    // Cancel cleanup task
    cleanup_task.abort();

    // Stop the server
    if let Err(e) = server_clone.stop().await {
        error!("Error stopping server: {}", e);
    }

    info!("✅ Server stopped gracefully");
    Ok(())
}

/// Validate configuration and exit
fn validate_configuration(config: NexusConfig) -> Result<()> {
    info!("Validating configuration...");

    // Configuration was already validated during loading
    info!("✅ Configuration is valid");

    // Print configuration summary
    println!("\n📋 Configuration Summary:");
    println!("  Domain: {}", config.cloudflare.domain);
    println!("  Primary domains: {:?}", config.domains.primary_domains);
    println!(
        "  gRPC Server: {}:{}",
        config.grpc_server.bind_address, config.grpc_server.port
    );
    println!("  Mutual TLS: {}", config.grpc_server.mutual_tls);
    println!("  Max connections: {}", config.grpc_server.max_connections);
    println!(
        "  Domain rotation: {} hours",
        config.domains.rotation_interval
    );

    Ok(())
}

/// Initialize infrastructure components
async fn init_infrastructure(config: NexusConfig, matches: &ArgMatches) -> Result<()> {
    let force = matches.get_flag("force");

    info!("🔧 Initializing Nexus infrastructure...");

    // Check if already initialized (unless force is specified)
    if !force && is_infrastructure_initialized(&config) {
        warn!("Infrastructure appears to already be initialized. Use --force to override.");
        return Ok(());
    }

    // Initialize certificate directories
    std::fs::create_dir_all(config.origin_cert.cert_path.parent().unwrap())
        .with_context(|| "Failed to create certificate directory")?;

    std::fs::create_dir_all(&config.letsencrypt.cert_storage_dir)
        .with_context(|| "Failed to create Let's Encrypt storage directory")?;

    // Initialize certificate manager
    let _cert_manager = initialize_cert_manager(&config.origin_cert).await?;

    // TODO: Initialize Cloudflare DNS records
    // TODO: Request Let's Encrypt certificates
    // TODO: Create initial domain records

    info!("✅ Infrastructure initialization complete");

    Ok(())
}

/// Initialize the certificate manager
async fn initialize_cert_manager(cert_config: &OriginCertConfig) -> InfraResult<Arc<CertManager>> {
    info!("Initializing certificate manager...");

    // Verify that all required certificates exist
    if !cert_config.cert_path.exists() {
        return Err(InfraError::ConfigError(format!(
            "Certificate file not found: {}. Please provide a valid certificate.",
            cert_config.cert_path.display()
        )));
    }

    if !cert_config.key_path.exists() {
        return Err(InfraError::ConfigError(format!(
            "Private key file not found: {}. Please provide a valid private key.",
            cert_config.key_path.display()
        )));
    }

    if !cert_config.ca_cert_path.exists() {
        return Err(InfraError::ConfigError(format!(
            "CA certificate file not found: {}. Please provide a valid CA certificate.",
            cert_config.ca_cert_path.display()
        )));
    }

    info!("✅ Using existing origin certificates");

    // Now create the certificate manager with existing certificates
    let cert_manager = CertManager::new(cert_config.clone())?;

    Ok(Arc::new(cert_manager))
}

/// Check if infrastructure is already initialized
fn is_infrastructure_initialized(config: &NexusConfig) -> bool {
    config.origin_cert.cert_path.exists()
        && config.origin_cert.key_path.exists()
        && config.letsencrypt.cert_storage_dir.exists()
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_creation() {
        let cli = create_cli();
        assert_eq!(cli.get_name(), "nexus-server");

        // Test default config argument
        let matches = cli.try_get_matches_from(&["nexus-server"]).unwrap();
        assert_eq!(matches.get_one::<String>("config").unwrap(), "nexus.toml");
    }

    #[test]
    fn test_cli_arguments() {
        let cli = create_cli();

        // Test with custom config
        let matches = cli
            .try_get_matches_from(&[
                "nexus-server",
                "--config",
                "test.toml",
                "--bind",
                "127.0.0.1",
                "--port",
                "8443",
            ])
            .unwrap();

        assert_eq!(matches.get_one::<String>("config").unwrap(), "test.toml");
        assert_eq!(matches.get_one::<String>("bind").unwrap(), "127.0.0.1");
        assert_eq!(*matches.get_one::<u16>("port").unwrap(), 8443);
    }

    #[test]
    fn test_infrastructure_check() {
        let temp_dir = tempdir().unwrap();
        let mut config = NexusConfig::default();

        config.origin_cert.cert_path = temp_dir.path().join("cert.pem");
        config.origin_cert.key_path = temp_dir.path().join("key.pem");
        config.letsencrypt.cert_storage_dir = temp_dir.path().join("letsencrypt");

        // Should not be initialized initially
        assert!(!is_infrastructure_initialized(&config));

        // Create the files
        std::fs::write(&config.origin_cert.cert_path, "dummy cert").unwrap();
        std::fs::write(&config.origin_cert.key_path, "dummy key").unwrap();
        std::fs::create_dir_all(&config.letsencrypt.cert_storage_dir).unwrap();

        // Should now be initialized
        assert!(is_infrastructure_initialized(&config));
    }
}
