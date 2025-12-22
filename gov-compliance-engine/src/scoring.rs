//! Compliance scoring engine
//!
//! Calculates compliance scores at control, framework, and organization levels.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::control::{ControlId, ControlStatus};
use crate::framework::FrameworkId;

/// Score for a single control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlScore {
    /// Control ID
    pub control_id: ControlId,

    /// Current status
    pub status: ControlStatus,

    /// Score as percentage (0-100)
    pub score: f64,

    /// Number of passed tests
    pub tests_passed: u32,

    /// Total number of tests
    pub tests_total: u32,

    /// Weight of this control in framework scoring
    pub weight: f64,

    /// Last assessed timestamp
    pub last_assessed: chrono::DateTime<chrono::Utc>,
}

impl ControlScore {
    /// Create a new control score
    pub fn new(control_id: ControlId) -> Self {
        Self {
            control_id,
            status: ControlStatus::NotAssessed,
            score: 0.0,
            tests_passed: 0,
            tests_total: 0,
            weight: 1.0,
            last_assessed: chrono::Utc::now(),
        }
    }

    /// Calculate score from test results
    pub fn from_tests(
        control_id: ControlId,
        passed: u32,
        total: u32,
        weight: f64,
    ) -> Self {
        let score = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let status = if total == 0 {
            ControlStatus::NotAssessed
        } else if passed == total {
            ControlStatus::Compliant
        } else if passed > 0 {
            ControlStatus::PartiallyCompliant
        } else {
            ControlStatus::NonCompliant
        };

        Self {
            control_id,
            status,
            score,
            tests_passed: passed,
            tests_total: total,
            weight,
            last_assessed: chrono::Utc::now(),
        }
    }

    /// Is this control compliant?
    pub fn is_compliant(&self) -> bool {
        matches!(self.status, ControlStatus::Compliant | ControlStatus::NotApplicable)
    }

    /// Get weighted score
    pub fn weighted_score(&self) -> f64 {
        self.score * self.weight
    }
}

/// Compliance score for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    /// Framework ID
    pub framework_id: FrameworkId,

    /// Overall score as percentage (0-100)
    pub overall_score: f64,

    /// Individual control scores
    pub control_scores: HashMap<String, ControlScore>,

    /// Domain/category scores
    pub domain_scores: HashMap<String, f64>,

    /// Number of compliant controls
    pub compliant_count: u32,

    /// Number of partially compliant controls
    pub partial_count: u32,

    /// Number of non-compliant controls
    pub non_compliant_count: u32,

    /// Number of not assessed controls
    pub not_assessed_count: u32,

    /// Number of not applicable controls
    pub not_applicable_count: u32,

    /// Total controls
    pub total_controls: u32,

    /// When the score was calculated
    pub calculated_at: chrono::DateTime<chrono::Utc>,

    /// Score trend (compared to last assessment)
    pub trend: ScoreTrend,
}

impl ComplianceScore {
    /// Create a new compliance score
    pub fn new(framework_id: FrameworkId) -> Self {
        Self {
            framework_id,
            overall_score: 0.0,
            control_scores: HashMap::new(),
            domain_scores: HashMap::new(),
            compliant_count: 0,
            partial_count: 0,
            non_compliant_count: 0,
            not_assessed_count: 0,
            not_applicable_count: 0,
            total_controls: 0,
            calculated_at: chrono::Utc::now(),
            trend: ScoreTrend::Stable,
        }
    }

    /// Calculate score from control scores
    pub fn calculate(framework_id: FrameworkId, control_scores: Vec<ControlScore>) -> Self {
        let total = control_scores.len() as u32;
        let mut compliant = 0u32;
        let mut partial = 0u32;
        let mut non_compliant = 0u32;
        let mut not_assessed = 0u32;
        let mut not_applicable = 0u32;

        let mut total_weighted_score = 0.0;
        let mut total_weight = 0.0;

        let mut scores_map = HashMap::new();

        for score in control_scores {
            match score.status {
                ControlStatus::Compliant => compliant += 1,
                ControlStatus::PartiallyCompliant => partial += 1,
                ControlStatus::NonCompliant => non_compliant += 1,
                ControlStatus::NotAssessed => not_assessed += 1,
                ControlStatus::NotApplicable => not_applicable += 1,
                _ => {}
            }

            // Only include in scoring if assessed and applicable
            if score.status != ControlStatus::NotAssessed
                && score.status != ControlStatus::NotApplicable
            {
                total_weighted_score += score.weighted_score();
                total_weight += score.weight;
            }

            scores_map.insert(score.control_id.to_string(), score);
        }

        let overall_score = if total_weight > 0.0 {
            total_weighted_score / total_weight
        } else {
            0.0
        };

        Self {
            framework_id,
            overall_score,
            control_scores: scores_map,
            domain_scores: HashMap::new(),
            compliant_count: compliant,
            partial_count: partial,
            non_compliant_count: non_compliant,
            not_assessed_count: not_assessed,
            not_applicable_count: not_applicable,
            total_controls: total,
            calculated_at: chrono::Utc::now(),
            trend: ScoreTrend::Stable,
        }
    }

