use thiserror::Error;

/// Errors that can occur in evidence collection and management
#[derive(Error, Debug)]
pub enum EvidenceError {
    #[error("Evidence not found: {0}")]
    NotFound(String),

    #[error("Invalid evidence hash: expected {expected}, got {actual}")]
    InvalidHash { expected: String, actual: String },

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Chain of custody violation: {0}")]
    CustodyViolation(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid evidence state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Evidence already exists: {0}")]
    AlreadyExists(String),
}

/// Result type alias for evidence operations
pub type Result<T> = std::result::Result<T, EvidenceError>;
