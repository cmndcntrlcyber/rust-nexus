// v1.0 overlay code is intentionally not refactored in v1.4. Two pre-
// existing clippy patterns that the new `-D warnings` CI gate would
// reject get module-level allows here:
//
// - `ambiguous_glob_reexports`: `pub use messages::*` and
//   `pub use tasks::*` both export a `TaskResult` type. They've been
//   ambiguous since v1.0; we keep both modules wildcard-re-exported
//   so existing call sites compile, and treat the ambiguity as a
//   known issue. A real refactor (one canonical TaskResult) is
//   queued for v1.5.
// - `too_many_arguments`: `agent::AgentSession::new` takes 9
//   parameters. Overlay-supplied; tidied to a builder pattern in v1.5.
#![allow(ambiguous_glob_reexports, clippy::too_many_arguments)]

use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// Existing modules (overlay) — unchanged.
pub mod agent;
pub mod crypto;
pub mod messages;
pub mod tasks;
pub mod technique;

// v1.1 simple-mesh integration additions.
pub mod identity;
pub mod os;
pub mod sealed;

pub use agent::*;
pub use crypto::*;
pub use messages::*;
pub use tasks::*;
pub use technique::*;

// New re-exports (v1.1).
pub use identity::{NodeIdentity, PeerId, IDENTITY_BLOB_LEN};
pub use os::OsKind;
pub use sealed::{ReplayWindow, SealedEnvelope};

#[derive(Error, Debug)]
pub enum NexusError {
    #[error("Encryption failed: {0}")]
    EncryptionError(String),
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    #[error("Serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    #[error("Agent error: {0}")]
    AgentError(String),
    #[error("Task execution failed: {0}")]
    TaskExecutionError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Unknown technique: {0}")]
    UnknownTechnique(String),

    // v1.1 additions — used by the new identity / sealed-envelope modules.
    /// Identity blob malformed (length / magic / public-key bytes).
    #[error("Invalid identity: {0}")]
    InvalidIdentity(String),
    /// Ed25519 signature did not verify against the supplied public key.
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    /// AEAD operation (AES-GCM encrypt / decrypt) or KDF failure.
    #[error("Crypto failure: {0}")]
    CryptoFailure(String),
    /// Bincode encode / decode failure (sealed envelopes, mesh framing).
    #[error("Bincode error: {0}")]
    BincodeError(String),
}

pub type Result<T> = std::result::Result<T, NexusError>;

impl From<std::io::Error> for NexusError {
    fn from(e: std::io::Error) -> Self {
        NexusError::NetworkError(format!("io: {}", e))
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
