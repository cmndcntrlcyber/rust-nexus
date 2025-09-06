use std::time::{SystemTime, UNIX_EPOCH};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::Aead, KeyInit};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use thiserror::Error;

pub mod crypto;
pub mod messages;
pub mod agent;
pub mod tasks;

pub use crypto::*;
pub use messages::*;
pub use agent::*;
pub use tasks::*;

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
}

pub type Result<T> = std::result::Result<T, NexusError>;

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
