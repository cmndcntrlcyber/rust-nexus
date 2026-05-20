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

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nexus_common::NodeIdentity;

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
    // v1.3.0 scaffold: defer real `run_a2a` wiring to Phase 1.3.2/3/6
    // (which add ACME bootstrap + mesh listener + metrics endpoint on
    // top of `nexus_infra::serve::run_a2a`). For now this binary
    // points operators at the production guide and prints the resolved
    // config path it would have loaded.
    let resolved = config_path.unwrap_or_else(default_config_path);
    eprintln!(
        "nexus-server: TODO — full `run_a2a` wiring lands in Phase 1.3.6.\n  resolved config: {}\n  see docs/deployment/production.md",
        resolved.display()
    );
    ExitCode::from(0)
}

fn default_config_path() -> PathBuf {
    let etc = PathBuf::from("/etc/nexus/nexus.toml");
    if etc.exists() {
        return etc;
    }
    PathBuf::from("./nexus.toml")
}
