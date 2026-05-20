//! `--shell-smoke-test` implementation.

use std::time::Duration;

use tokio::time::{timeout, Instant};
use tracing::info;

use crate::shell::{ShellSelect, ShellSession};

/// Outcome of a smoke-test run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeOutcome {
    /// Marker observed.
    Ok {
        /// Captured bytes including marker.
        captured: Vec<u8>,
    },
    /// `ShellSession::open` failed.
    OpenFailed(String),
    /// `ShellSession::write` failed.
    WriteFailed(String),
    /// Deadline exceeded.
    Timeout {
        /// Bytes captured before giving up.
        captured: Vec<u8>,
    },
    /// PTY closed early.
    ShellClosed {
        /// Bytes captured before EOF.
        captured: Vec<u8>,
    },
}

impl SmokeOutcome {
    /// Stable exit code.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Ok { .. } => 0,
            Self::OpenFailed(_) => 2,
            Self::WriteFailed(_) => 3,
            Self::Timeout { .. } => 4,
            Self::ShellClosed { .. } => 5,
        }
    }
}

/// Open the host shell, send a probe, observe the marker echoed back.
pub async fn run_shell_smoke_test(deadline: Duration) -> SmokeOutcome {
    let cmd = ShellSelect::for_host();
    info!(program = %cmd.program, args = ?cmd.args, "smoke test: opening shell");

    let mut session = match ShellSession::open(cmd, 80, 24) {
        Ok(s) => s,
        Err(err) => return SmokeOutcome::OpenFailed(format!("{err:#}")),
    };

    let probe: &[u8] = if cfg!(target_os = "windows") {
        b"Write-Host nexus-smoke-ok\r\n"
    } else {
        b"echo nexus-smoke-ok\n"
    };

    if let Err(err) = session.write(probe).await {
        return SmokeOutcome::WriteFailed(format!("{err:#}"));
    }

    let end = Instant::now() + deadline;
    let mut captured = Vec::new();
    loop {
        let remaining = end.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return SmokeOutcome::Timeout { captured };
        }
        match timeout(remaining, session.recv_output()).await {
            Ok(Some(chunk)) => {
                captured.extend_from_slice(&chunk);
                if String::from_utf8_lossy(&captured).contains("nexus-smoke-ok") {
                    let _ = session.kill();
                    return SmokeOutcome::Ok { captured };
                }
            }
            Ok(None) => return SmokeOutcome::ShellClosed { captured },
            Err(_) => return SmokeOutcome::Timeout { captured },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke_test_passes_on_host() {
        let outcome = run_shell_smoke_test(Duration::from_secs(10)).await;
        assert!(
            matches!(outcome, SmokeOutcome::Ok { .. }),
            "smoke test did not succeed: {outcome:?}"
        );
        assert_eq!(outcome.exit_code(), 0);
    }
}
