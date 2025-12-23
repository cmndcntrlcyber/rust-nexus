use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Severity level for policy violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational only
    Info,
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "Info"),
            Severity::Low => write!(f, "Low"),
            Severity::Medium => write!(f, "Medium"),
            Severity::High => write!(f, "High"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

/// Type of check to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckOperator {
    /// Value must equal expected
    Equals,
    /// Value must not equal expected
    NotEquals,
    /// Value must contain expected substring
    Contains,
    /// Value must not contain expected substring
    NotContains,
    /// Value must match regex pattern
    Matches,
    /// Value must not match regex pattern
    NotMatches,
    /// Numeric value must be greater than expected
    GreaterThan,
    /// Numeric value must be less than expected
    LessThan,
    /// Numeric value must be greater than or equal to expected
    GreaterThanOrEqual,
    /// Numeric value must be less than or equal to expected
    LessThanOrEqual,
    /// Value must exist (not null/empty)
    Exists,
    /// Value must not exist
    NotExists,
    /// Value must be in list
    InList,
    /// Value must not be in list
    NotInList,
}

/// A single policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Unique rule identifier
    pub id: String,
    /// Human-readable rule name
    pub name: String,
    /// Detailed description
    pub description: Option<String>,
    /// JSON path to the value to check
    pub path: String,
    /// Operator to apply
    pub operator: CheckOperator,
    /// Expected value (for comparison operators)
    pub expected: Option<serde_json::Value>,
    /// Severity of violation
    pub severity: Severity,
    /// Related compliance controls
    pub controls: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether this rule is enabled
    pub enabled: bool,
}

impl PolicyRule {
    /// Create a new policy rule
    pub fn new(id: &str, name: &str, path: &str, operator: CheckOperator) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            path: path.to_string(),
            operator,
            expected: None,
            severity: Severity::Medium,
            controls: Vec::new(),
            tags: Vec::new(),
            enabled: true,
        }
    }

    /// Set expected value
    pub fn with_expected(mut self, expected: serde_json::Value) -> Self {
        self.expected = Some(expected);
        self
    }

    /// Set severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Add control mapping
    pub fn with_control(mut self, control: &str) -> Self {
        self.controls.push(control.to_string());
        self
    }
}

/// A policy definition containing multiple rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier
    pub id: Uuid,
    /// Policy name
    pub name: String,
    /// Policy version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Target compliance framework
    pub framework: Option<String>,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// When policy was created
    pub created_at: DateTime<Utc>,
    /// When policy was last updated
    pub updated_at: DateTime<Utc>,
    /// Whether this policy is enabled
    pub enabled: bool,
}

impl Policy {
    /// Create a new policy
    pub fn new(name: &str, version: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            version: version.to_string(),
            description: None,
            framework: None,
            rules: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            enabled: true,
        }
    }

    /// Add a rule to the policy
    pub fn with_rule(mut self, rule: PolicyRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Set framework
    pub fn with_framework(mut self, framework: &str) -> Self {
        self.framework = Some(framework.to_string());
        self
    }

    /// Get enabled rules only
    pub fn enabled_rules(&self) -> Vec<&PolicyRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }

    /// Count rules by severity
    pub fn rules_by_severity(&self, severity: Severity) -> usize {
        self.rules.iter().filter(|r| r.severity == severity).count()
    }
}

/// Snapshot of baseline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    /// Unique snapshot identifier
    pub id: Uuid,
    /// Name/label for this baseline
    pub name: String,
    /// Asset or resource this baseline applies to
    pub target: String,
    /// The actual baseline data
    pub data: serde_json::Value,
    /// SHA-256 hash of baseline data
    pub hash: String,
    /// When baseline was captured
    pub captured_at: DateTime<Utc>,
    /// Who captured this baseline
    pub captured_by: String,
    /// Whether this is the active baseline
    pub active: bool,
}

