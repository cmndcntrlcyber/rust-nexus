//! # gov-reporting
//!
//! Audit-ready report generation for compliance frameworks.
//!
//! This crate provides report generation capabilities for compliance
//! assessments, with support for multiple output formats.
//!
//! ## Features
//!
//! - Compliance score calculation
//! - Executive summary generation
//! - Multiple output formats (JSON, HTML, Markdown, CSV)
//! - Control coverage tracking
//! - Trend analysis
//!
//! ## Example
//!
//! ```rust
//! use gov_reporting::{
//!     ReportGenerator, ReportMetadata, ControlCoverage, ControlStatus, OutputFormat
//! };
//!
//! let generator = ReportGenerator::new();
//!
//! let metadata = ReportMetadata {
//!     title: "Q1 2024 Compliance Report".to_string(),
//!     framework: Some("NIST-800-53".to_string()),
//!     ..Default::default()
//! };
//!
//! let coverage = vec![
//!     ControlCoverage {
//!         control_id: "AC-1".to_string(),
//!         control_name: "Access Control Policy".to_string(),
//!         status: ControlStatus::Implemented,
//!         evidence_count: 5,
//!         last_assessed: Some(chrono::Utc::now()),
//!         notes: None,
//!     },
//! ];
//!
//! let report = generator.generate(metadata, coverage, None).unwrap();
//!
//! // Export to different formats
//! let html = generator.export(&report, OutputFormat::Html).unwrap();
//! let json = generator.export(&report, OutputFormat::Json).unwrap();
//! ```

pub mod error;
pub mod generator;
pub mod types;

// Re-export main types
pub use error::{ReportError, Result};
pub use generator::ReportGenerator;
pub use types::{
    Classification, ComplianceScore, ControlCoverage, ControlStatus, ExecutiveSummary,
    OutputFormat, Report, ReportMetadata, ReportSection, ReportTemplate, TemplateSectionDef, Trend,
};
