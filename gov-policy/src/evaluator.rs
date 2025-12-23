use crate::error::Result;
use crate::types::{
    BaselineSnapshot, CheckOperator, DriftReport, DriftViolation, Policy, PolicyRule, Severity,
};
use regex::Regex;

/// Policy rule evaluator
pub struct PolicyEvaluator {
    /// Compiled regex cache for performance
    regex_cache: std::collections::HashMap<String, Regex>,
}

impl PolicyEvaluator {
    /// Create a new policy evaluator
    pub fn new() -> Self {
        Self {
            regex_cache: std::collections::HashMap::new(),
        }
    }

    /// Evaluate a policy against a data snapshot
    pub fn evaluate(
        &mut self,
        policy: &Policy,
        data: &serde_json::Value,
        baseline: &BaselineSnapshot,
    ) -> Result<DriftReport> {
        let mut report = DriftReport::new(&baseline.target, policy, baseline.id);

        for rule in policy.enabled_rules() {
            if let Err(violation) = self.evaluate_rule(rule, data) {
                report.add_violation(violation);
            }
        }

        Ok(report)
    }

    /// Evaluate a single rule against data
    fn evaluate_rule(
        &mut self,
        rule: &PolicyRule,
        data: &serde_json::Value,
    ) -> std::result::Result<(), DriftViolation> {
        let actual = self.extract_value(data, &rule.path);

        let passed = match &rule.operator {
            CheckOperator::Equals => {
                rule.expected.as_ref().map_or(false, |exp| actual.as_ref() == Some(exp))
            }
            CheckOperator::NotEquals => {
                rule.expected.as_ref().map_or(true, |exp| actual.as_ref() != Some(exp))
            }
            CheckOperator::Contains => {
                if let (Some(actual_str), Some(exp)) = (
                    actual.as_ref().and_then(|v| v.as_str()),
                    rule.expected.as_ref().and_then(|v| v.as_str()),
                ) {
                    actual_str.contains(exp)
                } else {
                    false
                }
            }
            CheckOperator::NotContains => {
                if let (Some(actual_str), Some(exp)) = (
                    actual.as_ref().and_then(|v| v.as_str()),
                    rule.expected.as_ref().and_then(|v| v.as_str()),
                ) {
                    !actual_str.contains(exp)
                } else {
                    true
                }
            }
            CheckOperator::Matches => {
                if let (Some(actual_str), Some(pattern)) = (
                    actual.as_ref().and_then(|v| v.as_str()),
                    rule.expected.as_ref().and_then(|v| v.as_str()),
                ) {
                    self.matches_pattern(actual_str, pattern).unwrap_or(false)
                } else {
                    false
                }
            }
            CheckOperator::NotMatches => {
                if let (Some(actual_str), Some(pattern)) = (
                    actual.as_ref().and_then(|v| v.as_str()),
                    rule.expected.as_ref().and_then(|v| v.as_str()),
                ) {
                    !self.matches_pattern(actual_str, pattern).unwrap_or(true)
                } else {
                    true
                }
            }
            CheckOperator::GreaterThan => {
                self.compare_numbers(&actual, &rule.expected, |a, b| a > b)
            }
            CheckOperator::LessThan => {
                self.compare_numbers(&actual, &rule.expected, |a, b| a < b)
            }
            CheckOperator::GreaterThanOrEqual => {
                self.compare_numbers(&actual, &rule.expected, |a, b| a >= b)
            }
            CheckOperator::LessThanOrEqual => {
                self.compare_numbers(&actual, &rule.expected, |a, b| a <= b)
            }
            CheckOperator::Exists => actual.is_some(),
            CheckOperator::NotExists => actual.is_none(),
            CheckOperator::InList => {
                if let Some(exp_list) = rule.expected.as_ref().and_then(|v| v.as_array()) {
                    actual.as_ref().map_or(false, |a| exp_list.contains(a))
                } else {
                    false
                }
            }
            CheckOperator::NotInList => {
                if let Some(exp_list) = rule.expected.as_ref().and_then(|v| v.as_array()) {
                    actual.as_ref().map_or(true, |a| !exp_list.contains(a))
                } else {
                    true
                }
            }
        };

        if passed {
            Ok(())
        } else {
            Err(DriftViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                path: rule.path.clone(),
                expected: rule.expected.clone(),
                actual,
                severity: rule.severity,
                controls: rule.controls.clone(),
                remediation: None,
            })
        }
    }

    /// Extract a value from JSON using a simple path notation
    /// Supports: $.key, $.key.subkey, $.array[0]
    fn extract_value(&self, data: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
        let path = path.strip_prefix("$.").unwrap_or(path);
        let parts: Vec<&str> = path.split('.').collect();

        let mut current = data.clone();

        for part in parts {
            // Check for array index: key[0]
            if let Some((key, idx_str)) = part.split_once('[') {
                let idx_str = idx_str.trim_end_matches(']');
                if let Ok(idx) = idx_str.parse::<usize>() {
                    current = current.get(key)?.get(idx)?.clone();
                } else {
                    return None;
                }
            } else {
                current = current.get(part)?.clone();
            }
        }

        Some(current)
    }

    /// Check if string matches regex pattern
    fn matches_pattern(&mut self, value: &str, pattern: &str) -> Result<bool> {
        let regex = if let Some(cached) = self.regex_cache.get(pattern) {
            cached
        } else {
            let compiled = Regex::new(pattern)?;
            self.regex_cache.insert(pattern.to_string(), compiled);
            self.regex_cache.get(pattern).unwrap()
        };

        Ok(regex.is_match(value))
    }

    /// Compare numeric values
    fn compare_numbers<F>(
        &self,
        actual: &Option<serde_json::Value>,
        expected: &Option<serde_json::Value>,
        compare: F,
    ) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        let actual_num = actual.as_ref().and_then(|v| v.as_f64());
        let expected_num = expected.as_ref().and_then(|v| v.as_f64());

        match (actual_num, expected_num) {
            (Some(a), Some(e)) => compare(a, e),
            _ => false,
        }
    }
}

