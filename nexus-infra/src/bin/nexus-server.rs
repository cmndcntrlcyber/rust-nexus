//! `nexus-server` binary (Phase 1.3.5).
//!
//! Entry point for the C2 server. Two modes:
//!
//! - `nexus-server --init-identity <path>` — first-run helper: generate
//!   a fresh NodeIdentity, persist with mode 0o600, exit. Refuses to
//!   overwrite an existing file.
//! - `nexus-server [--config <path>]` — load `nexus.toml` (default:
//!   `/etc/nexus/nexus.toml` or `./nexus.toml`) and start the A2A
//!   service via `nexus_infra::serve::run_a2a`.
//!
//! Future v1.3 phases extend this with `[acme]` bootstrap (1.3.2),
//! `[metrics]` server (1.3.6), `[mesh.bootstrap]` listener (1.3.3 +
//! 1.3.4), SIGHUP reload (1.3.5 + 1.3.7).

#![allow(clippy::too_many_lines)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use nexus_a2a::A2aServer;
use nexus_common::NodeIdentity;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use nexus_infra::a2a_lister::RegistryLister;
use nexus_infra::a2a_router::{AgentChannels, AgentRegistrar, OperatorRouter};
use nexus_infra::serve::default_agent_card;
use nexus_infra::sessions::SessionRegistry;

const HELP: &str = "\
nexus-server — rust-nexus C2 (v1.3)

USAGE:
    nexus-server [--config <PATH>]
    nexus-server --init-identity <PATH>
    nexus-server --help
    nexus-server --version

OPTIONS:
    --config <PATH>           Path to nexus.toml. Defaults to
                              /etc/nexus/nexus.toml then ./nexus.toml.
    --init-identity <PATH>    Generate a fresh NodeIdentity, persist to
                              <PATH> with mode 0o600, exit. Refuses to
                              overwrite an existing file.
    --help                    Print this help.
    --version                 Print the build version.

ENVIRONMENT:
    NEXUS_CA_CERT             CA bundle for mTLS (PEM, path or inline).
    NEXUS_SERVER_CERT         Server cert for mTLS.
    NEXUS_SERVER_KEY          Server key for mTLS.

See `docs/deployment/production.md` for the full deployment guide.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("nexus-server: {err}");
            eprintln!("\n{HELP}");
            return ExitCode::from(2);
        }
    };

    match parsed.action {
        Action::Help => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Action::Version => {
            println!("nexus-server {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Action::InitIdentity(path) => run_init_identity(&path),
        Action::Serve(config_path) => run_serve(config_path),
    }
}

enum Action {
    Help,
    Version,
    InitIdentity(PathBuf),
    Serve(Option<PathBuf>),
}

struct Parsed {
    action: Action,
}

fn parse_args(args: &[String]) -> Result<Parsed, String> {
    let mut config: Option<PathBuf> = None;
    let mut init_identity: Option<PathBuf> = None;
    let mut help = false;
    let mut version = false;
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            "--config" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--config requires a path".to_string())?;
                config = Some(PathBuf::from(v));
            }
            "--init-identity" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--init-identity requires a path".to_string())?;
                init_identity = Some(PathBuf::from(v));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if help {
        return Ok(Parsed {
            action: Action::Help,
        });
    }
    if version {
        return Ok(Parsed {
            action: Action::Version,
        });
    }
    if let Some(path) = init_identity {
        if config.is_some() {
            return Err("--init-identity and --config are mutually exclusive".to_string());
        }
        return Ok(Parsed {
            action: Action::InitIdentity(path),
        });
    }
    Ok(Parsed {
        action: Action::Serve(config),
    })
}

fn run_init_identity(path: &Path) -> ExitCode {
    if path.exists() {
        eprintln!(
            "nexus-server --init-identity: refusing to overwrite existing file {}",
            path.display()
        );
        return ExitCode::from(1);
    }
    let identity = NodeIdentity::generate();
    if let Err(err) = identity.save_to_file(path) {
        eprintln!(
            "nexus-server --init-identity: failed to write {}: {err}",
            path.display()
        );
        return ExitCode::from(1);
    }
    let peer_id_hex: String = identity
        .peer_id()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    println!(
        "wrote NodeIdentity to {} (mode 0o600) — peer_id={}",
        path.display(),
        peer_id_hex
    );
    ExitCode::SUCCESS
}

