//! Signature-based detection engine
//!
//! Provides pattern matching for known threats using signatures
//! ported from reverse-shell-detector and custom rules.

mod engine;
mod patterns;

pub use engine::SignatureEngine;
pub use patterns::{Pattern, PatternSet, PatternType};