impl Default for PolicyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Drift detector for comparing current state to baseline
pub struct DriftDetector {
    evaluator: PolicyEvaluator,
}

impl DriftDetector {
    /// Create a new drift detector
    pub fn new() -> Self {
        Self {
            evaluator: PolicyEvaluator::new(),
        }
    }

    /// Detect drift between current state and baseline
    pub fn detect(
        &mut self,
        policy: &Policy,
        current: &serde_json::Value,
        baseline: &BaselineSnapshot,
    ) -> Result<DriftReport> {
        self.evaluator.evaluate(policy, current, baseline)
    }

    /// Quick check if any drift exists
    pub fn has_drift(
        &mut self,
        policy: &Policy,
        current: &serde_json::Value,
        baseline: &BaselineSnapshot,
    ) -> Result<bool> {
        let report = self.detect(policy, current, baseline)?;
        Ok(!report.compliant)
    }

    /// Get violations above a certain severity
    pub fn critical_violations(
        &mut self,
        policy: &Policy,
        current: &serde_json::Value,
        baseline: &BaselineSnapshot,
        min_severity: Severity,
    ) -> Result<Vec<DriftViolation>> {
        let report = self.detect(policy, current, baseline)?;
        Ok(report
            .violations
            .into_iter()
            .filter(|v| v.severity >= min_severity)
            .collect())
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CheckOperator;

    fn create_test_data() -> serde_json::Value {
        serde_json::json!({
            "ssh": {
                "protocol": 2,
                "permitRootLogin": "no",
                "port": 22
            },
            "services": ["sshd", "nginx", "postgres"],
            "version": "2.0.1"
        })
    }

    #[test]
    fn test_equals_operator() {
        let mut evaluator = PolicyEvaluator::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0").with_rule(
            PolicyRule::new("rule-1", "Check Protocol", "$.ssh.protocol", CheckOperator::Equals)
                .with_expected(serde_json::json!(2)),
        );

        let report = evaluator.evaluate(&policy, &data, &baseline).unwrap();
        assert!(report.compliant);
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn test_equals_violation() {
        let mut evaluator = PolicyEvaluator::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0").with_rule(
            PolicyRule::new("rule-1", "Check Protocol", "$.ssh.protocol", CheckOperator::Equals)
                .with_expected(serde_json::json!(1)),
        );

        let report = evaluator.evaluate(&policy, &data, &baseline).unwrap();
        assert!(!report.compliant);
        assert_eq!(report.violations.len(), 1);
    }

    #[test]
    fn test_contains_operator() {
        let mut evaluator = PolicyEvaluator::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0").with_rule(
            PolicyRule::new("rule-1", "Check Version", "$.version", CheckOperator::Contains)
                .with_expected(serde_json::json!("2.0")),
        );

        let report = evaluator.evaluate(&policy, &data, &baseline).unwrap();
        assert!(report.compliant);
    }

    #[test]
    fn test_exists_operator() {
        let mut evaluator = PolicyEvaluator::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0")
            .with_rule(PolicyRule::new(
                "rule-1",
                "Check SSH exists",
                "$.ssh",
                CheckOperator::Exists,
            ))
            .with_rule(PolicyRule::new(
                "rule-2",
                "Check FTP not exists",
                "$.ftp",
                CheckOperator::NotExists,
            ));

        let report = evaluator.evaluate(&policy, &data, &baseline).unwrap();
        assert!(report.compliant);
    }

    #[test]
    fn test_greater_than_operator() {
        let mut evaluator = PolicyEvaluator::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0").with_rule(
            PolicyRule::new("rule-1", "Check Port", "$.ssh.port", CheckOperator::GreaterThan)
                .with_expected(serde_json::json!(21)),
        );

        let report = evaluator.evaluate(&policy, &data, &baseline).unwrap();
        assert!(report.compliant);
    }

    #[test]
    fn test_drift_detector() {
        let mut detector = DriftDetector::new();
        let data = create_test_data();
        let baseline = BaselineSnapshot::new("test", "server", data.clone(), "tester");

        let policy = Policy::new("test", "1.0.0").with_rule(
            PolicyRule::new("rule-1", "Check Protocol", "$.ssh.protocol", CheckOperator::Equals)
                .with_expected(serde_json::json!(2)),
        );

        assert!(!detector.has_drift(&policy, &data, &baseline).unwrap());
    }
}
