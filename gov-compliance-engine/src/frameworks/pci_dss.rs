//! PCI DSS - Payment Card Industry Data Security Standard
//!
//! Security standards for organizations that handle credit card data.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::{Control, ControlTest, CheckType, ComparisonOperator, Platform};
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the PCI DSS 4.0 framework definition
pub fn create_framework() -> Framework {
    Framework::new("pci_dss", "PCI DSS 4.0", FrameworkCategory::Financial)
        .with_description("Payment Card Industry Data Security Standard for handling credit card data")
        .with_version("4.0")
        .with_url("https://www.pcisecuritystandards.org")
        .with_domain(FrameworkDomain::new("R1", "Build and Maintain a Secure Network").with_order(1))
        .with_domain(FrameworkDomain::new("R2", "Protect Cardholder Data").with_order(2))
        .with_domain(FrameworkDomain::new("R3", "Maintain Vulnerability Management").with_order(3))
        .with_domain(FrameworkDomain::new("R4", "Implement Strong Access Control").with_order(4))
        .with_domain(FrameworkDomain::new("R5", "Monitor and Test Networks").with_order(5))
        .with_domain(FrameworkDomain::new("R6", "Information Security Policy").with_order(6))
        // Requirement 1: Network Security
        .with_control(Control::new("1.1", "Network Security Controls")
            .with_description("Establish and implement firewall and router configurations")
            .with_domain("R1")
            .with_priority(5)
            .with_test(ControlTest::new(
                "pci-1-1",
                "Check firewall status",
                CheckType::ServiceStatus,
                "ufw"
            ).with_expected("active").with_platforms(vec![Platform::Linux])))
        // Requirement 3: Protect Cardholder Data
        .with_control(Control::new("3.4", "Render PAN Unreadable")
            .with_description("Render PAN unreadable anywhere it is stored")
            .with_domain("R2")
            .with_priority(5)
            .with_mapping(ControlMapping::new("iso_27001", "A.10.1.1", "Cryptographic controls", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.DS-1", "Data-at-rest", MappingStrength::Strong)))
        // Requirement 7: Access Control
        .with_control(Control::new("7.1", "Limit Access")
            .with_description("Limit access to system components and cardholder data to only those individuals whose job requires such access")
            .with_domain("R4")
            .with_priority(5)
            .with_mapping(ControlMapping::new("iso_27001", "A.9.2.1", "User registration", MappingStrength::Strong)))
        // Requirement 8: Authentication
        .with_control(Control::new("8.1", "User Identification")
            .with_description("Define and implement policies and procedures to ensure proper user identification management")
            .with_domain("R4")
            .with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.AC-1", "Identity Management", MappingStrength::Strong)))
        .with_control(Control::new("8.2", "User Authentication")
            .with_description("Use proper user authentication management")
            .with_domain("R4")
            .with_priority(5)
            .with_test(ControlTest::new(
                "pci-8-2",
                "Check password policy",
                CheckType::FileContent,
                "/etc/login.defs"
            ).with_expected("PASS_MIN_LEN").with_operator(ComparisonOperator::Contains).with_platforms(vec![Platform::Linux])))
        // Requirement 10: Logging
        .with_control(Control::new("10.2", "Audit Trails")
            .with_description("Implement automated audit trails for all system components")
            .with_domain("R5")
            .with_priority(5)
            .with_test(ControlTest::new(
                "pci-10-2",
                "Verify audit logging",
                CheckType::ServiceStatus,
                "auditd"
            ).with_expected("active").with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("iso_27001", "A.12.4.1", "Event logging", MappingStrength::Strong)))
        // Requirement 12: Security Policy
        .with_control(Control::new("12.1", "Security Policy")
            .with_description("Establish, publish, maintain, and disseminate a security policy")
            .with_domain("R6")
            .with_priority(4))
}
