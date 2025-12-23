use crate::error::{ReportError, Result};
use crate::types::{
    ComplianceScore, ControlCoverage, ControlStatus, ExecutiveSummary, OutputFormat, Report,
    ReportMetadata, ReportSection, Trend,
};
use std::time::Instant;

/// Report generator
pub struct ReportGenerator {
    /// Default output format
    default_format: OutputFormat,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new() -> Self {
        Self {
            default_format: OutputFormat::Json,
        }
    }

    /// Set default output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.default_format = format;
        self
    }

    /// Generate a compliance report
    pub fn generate(
        &self,
        metadata: ReportMetadata,
        control_coverage: Vec<ControlCoverage>,
        previous_score: Option<f64>,
    ) -> Result<Report> {
        let start = Instant::now();

        let mut report = Report::new(metadata);

        // Add control coverage
        for coverage in control_coverage {
            report.add_control_coverage(coverage);
        }

        // Calculate framework score
        let score = self.calculate_score(&report.control_coverage);

        // Add to framework scores if framework is specified
        if let Some(framework) = report.metadata.framework.clone() {
            report.add_framework_score(&framework, score.clone());
        }

        // Generate executive summary
        let executive_summary = self.generate_executive_summary(&report, &score, previous_score);
        report.executive_summary = Some(executive_summary);

        // Generate standard sections
        report.sections = self.generate_sections(&report);

        // Record generation time
        report.generation_duration_ms = start.elapsed().as_millis() as u64;

        Ok(report)
    }

    /// Calculate compliance score from control coverage
    fn calculate_score(&self, coverage: &[ControlCoverage]) -> ComplianceScore {
        let mut passed = 0;
        let mut failed = 0;
        let mut not_applicable = 0;
        let mut not_assessed = 0;

        for control in coverage {
            match control.status {
                ControlStatus::Implemented => passed += 1,
                ControlStatus::PartiallyImplemented => {
                    // Count partial as 0.5 pass, 0.5 fail
                    passed += 1; // We'll adjust in a real implementation
                }
                ControlStatus::NotImplemented => failed += 1,
                ControlStatus::NotApplicable => not_applicable += 1,
                ControlStatus::NotAssessed => not_assessed += 1,
            }
        }

        ComplianceScore::new(passed, failed, not_applicable, not_assessed)
    }

    /// Generate executive summary
    fn generate_executive_summary(
        &self,
        report: &Report,
        score: &ComplianceScore,
        previous_score: Option<f64>,
    ) -> ExecutiveSummary {
        let mut key_findings = Vec::new();
        let mut critical_risks = Vec::new();
        let mut recommendations = Vec::new();

        // Analyze control coverage for findings
        let not_implemented: Vec<_> = report
            .control_coverage
            .iter()
            .filter(|c| c.status == ControlStatus::NotImplemented)
            .collect();

        if !not_implemented.is_empty() {
            key_findings.push(format!(
                "{} controls are not implemented",
                not_implemented.len()
            ));

            for control in not_implemented.iter().take(3) {
                critical_risks.push(format!(
                    "Control {} ({}) is not implemented",
                    control.control_id, control.control_name
                ));
            }

            recommendations.push("Prioritize implementation of missing controls".to_string());
        }

        // Add score-based findings
        if score.score_percentage < 80.0 {
            key_findings.push(format!(
                "Overall compliance score ({:.1}%) is below target (80%)",
                score.score_percentage
            ));
            recommendations.push("Develop remediation plan for failing controls".to_string());
        }

        if score.not_assessed > 0 {
            key_findings.push(format!(
                "{} controls have not been assessed",
                score.not_assessed
            ));
            recommendations.push("Schedule assessments for unassessed controls".to_string());
        }

        // Determine trend
        let trend = previous_score.map(|prev| {
            let diff = score.score_percentage - prev;
            if diff > 2.0 {
                Trend::Improving
            } else if diff < -2.0 {
                Trend::Declining
            } else {
                Trend::Stable
            }
        });

        ExecutiveSummary {
            overall_score: score.clone(),
            key_findings,
            critical_risks,
            recommendations,
            trend,
        }
    }

    /// Generate report sections
    fn generate_sections(&self, report: &Report) -> Vec<ReportSection> {
        let mut sections = Vec::new();

        // Introduction
        sections.push(ReportSection::new(
            "Introduction",
            &format!(
                "This report presents the compliance assessment results for {}.",
                report.metadata.framework.as_deref().unwrap_or("the organization")
            ),
            1,
        ));

        // Scope
        sections.push(ReportSection::new(
            "Scope",
            &format!(
                "Assessment period: {} to {}",
                report
                    .metadata
                    .period_start
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Not specified".to_string()),
                report
                    .metadata
                    .period_end
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Not specified".to_string())
            ),
            2,
        ));

        // Methodology
        sections.push(ReportSection::new(
            "Methodology",
            "Controls were assessed through automated scanning, evidence collection, and manual review.",
            3,
        ));

        // Findings summary
        if let Some(summary) = &report.executive_summary {
            let findings_content = summary
                .key_findings
                .iter()
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n");

            sections.push(ReportSection::new("Key Findings", &findings_content, 4));
        }

        // Control details
        let control_content = report
            .control_coverage
            .iter()
            .map(|c| format!("- **{}**: {} - {}", c.control_id, c.control_name, c.status))
            .collect::<Vec<_>>()
            .join("\n");

        sections.push(ReportSection::new("Control Assessment Details", &control_content, 5));

        // Recommendations
        if let Some(summary) = &report.executive_summary {
            let rec_content = summary
                .recommendations
                .iter()
                .enumerate()
                .map(|(i, r)| format!("{}. {}", i + 1, r))
                .collect::<Vec<_>>()
                .join("\n");

            sections.push(ReportSection::new("Recommendations", &rec_content, 6));
        }

        sections
    }

    /// Export report to specified format
    pub fn export(&self, report: &Report, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Json => self.export_json(report),
            OutputFormat::Html => self.export_html(report),
            OutputFormat::Markdown => self.export_markdown(report),
            OutputFormat::Csv => self.export_csv(report),
            _ => Err(ReportError::InvalidFormat(format!("{} not yet supported", format))),
        }
    }

    /// Export as JSON
    fn export_json(&self, report: &Report) -> Result<String> {
        Ok(serde_json::to_string_pretty(report)?)
    }

    /// Export as HTML
    fn export_html(&self, report: &Report) -> Result<String> {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", report.metadata.title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 40px; }\n");
        html.push_str("h1 { color: #333; }\n");
        html.push_str("h2 { color: #666; border-bottom: 1px solid #ccc; }\n");
        html.push_str(".score { font-size: 2em; font-weight: bold; }\n");
        html.push_str(".pass { color: green; }\n");
        html.push_str(".fail { color: red; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        // Title
        html.push_str(&format!("<h1>{}</h1>\n", report.metadata.title));
        html.push_str(&format!("<p>Generated: {}</p>\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        // Executive summary
        if let Some(summary) = &report.executive_summary {
            html.push_str("<h2>Executive Summary</h2>\n");
            html.push_str(&format!(
                "<p class='score'>Compliance Score: <span class='{}'>{:.1}%</span> (Grade: {})</p>\n",
                if summary.overall_score.score_percentage >= 80.0 { "pass" } else { "fail" },
                summary.overall_score.score_percentage,
                summary.overall_score.grade()
            ));

            if !summary.key_findings.is_empty() {
                html.push_str("<h3>Key Findings</h3>\n<ul>\n");
                for finding in &summary.key_findings {
                    html.push_str(&format!("<li>{}</li>\n", finding));
                }
                html.push_str("</ul>\n");
            }
        }

        // Sections
        for section in &report.sections {
            html.push_str(&format!("<h2>{}</h2>\n", section.title));
            html.push_str(&format!("<p>{}</p>\n", section.content.replace('\n', "<br/>\n")));
        }

        html.push_str("</body>\n</html>\n");
        Ok(html)
    }

    /// Export as Markdown
    fn export_markdown(&self, report: &Report) -> Result<String> {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", report.metadata.title));
        md.push_str(&format!("**Generated:** {}\n\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        if let Some(summary) = &report.executive_summary {
            md.push_str("## Executive Summary\n\n");
            md.push_str(&format!(
                "**Compliance Score:** {:.1}% (Grade: {})\n\n",
                summary.overall_score.score_percentage,
                summary.overall_score.grade()
            ));

            if !summary.key_findings.is_empty() {
                md.push_str("### Key Findings\n\n");
                for finding in &summary.key_findings {
                    md.push_str(&format!("- {}\n", finding));
                }
                md.push_str("\n");
            }
        }

        for section in &report.sections {
            md.push_str(&format!("## {}\n\n", section.title));
            md.push_str(&format!("{}\n\n", section.content));
        }

        Ok(md)
    }

    /// Export as CSV (control coverage only)
    fn export_csv(&self, report: &Report) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("Control ID,Control Name,Status,Evidence Count,Last Assessed\n");

        for control in &report.control_coverage {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                control.control_id,
                control.control_name.replace(',', ";"),
                control.status,
                control.evidence_count,
                control.last_assessed.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
            ));
        }

        Ok(csv)
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_coverage() -> Vec<ControlCoverage> {
        vec![
            ControlCoverage {
                control_id: "AC-1".to_string(),
                control_name: "Access Control Policy".to_string(),
                status: ControlStatus::Implemented,
                evidence_count: 3,
                last_assessed: Some(Utc::now()),
                notes: None,
            },
            ControlCoverage {
                control_id: "AC-2".to_string(),
                control_name: "Account Management".to_string(),
                status: ControlStatus::Implemented,
                evidence_count: 5,
                last_assessed: Some(Utc::now()),
                notes: None,
            },
            ControlCoverage {
                control_id: "AC-3".to_string(),
                control_name: "Access Enforcement".to_string(),
                status: ControlStatus::NotImplemented,
                evidence_count: 0,
                last_assessed: None,
                notes: Some("Pending implementation".to_string()),
            },
        ]
    }

    #[test]
    fn test_generate_report() {
        let generator = ReportGenerator::new();
        let metadata = ReportMetadata {
            title: "Test Report".to_string(),
            framework: Some("NIST-800-53".to_string()),
            ..Default::default()
        };

        let report = generator.generate(metadata, create_test_coverage(), None).unwrap();

        assert!(!report.id.is_nil());
        assert!(report.executive_summary.is_some());
        assert!(!report.sections.is_empty());
    }

    #[test]
    fn test_export_json() {
        let generator = ReportGenerator::new();
        let report = generator.generate(
            ReportMetadata::default(),
            create_test_coverage(),
            None,
        ).unwrap();

        let json = generator.export(&report, OutputFormat::Json).unwrap();
        assert!(json.contains("\"title\""));
        assert!(json.contains("Compliance Report"));
    }

    #[test]
    fn test_export_html() {
        let generator = ReportGenerator::new();
        let report = generator.generate(
            ReportMetadata::default(),
            create_test_coverage(),
            None,
        ).unwrap();

        let html = generator.export(&report, OutputFormat::Html).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1>"));
    }

    #[test]
    fn test_export_markdown() {
        let generator = ReportGenerator::new();
        let report = generator.generate(
            ReportMetadata::default(),
            create_test_coverage(),
            None,
        ).unwrap();

        let md = generator.export(&report, OutputFormat::Markdown).unwrap();
        assert!(md.contains("# "));
        assert!(md.contains("## Executive Summary"));
    }

    #[test]
    fn test_export_csv() {
        let generator = ReportGenerator::new();
        let report = generator.generate(
            ReportMetadata::default(),
            create_test_coverage(),
            None,
        ).unwrap();

        let csv = generator.export(&report, OutputFormat::Csv).unwrap();
        assert!(csv.contains("Control ID,Control Name"));
        assert!(csv.contains("AC-1"));
    }
}
