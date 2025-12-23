//! Dashboard data models for compliance monitoring
//!
//! This module contains all the core data structures for the governance dashboard.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Framework Models
// ============================================================================

/// Compliance framework definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub control_count: usize,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Framework {
    /// Create a new framework
    pub fn new(id: &str, name: &str, version: &str, description: &str, category: &str) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            control_count: 0,
            category: category.to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// Control Models
// ============================================================================

/// Control implementation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStatus {
    Pass,
    Fail,
    Pending,
    NotApplicable,
}

impl std::fmt::Display for ControlStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlStatus::Pass => write!(f, "Pass"),
            ControlStatus::Fail => write!(f, "Fail"),
            ControlStatus::Pending => write!(f, "Pending"),
            ControlStatus::NotApplicable => write!(f, "Not Applicable"),
        }
    }
}

/// Compliance control definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    pub id: String,
    pub framework_id: String,
    pub control_number: String,
    pub title: String,
    pub requirement: String,
    pub status: ControlStatus,
    pub owner: String,
    pub last_assessed: Option<DateTime<Utc>>,
    pub next_assessment: Option<DateTime<Utc>>,
    pub evidence_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Control {
    /// Create a new control
    pub fn new(
        id: &str,
        framework_id: &str,
        control_number: &str,
        title: &str,
        requirement: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            framework_id: framework_id.to_string(),
            control_number: control_number.to_string(),
            title: title.to_string(),
            requirement: requirement.to_string(),
            status: ControlStatus::Pending,
            owner: String::new(),
            last_assessed: None,
            next_assessment: None,
            evidence_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Aggregated control view with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStatusView {
    pub control: Control,
    pub evidence_count: usize,
    pub latest_evidence: Option<Evidence>,
    pub days_since_assessment: Option<i64>,
    pub days_until_next: Option<i64>,
}

// ============================================================================
// Asset Models
// ============================================================================

/// Type of asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Server,
    Database,
    Application,
    Network,
    Storage,
    Endpoint,
    Cloud,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetType::Server => write!(f, "Server"),
            AssetType::Database => write!(f, "Database"),
            AssetType::Application => write!(f, "Application"),
            AssetType::Network => write!(f, "Network"),
            AssetType::Storage => write!(f, "Storage"),
            AssetType::Endpoint => write!(f, "Endpoint"),
            AssetType::Cloud => write!(f, "Cloud"),
        }
    }
}

/// Deployment environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    Production,
    Staging,
    Development,
    Testing,
}

/// Asset criticality level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Criticality {
    Critical,
    High,
    Medium,
    Low,
}

/// Asset inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub description: String,
    pub owner: String,
    pub location: String,
    pub environment: Environment,
    pub criticality: Criticality,
    pub compliance_status: HashMap<String, bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Asset {
    /// Create a new asset
    pub fn new(id: &str, name: &str, asset_type: AssetType) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            asset_type,
            description: String::new(),
            owner: String::new(),
            location: String::new(),
            environment: Environment::Production,
            criticality: Criticality::Medium,
            compliance_status: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Asset compliance summary view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetComplianceView {
    pub asset: Asset,
    pub total_frameworks: usize,
    pub compliant_frameworks: usize,
    pub non_compliant_frameworks: usize,
    pub compliance_percentage: f64,
    pub failing_controls: Vec<String>,
}

// ============================================================================
// Evidence Models
// ============================================================================

/// Type of evidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    Screenshot,
    LogFile,
    ConfigFile,
    ScanReport,
    PolicyDocument,
    Attestation,
}

/// Evidence review status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceStatus {
    Collected,
    UnderReview,
    Approved,
    Rejected,
}

/// Chain of custody event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEvent {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub notes: Option<String>,
}

/// Compliance evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub control_id: String,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub collected_by: String,
    pub collected_at: DateTime<Utc>,
    pub chain_of_custody: Vec<CustodyEvent>,
    pub hash: String,
    pub status: EvidenceStatus,
}

impl Evidence {
    /// Create new evidence
    pub fn new(id: &str, control_id: &str, evidence_type: EvidenceType, title: &str) -> Self {
        Self {
            id: id.to_string(),
            control_id: control_id.to_string(),
            evidence_type,
            title: title.to_string(),
            description: String::new(),
            file_path: None,
            collected_by: String::new(),
            collected_at: Utc::now(),
            chain_of_custody: Vec::new(),
            hash: String::new(),
            status: EvidenceStatus::Collected,
        }
    }
}

// ============================================================================
// Compliance Score Models
// ============================================================================

