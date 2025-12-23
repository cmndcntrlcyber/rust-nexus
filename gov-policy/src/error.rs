use thiserror::Error;

/// Errors that can occur in policy operations
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Policy not found: {0}")]
    NotFound(String),

    #[error("Policy already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid policy rule: {0}")]
    InvalidRule(String),

    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Baseline not found: {0}")]
    BaselineNotFound(String),

    #[error("Drift detected: {count} violations found")]
    DriftDetected { count: usize },

    #[error("Invalid regex pattern: {0}")]
    InvalidPattern(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Result type alias for policy operations
pub type Result<T> = std::result::Result<T, PolicyError>;
