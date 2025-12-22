//! Evidence collection and chain of custody
//!
//! Provides structures for collecting, storing, and tracking
//! compliance evidence with full chain of custody support.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Type of evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Screenshot of configuration
    Screenshot,
    /// Configuration file content
    ConfigFile,
    /// Log entries
    LogEntries,
    /// Automated test result
    TestResult,
    /// Policy document
    PolicyDocument,
    /// Procedure document
    ProcedureDocument,
    /// Training record
    TrainingRecord,
    /// Audit report
    AuditReport,
    /// System output/report
    SystemReport,
    /// Interview notes
    InterviewNotes,
    /// Attestation from personnel
    Attestation,
    /// Third-party certification
    ThirdPartyCert,
    /// Risk assessment
    RiskAssessment,
    /// Vulnerability scan result
    VulnerabilityScan,
    /// Penetration test result
    PenTestResult,
    /// Network diagram
    NetworkDiagram,
    /// Data flow diagram
    DataFlowDiagram,
    /// Other
    Other(String),
}

impl EvidenceType {
    pub fn display_name(&self) -> String {
        match self {
            Self::Screenshot => "Screenshot".to_string(),
            Self::ConfigFile => "Configuration File".to_string(),
            Self::LogEntries => "Log Entries".to_string(),
            Self::TestResult => "Automated Test Result".to_string(),
            Self::PolicyDocument => "Policy Document".to_string(),
            Self::ProcedureDocument => "Procedure Document".to_string(),
            Self::TrainingRecord => "Training Record".to_string(),
            Self::AuditReport => "Audit Report".to_string(),
            Self::SystemReport => "System Report".to_string(),
            Self::InterviewNotes => "Interview Notes".to_string(),
            Self::Attestation => "Attestation".to_string(),
            Self::ThirdPartyCert => "Third-Party Certification".to_string(),
            Self::RiskAssessment => "Risk Assessment".to_string(),
            Self::VulnerabilityScan => "Vulnerability Scan".to_string(),
            Self::PenTestResult => "Penetration Test Result".to_string(),
            Self::NetworkDiagram => "Network Diagram".to_string(),
            Self::DataFlowDiagram => "Data Flow Diagram".to_string(),
            Self::Other(s) => s.clone(),
        }
    }
}

/// Status of evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceStatus {
    /// Evidence is current and valid
    Current,
    /// Evidence is stale and needs refresh
    Stale,
    /// Evidence has expired
    Expired,
    /// Evidence is pending collection
    Pending,
    /// Evidence is under review
    UnderReview,
    /// Evidence has been rejected
    Rejected,
    /// Evidence is archived
    Archived,
}

/// A piece of compliance evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique identifier
    pub id: String,

    /// Control ID this evidence supports
    pub control_id: String,

    /// Framework ID
    pub framework_id: String,

    /// Type of evidence
    pub evidence_type: EvidenceType,

    /// Title/description
    pub title: String,

    /// Detailed description
    pub description: String,

    /// When the evidence was collected
    pub collected_at: DateTime<Utc>,

    /// Who/what collected the evidence
    pub collected_by: String,

    /// Data content (may be base64 encoded for binary)
    pub data: Option<Vec<u8>>,

    /// Text content
    pub text_content: Option<String>,

    /// File path if stored externally
    pub file_path: Option<String>,

    /// MIME type
    pub mime_type: Option<String>,

    /// SHA-256 hash of the data for integrity verification
    pub hash: String,

    /// Status of the evidence
    pub status: EvidenceStatus,

    /// Expiration date (for time-sensitive evidence)
    pub expires_at: Option<DateTime<Utc>>,

    /// Chain of custody entries
    pub chain_of_custody: Vec<CustodyEntry>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Evidence {
    /// Create new evidence
    pub fn new(
        id: impl Into<String>,
        control_id: impl Into<String>,
        framework_id: impl Into<String>,
        evidence_type: EvidenceType,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            control_id: control_id.into(),
            framework_id: framework_id.into(),
            evidence_type,
            title: title.into(),
            description: String::new(),
            collected_at: Utc::now(),
            collected_by: "system".to_string(),
            data: None,
            text_content: None,
            file_path: None,
            mime_type: None,
            hash: String::new(),
            status: EvidenceStatus::Current,
            expires_at: None,
            chain_of_custody: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set collector
    pub fn with_collected_by(mut self, collector: impl Into<String>) -> Self {
        self.collected_by = collector.into();
        self
    }

    /// Set text content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.hash = Self::compute_hash(text.as_bytes());
        self.text_content = Some(text);
        self.mime_type = Some("text/plain".to_string());
        self
    }

    /// Set binary data
    pub fn with_data(mut self, data: Vec<u8>, mime_type: &str) -> Self {
        self.hash = Self::compute_hash(&data);
        self.data = Some(data);
        self.mime_type = Some(mime_type.to_string());
        self
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add a custody entry
    pub fn add_custody_entry(&mut self, entry: CustodyEntry) {
        self.chain_of_custody.push(entry);
    }

    /// Compute SHA-256 hash (simplified - would use proper crypto in production)
    fn compute_hash(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Check if evidence is current
    pub fn is_current(&self) -> bool {
        if self.status != EvidenceStatus::Current {
            return false;
        }
        if let Some(expires_at) = self.expires_at {
            return Utc::now() < expires_at;
        }
        true
    }

    /// Verify integrity
    pub fn verify_integrity(&self) -> bool {
        if let Some(ref text) = self.text_content {
            return self.hash == Self::compute_hash(text.as_bytes());
        }
        if let Some(ref data) = self.data {
            return self.hash == Self::compute_hash(data);
        }
        // No content to verify
        true
    }
}

/// An entry in the chain of custody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEntry {
    /// When the action occurred
    pub timestamp: DateTime<Utc>,

    /// Who performed the action
    pub actor: String,

    /// What action was taken
    pub action: CustodyAction,

    /// Additional notes
    pub notes: Option<String>,

    /// IP address of the actor (if applicable)
    pub ip_address: Option<String>,
}

impl CustodyEntry {
    /// Create a new custody entry
    pub fn new(actor: impl Into<String>, action: CustodyAction) -> Self {
        Self {
            timestamp: Utc::now(),
            actor: actor.into(),
            action,
            notes: None,
            ip_address: None,
        }
    }

    /// Add notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Add IP address
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }
}

