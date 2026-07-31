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
use nexus_infra::PkiManager;

const HELP: &str = "\
nexus-server — rust-nexus C2 (v1.3)

USAGE:
    nexus-server [--config <PATH>]
    nexus-server --init-identity <PATH>
    nexus-server pki init   --domain <DOMAIN> --ip <IP> [--agents <N>] [--out <DIR>]
    nexus-server pki agent  --certs-dir <DIR> --name <NAME>
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

PKI SUBCOMMANDS:
    pki init   Generate a complete PKI in one atomic operation:
               CA + server cert + operator cert + N agent certs.
               --domain     C2 hostname (added as DNS SAN to server cert)
               --ip         C2 IP address (added as IP SAN to server cert)
               --agents     Number of agent certs to mint (default: 1)
               --out        Output directory (default: ./certs/prod)
               --cf-token     CF API token (Zone:SSL+Certs:Edit + Zone:ClientCerts:Edit).
                              Server cert → CF Origin CA (POST /certificates).
                              Client certs → CF Client Certs (POST /zones/{id}/client_certificates).
               --cf-zone-id   Zone ID for CF Client Certs API.
                              Required when --cf-token is provided.
               --cf-client-ca Path to CF zone managed CA cert PEM.
                              Required when --cf-zone-id is set.
                              Download: CF Dashboard → SSL/TLS → Client Certificates
                              → Certificate Authorities → Download.

    pki agent  Mint one additional agent cert.
               --certs-dir    Directory written by pki init
               --name         CN / file stem for the new agent cert
               --cf-token     CF API token (CF mode; same token as pki init)
               --cf-zone-id   Zone ID (CF mode)

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
        Action::PkiInit { domain, ip, agents, out, cf_token, cf_zone_id, cf_client_ca } => {
            run_pki_init(
                &domain, &ip, agents, &out,
                cf_token.as_deref(),
                cf_zone_id.as_deref(),
                cf_client_ca.as_deref(),
            )
        }
        Action::PkiAgent { certs_dir, name, cf_token, cf_zone_id } => {
            run_pki_agent(&certs_dir, &name, cf_token.as_deref(), cf_zone_id.as_deref())
        }
    }
}

enum Action {
    Help,
    Version,
    InitIdentity(PathBuf),
    Serve(Option<PathBuf>),
    PkiInit {
        domain: String,
        ip: String,
        agents: u8,
        out: PathBuf,
        cf_token: Option<String>,
        cf_zone_id: Option<String>,
        cf_client_ca: Option<PathBuf>,
    },
    PkiAgent {
        certs_dir: PathBuf,
        name: String,
        cf_token: Option<String>,
        cf_zone_id: Option<String>,
    },
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
            "pki" => {
                let sub = iter
                    .next()
                    .ok_or_else(|| "pki requires a subcommand: init | agent".to_string())?;
                return Ok(Parsed {
                    action: parse_pki_args(sub.as_str(), &mut iter)?,
                });
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

fn parse_pki_args(
    sub: &str,
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, String>>,
) -> Result<Action, String> {
    match sub {
        "init" => {
            let mut domain: Option<String> = None;
            let mut ip: Option<String> = None;
            let mut agents: u8 = 1;
            let mut out = PathBuf::from("./certs/prod");
            let mut cf_token: Option<String> = None;
            let mut cf_zone_id: Option<String> = None;
            let mut cf_client_ca: Option<PathBuf> = None;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--domain" => {
                        domain = Some(iter.next().ok_or("--domain requires a value")?.clone());
                    }
                    "--ip" => {
                        ip = Some(iter.next().ok_or("--ip requires a value")?.clone());
                    }
                    "--agents" => {
                        let v = iter.next().ok_or("--agents requires a number")?;
                        agents = v
                            .parse::<u8>()
                            .map_err(|_| format!("--agents: not a valid u8: {v}"))?;
                    }
                    "--out" => {
                        out = PathBuf::from(iter.next().ok_or("--out requires a path")?.as_str());
                    }
                    "--cf-token" => {
                        cf_token = Some(iter.next().ok_or("--cf-token requires a value")?.clone());
                    }
                    "--cf-zone-id" => {
                        cf_zone_id = Some(iter.next().ok_or("--cf-zone-id requires a value")?.clone());
                    }
                    "--cf-client-ca" => {
                        cf_client_ca = Some(PathBuf::from(
                            iter.next().ok_or("--cf-client-ca requires a path")?.as_str(),
                        ));
                    }
                    other => return Err(format!("pki init: unknown flag: {other}")),
                }
            }
            Ok(Action::PkiInit {
                domain: domain.ok_or("pki init: --domain is required")?,
                ip: ip.ok_or("pki init: --ip is required")?,
                agents,
                out,
                cf_token,
                cf_zone_id,
                cf_client_ca,
            })
        }
        "agent" => {
            let mut certs_dir: Option<PathBuf> = None;
            let mut name: Option<String> = None;
            let mut cf_token: Option<String> = None;
            let mut cf_zone_id: Option<String> = None;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--certs-dir" => {
                        certs_dir = Some(PathBuf::from(
                            iter.next().ok_or("--certs-dir requires a path")?.as_str(),
                        ));
                    }
                    "--name" => {
                        name = Some(iter.next().ok_or("--name requires a value")?.clone());
                    }
                    "--cf-token" => {
                        cf_token = Some(iter.next().ok_or("--cf-token requires a value")?.clone());
                    }
                    "--cf-zone-id" => {
                        cf_zone_id = Some(iter.next().ok_or("--cf-zone-id requires a value")?.clone());
                    }
                    other => return Err(format!("pki agent: unknown flag: {other}")),
                }
            }
            Ok(Action::PkiAgent {
                certs_dir: certs_dir.ok_or("pki agent: --certs-dir is required")?,
                name: name.ok_or("pki agent: --name is required")?,
                cf_token,
                cf_zone_id,
            })
        }
        other => Err(format!("pki: unknown subcommand: {other}; expected init | agent")),
    }
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

