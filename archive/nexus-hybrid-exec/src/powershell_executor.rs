//! PowerShell-based execution implementation.
//!
//! Spawns `pwsh` (cross-platform PowerShell 7+) with a fallback to
//! `powershell.exe` on Windows. No external crate required — uses
//! `tokio::process::Command` directly so the `powershell` Cargo feature
//! remains a no-op opt-in (the `pwsh` 0.1.0 crate is a stub).

use crate::*;
use tokio::process::Command;

pub struct PowerShellExecutor {
    config: HybridExecConfig,
}

impl PowerShellExecutor {
    pub fn new(config: HybridExecConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let timeout_secs = request.timeout.unwrap_or(self.config.default_timeout);

        let pwsh_bin = Self::find_pwsh();
        let mut cmd = Command::new(&pwsh_bin);
        cmd.args(["-NonInteractive", "-NoProfile", "-Command", &request.command]);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| NexusError::TaskExecutionError("PowerShell execution timed out".into()))?
        .map_err(|e| NexusError::TaskExecutionError(format!("spawn {}: {}", pwsh_bin, e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(ExecutionResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() { None } else { Some(stderr) },
            exit_code: output.status.code(),
            execution_method: ExecutionProtocol::PowerShell,
            target_endpoint: request.target_endpoint.clone(),
            duration: start.elapsed(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn find_pwsh() -> String {
        // Prefer cross-platform pwsh; fall back to Windows inbox powershell.exe.
        for candidate in &["pwsh", "pwsh.exe", "powershell.exe"] {
            if which::which(candidate).is_ok() {
                return candidate.to_string();
            }
        }
        "pwsh".to_string()
    }
}
