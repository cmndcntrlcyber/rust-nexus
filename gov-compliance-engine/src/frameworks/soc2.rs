//! SOC 2 - Service Organization Control 2
//!
//! Trust Services Criteria for security, availability, processing integrity,
//! confidentiality, and privacy.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::{Control, ControlTest, CheckType, Platform};
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the SOC 2 framework definition
pub fn create_framework() -> Framework {
    Framework::new("soc2", "SOC 2 Type II", FrameworkCategory::AuditTrust)
        .with_description("Service Organization Controls for security, availability, processing integrity, confidentiality, and privacy")
        .with_version("2017")
        .with_url("https://www.aicpa.org/soc4so")
        // Trust Services Categories
        .with_domain(FrameworkDomain::new("CC", "Common Criteria")
            .with_description("Common criteria across all trust services")
            .with_order(1))
        .with_domain(FrameworkDomain::new("A", "Availability")
            .with_description("System availability for operation and use")
            .with_order(2))
        .with_domain(FrameworkDomain::new("PI", "Processing Integrity")
            .with_description("System processing is complete, accurate, timely, and authorized")
            .with_order(3))
        .with_domain(FrameworkDomain::new("C", "Confidentiality")
            .with_description("Information designated as confidential is protected")
            .with_order(4))
        .with_domain(FrameworkDomain::new("P", "Privacy")
            .with_description("Personal information is collected, used, retained, disclosed, and disposed of properly")
            .with_order(5))
        // Common Criteria Controls
        .with_control(Control::new("CC1.1", "COSO Principle 1")
            .with_description("The entity demonstrates a commitment to integrity and ethical values")
            .with_domain("CC")
            .with_family("Control Environment")
            .with_priority(4))
        .with_control(Control::new("CC6.1", "Logical and Physical Access Controls")
            .with_description("The entity implements logical access security software, infrastructure, and architectures over protected information assets")
            .with_domain("CC")
            .with_family("Logical and Physical Access")
            .with_priority(5)
            .with_test(ControlTest::new(
                "cc61-1",
                "Check access control configuration",
                CheckType::FileExists,
                "/etc/pam.d/system-auth"
            ).with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.AC-1", "Identity Management", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("iso_27001", "A.9.2.1", "User registration", MappingStrength::Strong)))
        .with_control(Control::new("CC6.6", "Logical Access Security Measures")
            .with_description("The entity implements logical access security measures to protect against threats from sources outside its system boundaries")
            .with_domain("CC")
            .with_family("Logical and Physical Access")
            .with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.DS-1", "Data-at-rest", MappingStrength::Strong)))
        .with_control(Control::new("CC7.2", "Security Monitoring")
            .with_description("The entity monitors system components and the operation of those components for anomalies that are indicative of malicious acts, natural disasters, and errors")
            .with_domain("CC")
            .with_family("System Operations")
            .with_priority(4)
            .with_test(ControlTest::new(
                "cc72-1",
                "Verify monitoring enabled",
                CheckType::ServiceStatus,
                "auditd"
            ).with_expected("active").with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-1", "Network Monitoring", MappingStrength::Strong)))
        .with_control(Control::new("CC7.4", "Incident Response")
            .with_description("The entity responds to identified security incidents by executing a defined incident response program")
            .with_domain("CC")
            .with_family("System Operations")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "RS.RP-1", "Response Planning", MappingStrength::Strong)))
        .with_control(Control::new("CC8.1", "Change Management")
            .with_description("The entity authorizes, designs, develops or acquires, configures, documents, tests, approves, and implements changes to infrastructure")
            .with_domain("CC")
            .with_family("Change Management")
            .with_priority(4))
        // Availability Controls
        .with_control(Control::new("A1.1", "Availability Commitments")
            .with_description("The entity maintains, monitors, and evaluates current processing capacity and use")
            .with_domain("A")
            .with_family("Availability")
            .with_priority(3))
        // Confidentiality Controls
        .with_control(Control::new("C1.1", "Confidential Information Protection")
            .with_description("The entity identifies and maintains confidential information")
            .with_domain("C")
            .with_family("Confidentiality")
            .with_priority(4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soc2_framework() {
        let framework = create_framework();
        assert_eq!(framework.id.as_str(), "soc2");
    }
}
