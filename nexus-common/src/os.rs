//! Host OS detection (v1.1 simple-mesh layer).
//!
//! Used by:
//! - `nexus-agent` to pick a shell (PowerShell vs `$SHELL` vs `bash` vs `sh`)
//! - `nexus-console` to display an OS label next to each registered agent
//! - The A2A `RegisteredAgent` message's `os` field
//!
//! Coexists with — and is distinct from — the overlay's `Platform` enum in
//! `nexus_common::technique` which is ATT&CK-flavored (Windows / Linux /
//! macOS / Android / iOS).

use serde::{Deserialize, Serialize};

/// Coarse host OS classification. v1.0 deliberately collapses BSDs, illumos,
/// etc. into `Other` — the agent fallback for `Other` is `/bin/sh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    /// Microsoft Windows.
    Windows,
    /// Any Linux distribution.
    Linux,
    /// Apple macOS.
    MacOS,
    /// Anything else (BSDs, illumos, unknown).
    Other,
}

impl OsKind {
    /// Detect the host OS at compile time per the target_os cfg.
    #[must_use]
    pub const fn detect() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOS
        } else {
            Self::Other
        }
    }

    /// Lowercase string suitable for protocol payloads and logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for OsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_is_concrete_on_supported_hosts() {
        let kind = OsKind::detect();
        #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
        assert_ne!(kind, OsKind::Other);
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        assert_eq!(kind, OsKind::Other);
    }

    #[test]
    fn as_str_matches_serde_repr() {
        let kind = OsKind::detect();
        let json = serde_json::to_string(&kind).expect("serialize");
        let trimmed = json.trim_matches('"');
        assert_eq!(trimmed, kind.as_str());
    }
}