impl BaselineSnapshot {
    /// Create a new baseline snapshot
    pub fn new(name: &str, target: &str, data: serde_json::Value, captured_by: &str) -> Self {
        use sha2::{Digest, Sha256};

        let json_str = serde_json::to_string(&data).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json_str.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            target: target.to_string(),
            data,
            hash,
            captured_at: Utc::now(),
            captured_by: captured_by.to_string(),
            active: true,
        }
    }
}

/// A single drift violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftViolation {
    /// Rule that was violated
    pub rule_id: String,
    /// Rule name
    pub rule_name: String,
    /// Path where drift occurred
    pub path: String,
    /// Expected value
    pub expected: Option<serde_json::Value>,
    /// Actual value found
    pub actual: Option<serde_json::Value>,
    /// Severity of this violation
    pub severity: Severity,
    /// Related controls
    pub controls: Vec<String>,
    /// Suggested remediation
    pub remediation: Option<String>,
}

/// Report of detected drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Unique report identifier
    pub id: Uuid,
    /// Target asset/resource
    pub target: String,
    /// Policy used for evaluation
    pub policy_id: Uuid,
    /// Policy name
    pub policy_name: String,
    /// Baseline compared against
    pub baseline_id: Uuid,
    /// List of violations
    pub violations: Vec<DriftViolation>,
    /// When check was performed
    pub checked_at: DateTime<Utc>,
    /// Overall status (true = compliant, no drift)
    pub compliant: bool,
}

impl DriftReport {
    /// Create a new drift report
    pub fn new(target: &str, policy: &Policy, baseline_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            target: target.to_string(),
            policy_id: policy.id,
            policy_name: policy.name.clone(),
            baseline_id,
            violations: Vec::new(),
            checked_at: Utc::now(),
            compliant: true,
        }
    }

    /// Add a violation
    pub fn add_violation(&mut self, violation: DriftViolation) {
        self.violations.push(violation);
        self.compliant = false;
    }

    /// Count violations by severity
    pub fn violations_by_severity(&self, severity: Severity) -> usize {
        self.violations.iter().filter(|v| v.severity == severity).count()
    }

    /// Get highest severity in report
    pub fn highest_severity(&self) -> Option<Severity> {
        self.violations.iter().map(|v| v.severity).max()
    }
}

/// Suggested remediation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationSuggestion {
    /// Violation this addresses
    pub violation_id: String,
    /// Human-readable description
    pub description: String,
    /// Step-by-step remediation steps
    pub steps: Vec<String>,
    /// Whether this can be auto-remediated
    pub auto_remediate: bool,
    /// Script/command for auto-remediation (if supported)
    pub remediation_script: Option<String>,
    /// Estimated effort (minutes)
    pub estimated_effort_minutes: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new("Test Policy", "1.0.0")
            .with_framework("NIST-800-53")
            .with_rule(PolicyRule::new(
                "rule-1",
                "Check SSH Protocol",
                "$.sshd.protocol",
                CheckOperator::Equals,
            ).with_expected(serde_json::json!(2)));

        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.framework, Some("NIST-800-53".to_string()));
    }

    #[test]
    fn test_policy_rule() {
        let rule = PolicyRule::new("rule-1", "Test", "$.path", CheckOperator::Equals)
            .with_expected(serde_json::json!("expected"))
            .with_severity(Severity::High)
            .with_control("AC-2");

        assert_eq!(rule.severity, Severity::High);
        assert_eq!(rule.controls.len(), 1);
    }

    #[test]
    fn test_drift_report() {
        let policy = Policy::new("Test", "1.0.0");
        let baseline_id = Uuid::new_v4();
        let mut report = DriftReport::new("server-01", &policy, baseline_id);

        assert!(report.compliant);

        report.add_violation(DriftViolation {
            rule_id: "rule-1".to_string(),
            rule_name: "Test Rule".to_string(),
            path: "$.config".to_string(),
            expected: Some(serde_json::json!("expected")),
            actual: Some(serde_json::json!("actual")),
            severity: Severity::High,
            controls: vec!["AC-2".to_string()],
            remediation: None,
        });

        assert!(!report.compliant);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.highest_severity(), Some(Severity::High));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }
}
