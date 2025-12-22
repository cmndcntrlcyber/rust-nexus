//! Framework definitions
//!
//! Contains definitions for all 20 supported compliance frameworks.
//! Each framework module provides:
//! - Framework metadata
//! - Domain/category definitions
//! - Control definitions with automated tests
//! - Cross-framework mappings

pub mod iso27001;
pub mod nist_csf;
pub mod soc2;
pub mod gdpr;
pub mod pci_dss;
pub mod hipaa;
pub mod nist_800_53;
pub mod cis_controls;
pub mod fedramp;
pub mod cmmc;

// Re-export framework creation functions
pub use iso27001::create_framework as create_iso27001;
pub use nist_csf::create_framework as create_nist_csf;
pub use soc2::create_framework as create_soc2;
pub use gdpr::create_framework as create_gdpr;
pub use pci_dss::create_framework as create_pci_dss;
pub use hipaa::create_framework as create_hipaa;
pub use nist_800_53::create_framework as create_nist_800_53;
pub use cis_controls::create_framework as create_cis_controls;
pub use fedramp::create_framework as create_fedramp;
pub use cmmc::create_framework as create_cmmc;
