//! Common types for detection operations

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Severity level for detections
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational - no immediate action required
    Info,
    /// Low - minor concern, monitor
    Low,
    /// Medium - potential threat, investigate
    Medium,
    /// High - likely threat, respond promptly
    High,
    /// Critical - confirmed threat, immediate action required
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Info
    }
}

/// Detection event from any detection source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEvent {
    /// Unique event identifier
    pub id: String,
    /// Timestamp of detection
    pub timestamp: DateTime<Utc>,
    /// Detection source (signature, behavioral, network, process)
    pub source: DetectionSource,
    /// Severity level
    pub severity: Severity,
    /// Rule or pattern that triggered detection
    pub rule_id: String,
    /// Human-readable description
    pub description: String,
    /// Affected asset identifier
    pub asset_id: Option<String>,
    /// Additional context data
    pub context: DetectionContext,
    /// MITRE ATT&CK technique IDs if applicable
    pub mitre_techniques: Vec<String>,
}

impl DetectionEvent {
    /// Create a new detection event
    pub fn new(
        source: DetectionSource,
        severity: Severity,
        rule_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            source,
            severity,
            rule_id: rule_id.into(),
            description: description.into(),
            asset_id: None,
            context: DetectionContext::default(),
            mitre_techniques: Vec::new(),
        }
    }

    /// Add asset identifier
    pub fn with_asset(mut self, asset_id: impl Into<String>) -> Self {
        self.asset_id = Some(asset_id.into());
        self
    }

    /// Add MITRE ATT&CK technique
    pub fn with_mitre(mut self, technique_id: impl Into<String>) -> Self {
        self.mitre_techniques.push(technique_id.into());
        self
    }
}

/// Source of detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionSource {
    /// Signature-based detection
    Signature,
    /// Behavioral analysis
    Behavioral,
    /// Network traffic analysis
    Network,
    /// Process monitoring
    Process,
    /// LitterBox sandbox analysis
    Sandbox,
    /// Correlated from multiple sources
    Correlation,
}

/// Additional context for detection events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionContext {
    /// Process information if applicable
    pub process: Option<ProcessContext>,
    /// Network information if applicable
    pub network: Option<NetworkContext>,
    /// File information if applicable
    pub file: Option<FileContext>,
    /// Raw data or additional metadata
    pub raw_data: Option<serde_json::Value>,
}

/// Process context for detections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessContext {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Executable path
    pub path: Option<String>,
    /// Command line arguments
    pub command_line: Option<String>,
    /// Parent process ID
    pub parent_pid: Option<u32>,
    /// User running the process
    pub user: Option<String>,
}

/// Network context for detections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContext {
    /// Source IP address
    pub src_ip: Option<String>,
    /// Source port
    pub src_port: Option<u16>,
    /// Destination IP address
    pub dst_ip: Option<String>,
    /// Destination port
    pub dst_port: Option<u16>,
    /// Protocol (TCP, UDP, etc.)
    pub protocol: Option<String>,
    /// Bytes transferred
    pub bytes: Option<u64>,
}

/// File context for detections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    /// File path
    pub path: String,
    /// File hash (SHA256)
    pub sha256: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    /// File type or MIME type
    pub file_type: Option<String>,
}

/// Indicator of Compromise (IOC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOC {
    /// IOC type
    pub ioc_type: IOCType,
    /// IOC value
    pub value: String,
    /// Source of the IOC
    pub source: String,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// When the IOC was first seen
    pub first_seen: Option<DateTime<Utc>>,
    /// When the IOC was last seen
    pub last_seen: Option<DateTime<Utc>>,
    /// Associated tags
    pub tags: Vec<String>,
}

/// Types of Indicators of Compromise
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IOCType {
    /// IP address
    IpAddress,
    /// Domain name
    Domain,
    /// URL
    Url,
    /// File hash (MD5, SHA1, SHA256)
    FileHash,
    /// Email address
    Email,
    /// File path pattern
    FilePath,
    /// Registry key
    RegistryKey,
    /// Process name pattern
    ProcessName,
    /// Command line pattern
    CommandLine,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_event_creation() {
        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "SIG-001",
            "Reverse shell pattern detected",
        )
        .with_asset("endpoint-001")
        .with_mitre("T1059.004");

        assert_eq!(event.severity, Severity::High);
        assert_eq!(event.source, DetectionSource::Signature);
        assert_eq!(event.asset_id, Some("endpoint-001".to_string()));
        assert!(event.mitre_techniques.contains(&"T1059.004".to_string()));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }
}
