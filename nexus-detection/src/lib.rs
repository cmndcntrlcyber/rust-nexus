//! # nexus-detection
//!
//! Threat detection capabilities for the d3tect-nexus SOC platform.
//!
//! This crate provides:
//! - **Signature-based detection**: Pattern matching for known threats
//! - **Behavioral analysis**: Process and network behavior monitoring
//! - **LitterBox integration**: Automated malware sandbox analysis
//! - **Event correlation**: Multi-source event aggregation and alerting
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │ nexus-detection │────►│   LitterBox     │
//! │                 │     │   (Sandbox)     │
//! ├─────────────────┤     └─────────────────┘
//! │ • Signatures    │              │
//! │ • Behavioral    │              ▼
//! │ • Network       │     ┌─────────────────┐
//! │ • Process       │◄────│ Analysis Results│
//! └─────────────────┘     └─────────────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Event Pipeline  │
//! └─────────────────┘
//! ```

use thiserror::Error;

pub mod types;
pub mod signature;
pub mod behavioral;
pub mod network;
pub mod process;
pub mod correlation;
pub mod litterbox;

pub use types::*;
pub use signature::SignatureEngine;
pub use behavioral::BehavioralAnalyzer;
pub use network::NetworkMonitor;
pub use process::ProcessMonitor;
pub use correlation::{EventCorrelator, EventPipeline, PipelineConfig, PipelineStats, EventHandler, LoggingHandler};
pub use litterbox::LitterBoxClient;

/// Detection-specific errors
#[derive(Error, Debug)]
pub enum DetectionError {
    #[error("Signature matching failed: {0}")]
    SignatureError(String),

    #[error("Behavioral analysis failed: {0}")]
    BehavioralError(String),

    #[error("Network monitoring failed: {0}")]
    NetworkError(String),

    #[error("Process monitoring failed: {0}")]
    ProcessError(String),

    #[error("Event correlation failed: {0}")]
    CorrelationError(String),

    #[error("LitterBox API error: {0}")]
    LitterBoxError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for detection operations
pub type Result<T> = std::result::Result<T, DetectionError>;

/// Detection engine configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DetectionConfig {
    /// Enable signature-based detection
    pub signatures_enabled: bool,
    /// Enable behavioral analysis
    pub behavioral_enabled: bool,
    /// Enable network monitoring
    pub network_enabled: bool,
    /// Enable process monitoring
    pub process_enabled: bool,
    /// LitterBox sandbox URL
    pub litterbox_url: Option<String>,
    /// Event correlation window in seconds
    pub correlation_window_secs: u64,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            signatures_enabled: true,
            behavioral_enabled: true,
            network_enabled: true,
            process_enabled: true,
            litterbox_url: None,
            correlation_window_secs: 300, // 5 minutes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DetectionConfig::default();
        assert!(config.signatures_enabled);
        assert!(config.behavioral_enabled);
        assert_eq!(config.correlation_window_secs, 300);
    }

    #[test]
    fn test_error_display() {
        let err = DetectionError::SignatureError("test error".to_string());
        assert!(err.to_string().contains("Signature matching failed"));
    }
}