/// Score trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreTrend {
    Improving,
    Declining,
    Stable,
}

/// Compliance score for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    pub framework_id: String,
    pub total_controls: usize,
    pub passing_controls: usize,
    pub failing_controls: usize,
    pub pending_controls: usize,
    pub na_controls: usize,
    pub percentage: f64,
    pub last_calculated: DateTime<Utc>,
    pub trend: ScoreTrend,
}

impl ComplianceScore {
    /// Create a new compliance score
    pub fn new(framework_id: &str) -> Self {
        Self {
            framework_id: framework_id.to_string(),
            total_controls: 0,
            passing_controls: 0,
            failing_controls: 0,
            pending_controls: 0,
            na_controls: 0,
            percentage: 0.0,
            last_calculated: Utc::now(),
            trend: ScoreTrend::Stable,
        }
    }

    /// Calculate score from counts
    pub fn calculate(&mut self) {
        let applicable = self.total_controls.saturating_sub(self.na_controls);
        self.percentage = if applicable > 0 {
            (self.passing_controls as f64 / applicable as f64) * 100.0
        } else {
            0.0
        };
        self.last_calculated = Utc::now();
    }

    /// Get grade based on score
    pub fn grade(&self) -> &'static str {
        match self.percentage as u32 {
            95..=100 => "A+",
            90..=94 => "A",
            85..=89 => "A-",
            80..=84 => "B+",
            75..=79 => "B",
            70..=74 => "B-",
            65..=69 => "C+",
            60..=64 => "C",
            55..=59 => "C-",
            50..=54 => "D",
            _ => "F",
        }
    }
}

// ============================================================================
// Report Models
// ============================================================================

/// Report type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    FullAudit,
    ExecutiveSummary,
    ControlMatrix,
    EvidencePackage,
}

/// Report generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
    Queued,
    Generating,
    Completed,
    Failed,
}

/// Report generation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportJob {
    pub id: String,
    pub report_type: ReportType,
    pub framework_ids: Vec<String>,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub status: ReportStatus,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

impl ReportJob {
    /// Create a new report job
    pub fn new(id: &str, report_type: ReportType, framework_ids: Vec<String>, requested_by: &str) -> Self {
        Self {
            id: id.to_string(),
            report_type,
            framework_ids,
            requested_by: requested_by.to_string(),
            requested_at: Utc::now(),
            status: ReportStatus::Queued,
            file_path: None,
            error: None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_creation() {
        let fw = Framework::new("nist-800-53", "NIST 800-53", "Rev 5", "Security Controls", "US Government");
        assert_eq!(fw.id, "nist-800-53");
        assert_eq!(fw.name, "NIST 800-53");
    }

    #[test]
    fn test_control_status() {
        assert_eq!(ControlStatus::Pass.to_string(), "Pass");
        assert_eq!(ControlStatus::NotApplicable.to_string(), "Not Applicable");
    }

    #[test]
    fn test_compliance_score_calculation() {
        let mut score = ComplianceScore::new("test");
        score.total_controls = 100;
        score.passing_controls = 80;
        score.failing_controls = 15;
        score.pending_controls = 3;
        score.na_controls = 2;
        score.calculate();

        // 80 / 98 (100-2) = 81.6%
        assert!(score.percentage > 81.0 && score.percentage < 82.0);
        assert_eq!(score.grade(), "B+");
    }

    #[test]
    fn test_compliance_score_grades() {
        let mut score = ComplianceScore::new("test");
        score.total_controls = 100;
        score.na_controls = 0;

        score.passing_controls = 100;
        score.calculate();
        assert_eq!(score.grade(), "A+");

        score.passing_controls = 75;
        score.calculate();
        assert_eq!(score.grade(), "B");

        score.passing_controls = 45;
        score.calculate();
        assert_eq!(score.grade(), "F");
    }

    #[test]
    fn test_evidence_creation() {
        let ev = Evidence::new("ev-001", "ctrl-001", EvidenceType::PolicyDocument, "Policy v1");
        assert_eq!(ev.id, "ev-001");
        assert_eq!(ev.control_id, "ctrl-001");
        assert_eq!(ev.status, EvidenceStatus::Collected);
    }

    #[test]
    fn test_asset_creation() {
        let asset = Asset::new("asset-001", "web-server-01", AssetType::Server);
        assert_eq!(asset.id, "asset-001");
        assert_eq!(asset.asset_type, AssetType::Server);
        assert_eq!(asset.criticality, Criticality::Medium);
    }
}
