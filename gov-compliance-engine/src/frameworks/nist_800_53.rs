//! NIST 800-53 - Security and Privacy Controls
//!
//! Comprehensive catalog of security and privacy controls for federal systems.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the NIST 800-53 Rev 5 framework definition
pub fn create_framework() -> Framework {
    Framework::new("nist_800_53", "NIST 800-53 Rev 5", FrameworkCategory::UsGovernment)
        .with_description("Security and Privacy Controls for Information Systems and Organizations")
        .with_version("Rev 5")
        .with_url("https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final")
        .with_domain(FrameworkDomain::new("AC", "Access Control").with_order(1))
        .with_domain(FrameworkDomain::new("AU", "Audit and Accountability").with_order(2))
        .with_domain(FrameworkDomain::new("AT", "Awareness and Training").with_order(3))
        .with_domain(FrameworkDomain::new("CM", "Configuration Management").with_order(4))
        .with_domain(FrameworkDomain::new("CP", "Contingency Planning").with_order(5))
        .with_domain(FrameworkDomain::new("IA", "Identification and Authentication").with_order(6))
        .with_domain(FrameworkDomain::new("IR", "Incident Response").with_order(7))
        .with_domain(FrameworkDomain::new("MA", "Maintenance").with_order(8))
        .with_domain(FrameworkDomain::new("MP", "Media Protection").with_order(9))
        .with_domain(FrameworkDomain::new("PE", "Physical and Environmental").with_order(10))
        .with_domain(FrameworkDomain::new("PL", "Planning").with_order(11))
        .with_domain(FrameworkDomain::new("PS", "Personnel Security").with_order(12))
        .with_domain(FrameworkDomain::new("RA", "Risk Assessment").with_order(13))
        .with_domain(FrameworkDomain::new("CA", "Assessment and Authorization").with_order(14))
        .with_domain(FrameworkDomain::new("SC", "System and Communications Protection").with_order(15))
        .with_domain(FrameworkDomain::new("SI", "System and Information Integrity").with_order(16))
        .with_domain(FrameworkDomain::new("SA", "System and Services Acquisition").with_order(17))
        // Access Control Family
        .with_control(Control::new("AC-1", "Policy and Procedures")
            .with_domain("AC").with_priority(4))
        .with_control(Control::new("AC-2", "Account Management")
            .with_domain("AC").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.AC-1", "Identity Management", MappingStrength::Strong)))
        .with_control(Control::new("AC-3", "Access Enforcement")
            .with_domain("AC").with_priority(5))
        .with_control(Control::new("AC-6", "Least Privilege")
            .with_domain("AC").with_priority(5))
        // Audit Family
        .with_control(Control::new("AU-2", "Audit Events")
            .with_domain("AU").with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-1", "Network Monitoring", MappingStrength::Strong)))
        .with_control(Control::new("AU-3", "Content of Audit Records")
            .with_domain("AU").with_priority(4))
        // Configuration Management
        .with_control(Control::new("CM-2", "Baseline Configuration")
            .with_domain("CM").with_priority(4))
        .with_control(Control::new("CM-6", "Configuration Settings")
            .with_domain("CM").with_priority(4))
        // Incident Response
        .with_control(Control::new("IR-4", "Incident Handling")
            .with_domain("IR").with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "RS.RP-1", "Response Planning", MappingStrength::Strong)))
        // System Protection
        .with_control(Control::new("SC-7", "Boundary Protection")
            .with_domain("SC").with_priority(5))
        .with_control(Control::new("SC-28", "Protection of Information at Rest")
            .with_domain("SC").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.DS-1", "Data-at-rest", MappingStrength::Strong)))
}
