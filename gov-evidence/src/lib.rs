//! # gov-evidence
//!
//! Evidence collection and chain of custody for compliance auditing.
//!
//! This crate provides cryptographically-secured evidence collection with
//! comprehensive chain of custody tracking for compliance and audit purposes.
//!
//! ## Features
//!
//! - Evidence storage with SHA-256 hashing
//! - Digital signatures using Ed25519 (optional feature)
//! - Chain of custody with detailed audit trail
//! - Pluggable storage backends
//! - Evidence search and retrieval
//! - Metadata tagging and classification
//!
//! ## Example
//!
//! ```rust
//! use gov_evidence::{EvidenceCollector, EvidenceMetadata, MemoryStorage};
//!
//! let storage = Box::new(MemoryStorage::new());
//! let mut collector = EvidenceCollector::new(storage);
//!
//! let metadata = EvidenceMetadata {
//!     title: "Server Configuration".to_string(),
//!     frameworks: vec!["NIST-800-53".to_string()],
//!     controls: vec!["CM-2".to_string()],
//!     ..Default::default()
//! };
//!
//! let data = serde_json::json!({
//!     "hostname": "web-server-01",
//!     "os": "Ubuntu 22.04",
//!     "patch_level": "2024-01"
//! });
//!
//! let evidence_id = collector.collect(
//!     metadata,
//!     data,
//!     "automation@example.com".to_string()
//! ).unwrap();
//!
//! // Evidence is now stored with chain of custody
//! let evidence = collector.get(&evidence_id).unwrap();
//! assert!(evidence.verify_hash());
//! ```

pub mod collector;
pub mod error;
pub mod storage;
pub mod types;

// Re-export main types
pub use collector::EvidenceCollector;
pub use error::{EvidenceError, Result};
pub use storage::{MemoryStorage, StorageBackend};
pub use types::{CustodyEntry, CustodyState, Evidence, EvidenceMetadata};
