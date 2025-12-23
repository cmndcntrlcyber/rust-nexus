use thiserror::Error;

/// Errors that can occur in multi-tenancy operations
#[derive(Error, Debug)]
pub enum TenancyError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Tenant already exists: {0}")]
    TenantAlreadyExists(String),

    #[error("Tenant context not set")]
    NoTenantContext,

    #[error("Cross-tenant access denied: cannot access {target} from tenant {current}")]
    CrossTenantAccess { current: String, target: String },

    #[error("Quota exceeded for tenant {tenant}: {resource} limit is {limit}, requested {requested}")]
    QuotaExceeded {
        tenant: String,
        resource: String,
        limit: u64,
        requested: u64,
    },

    #[error("Tenant suspended: {0}")]
    TenantSuspended(String),

    #[error("Invalid tenant configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for tenancy operations
pub type Result<T> = std::result::Result<T, TenancyError>;
