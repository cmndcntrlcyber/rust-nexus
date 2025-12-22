//! Control definitions and tests
//!
//! Defines the Control struct for compliance controls and
//! the ControlTest struct for automated compliance testing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::mapping::ControlMapping;

/// Control identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlId(pub String);

impl ControlId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ControlId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for ControlId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a control assessment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStatus {
    /// Control has not been assessed
    NotAssessed,
    /// Control is compliant
    Compliant,
    /// Control is partially compliant
    PartiallyCompliant,
    /// Control is non-compliant
    NonCompliant,
    /// Control is not applicable to this environment
    NotApplicable,
    /// Control requires manual review
    ManualReviewRequired,
    /// Error occurred during assessment
    Error,
}

impl ControlStatus {
    pub fn is_compliant(&self) -> bool {
        matches!(self, Self::Compliant | Self::NotApplicable)
    }

    pub fn requires_action(&self) -> bool {
        matches!(
            self,
            Self::NonCompliant | Self::PartiallyCompliant | Self::ManualReviewRequired
        )
    }
}

/// Implementation status of a control
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationStatus {
    /// Not yet implemented
    NotImplemented,
    /// Partially implemented
    InProgress,
    /// Fully implemented
    Implemented,
    /// Implementation planned
    Planned,
    /// Not applicable
    NotApplicable,
}

/// A compliance control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    /// Control identifier (e.g., "AC-1", "A.5.1")
    pub id: ControlId,

    /// Control name
    pub name: String,

    /// Detailed description
    pub description: String,

    /// Domain/category within the framework (e.g., "Access Control")
    pub domain_id: Option<String>,

    /// Control family (for grouping related controls)
    pub family: Option<String>,

    /// Guidance for implementing the control
    pub implementation_guidance: Option<String>,

    /// Assessment procedures
    pub assessment_procedure: Option<String>,

    /// Automated tests for this control
    pub tests: Vec<ControlTest>,

    /// Mappings to controls in other frameworks
    pub cross_mappings: Vec<ControlMapping>,

    /// Evidence types required for this control
    pub evidence_requirements: Vec<String>,

    /// Priority/importance level (1-5, 5 being highest)
    pub priority: u8,

    /// Whether this control can be tested automatically
    pub automatable: bool,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Control {
    /// Create a new control
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: ControlId::new(id),
            name: name.into(),
            description: String::new(),
            domain_id: None,
            family: None,
            implementation_guidance: None,
            assessment_procedure: None,
            tests: Vec::new(),
            cross_mappings: Vec::new(),
            evidence_requirements: Vec::new(),
            priority: 3,
            automatable: false,
            metadata: HashMap::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the domain
    pub fn with_domain(mut self, domain_id: impl Into<String>) -> Self {
        self.domain_id = Some(domain_id.into());
        self
    }

    /// Set the family
    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    /// Add implementation guidance
    pub fn with_guidance(mut self, guidance: impl Into<String>) -> Self {
        self.implementation_guidance = Some(guidance.into());
        self
    }

    /// Add a test
    pub fn with_test(mut self, test: ControlTest) -> Self {
        self.tests.push(test);
        self.automatable = true;
        self
    }

    /// Add a cross-mapping
    pub fn with_mapping(mut self, mapping: ControlMapping) -> Self {
        self.cross_mappings.push(mapping);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 5);
        self
    }

    /// Add an evidence requirement
    pub fn with_evidence(mut self, evidence_type: impl Into<String>) -> Self {
        self.evidence_requirements.push(evidence_type.into());
        self
    }
}

/// Type of automated check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckType {
    /// Query Windows registry
    RegistryQuery,
    /// Check file permissions
    FilePermissions,
    /// Check if file exists
    FileExists,
    /// Check file content
    FileContent,
    /// Check service status
    ServiceStatus,
    /// Check if process is running
    ProcessRunning,
    /// Execute SCAP/OVAL check
    ScapOval,
    /// Execute CIS Benchmark check
    CisBenchmark,
    /// Custom script (read-only)
    CustomScript,
    /// API endpoint check
    ApiCheck,
    /// Configuration file check
    ConfigCheck,
    /// Network port check
    PortCheck,
    /// User/group membership check
    UserGroupCheck,
    /// Audit log check
    AuditLogCheck,
    /// Certificate validity check
    CertificateCheck,
}

/// Comparison operator for check results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Exists,
    NotExists,
    Matches,      // Regex match
    StartsWith,
    EndsWith,
}

