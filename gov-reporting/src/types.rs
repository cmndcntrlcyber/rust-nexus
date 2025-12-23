use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Output format for reports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// JSON format
    Json,
    /// HTML format
    Html,
    /// PDF format (future)
    Pdf,
    /// DOCX format (future)
    Docx,
    /// CSV format
    Csv,
    /// Markdown format
    Markdown,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "JSON"),
            OutputFormat::Html => write!(f, "HTML"),
            OutputFormat::Pdf => write!(f, "PDF"),
            OutputFormat::Docx => write!(f, "DOCX"),
            OutputFormat::Csv => write!(f, "CSV"),
            OutputFormat::Markdown => write!(f, "Markdown"),
        }
    }
}

/// Report classification/distribution level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Classification {
    /// Public distribution
    Public,
    /// Internal only
    Internal,
    /// Confidential
    Confidential,
    /// Restricted
    Restricted,
}

impl Default for Classification {
    fn default() -> Self {
        Classification::Internal
    }
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Report title
    pub title: String,
    /// Report description
    pub description: Option<String>,
    /// Target compliance framework
    pub framework: Option<String>,
    /// Report version
    pub version: String,
    /// Author
    pub author: String,
    /// Classification level
    pub classification: Classification,
    /// Distribution list
    pub distribution: Vec<String>,
    /// Report period start
    pub period_start: Option<DateTime<Utc>>,
    /// Report period end
    pub period_end: Option<DateTime<Utc>>,
}

impl Default for ReportMetadata {
    fn default() -> Self {
        Self {
            title: "Compliance Report".to_string(),
            description: None,
            framework: None,
            version: "1.0.0".to_string(),
            author: "System".to_string(),
            classification: Classification::default(),
            distribution: Vec::new(),
            period_start: None,
            period_end: None,
        }
    }
}

/// Compliance score for a control or framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    /// Total controls
    pub total: usize,
    /// Passing controls
    pub passed: usize,
    /// Failing controls
    pub failed: usize,
    /// Not applicable controls
    pub not_applicable: usize,
    /// Not assessed controls
    pub not_assessed: usize,
    /// Score percentage (0-100)
    pub score_percentage: f64,
}

impl ComplianceScore {
    /// Create a new compliance score
    pub fn new(passed: usize, failed: usize, not_applicable: usize, not_assessed: usize) -> Self {
        let total = passed + failed + not_applicable + not_assessed;
        let assessed = passed + failed;
        let score_percentage = if assessed > 0 {
            (passed as f64 / assessed as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            passed,
            failed,
            not_applicable,
            not_assessed,
            score_percentage,
        }
    }

    /// Get grade based on score
    pub fn grade(&self) -> &'static str {
        match self.score_percentage as u32 {
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

/// Control implementation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCoverage {
    /// Control ID
    pub control_id: String,
    /// Control name
    pub control_name: String,
    /// Implementation status
    pub status: ControlStatus,
    /// Evidence count
    pub evidence_count: usize,
    /// Last assessment date
    pub last_assessed: Option<DateTime<Utc>>,
    /// Notes
    pub notes: Option<String>,
}

/// Status of a control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStatus {
    /// Control is implemented and passing
    Implemented,
    /// Control is partially implemented
    PartiallyImplemented,
    /// Control is not implemented
    NotImplemented,
    /// Control is not applicable
    NotApplicable,
    /// Control has not been assessed
    NotAssessed,
}

impl std::fmt::Display for ControlStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlStatus::Implemented => write!(f, "Implemented"),
            ControlStatus::PartiallyImplemented => write!(f, "Partially Implemented"),
            ControlStatus::NotImplemented => write!(f, "Not Implemented"),
            ControlStatus::NotApplicable => write!(f, "Not Applicable"),
            ControlStatus::NotAssessed => write!(f, "Not Assessed"),
        }
    }
}

/// A section within a report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    /// Section title
    pub title: String,
    /// Section content (markdown/HTML)
    pub content: String,
    /// Order in report
    pub order: usize,
    /// Whether to include in table of contents
    pub in_toc: bool,
    /// Subsections
    pub subsections: Vec<ReportSection>,
}

impl ReportSection {
    /// Create a new section
    pub fn new(title: &str, content: &str, order: usize) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            order,
            in_toc: true,
            subsections: Vec::new(),
        }
    }

    /// Add a subsection
    pub fn with_subsection(mut self, subsection: ReportSection) -> Self {
        self.subsections.push(subsection);
        self
    }
}

