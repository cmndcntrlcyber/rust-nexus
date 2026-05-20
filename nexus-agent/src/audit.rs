//! v1.4 per-agent audit log (Phase 1.4.9 / D-V1.4-F).
//!
//! Records local actions on the agent host: shell session
//! start/stop, signal received, etc. Persists to a platform-specific
//! path with mode 0o600.
//!
//! Uses the same `FileSink` machinery as the C2's audit log (see
//! `nexus_a2a::audit`); per-agent records are independent of the C2
//! chain.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use nexus_a2a::audit::{make_record, AuditSink, FileSink};

/// Default audit log path for the agent on Unix systems.
#[cfg(unix)]
#[must_use]
pub fn default_audit_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/var/db/nexus-agent/audit.log")
    } else {
        PathBuf::from("/var/lib/nexus-agent/audit.log")
    }
}

/// Default audit log path on Windows.
#[cfg(windows)]
#[must_use]
pub fn default_audit_path() -> PathBuf {
    let programdata =
        std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    PathBuf::from(programdata)
        .join("nexus-agent")
        .join("audit.log")
}

/// Thin handle around the agent's per-host audit sink. Cloneable
/// (Arc'd internally) so multiple subsystems can emit without
/// fighting over a single mutex.
#[derive(Clone)]
pub struct AgentAudit {
    sink: Arc<FileSink>,
    host_actor: String,
}

impl AgentAudit {
    /// Open the per-agent audit log at `path` (created if missing,
    /// mode 0o600 enforced by `FileSink::open`). `host_actor` becomes
    /// the `actor` field on every emitted record (typically the host's
    /// peer-id hex prefix).
    pub fn open(path: &std::path::Path, host_actor: impl Into<String>) -> std::io::Result<Self> {
        let sink = FileSink::open(path)?;
        Ok(Self {
            sink: Arc::new(sink),
            host_actor: host_actor.into(),
        })
    }

    /// Emit a `shell_session_open` record.
    pub async fn shell_session_open(&self, session_id: u64) {
        self.sink
            .append(make_record(
                &self.host_actor,
                "shell_session_open",
                &session_id.to_string(),
            ))
            .await;
    }

    /// Emit a `shell_session_close` record.
    pub async fn shell_session_close(&self, session_id: u64, exit_code: Option<i32>) {
        let resource = match exit_code {
            Some(c) => format!("{session_id}:{c}"),
            None => format!("{session_id}:?"),
        };
        self.sink
            .append(make_record(
                &self.host_actor,
                "shell_session_close",
                &resource,
            ))
            .await;
    }

    /// Emit an arbitrary `(action, resource)` record.
    pub async fn record(&self, action: &str, resource: &str) {
        self.sink
            .append(make_record(&self.host_actor, action, resource))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_records_a_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("audit.log");
        let audit = AgentAudit::open(&path, "host-aabbccdd").expect("open");
        audit.shell_session_open(7).await;
        audit.shell_session_close(7, Some(0)).await;

        let raw = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("shell_session_open"));
        assert!(lines[0].contains("host-aabbccdd"));
        assert!(lines[1].contains("shell_session_close"));
        assert!(lines[1].contains("7:0"));
    }

    #[test]
    fn default_audit_path_is_platform_specific() {
        let p = default_audit_path();
        let s = p.display().to_string();
        #[cfg(target_os = "linux")]
        assert!(s.contains("/var/lib/nexus-agent/"));
        #[cfg(target_os = "macos")]
        assert!(s.contains("/var/db/nexus-agent/"));
        #[cfg(target_os = "windows")]
        assert!(s.contains("nexus-agent"));
        let _ = s; // silence unused if no cfg matches
    }
}
