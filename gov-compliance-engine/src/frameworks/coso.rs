//! COSO - Committee of Sponsoring Organizations
//!
//! Enterprise Risk Management framework.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the COSO ERM framework definition
pub fn create_framework() -> Framework {
    Framework::new("coso", "COSO ERM", FrameworkCategory::AuditTrust)
        .with_description("Enterprise Risk Management - Integrating with Strategy and Performance")
        .with_version("2017")
        .with_url("https://www.coso.org/guidance-erm")
        // COSO ERM Components
        .with_domain(FrameworkDomain::new("GOV", "Governance and Culture").with_order(1))
        .with_domain(FrameworkDomain::new("STR", "Strategy and Objective-Setting").with_order(2))
        .with_domain(FrameworkDomain::new("PER", "Performance").with_order(3))
        .with_domain(FrameworkDomain::new("REV", "Review and Revision").with_order(4))
        .with_domain(FrameworkDomain::new("INF", "Information, Communication, and Reporting").with_order(5))
        // Governance and Culture
        .with_control(Control::new("GOV-1", "Board Risk Oversight")
            .with_description("The board of directors provides oversight of the strategy and risk management")
            .with_domain("GOV").with_priority(5))
        .with_control(Control::new("GOV-2", "Operating Structure")
            .with_description("The organization establishes operating structures in pursuit of strategy")
            .with_domain("GOV").with_priority(4))
        .with_control(Control::new("GOV-3", "Desired Culture")
            .with_description("The organization defines desired behaviors that characterize the entity's culture")
            .with_domain("GOV").with_priority(4))
        .with_control(Control::new("GOV-4", "Core Values Commitment")
            .with_description("The organization demonstrates a commitment to core values")
            .with_domain("GOV").with_priority(4))
        .with_control(Control::new("GOV-5", "Talent Development")
            .with_description("The organization is committed to developing human capital aligned with strategy")
            .with_domain("GOV").with_priority(3))
        // Strategy and Objective-Setting
        .with_control(Control::new("STR-1", "Business Context Analysis")
            .with_description("The organization analyzes business context to support risk identification")
            .with_domain("STR").with_priority(5)
            .with_mapping(ControlMapping::new("iso27001", "A.5.1", "Context Analysis", MappingStrength::Moderate)))
        .with_control(Control::new("STR-2", "Risk Appetite Definition")
            .with_description("The organization defines risk appetite in the context of creating value")
            .with_domain("STR").with_priority(5))
        .with_control(Control::new("STR-3", "Strategy Evaluation")
            .with_description("The organization considers risk while establishing business objectives")
            .with_domain("STR").with_priority(4))
        .with_control(Control::new("STR-4", "Business Objectives")
            .with_description("The organization considers risk while establishing business objectives at various levels")
            .with_domain("STR").with_priority(4))
        // Performance
        .with_control(Control::new("PER-1", "Risk Identification")
            .with_description("The organization identifies risk that impacts the achievement of objectives")
            .with_domain("PER").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf", "ID.RA-1", "Risk Identification", MappingStrength::Strong)))
        .with_control(Control::new("PER-2", "Risk Severity Assessment")
            .with_description("The organization assesses the severity of risk")
            .with_domain("PER").with_priority(5))
        .with_control(Control::new("PER-3", "Risk Prioritization")
            .with_description("The organization prioritizes risks as a basis for selecting responses")
            .with_domain("PER").with_priority(4))
        .with_control(Control::new("PER-4", "Risk Response Implementation")
            .with_description("The organization identifies and selects risk responses")
            .with_domain("PER").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf", "ID.RA-6", "Risk Response", MappingStrength::Strong)))
        .with_control(Control::new("PER-5", "Portfolio Risk View")
            .with_description("The organization develops and evaluates a portfolio view of risk")
            .with_domain("PER").with_priority(4))
        // Review and Revision
        .with_control(Control::new("REV-1", "Substantial Change Assessment")
            .with_description("The organization identifies and assesses changes that may affect strategy")
            .with_domain("REV").with_priority(4))
        .with_control(Control::new("REV-2", "Risk and Performance Review")
            .with_description("The organization reviews entity performance and considers risk")
            .with_domain("REV").with_priority(4))
        .with_control(Control::new("REV-3", "ERM Improvement")
            .with_description("The organization pursues improvement of enterprise risk management")
            .with_domain("REV").with_priority(3))
        // Information, Communication, and Reporting
        .with_control(Control::new("INF-1", "IT Systems Leverage")
            .with_description("The organization leverages the entity's information and technology systems")
            .with_domain("INF").with_priority(4))
        .with_control(Control::new("INF-2", "Risk Communication")
            .with_description("The organization uses communication channels to support risk management")
            .with_domain("INF").with_priority(4))
        .with_control(Control::new("INF-3", "Risk Reporting")
            .with_description("The organization reports on risk, culture, and performance at multiple levels")
            .with_domain("INF").with_priority(5))
}