/// Executive summary for a report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    /// Overall compliance score
    pub overall_score: ComplianceScore,
    /// Key findings
    pub key_findings: Vec<String>,
    /// Critical risks
    pub critical_risks: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Trend compared to previous period
    pub trend: Option<Trend>,
}

/// Trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Trend {
    /// Score is improving
    Improving,
    /// Score is stable
    Stable,
    /// Score is declining
    Declining,
}

/// Complete report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Unique report identifier
    pub id: Uuid,
    /// Report metadata
    pub metadata: ReportMetadata,
    /// Executive summary
    pub executive_summary: Option<ExecutiveSummary>,
    /// Report sections
    pub sections: Vec<ReportSection>,
    /// Control coverage details
    pub control_coverage: Vec<ControlCoverage>,
    /// Framework-level scores
    pub framework_scores: std::collections::HashMap<String, ComplianceScore>,
    /// When report was generated
    pub generated_at: DateTime<Utc>,
    /// Generation duration in milliseconds
    pub generation_duration_ms: u64,
}

impl Report {
    /// Create a new report
    pub fn new(metadata: ReportMetadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            metadata,
            executive_summary: None,
            sections: Vec::new(),
            control_coverage: Vec::new(),
            framework_scores: std::collections::HashMap::new(),
            generated_at: Utc::now(),
            generation_duration_ms: 0,
        }
    }

    /// Add a section
    pub fn with_section(mut self, section: ReportSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Set executive summary
    pub fn with_executive_summary(mut self, summary: ExecutiveSummary) -> Self {
        self.executive_summary = Some(summary);
        self
    }

    /// Add control coverage
    pub fn add_control_coverage(&mut self, coverage: ControlCoverage) {
        self.control_coverage.push(coverage);
    }

    /// Add framework score
    pub fn add_framework_score(&mut self, framework: &str, score: ComplianceScore) {
        self.framework_scores.insert(framework.to_string(), score);
    }
}

/// Report template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Target framework
    pub framework: Option<String>,
    /// Template description
    pub description: Option<String>,
    /// Section definitions
    pub sections: Vec<TemplateSectionDef>,
    /// Whether to include executive summary
    pub include_executive_summary: bool,
    /// Whether to include control details
    pub include_control_details: bool,
    /// Whether to include evidence
    pub include_evidence: bool,
}

/// Template section definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSectionDef {
    /// Section key
    pub key: String,
    /// Section title
    pub title: String,
    /// Whether section is required
    pub required: bool,
    /// Section order
    pub order: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_score() {
        let score = ComplianceScore::new(80, 15, 3, 2);

        assert_eq!(score.total, 100);
        assert_eq!(score.passed, 80);
        assert_eq!(score.failed, 15);
        // 80/95 = 84.2%
        assert!(score.score_percentage > 84.0 && score.score_percentage < 85.0);
        assert_eq!(score.grade(), "B+");
    }

    #[test]
    fn test_compliance_score_grades() {
        assert_eq!(ComplianceScore::new(100, 0, 0, 0).grade(), "A+");
        assert_eq!(ComplianceScore::new(91, 9, 0, 0).grade(), "A");
        assert_eq!(ComplianceScore::new(75, 25, 0, 0).grade(), "B");
        assert_eq!(ComplianceScore::new(50, 50, 0, 0).grade(), "D");
        assert_eq!(ComplianceScore::new(40, 60, 0, 0).grade(), "F");
    }

    #[test]
    fn test_report_creation() {
        let metadata = ReportMetadata {
            title: "Q1 2024 Compliance Report".to_string(),
            framework: Some("NIST-800-53".to_string()),
            ..Default::default()
        };

        let report = Report::new(metadata)
            .with_section(ReportSection::new("Introduction", "Overview of the assessment", 1));

        assert!(!report.id.is_nil());
        assert_eq!(report.sections.len(), 1);
        assert_eq!(report.metadata.title, "Q1 2024 Compliance Report");
    }

    #[test]
    fn test_report_with_scores() {
        let mut report = Report::new(ReportMetadata::default());

        report.add_framework_score("NIST-800-53", ComplianceScore::new(85, 10, 3, 2));
        report.add_framework_score("ISO-27001", ComplianceScore::new(90, 8, 2, 0));

        assert_eq!(report.framework_scores.len(), 2);
        assert!(report.framework_scores.contains_key("NIST-800-53"));
    }

    #[test]
    fn test_control_status_display() {
        assert_eq!(ControlStatus::Implemented.to_string(), "Implemented");
        assert_eq!(ControlStatus::PartiallyImplemented.to_string(), "Partially Implemented");
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Json.to_string(), "JSON");
        assert_eq!(OutputFormat::Pdf.to_string(), "PDF");
    }
}
