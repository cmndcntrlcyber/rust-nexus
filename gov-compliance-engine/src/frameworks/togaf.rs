//! TOGAF - The Open Group Architecture Framework
//!
//! Enterprise architecture framework for IT governance.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the TOGAF framework definition
pub fn create_framework() -> Framework {
    Framework::new("togaf", "TOGAF", FrameworkCategory::AuditTrust)
        .with_description("The Open Group Architecture Framework")
        .with_version("10")
        .with_url("https://www.opengroup.org/togaf")
        // TOGAF ADM Phases
        .with_domain(FrameworkDomain::new("PRE", "Preliminary Phase").with_order(1))
        .with_domain(FrameworkDomain::new("A", "Architecture Vision").with_order(2))
        .with_domain(FrameworkDomain::new("B", "Business Architecture").with_order(3))
        .with_domain(FrameworkDomain::new("C", "Information Systems Architecture").with_order(4))
        .with_domain(FrameworkDomain::new("D", "Technology Architecture").with_order(5))
        .with_domain(FrameworkDomain::new("E", "Opportunities and Solutions").with_order(6))
        .with_domain(FrameworkDomain::new("F", "Migration Planning").with_order(7))
        .with_domain(FrameworkDomain::new("G", "Implementation Governance").with_order(8))
        .with_domain(FrameworkDomain::new("H", "Architecture Change Management").with_order(9))
        .with_domain(FrameworkDomain::new("REQ", "Requirements Management").with_order(10))
        // Preliminary Phase
        .with_control(Control::new("PRE-1", "Architecture Principles")
            .with_description("Define architecture principles that guide architecture development")
            .with_domain("PRE").with_priority(5))
        .with_control(Control::new("PRE-2", "Architecture Framework")
            .with_description("Establish the architecture framework and methodology")
            .with_domain("PRE").with_priority(5))
        .with_control(Control::new("PRE-3", "Governance Structure")
            .with_description("Establish the architecture governance structure")
            .with_domain("PRE").with_priority(4))
        // Phase A - Architecture Vision
        .with_control(Control::new("A-1", "Architecture Vision")
            .with_description("Develop a high-level vision of capabilities and business value")
            .with_domain("A").with_priority(5))
        .with_control(Control::new("A-2", "Stakeholder Management")
            .with_description("Identify stakeholders and their concerns")
            .with_domain("A").with_priority(4))
        .with_control(Control::new("A-3", "Business Requirements")
            .with_description("Confirm and elaborate business goals and drivers")
            .with_domain("A").with_priority(4))
        // Phase B - Business Architecture
        .with_control(Control::new("B-1", "Business Architecture Baseline")
            .with_description("Develop baseline business architecture description")
            .with_domain("B").with_priority(4))
        .with_control(Control::new("B-2", "Business Architecture Target")
            .with_description("Develop target business architecture description")
            .with_domain("B").with_priority(4))
        .with_control(Control::new("B-3", "Gap Analysis")
            .with_description("Perform gap analysis between baseline and target")
            .with_domain("B").with_priority(4))
        // Phase C - Information Systems Architecture
        .with_control(Control::new("C-1", "Data Architecture")
            .with_description("Develop data architecture for baseline and target states")
            .with_domain("C").with_priority(4)
            .with_mapping(ControlMapping::new("iso27001", "A.8.1", "Asset Management", MappingStrength::Moderate)))
        .with_control(Control::new("C-2", "Application Architecture")
            .with_description("Develop application architecture for baseline and target states")
            .with_domain("C").with_priority(4))
        // Phase D - Technology Architecture
        .with_control(Control::new("D-1", "Technology Architecture")
            .with_description("Develop technology architecture to support applications and data")
            .with_domain("D").with_priority(4))
        .with_control(Control::new("D-2", "Technology Standards")
            .with_description("Define technology standards and guidelines")
            .with_domain("D").with_priority(4))
        // Phase E - Opportunities and Solutions
        .with_control(Control::new("E-1", "Implementation Projects")
            .with_description("Identify and group work packages into projects")
            .with_domain("E").with_priority(4))
        .with_control(Control::new("E-2", "Transition Architectures")
            .with_description("Identify transition architectures if necessary")
            .with_domain("E").with_priority(3))
        // Phase F - Migration Planning
        .with_control(Control::new("F-1", "Migration Plan")
            .with_description("Finalize detailed implementation and migration plan")
            .with_domain("F").with_priority(4))
        .with_control(Control::new("F-2", "Architecture Roadmap")
            .with_description("Finalize the architecture roadmap")
            .with_domain("F").with_priority(4))
        // Phase G - Implementation Governance
        .with_control(Control::new("G-1", "Architecture Compliance")
            .with_description("Ensure conformance with the target architecture by implementation projects")
            .with_domain("G").with_priority(5)
            .with_mapping(ControlMapping::new("cobit", "BAI01", "Program Management", MappingStrength::Moderate)))
        .with_control(Control::new("G-2", "Architecture Contract")
            .with_description("Govern and manage the architecture contract")
            .with_domain("G").with_priority(4))
        // Phase H - Architecture Change Management
        .with_control(Control::new("H-1", "Change Management Process")
            .with_description("Establish architecture change management process")
            .with_domain("H").with_priority(4))
        .with_control(Control::new("H-2", "Architecture Updates")
            .with_description("Manage changes to the architecture in a controlled manner")
            .with_domain("H").with_priority(4))
        // Requirements Management
        .with_control(Control::new("REQ-1", "Requirements Repository")
            .with_description("Manage architecture requirements throughout the ADM cycle")
            .with_domain("REQ").with_priority(4))
        .with_control(Control::new("REQ-2", "Impact Assessment")
            .with_description("Assess impact of changed requirements on architectures")
            .with_domain("REQ").with_priority(4))
}
