//! NIST Cybersecurity Framework 2.0
//!
//! The NIST CSF provides a policy framework of computer security guidance
//! for how organizations can assess and improve their ability to prevent,
//! detect, and respond to cyber attacks.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::{Control, ControlTest, CheckType, ComparisonOperator, Platform};
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the NIST CSF 2.0 framework definition
pub fn create_framework() -> Framework {
    Framework::new("nist_csf_2", "NIST Cybersecurity Framework 2.0", FrameworkCategory::SecurityStandards)
        .with_description("A voluntary framework consisting of standards, guidelines, and best practices to manage cybersecurity-related risk")
        .with_version("2.0")
        .with_url("https://www.nist.gov/cyberframework")
        // Domains
        .with_domain(FrameworkDomain::new("GV", "Govern")
            .with_description("Establish and monitor the organization's cybersecurity risk management strategy, expectations, and policy")
            .with_order(1))
        .with_domain(FrameworkDomain::new("ID", "Identify")
            .with_description("Understand organizational cybersecurity risk to systems, assets, data, and capabilities")
            .with_order(2))
        .with_domain(FrameworkDomain::new("PR", "Protect")
            .with_description("Implement appropriate safeguards to ensure delivery of critical services")
            .with_order(3))
        .with_domain(FrameworkDomain::new("DE", "Detect")
            .with_description("Implement appropriate activities to identify the occurrence of a cybersecurity event")
            .with_order(4))
        .with_domain(FrameworkDomain::new("RS", "Respond")
            .with_description("Take action regarding a detected cybersecurity incident")
            .with_order(5))
        .with_domain(FrameworkDomain::new("RC", "Recover")
            .with_description("Maintain plans for resilience and restore capabilities or services impaired by cybersecurity incidents")
            .with_order(6))
        // Identify Controls
        .with_control(Control::new("ID.AM-1", "Asset Management - Physical Devices")
            .with_description("Physical devices and systems within the organization are inventoried")
            .with_domain("ID")
            .with_family("Asset Management")
            .with_priority(4)
            .with_test(ControlTest::new(
                "id-am-1-1",
                "Verify asset inventory exists",
                CheckType::FileExists,
                "/etc/asset-inventory.json"
            ).with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("iso_27001", "A.8.1.1", "Inventory of assets", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("soc2", "CC6.1", "Logical and Physical Access Controls", MappingStrength::Strong)))
        .with_control(Control::new("ID.AM-2", "Asset Management - Software Platforms")
            .with_description("Software platforms and applications within the organization are inventoried")
            .with_domain("ID")
            .with_family("Asset Management")
            .with_priority(4)
            .with_mapping(ControlMapping::new("iso_27001", "A.8.1.1", "Inventory of assets", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("cis_controls", "2.1", "Maintain Inventory of Authorized Software", MappingStrength::Strong)))
        .with_control(Control::new("ID.AM-5", "Asset Prioritization")
            .with_description("Resources are prioritized based on their classification, criticality, and business value")
            .with_domain("ID")
            .with_family("Asset Management")
            .with_priority(3))
        // Protect Controls
        .with_control(Control::new("PR.AC-1", "Identity Management and Access Control")
            .with_description("Identities and credentials are issued, managed, verified, revoked, and audited for authorized devices, users, and processes")
            .with_domain("PR")
            .with_family("Identity Management")
            .with_priority(5)
            .with_test(ControlTest::new(
                "pr-ac-1-1",
                "Verify password policy",
                CheckType::FileContent,
                "/etc/security/pwquality.conf"
            ).with_expected("minlen").with_operator(ComparisonOperator::Contains).with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("iso_27001", "A.9.2.1", "User registration and de-registration", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("soc2", "CC6.1", "Logical and Physical Access Controls", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("pci_dss", "8.1", "User Identification Management", MappingStrength::Strong)))
        .with_control(Control::new("PR.AC-3", "Remote Access Management")
            .with_description("Remote access is managed")
            .with_domain("PR")
            .with_family("Identity Management")
            .with_priority(4)
            .with_test(ControlTest::new(
                "pr-ac-3-1",
                "Verify SSH configuration",
                CheckType::FileContent,
                "/etc/ssh/sshd_config"
            ).with_expected("PermitRootLogin no").with_operator(ComparisonOperator::Contains).with_platforms(vec![Platform::Linux])))
        .with_control(Control::new("PR.DS-1", "Data-at-rest Protection")
            .with_description("Data-at-rest is protected")
            .with_domain("PR")
            .with_family("Data Security")
            .with_priority(5)
            .with_mapping(ControlMapping::new("iso_27001", "A.8.2.3", "Handling of assets", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("pci_dss", "3.4", "Render PAN unreadable", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("gdpr", "Art.32", "Security of processing", MappingStrength::Strong)))
        .with_control(Control::new("PR.DS-2", "Data-in-transit Protection")
            .with_description("Data-in-transit is protected")
            .with_domain("PR")
            .with_family("Data Security")
            .with_priority(5)
            .with_test(ControlTest::new(
                "pr-ds-2-1",
                "Check TLS version",
                CheckType::ConfigCheck,
                "/etc/ssl/openssl.cnf"
            ).with_platforms(vec![Platform::Linux])))
        // Detect Controls
        .with_control(Control::new("DE.CM-1", "Network Monitoring")
            .with_description("The network is monitored to detect potential cybersecurity events")
            .with_domain("DE")
            .with_family("Security Continuous Monitoring")
            .with_priority(4)
            .with_test(ControlTest::new(
                "de-cm-1-1",
                "Verify logging service running",
                CheckType::ServiceStatus,
                "rsyslog"
            ).with_expected("active").with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("iso_27001", "A.12.4.1", "Event logging", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("soc2", "CC7.2", "Security Monitoring", MappingStrength::Strong)))
        .with_control(Control::new("DE.CM-4", "Malicious Code Detection")
            .with_description("Malicious code is detected")
            .with_domain("DE")
            .with_family("Security Continuous Monitoring")
            .with_priority(4)
            .with_test(ControlTest::new(
                "de-cm-4-1",
                "Verify antivirus running",
                CheckType::ProcessRunning,
                "clamd"
            ).with_platforms(vec![Platform::Linux])))
        // Respond Controls
        .with_control(Control::new("RS.RP-1", "Response Planning")
            .with_description("Response plan is executed during or after an incident")
            .with_domain("RS")
            .with_family("Response Planning")
            .with_priority(4)
            .with_mapping(ControlMapping::new("iso_27001", "A.16.1.5", "Response to incidents", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("soc2", "CC7.4", "Incident Response", MappingStrength::Strong)))
        .with_control(Control::new("RS.AN-1", "Incident Analysis")
            .with_description("Notifications from detection systems are investigated")
            .with_domain("RS")
            .with_family("Analysis")
            .with_priority(3))
        // Recover Controls
        .with_control(Control::new("RC.RP-1", "Recovery Planning")
            .with_description("Recovery plan is executed during or after a cybersecurity incident")
            .with_domain("RC")
            .with_family("Recovery Planning")
            .with_priority(4)
            .with_mapping(ControlMapping::new("iso_27001", "A.17.1.2", "Implementing continuity", MappingStrength::Strong)))
        .with_control(Control::new("RC.CO-1", "Communications")
            .with_description("Public relations are managed")
            .with_domain("RC")
            .with_family("Communications")
            .with_priority(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nist_csf_framework() {
        let framework = create_framework();
        assert_eq!(framework.id.as_str(), "nist_csf_2");
        assert_eq!(framework.domains.len(), 6);
        assert!(!framework.controls.is_empty());
    }
}
