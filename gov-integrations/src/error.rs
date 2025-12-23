use thiserror::Error;

/// Errors that can occur in integrations
#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("Integration not found: {0}")]
    NotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("API error: {code} - {message}")]
    ApiError { code: u16, message: String },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Result type alias for integration operations
pub type Result<T> = std::result::Result<T, IntegrationError>;
