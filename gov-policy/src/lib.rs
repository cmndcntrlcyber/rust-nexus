//! # gov-policy
//!
//! Policy engine for drift detection and compliance enforcement.
//!
//! This crate provides policy definition, evaluation, and drift detection
//! capabilities for the compliance platform.
//!
//! ## Features
//!
//! - Policy definitions with multiple rules
//! - Various check operators (equals, contains, matches, etc.)
//! - Baseline snapshot management
//! - Drift detection and reporting
//! - Severity-based violation tracking
//! - Control mapping for compliance
//!
//! ## Example
//!
//! ```rust
//! use gov_policy::{Policy, PolicyRule, CheckOperator, DriftDetector, BaselineSnapshot};
//!
//! // Create a policy
//! let policy = Policy::new("SSH Security Policy", "1.0.0")
//!     .with_framework("NIST-800-53")
//!     .with_rule(PolicyRule::new(
//!         "ssh-protocol",
//!         "SSH Protocol Version",
//!         "$.ssh.protocol",
//!         CheckOperator::Equals,
//!     ).with_expected(serde_json::json!(2)));
//!
//! // Create baseline from known-good configuration
//! let baseline_data = serde_json::json!({
//!     "ssh": { "protocol": 2, "permitRootLogin": "no" }
//! });
//! let baseline = BaselineSnapshot::new("prod-baseline", "server-01", baseline_data.clone(), "admin");
//!
//! // Check current state for drift
//! let mut detector = DriftDetector::new();
//! let report = detector.detect(&policy, &baseline_data, &baseline).unwrap();
//!
//! if report.compliant {
//!     println!("No drift detected!");
//! } else {
//!     println!("Found {} violations", report.violations.len());
//! }
//! ```

pub mod error;
pub mod evaluator;
pub mod types;

// Re-export main types
pub use error::{PolicyError, Result};
pub use evaluator::{DriftDetector, PolicyEvaluator};
pub use types::{
    BaselineSnapshot, CheckOperator, DriftReport, DriftViolation, Policy, PolicyRule,
    RemediationSuggestion, Severity,
};
