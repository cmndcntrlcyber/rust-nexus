use thiserror::Error;

/// Errors that can occur in report generation
#[derive(Error, Debug)]
pub enum ReportError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Report not found: {0}")]
    ReportNotFound(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error("Missing required data: {0}")]
    MissingData(String),

    #[error("Invalid output format: {0}")]
    InvalidFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for report operations
pub type Result<T> = std::result::Result<T, ReportError>;
