//! Gov-Compliance-Engine
//!
//! A comprehensive compliance engine supporting 20 governance frameworks
//! with cross-framework control mappings and automated testing capabilities.
//!
//! # Supported Frameworks
//!
//! ## Security Standards
//! - ISO 27001 - Information Security Management
//! - NIST CSF 2.0 - Cybersecurity Framework
//! - CIS Controls - Center for Internet Security Controls
//!
//! ## US Government
//! - FedRAMP - Federal Risk and Authorization Management
//! - CMMC - Cybersecurity Maturity Model Certification
//! - NIST 800-53 - Security and Privacy Controls
//! - NIST 800-171 - Protecting CUI
//! - FISMA - Federal Information Security Management Act
//!
//! ## Audit/Trust
//! - SOC 2 - Service Organization Controls
//! - COBIT - Control Objectives for IT
//! - COSO - Committee of Sponsoring Organizations
//! - TOGAF - The Open Group Architecture Framework
//!
//! ## Privacy
//! - GDPR - General Data Protection Regulation
//! - EU AI Act - EU Artificial Intelligence Act
//!
//! ## Healthcare
//! - HIPAA - Health Insurance Portability and Accountability
//! - HITRUST CSF - Health Information Trust Alliance
//!
//! ## Financial
//! - PCI DSS - Payment Card Industry Data Security Standard
//! - DORA - Digital Operational Resilience Act
//!
//! ## Emerging
//! - ISO 42001 - AI Management System
//! - ISO 22301 - Business Continuity Management

pub mod framework;
pub mod control;
pub mod mapping;
pub mod evidence;
pub mod scoring;
pub mod frameworks;

pub use framework::*;
pub use control::*;
pub use mapping::*;
pub use evidence::*;
pub use scoring::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::framework::{Framework, FrameworkCategory, FrameworkId, FrameworkRegistry};
    pub use crate::control::{Control, ControlId, ControlTest, CheckType, ControlStatus};
    pub use crate::mapping::{ControlMapping, MappingStrength};
    pub use crate::evidence::{Evidence, EvidenceType, CustodyEntry};
    pub use crate::scoring::{ComplianceScore, ControlScore};
}
