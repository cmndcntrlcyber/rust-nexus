//! FISMA - Federal Information Security Modernization Act
//!
//! Federal cybersecurity requirements for government agencies.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the FISMA framework definition
pub fn create_framework() -> Framework {
    Framework::new("fisma", "FISMA", FrameworkCategory::UsGovernment)
        .with_description("Federal Information Security Modernization Act of 2014")
        .with_version("2014")
        .with_url("https://www.cisa.gov/federal-information-security-modernization-act")
        // FISMA domains aligned with NIST RMF
        .with_domain(FrameworkDomain::new("CAT", "Categorize").with_order(1))
        .with_domain(FrameworkDomain::new("SEL", "Select").with_order(2))
        .with_domain(FrameworkDomain::new("IMP", "Implement").with_order(3))
        .with_domain(FrameworkDomain::new("ASS", "Assess").with_order(4))
        .with_domain(FrameworkDomain::new("AUT", "Authorize").with_order(5))
        .with_domain(FrameworkDomain::new("MON", "Monitor").with_order(6))
        .with_domain(FrameworkDomain::new("RPT", "Reporting").with_order(7))
        // Categorize
        .with_control(Control::new("CAT-1", "System Categorization")
            .with_description("Categorize information systems based on impact levels (Low, Moderate, High)")
            .with_domain("CAT").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "RA-2", "Security Categorization", MappingStrength::Strong)))
        .with_control(Control::new("CAT-2", "Information Types")
            .with_description("Identify information types processed, stored, or transmitted")
            .with_domain("CAT").with_priority(4))
        // Select Controls
        .with_control(Control::new("SEL-1", "Control Selection")
            .with_description("Select appropriate security controls based on system categorization")
            .with_domain("SEL").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "PL-2", "Security Plan", MappingStrength::Strong)))
        .with_control(Control::new("SEL-2", "Control Tailoring")
            .with_description("Tailor selected controls to organizational needs")
            .with_domain("SEL").with_priority(4))
        .with_control(Control::new("SEL-3", "Control Baselines")
            .with_description("Apply appropriate control baselines (Low, Moderate, High)")
            .with_domain("SEL").with_priority(5))
        // Implement
        .with_control(Control::new("IMP-1", "Control Implementation")
            .with_description("Implement security controls as documented in security plan")
            .with_domain("IMP").with_priority(5))
        .with_control(Control::new("IMP-2", "Implementation Documentation")
            .with_description("Document how controls are implemented")
            .with_domain("IMP").with_priority(4))
        // Assess
        .with_control(Control::new("ASS-1", "Security Assessment")
            .with_description("Assess security controls using appropriate assessment procedures")
            .with_domain("ASS").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "CA-2", "Security Assessments", MappingStrength::Strong)))
        .with_control(Control::new("ASS-2", "Assessment Report")
            .with_description("Prepare security assessment report documenting findings")
            .with_domain("ASS").with_priority(4))
        .with_control(Control::new("ASS-3", "Remediation Actions")
            .with_description("Conduct remediation actions based on assessment findings")
            .with_domain("ASS").with_priority(4))
        // Authorize
        .with_control(Control::new("AUT-1", "Authorization Decision")
            .with_description("Authorize system operation based on risk determination")
            .with_domain("AUT").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "CA-6", "Security Authorization", MappingStrength::Strong)))
        .with_control(Control::new("AUT-2", "Authorization Package")
            .with_description("Prepare authorization package with required documentation")
            .with_domain("AUT").with_priority(4))
        .with_control(Control::new("AUT-3", "ATO Management")
            .with_description("Manage Authority to Operate (ATO) lifecycle")
            .with_domain("AUT").with_priority(5))
        // Monitor
        .with_control(Control::new("MON-1", "Continuous Monitoring")
            .with_description("Monitor security controls on an ongoing basis")
            .with_domain("MON").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "CA-7", "Continuous Monitoring", MappingStrength::Strong)))
        .with_control(Control::new("MON-2", "Configuration Change Monitoring")
            .with_description("Monitor configuration changes that affect security posture")
            .with_domain("MON").with_priority(4))
        .with_control(Control::new("MON-3", "Security Status Reporting")
            .with_description("Report security status to authorizing officials")
            .with_domain("MON").with_priority(4))
        // Reporting
        .with_control(Control::new("RPT-1", "Annual FISMA Report")
            .with_description("Submit annual FISMA report to OMB and Congress")
            .with_domain("RPT").with_priority(5))
        .with_control(Control::new("RPT-2", "CyberScope Reporting")
            .with_description("Report security metrics through CyberScope")
            .with_domain("RPT").with_priority(4))
        .with_control(Control::new("RPT-3", "Incident Reporting")
            .with_description("Report security incidents per US-CERT guidelines")
            .with_domain("RPT").with_priority(5))
        .with_control(Control::new("RPT-4", "POA&M Management")
            .with_description("Maintain Plan of Action and Milestones for identified weaknesses")
            .with_domain("RPT").with_priority(4))
}
