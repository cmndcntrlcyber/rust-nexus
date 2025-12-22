//! CMMC - Cybersecurity Maturity Model Certification
//!
//! DoD cybersecurity standard for defense contractors.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the CMMC 2.0 framework definition
pub fn create_framework() -> Framework {
    Framework::new("cmmc", "CMMC 2.0", FrameworkCategory::UsGovernment)
        .with_description("Cybersecurity Maturity Model Certification for Defense Industrial Base")
        .with_version("2.0")
        .with_url("https://www.acq.osd.mil/cmmc")
        .with_domain(FrameworkDomain::new("L1", "Level 1 - Foundational").with_order(1))
        .with_domain(FrameworkDomain::new("L2", "Level 2 - Advanced").with_order(2))
        .with_domain(FrameworkDomain::new("L3", "Level 3 - Expert").with_order(3))
        // Level 1 - Basic Safeguarding (17 practices based on FAR 52.204-21)
        .with_control(Control::new("AC.L1-3.1.1", "Authorized Access Control")
            .with_description("Limit system access to authorized users")
            .with_domain("L1").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_171", "3.1.1", "Limit system access", MappingStrength::Strong)))
        .with_control(Control::new("AC.L1-3.1.2", "Transaction Access Control")
            .with_description("Limit access to transactions and functions")
            .with_domain("L1").with_priority(4))
        .with_control(Control::new("IA.L1-3.5.1", "Identification")
            .with_description("Identify information system users")
            .with_domain("L1").with_priority(5))
        .with_control(Control::new("IA.L1-3.5.2", "Authentication")
            .with_description("Authenticate users prior to access")
            .with_domain("L1").with_priority(5))
        .with_control(Control::new("SC.L1-3.13.1", "Boundary Protection")
            .with_description("Monitor, control, and protect organizational communications")
            .with_domain("L1").with_priority(5))
        // Level 2 - Includes all 110 NIST 800-171 practices
        .with_control(Control::new("AC.L2-3.1.3", "Control CUI Flow")
            .with_description("Control the flow of CUI in accordance with approved authorizations")
            .with_domain("L2").with_priority(4)
            .with_mapping(ControlMapping::new("nist_800_171", "3.1.3", "Control CUI flow", MappingStrength::Strong)))
        .with_control(Control::new("AU.L2-3.3.1", "System Auditing")
            .with_description("Create and retain system audit logs and records")
            .with_domain("L2").with_priority(5)
            .with_mapping(ControlMapping::new("nist_800_171", "3.3.1", "System auditing", MappingStrength::Strong)))
        .with_control(Control::new("IR.L2-3.6.1", "Incident Handling")
            .with_description("Establish incident handling capability")
            .with_domain("L2").with_priority(4))
        .with_control(Control::new("SC.L2-3.13.8", "Cryptographic Protection")
            .with_description("Implement cryptographic mechanisms to prevent unauthorized disclosure of CUI")
            .with_domain("L2").with_priority(5))
}
