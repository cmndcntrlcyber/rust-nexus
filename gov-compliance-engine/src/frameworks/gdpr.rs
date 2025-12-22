//! GDPR - General Data Protection Regulation
//!
//! EU regulation on data protection and privacy.

use crate::framework::{Framework, FrameworkCategory, FrameworkDomain};
use crate::control::Control;
use crate::mapping::{ControlMapping, MappingStrength};

/// Create the GDPR framework definition
pub fn create_framework() -> Framework {
    Framework::new("gdpr", "General Data Protection Regulation", FrameworkCategory::Privacy)
        .with_description("European Union regulation on data protection and privacy")
        .with_version("2016/679")
        .with_url("https://gdpr.eu")
        .with_domain(FrameworkDomain::new("Ch2", "Principles").with_order(1))
        .with_domain(FrameworkDomain::new("Ch3", "Rights of Data Subject").with_order(2))
        .with_domain(FrameworkDomain::new("Ch4", "Controller and Processor").with_order(3))
        .with_domain(FrameworkDomain::new("Ch5", "Transfers to Third Countries").with_order(4))
        // Key Articles
        .with_control(Control::new("Art.5", "Principles of Processing")
            .with_description("Personal data shall be processed lawfully, fairly, and transparently")
            .with_domain("Ch2")
            .with_priority(5))
        .with_control(Control::new("Art.6", "Lawfulness of Processing")
            .with_description("Processing shall be lawful only if and to the extent that at least one legal basis applies")
            .with_domain("Ch2")
            .with_priority(5))
        .with_control(Control::new("Art.17", "Right to Erasure")
            .with_description("The data subject has the right to obtain erasure of personal data")
            .with_domain("Ch3")
            .with_priority(4))
        .with_control(Control::new("Art.25", "Data Protection by Design")
            .with_description("Controller shall implement appropriate technical and organisational measures")
            .with_domain("Ch4")
            .with_priority(4))
        .with_control(Control::new("Art.32", "Security of Processing")
            .with_description("Implement appropriate technical and organisational measures to ensure security")
            .with_domain("Ch4")
            .with_priority(5)
            .with_mapping(ControlMapping::new("nist_csf_2", "PR.DS-1", "Data-at-rest", MappingStrength::Strong))
            .with_mapping(ControlMapping::new("iso_27001", "A.10.1.1", "Cryptographic controls", MappingStrength::Strong)))
        .with_control(Control::new("Art.33", "Breach Notification - Supervisory Authority")
            .with_description("Notify supervisory authority within 72 hours of breach discovery")
            .with_domain("Ch4")
            .with_priority(5))
        .with_control(Control::new("Art.35", "Data Protection Impact Assessment")
            .with_description("Carry out assessment of impact of processing operations on protection of personal data")
            .with_domain("Ch4")
            .with_priority(4))
}
