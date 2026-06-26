//! Shared library for the rust-nexus C2 framework: crypto, message types, identity, and error handling.

// v1.5: both v1.4 module-level allows removed.
//   - `ambiguous_glob_reexports`: resolved — `messages::TaskResult` renamed
//     to `LegacyTaskResult`; `tasks::TaskResult` is now canonical.
//   - `too_many_arguments`: resolved — `Agent::new` (9-arg) replaced by
//     `AgentBuilder` in the `agent` module.

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

pub use agent::{
    Agent, AgentBuilder, AgentCapabilities, AgentSession, AgentStatus,
};
pub use crypto::*;
pub use messages::*;
pub use tasks::*;
pub use technique::*;

// New re-exports (v1.1).
pub use identity::{NodeIdentity, PeerId, IDENTITY_BLOB_LEN};
pub use os::OsKind;
pub use sealed::{ReplayWindow, SealedEnvelope};

/// Errors produced by nexus-common modules.
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

/// Convenience alias for `std::result::Result<T, NexusError>`.
pub type Result<T> = std::result::Result<T, NexusError>;

impl From<std::io::Error> for NexusError {
    fn from(e: std::io::Error) -> Self {
        NexusError::NetworkError(format!("io: {}", e))
    }
}

/// Current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate a random v4 UUID string.
pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