/// Actions that can be taken on evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustodyAction {
    /// Evidence was collected
    Collected,
    /// Evidence was viewed
    Viewed,
    /// Evidence was downloaded
    Downloaded,
    /// Evidence was modified
    Modified,
    /// Evidence was verified
    Verified,
    /// Evidence was approved
    Approved,
    /// Evidence was rejected
    Rejected,
    /// Evidence was transferred
    Transferred,
    /// Evidence was archived
    Archived,
    /// Evidence was restored
    Restored,
    /// Evidence was deleted
    Deleted,
}

/// Evidence collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceConfig {
    /// Maximum retention period in days
    pub retention_days: u32,

    /// Whether to auto-archive expired evidence
    pub auto_archive: bool,

    /// Whether to require approval for evidence
    pub require_approval: bool,

    /// Allowed evidence types
    pub allowed_types: Vec<EvidenceType>,

    /// Maximum file size in bytes
    pub max_file_size: u64,
}

impl Default for EvidenceConfig {
    fn default() -> Self {
        Self {
            retention_days: 365,
            auto_archive: true,
            require_approval: false,
            allowed_types: vec![
                EvidenceType::Screenshot,
                EvidenceType::ConfigFile,
                EvidenceType::LogEntries,
                EvidenceType::TestResult,
                EvidenceType::SystemReport,
            ],
            max_file_size: 50 * 1024 * 1024, // 50 MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_creation() {
        let evidence = Evidence::new(
            "ev-001",
            "AC-1",
            "nist_800_53",
            EvidenceType::ConfigFile,
            "Access Control Configuration",
        )
        .with_text("key=value\nother=setting")
        .with_collected_by("compliance-agent");

        assert!(evidence.is_current());
        assert!(evidence.verify_integrity());
    }

    #[test]
    fn test_chain_of_custody() {
        let mut evidence = Evidence::new(
            "ev-002",
            "AC-2",
            "nist_800_53",
            EvidenceType::TestResult,
            "User Account Test",
        );

        evidence.add_custody_entry(
            CustodyEntry::new("system", CustodyAction::Collected)
                .with_notes("Automated collection")
        );

        evidence.add_custody_entry(
            CustodyEntry::new("auditor@example.com", CustodyAction::Viewed)
        );

        assert_eq!(evidence.chain_of_custody.len(), 2);
    }

    #[test]
    fn test_evidence_expiration() {
        let expired = Evidence::new(
            "ev-003",
            "AC-3",
            "nist_800_53",
            EvidenceType::Attestation,
            "Manager Attestation",
        )
        .with_expiration(Utc::now() - chrono::Duration::days(1));

        assert!(!expired.is_current());
    }
}
