//! CIS Controls v8
//!
//! Prioritized set of actions for cyber defense.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::{Control, ControlTest, CheckType, Platform};
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the CIS Controls v8 framework definition
pub fn create_framework() -> Framework {
    Framework::new("cis_controls", "CIS Controls v8", FrameworkCategory::SecurityStandards)
        .with_description("Prioritized set of actions to protect organizations and data from cyber attack vectors")
        .with_version("8.0")
        .with_url("https://www.cisecurity.org/controls")
        .with_domain(FrameworkDomain::new("IG1", "Implementation Group 1").with_order(1))
        .with_domain(FrameworkDomain::new("IG2", "Implementation Group 2").with_order(2))
        .with_domain(FrameworkDomain::new("IG3", "Implementation Group 3").with_order(3))
        // Control 1: Enterprise Asset Inventory
        .with_control(Control::new("1.1", "Establish Enterprise Asset Inventory")
            .with_domain("IG1").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "ID.AM-1", "Physical devices", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("iso_27001", "A.8.1.1", "Asset inventory", MappingStrength::Strong)))
        // Control 2: Software Inventory
        .with_control(Control::new("2.1", "Maintain Inventory of Authorized Software")
            .with_domain("IG1").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "ID.AM-2", "Software platforms", MappingStrength::Strong)))
        // Control 3: Data Protection
        .with_control(Control::new("3.1", "Establish Data Management Process")
            .with_domain("IG1").with_priority(4))
        // Control 4: Secure Configuration
        .with_control(Control::new("4.1", "Establish Secure Configuration Process")
            .with_domain("IG1").with_priority(5)
            .with_test(ControlTest::new("cis-4-1", "Check secure boot", CheckType::FileExists, "/sys/firmware/efi").with_platforms(vec![Platform::Linux])))
        // Control 5: Account Management
        .with_control(Control::new("5.1", "Establish Account Inventory")
            .with_domain("IG1").with_priority(5)
            .with_test(ControlTest::new("cis-5-1", "Check user accounts", CheckType::FileExists, "/etc/passwd").with_platforms(vec![Platform::Linux])))
        // Control 6: Access Control
        .with_control(Control::new("6.1", "Establish Access Granting Process")
            .with_domain("IG1").with_priority(5))
        // Control 8: Audit Log Management
        .with_control(Control::new("8.2", "Collect Audit Logs")
            .with_domain("IG1").with_priority(4)
            .with_test(ControlTest::new("cis-8-2", "Check audit service", CheckType::ServiceStatus, "auditd").with_expected("active").with_platforms(vec![Platform::Linux]))
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-1", "Network Monitoring", MappingStrength::Strong)))
        // Control 10: Malware Defenses
        .with_control(Control::new("10.1", "Deploy Anti-Malware Software")
            .with_domain("IG1").with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "DE.CM-4", "Malicious code", MappingStrength::Strong)))
        // Control 13: Network Monitoring
        .with_control(Control::new("13.1", "Maintain Firewall")
            .with_domain("IG1").with_priority(5))
}
