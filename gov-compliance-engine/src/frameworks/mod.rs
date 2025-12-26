//! Framework definitions
//!
//! Contains definitions for all 20 supported compliance frameworks.
//! Each framework module provides:
//! - Framework metadata
//! - Domain/category definitions
//! - Control definitions with automated tests
//! - Cross-framework mappings

// Security Standards
pub mod iso27001;
pub mod nist_csf;
pub mod cis_controls;

// US Government
pub mod nist_800_53;
pub mod nist_800_171;
pub mod cmmc;
pub mod fedramp;
pub mod fisma;

// Audit/Trust
pub mod soc2;
pub mod cobit;
pub mod coso;
pub mod togaf;

// Privacy
pub mod gdpr;
pub mod eu_ai_act;

// Healthcare
pub mod hipaa;
pub mod hitrust;

// Financial
pub mod pci_dss;
pub mod dora;

// Emerging
pub mod iso42001;
pub mod iso22301;

// Re-export framework creation functions
// Security Standards
pub use iso27001::create_framework as create_iso27001;
pub use nist_csf::create_framework as create_nist_csf;
pub use cis_controls::create_framework as create_cis_controls;

// US Government
pub use nist_800_53::create_framework as create_nist_800_53;
pub use nist_800_171::create_framework as create_nist_800_171;
pub use cmmc::create_framework as create_cmmc;
pub use fedramp::create_framework as create_fedramp;
pub use fisma::create_framework as create_fisma;

// Audit/Trust
pub use soc2::create_framework as create_soc2;
pub use cobit::create_framework as create_cobit;
pub use coso::create_framework as create_coso;
pub use togaf::create_framework as create_togaf;

// Privacy
pub use gdpr::create_framework as create_gdpr;
pub use eu_ai_act::create_framework as create_eu_ai_act;

// Healthcare
pub use hipaa::create_framework as create_hipaa;
pub use hitrust::create_framework as create_hitrust;

// Financial
pub use pci_dss::create_framework as create_pci_dss;
pub use dora::create_framework as create_dora;

// Emerging
pub use iso42001::create_framework as create_iso42001;
pub use iso22301::create_framework as create_iso22301;