/// An automated test for a control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlTest {
    /// Test identifier
    pub id: String,

    /// Test name
    pub name: String,

    /// Description of what this test verifies
    pub description: String,

    /// Type of check to perform
    pub check_type: CheckType,

    /// Target to check (path, registry key, service name, etc.)
    pub target: String,

    /// Expected value (if applicable)
    pub expected_value: Option<String>,

    /// Comparison operator
    pub operator: ComparisonOperator,

    /// Pass criteria description
    pub pass_criteria: String,

    /// Platforms this test applies to
    pub platforms: Vec<Platform>,

    /// Weight of this test in scoring (0.0 - 1.0)
    pub weight: f64,

    /// Is this test enabled?
    pub enabled: bool,
}

impl ControlTest {
    /// Create a new control test
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        check_type: CheckType,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            check_type,
            target: target.into(),
            expected_value: None,
            operator: ComparisonOperator::Equals,
            pass_criteria: String::new(),
            platforms: vec![Platform::All],
            weight: 1.0,
            enabled: true,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set expected value
    pub fn with_expected(mut self, value: impl Into<String>) -> Self {
        self.expected_value = Some(value.into());
        self
    }

    /// Set operator
    pub fn with_operator(mut self, operator: ComparisonOperator) -> Self {
        self.operator = operator;
        self
    }

    /// Set pass criteria
    pub fn with_pass_criteria(mut self, criteria: impl Into<String>) -> Self {
        self.pass_criteria = criteria.into();
        self
    }

    /// Set platforms
    pub fn with_platforms(mut self, platforms: Vec<Platform>) -> Self {
        self.platforms = platforms;
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
}

/// Platform for test applicability
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    All,
    Windows,
    Linux,
    MacOS,
    Unix,        // Linux + MacOS
    Cloud,       // AWS, Azure, GCP
    Kubernetes,
    Docker,
}

/// Result of running a control test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test ID
    pub test_id: String,

    /// Control ID
    pub control_id: ControlId,

    /// Status of the test
    pub status: ControlStatus,

    /// Actual value found
    pub actual_value: Option<String>,

    /// Expected value
    pub expected_value: Option<String>,

    /// Human-readable message
    pub message: String,

    /// Evidence collected
    pub evidence: Option<String>,

    /// Timestamp of the test
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Duration of the test in milliseconds
    pub duration_ms: u64,
}

impl TestResult {
    pub fn pass(test_id: String, control_id: ControlId, message: String) -> Self {
        Self {
            test_id,
            control_id,
            status: ControlStatus::Compliant,
            actual_value: None,
            expected_value: None,
            message,
            evidence: None,
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
        }
    }

    pub fn fail(
        test_id: String,
        control_id: ControlId,
        message: String,
        actual: Option<String>,
        expected: Option<String>,
    ) -> Self {
        Self {
            test_id,
            control_id,
            status: ControlStatus::NonCompliant,
            actual_value: actual,
            expected_value: expected,
            message,
            evidence: None,
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
        }
    }

    pub fn error(test_id: String, control_id: ControlId, message: String) -> Self {
        Self {
            test_id,
            control_id,
            status: ControlStatus::Error,
            actual_value: None,
            expected_value: None,
            message,
            evidence: None,
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
        }
    }

    pub fn with_evidence(mut self, evidence: String) -> Self {
        self.evidence = Some(evidence);
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_creation() {
        let control = Control::new("AC-1", "Access Control Policy")
            .with_description("Establish access control policy")
            .with_domain("AC")
            .with_priority(4);

        assert_eq!(control.id.as_str(), "AC-1");
        assert_eq!(control.priority, 4);
    }

    #[test]
    fn test_control_test_creation() {
        let test = ControlTest::new(
            "test-1",
            "Check Password Policy",
            CheckType::RegistryQuery,
            r"HKLM\SYSTEM\CurrentControlSet\Services\Netlogon\Parameters",
        )
        .with_expected("1")
        .with_operator(ComparisonOperator::GreaterThanOrEqual);

        assert_eq!(test.check_type, CheckType::RegistryQuery);
    }

    #[test]
    fn test_control_status() {
        assert!(ControlStatus::Compliant.is_compliant());
        assert!(ControlStatus::NotApplicable.is_compliant());
        assert!(!ControlStatus::NonCompliant.is_compliant());
        assert!(ControlStatus::NonCompliant.requires_action());
    }
}