    /// Get compliance percentage (considering only assessed controls)
    pub fn compliance_percentage(&self) -> f64 {
        let assessed = self.total_controls - self.not_assessed_count - self.not_applicable_count;
        if assessed == 0 {
            return 0.0;
        }
        (self.compliant_count as f64 / assessed as f64) * 100.0
    }

    /// Get risk level based on score
    pub fn risk_level(&self) -> RiskLevel {
        match self.overall_score as u32 {
            90..=100 => RiskLevel::Low,
            70..=89 => RiskLevel::Medium,
            50..=69 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Get a grade (A-F)
    pub fn grade(&self) -> char {
        match self.overall_score as u32 {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        }
    }

    /// Compare with previous score and set trend
    pub fn with_trend(mut self, previous_score: f64) -> Self {
        let diff = self.overall_score - previous_score;
        self.trend = if diff > 2.0 {
            ScoreTrend::Improving
        } else if diff < -2.0 {
            ScoreTrend::Declining
        } else {
            ScoreTrend::Stable
        };
        self
    }
}

/// Score trend indicator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreTrend {
    /// Score is improving
    Improving,
    /// Score is stable
    Stable,
    /// Score is declining
    Declining,
}

/// Risk level based on compliance score
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Low => "Low Risk",
            Self::Medium => "Medium Risk",
            Self::High => "High Risk",
            Self::Critical => "Critical Risk",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Low => "green",
            Self::Medium => "yellow",
            Self::High => "orange",
            Self::Critical => "red",
        }
    }
}

/// Organization-wide compliance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationScore {
    /// Name/ID of the organization
    pub organization_id: String,

    /// Scores per framework
    pub framework_scores: HashMap<String, ComplianceScore>,

    /// Average score across all frameworks
    pub average_score: f64,

    /// Overall risk level
    pub risk_level: RiskLevel,

    /// When calculated
    pub calculated_at: chrono::DateTime<chrono::Utc>,
}

impl OrganizationScore {
    /// Calculate organization score from framework scores
    pub fn calculate(organization_id: String, framework_scores: Vec<ComplianceScore>) -> Self {
        let total_score: f64 = framework_scores.iter().map(|s| s.overall_score).sum();
        let avg = if framework_scores.is_empty() {
            0.0
        } else {
            total_score / framework_scores.len() as f64
        };

        let risk_level = match avg as u32 {
            90..=100 => RiskLevel::Low,
            70..=89 => RiskLevel::Medium,
            50..=69 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        let mut scores_map = HashMap::new();
        for score in framework_scores {
            scores_map.insert(score.framework_id.to_string(), score);
        }

        Self {
            organization_id,
            framework_scores: scores_map,
            average_score: avg,
            risk_level,
            calculated_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_score() {
        let score = ControlScore::from_tests(
            ControlId::new("AC-1"),
            8,
            10,
            1.0,
        );

        assert_eq!(score.score, 80.0);
        assert_eq!(score.status, ControlStatus::PartiallyCompliant);
    }

    #[test]
    fn test_compliance_score() {
        let control_scores = vec![
            ControlScore::from_tests(ControlId::new("AC-1"), 10, 10, 1.0),
            ControlScore::from_tests(ControlId::new("AC-2"), 5, 10, 1.0),
            ControlScore::from_tests(ControlId::new("AC-3"), 0, 10, 1.0),
        ];

        let framework_score = ComplianceScore::calculate(
            FrameworkId::new("test"),
            control_scores,
        );

        assert_eq!(framework_score.compliant_count, 1);
        assert_eq!(framework_score.partial_count, 1);
        assert_eq!(framework_score.non_compliant_count, 1);
    }

    #[test]
    fn test_risk_level() {
        let mut score = ComplianceScore::new(FrameworkId::new("test"));

        score.overall_score = 95.0;
        assert_eq!(score.risk_level(), RiskLevel::Low);

        score.overall_score = 75.0;
        assert_eq!(score.risk_level(), RiskLevel::Medium);

        score.overall_score = 55.0;
        assert_eq!(score.risk_level(), RiskLevel::High);

        score.overall_score = 30.0;
        assert_eq!(score.risk_level(), RiskLevel::Critical);
    }
}
