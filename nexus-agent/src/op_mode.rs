//! Operational mode for the agent — Lab vs. Field.
//!
//! Controls evasion behavior, logging verbosity, heartbeat cadence, and
//! other posture knobs that change between development/testing and
//! real-world deployment.

use serde::{Deserialize, Serialize};

/// Operational mode toggle.
///
/// * **Lab** — permissive logging, fast heartbeat, evasion disabled.
/// * **Field** — minimal logging, jittered heartbeat, evasion enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpMode {
    /// Development / lab environment.
    Lab,
    /// Operational / field deployment.
    Field,
}

impl Default for OpMode {
    fn default() -> Self {
        OpMode::Lab
    }
}

impl OpMode {
    /// Returns `true` when in lab mode.
    pub fn is_lab(&self) -> bool {
        matches!(self, OpMode::Lab)
    }

    /// Returns `true` when in field mode.
    pub fn is_field(&self) -> bool {
        matches!(self, OpMode::Field)
    }

    /// Whether evasion checks should run.
    pub fn evasion_enabled(&self) -> bool {
        self.is_field()
    }

    /// Recommended tracing / log level for this mode.
    pub fn log_level(&self) -> &'static str {
        match self {
            OpMode::Lab => "debug",
            OpMode::Field => "warn",
        }
    }

    /// Base heartbeat interval in seconds.
    pub fn heartbeat_base_secs(&self) -> u64 {
        match self {
            OpMode::Lab => 5,
            OpMode::Field => 60,
        }
    }

    /// Jitter range `(min, max)` in seconds added to the heartbeat base.
    pub fn heartbeat_jitter_range(&self) -> (u64, u64) {
        match self {
            OpMode::Lab => (0, 2),
            OpMode::Field => (30, 90),
        }
    }
}

impl std::fmt::Display for OpMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpMode::Lab => write!(f, "lab"),
            OpMode::Field => write!(f, "field"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_lab() {
        assert_eq!(OpMode::default(), OpMode::Lab);
    }

    #[test]
    fn test_lab_predicates() {
        let mode = OpMode::Lab;
        assert!(mode.is_lab());
        assert!(!mode.is_field());
        assert!(!mode.evasion_enabled());
    }

    #[test]
    fn test_field_predicates() {
        let mode = OpMode::Field;
        assert!(!mode.is_lab());
        assert!(mode.is_field());
        assert!(mode.evasion_enabled());
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(OpMode::Lab.log_level(), "debug");
        assert_eq!(OpMode::Field.log_level(), "warn");
    }

    #[test]
    fn test_heartbeat_base() {
        assert_eq!(OpMode::Lab.heartbeat_base_secs(), 5);
        assert_eq!(OpMode::Field.heartbeat_base_secs(), 60);
    }

    #[test]
    fn test_heartbeat_jitter() {
        let (min, max) = OpMode::Lab.heartbeat_jitter_range();
        assert_eq!(min, 0);
        assert_eq!(max, 2);

        let (min, max) = OpMode::Field.heartbeat_jitter_range();
        assert_eq!(min, 30);
        assert_eq!(max, 90);
    }

    #[test]
    fn test_serde_roundtrip() {
        let lab_json = serde_json::to_string(&OpMode::Lab).unwrap();
        assert_eq!(lab_json, "\"lab\"");
        let deserialized: OpMode = serde_json::from_str(&lab_json).unwrap();
        assert_eq!(deserialized, OpMode::Lab);

        let field_json = serde_json::to_string(&OpMode::Field).unwrap();
        assert_eq!(field_json, "\"field\"");
        let deserialized: OpMode = serde_json::from_str(&field_json).unwrap();
        assert_eq!(deserialized, OpMode::Field);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", OpMode::Lab), "lab");
        assert_eq!(format!("{}", OpMode::Field), "field");
    }
}
