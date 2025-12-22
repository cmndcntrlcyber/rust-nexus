//! FedRAMP - Federal Risk and Authorization Management Program
//!
//! US government-wide program for cloud service provider security.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the FedRAMP framework definition (based on NIST 800-53)
pub fn create_framework() -> Framework {
    Framework::new("fedramp", "FedRAMP High Baseline", FrameworkCategory::UsGovernment)
        .with_description("Federal Risk and Authorization Management Program for cloud services")
        .with_version("Rev 5")
        .with_url("https://www.fedramp.gov")
        .with_domain(FrameworkDomain::new("AC", "Access Control").with_order(1))
        .with_domain(FrameworkDomain::new("AU", "Audit and Accountability").with_order(2))
        .with_domain(FrameworkDomain::new("CA", "Assessment and Authorization").with_order(3))
        .with_domain(FrameworkDomain::new("CM", "Configuration Management").with_order(4))
        .with_domain(FrameworkDomain::new("CP", "Contingency Planning").with_order(5))
        .with_domain(FrameworkDomain::new("IA", "Identification and Authentication").with_order(6))
        .with_domain(FrameworkDomain::new("IR", "Incident Response").with_order(7))
        .with_domain(FrameworkDomain::new("SC", "System and Communications Protection").with_order(8))
        .with_domain(FrameworkDomain::new("SI", "System and Information Integrity").with_order(9))
        // FedRAMP inherits from NIST 800-53 with additional requirements
        .with_control(Control::new("AC-2", "Account Management")
            .with_domain("AC").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "AC-2", "Account Management", MappingStrength::Strong)))
        .with_control(Control::new("AC-17", "Remote Access")
            .with_domain("AC").with_priority(5))
        .with_control(Control::new("AU-2", "Audit Events")
            .with_domain("AU").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_53", "AU-2", "Audit Events", MappingStrength::Strong)))
        .with_control(Control::new("CA-7", "Continuous Monitoring")
            .with_domain("CA").with_priority(5))
        .with_control(Control::new("CM-6", "Configuration Settings")
            .with_domain("CM").with_priority(4))
        .with_control(Control::new("IA-2", "Identification and Authentication")
            .with_domain("IA").with_priority(5)
            .with_description("Uniquely identify and authenticate organizational users"))
        .with_control(Control::new("IR-4", "Incident Handling")
            .with_domain("IR").with_priority(5))
        .with_control(Control::new("SC-7", "Boundary Protection")
            .with_domain("SC").with_priority(5))
        .with_control(Control::new("SC-28", "Protection of Information at Rest")
            .with_domain("SC").with_priority(5))
}