fn run_pki_init(
    domain: &str,
    ip: &str,
    agents: u8,
    out: &Path,
    cf_token: Option<&str>,
    cf_zone_id: Option<&str>,
    cf_client_ca: Option<&Path>,
) -> ExitCode {
    let ip_addr: std::net::IpAddr = match ip.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nexus-server pki init: invalid --ip '{ip}': {e}");
            return ExitCode::from(1);
        }
    };
    match PkiManager::init(domain, ip_addr, agents, out, cf_token, cf_zone_id, cf_client_ca) {
        Ok(b) => {
            println!("PKI initialized in {}", b.out_dir.display());
            println!();
            println!("Server env vars:");
            println!("  NEXUS_SERVER_CERT={}", b.server_cert.display());
            println!("  NEXUS_SERVER_KEY={}", b.server_key.display());
            println!("  NEXUS_CA_CERT={}   # CF zone client CA — validates incoming client certs", b.client_ca_cert.display());
            println!();
            println!("Agent env vars (per agent):");
            println!("  NEXUS_CA_CERT={}   # CF Origin CA ECC root — verifies server cert", b.server_ca_cert.display());
            if let Some(a) = b.agents.first() {
                println!("  NEXUS_CLIENT_CERT={}", a.cert.display());
                println!("  NEXUS_CLIENT_KEY={}", a.key.display());
            }
            println!();
            println!("Console env vars:");
            println!("  NEXUS_CA_CERT={}", b.server_ca_cert.display());
            println!("  NEXUS_CLIENT_CERT={}", b.console_cert.display());
            println!("  NEXUS_CLIENT_KEY={}", b.console_key.display());
            println!();
            println!("Operator env vars:");
            println!("  NEXUS_CA_CERT={}", b.server_ca_cert.display());
            println!("  NEXUS_CLIENT_CERT={}", b.operator_cert.display());
            println!("  NEXUS_CLIENT_KEY={}", b.operator_key.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nexus-server pki init: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_pki_agent(
    certs_dir: &Path,
    name: &str,
    cf_token: Option<&str>,
    cf_zone_id: Option<&str>,
) -> ExitCode {
    match PkiManager::add_agent(certs_dir, name, cf_token, cf_zone_id) {
        Ok(ab) => {
            println!("Agent cert minted:");
            println!("  cert: {}", ab.cert.display());
            println!("  key:  {}", ab.key.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nexus-server pki agent: {e}");
            ExitCode::from(1)
        }
    }
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
