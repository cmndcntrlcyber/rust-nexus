//! # gov-tenancy
//!
//! Multi-tenant SaaS support for the compliance platform.
//!
//! This crate provides tenant isolation, quota management, and request-scoped
//! tenant context for multi-tenant deployments.
//!
//! ## Features
//!
//! - Tenant creation and management
//! - Subscription tiers with resource quotas
//! - Tenant context propagation
//! - Cross-tenant access prevention
//! - Pluggable tenant resolution (subdomain, API key, JWT)
//! - SSO/SAML configuration support
//!
//! ## Example
//!
//! ```rust
//! use gov_tenancy::{Tenant, TenantContext, TenantStore, MemoryTenantStore};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create tenant store
//!     let mut store = MemoryTenantStore::new();
//!
//!     // Create a new tenant
//!     let mut tenant = Tenant::new(
//!         "Acme Corp".to_string(),
//!         "acme".to_string(),
//!         "admin@acme.com".to_string(),
//!     );
//!     tenant.activate();
//!     store.create(&tenant).await?;
//!
//!     // Create tenant context for request
//!     let context = TenantContext::new(tenant.id, tenant.slug.clone())
//!         .with_user("user123".to_string(), vec!["admin".to_string()]);
//!
//!     assert!(context.is_admin());
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod resolver;
pub mod types;

// Re-export main types
pub use error::{Result, TenancyError};
pub use resolver::{MemoryTenantStore, TenantIsolation, TenantResolver, TenantStore};
pub use types::{
    BrandingConfig, NotificationConfig, ResourceQuota, SsoConfig, SubscriptionTier, Tenant,
    TenantConfig, TenantContext, TenantStatus,
};
