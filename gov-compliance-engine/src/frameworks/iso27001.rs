//! ISO 27001:2022
//!
//! International standard for information security management systems (ISMS).

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::{Control, ControlTest, CheckType, ComparisonOperator, Platform};
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the ISO 27001:2022 framework definition
pub fn create_framework() -> Framework {
    Framework::new("iso_27001", "ISO/IEC 27001:2022", FrameworkCategory::SecurityStandards)
        .with_description("International standard for establishing, implementing, maintaining and continually improving an information security management system")
        .with_version("2022")
        .with_url("https://www.iso.org/standard/27001")
        // Domains (Annex A)
        .with_domain(FrameworkDomain::new("A.5", "Organizational Controls")
            .with_description("37 organizational controls")
            .with_order(1))
        .with_domain(FrameworkDomain::new("A.6", "People Controls")
            .with_description("8 people controls")
            .with_order(2))
        .with_domain(FrameworkDomain::new("A.7", "Physical Controls")
            .with_description("14 physical controls")
            .with_order(3))
        .with_domain(FrameworkDomain::new("A.8", "Technological Controls")
            .with_description("34 technological controls")
            .with_order(4))
        // A.5 Organizational Controls
        .with_control(Control::new("A.5.1", "Policies for Information Security")
            .with_description("Information security policy and topic-specific policies shall be defined, approved by management, published, communicated to and acknowledged by relevant personnel")
            .with_domain("A.5")
            .with_family("Policies")
            .with_priority(5)
            .with_evidence("Policy document")
            .with_mapping(ControlMapping::new("nist_csf_2", "GV.PO-1", "Organizational Policy", MappingStrength::Strong)))
        .with_control(Control::new("A.5.2", "Information Security Roles and Responsibilities")
            .with_description("Information security roles and responsibilities shall be defined and allocated")
            .with_domain("A.5")
            .with_family("Organization")
            .with_priority(4))
        .with_control(Control::new("A.5.3", "Segregation of Duties")
            .with_description("Conflicting duties and conflicting areas of responsibility shall be segregated")
            .with_domain("A.5")
            .with_family("Organization")
            .with_priority(4))
        // A.8 Technological Controls
        .with_control(Control::new("A.8.1.1", "Inventory of Assets")
            .with_description("Assets associated with information and information processing facilities shall be identified and an inventory maintained")
            .with_domain("A.8")
            .with_family("Asset Management")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "ID.AM-1", "Physical devices inventoried", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("cis_controls", "1.1", "Enterprise Asset Inventory", MappingStrength::Strong)))
        .with_control(Control::new("A.8.2.3", "Handling of Assets")
            .with_description("Procedures for handling assets shall be developed and implemented")
            .with_domain("A.8")
            .with_family("Information Classification")
            .with_priority(4))
        .with_control(Control::new("A.8.6", "Capacity Management")
            .with_description("The use of resources shall be monitored and adjusted")
            .with_domain("A.8")
            .with_family("System Acquisition")
            .with_priority(3))
        // A.9 Access Control
        .with_control(Control::new("A.9.2.1", "User Registration and De-registration")
            .with_description("A formal user registration and de-registration process shall be implemented")
            .with_domain("A.8")
            .with_family("Access Control")
            .with_priority(5)
            .with_test(ControlTest::new(
                "a921-1",
                "Check user management process",
                CheckType::FileExists,
                "/etc/passwd"
            ).with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.AC-1", "Identity Management", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("soc2", "CC6.1", "Access Controls", MappingStrength::Strong)))
        .with_control(Control::new("A.9.4.3", "Password Management System")
            .with_description("Password management systems shall be interactive and shall ensure quality passwords")
            .with_domain("A.8")
            .with_family("Access Control")
            .with_priority(5)
            .with_test(ControlTest::new(
                "a943-1",
                "Check password complexity",
                CheckType::FileContent,
                "/etc/security/pwquality.conf"
            ).with_expected("minlen").with_operator(ComparisonOperator::Contains).with_platforms(vec![Platform::Linux])))
        // A.10 Cryptography
        .with_control(Control::new("A.10.1.1", "Policy on Use of Cryptographic Controls")
            .with_description("A policy on the use of cryptographic controls shall be developed and implemented")
            .with_domain("A.8")
            .with_family("Cryptography")
            .with_priority(4)
            .with_mapping(ControlMapping::new("pci_dss", "3.4", "Render PAN unreadable", MappingStrength::Moderate)))
        // A.12 Operations Security
        .with_control(Control::new("A.12.4.1", "Event Logging")
            .with_description("Event logs recording user activities, exceptions, faults and information security events shall be produced, kept and regularly reviewed")
            .with_domain("A.8")
            .with_family("Operations Security")
            .with_priority(5)
            .with_test(ControlTest::new(
                "a1241-1",
                "Verify logging enabled",
                CheckType::ServiceStatus,
                "rsyslog"
            ).with_expected("active").with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-1", "Network Monitoring", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("pci_dss", "10.2", "Audit trails", MappingStrength::Strong)))
        // A.16 Incident Management
        .with_control(Control::new("A.16.1.5", "Response to Information Security Incidents")
            .with_description("Information security incidents shall be responded to in accordance with the documented procedures")
            .with_domain("A.5")
            .with_family("Incident Management")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "RS.RP-1", "Response Planning", MappingStrength::Strong)))
        // A.17 Business Continuity
        .with_control(Control::new("A.17.1.2", "Implementing Information Security Continuity")
            .with_description("The organization shall establish, document, implement and maintain processes, procedures and controls to ensure continuity of information security")
            .with_domain("A.5")
            .with_family("Business Continuity")
            .with_priority(4)
            .with_mapping(ControlMapping::new("nist_csf_2", "RC.RP-1", "Recovery Planning", MappingStrength::Strong)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso27001_framework() {
        let framework = create_framework();
        assert_eq!(framework.id.as_str(), "iso_27001");
        assert!(!framework.controls.is_empty());
    }
}
