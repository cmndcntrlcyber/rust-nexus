//! HIPAA - Health Insurance Portability and Accountability Act
//!
//! US federal law for protecting sensitive patient health information.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the HIPAA framework definition
pub fn create_framework() -> Framework {
    Framework::new("hipaa", "HIPAA Security Rule", FrameworkCategory::Healthcare)
        .with_description("Health Insurance Portability and Accountability Act Security Rule")
        .with_version("2013")
        .with_url("https://www.hhs.gov/hipaa")
        .with_domain(FrameworkDomain::new("Admin", "Administrative Safeguards").with_order(1))
        .with_domain(FrameworkDomain::new("Physical", "Physical Safeguards").with_order(2))
        .with_domain(FrameworkDomain::new("Technical", "Technical Safeguards").with_order(3))
        // Administrative Safeguards
        .with_control(Control::new("164.308(a)(1)", "Security Management Process")
            .with_description("Implement policies and procedures to prevent, detect, contain, and correct security violations")
            .with_domain("Admin")
            .with_priority(5))
        .with_control(Control::new("164.308(a)(3)", "Workforce Security")
            .with_description("Implement policies and procedures to ensure all workforce members have appropriate access")
            .with_domain("Admin")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.AC-1", "Identity Management", MappingStrength::Strong)))
        .with_control(Control::new("164.308(a)(5)", "Security Awareness Training")
            .with_description("Implement a security awareness and training program for all workforce members")
            .with_domain("Admin")
            .with_priority(4))
        .with_control(Control::new("164.308(a)(6)", "Security Incident Procedures")
            .with_description("Implement policies and procedures to address security incidents")
            .with_domain("Admin")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "RS.RP-1", "Response Planning", MappingStrength::Strong)))
        // Technical Safeguards
        .with_control(Control::new("164.312(a)(1)", "Access Control")
            .with_description("Implement technical policies and procedures to allow access only to authorized persons")
            .with_domain("Technical")
            .with_priority(5)
            .with_mapping(ControlMapping::new("iso_27001", "A.9.2.1", "User registration", MappingStrength::Strong)))
        .with_control(Control::new("164.312(b)", "Audit Controls")
            .with_description("Implement hardware, software, and/or procedural mechanisms to record and examine access")
            .with_domain("Technical")
            .with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-1", "Network Monitoring", MappingStrength::Strong)))
        .with_control(Control::new("164.312(c)(1)", "Integrity Controls")
            .with_description("Implement policies and procedures to protect ePHI from improper alteration or destruction")
            .with_domain("Technical")
            .with_priority(4))
        .with_control(Control::new("164.312(e)(1)", "Transmission Security")
            .with_description("Implement technical security measures to guard against unauthorized access during transmission")
            .with_domain("Technical")
            .with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.DS-2", "Data-in-transit", MappingStrength::Strong)))
}