fn run_serve(config_path: Option<PathBuf>) -> ExitCode {
    let resolved = config_path.unwrap_or_else(default_config_path);

    let cfg = match load_serve_config(&resolved) {
        Ok(c) => c,
        Err(err) => {
            eprintln!(
                "nexus-server: failed to load config {}: {err:#}",
                resolved.display()
            );
            return ExitCode::from(1);
        }
    };

    init_tracing();

    let bind: SocketAddr = match cfg.a2a.bind.parse() {
        Ok(a) => a,
        Err(err) => {
            error!(bind = %cfg.a2a.bind, error = %err, "invalid [a2a].bind");
            return ExitCode::from(1);
        }
    };

    let identity_path = PathBuf::from(&cfg.a2a.identity_path);
    let identity = match NodeIdentity::load_or_create(&identity_path) {
        Ok(id) => Arc::new(id),
        Err(err) => {
            error!(path = %identity_path.display(), error = %err, "load NodeIdentity");
            return ExitCode::from(1);
        }
    };
    let peer_id_hex: String = identity
        .peer_id()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    info!(identity_path = %identity_path.display(), peer_id = %peer_id_hex, "NodeIdentity loaded");

    let tls = match nexus_a2a::tls::load_server_config_from_env() {
        Ok(t) => Some(t),
        Err(err) => {
            if cfg.a2a.insecure_network {
                warn!(error = %err, "no NEXUS_*_CERT env vars — running plaintext (insecure_network=true)");
                None
            } else {
                error!(error = %err, "mTLS required (insecure_network=false) but cert env vars missing");
                return ExitCode::from(1);
            }
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = %err, "build tokio runtime");
            return ExitCode::from(1);
        }
    };

    let result = runtime.block_on(async move {
        let agents_view = Arc::new(RwLock::new(HashMap::new()));
        let a2a_agents = AgentChannels::new();
        let sessions = SessionRegistry::new();

        let router = OperatorRouter::new(a2a_agents.clone(), sessions.clone());
        let registrar = AgentRegistrar::new(a2a_agents.clone(), sessions.clone());
        let lister = RegistryLister::new(agents_view.clone());

        let mut card = default_agent_card();
        nexus_a2a::cards::sign(&mut card, identity.as_ref());
        info!("A2A AgentCard signed with server identity");

        let server = A2aServer::new(card, router)
            .with_lister(lister)
            .with_agent_registration(registrar);

        let shutdown = shutdown_signal();
        server
            .serve_with_optional_tls(bind, cfg.a2a.insecure_network, tls, shutdown)
            .await
    });

    match result {
        Ok(()) => {
            info!("nexus-server: shutdown clean");
            ExitCode::SUCCESS
        }
        Err(err) => {
            error!(error = %format!("{err:#}"), "A2A server terminated with error");
            ExitCode::from(1)
        }
    }
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let terminate = async {
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(err) => {
                error!(error = %err, "install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        _ = ctrl_c => info!("received SIGINT, shutting down"),
        _ = terminate => info!("received SIGTERM, shutting down"),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

#[derive(Debug, Deserialize)]
struct ServeConfig {
    a2a: A2aSection,
}

#[derive(Debug, Deserialize)]
struct A2aSection {
    bind: String,
    #[serde(default)]
    insecure_network: bool,
    identity_path: String,
}

fn load_serve_config(path: &Path) -> anyhow::Result<ServeConfig> {
    use anyhow::Context;
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let cfg: ServeConfig =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(cfg)
}

fn default_config_path() -> PathBuf {
    let etc = PathBuf::from("/etc/nexus/nexus.toml");
    if etc.exists() {
        return etc;
    }
    PathBuf::from("./nexus.toml")
}
