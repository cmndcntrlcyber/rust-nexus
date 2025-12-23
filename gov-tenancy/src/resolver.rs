use crate::error::{Result, TenancyError};
use crate::types::{Tenant, TenantContext};
use async_trait::async_trait;
use uuid::Uuid;

/// Trait for resolving tenant from request context
#[async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolve tenant from a subdomain (e.g., "acme" from "acme.govnexus.io")
    async fn resolve_by_subdomain(&self, subdomain: &str) -> Result<Tenant>;

    /// Resolve tenant from a slug
    async fn resolve_by_slug(&self, slug: &str) -> Result<Tenant>;

    /// Resolve tenant from ID
    async fn resolve_by_id(&self, id: &Uuid) -> Result<Tenant>;

    /// Resolve tenant from API key
    async fn resolve_by_api_key(&self, api_key: &str) -> Result<Tenant>;

    /// Resolve tenant from JWT claims
    async fn resolve_from_jwt(&self, tenant_claim: &str) -> Result<Tenant>;
}

/// Trait for managing tenant storage
#[async_trait]
pub trait TenantStore: Send + Sync {
    /// Create a new tenant
    async fn create(&mut self, tenant: &Tenant) -> Result<()>;

    /// Get tenant by ID
    async fn get(&self, id: &Uuid) -> Result<Tenant>;

    /// Get tenant by slug
    async fn get_by_slug(&self, slug: &str) -> Result<Tenant>;

    /// Update tenant
    async fn update(&mut self, tenant: &Tenant) -> Result<()>;

    /// Delete tenant
    async fn delete(&mut self, id: &Uuid) -> Result<()>;

    /// List all tenants
    async fn list(&self) -> Result<Vec<Tenant>>;

    /// Check if slug is available
    async fn is_slug_available(&self, slug: &str) -> Result<bool>;
}

/// In-memory tenant store (for testing and development)
#[derive(Debug, Default)]
pub struct MemoryTenantStore {
    tenants: std::collections::HashMap<Uuid, Tenant>,
}

impl MemoryTenantStore {
    /// Create a new in-memory tenant store
    pub fn new() -> Self {
        Self {
            tenants: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl TenantStore for MemoryTenantStore {
    async fn create(&mut self, tenant: &Tenant) -> Result<()> {
        if self.tenants.contains_key(&tenant.id) {
            return Err(TenancyError::TenantAlreadyExists(tenant.id.to_string()));
        }

        // Check if slug is available
        if self.tenants.values().any(|t| t.slug == tenant.slug) {
            return Err(TenancyError::TenantAlreadyExists(tenant.slug.clone()));
        }

        self.tenants.insert(tenant.id, tenant.clone());
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Tenant> {
        self.tenants
            .get(id)
            .cloned()
            .ok_or_else(|| TenancyError::TenantNotFound(id.to_string()))
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Tenant> {
        self.tenants
            .values()
            .find(|t| t.slug == slug)
            .cloned()
            .ok_or_else(|| TenancyError::TenantNotFound(slug.to_string()))
    }

    async fn update(&mut self, tenant: &Tenant) -> Result<()> {
        if !self.tenants.contains_key(&tenant.id) {
            return Err(TenancyError::TenantNotFound(tenant.id.to_string()));
        }
        self.tenants.insert(tenant.id, tenant.clone());
        Ok(())
    }

    async fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.tenants
            .remove(id)
            .ok_or_else(|| TenancyError::TenantNotFound(id.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Tenant>> {
        Ok(self.tenants.values().cloned().collect())
    }

    async fn is_slug_available(&self, slug: &str) -> Result<bool> {
        Ok(!self.tenants.values().any(|t| t.slug == slug))
    }
}

#[async_trait]
impl TenantResolver for MemoryTenantStore {
    async fn resolve_by_subdomain(&self, subdomain: &str) -> Result<Tenant> {
        self.get_by_slug(subdomain).await
    }

    async fn resolve_by_slug(&self, slug: &str) -> Result<Tenant> {
        self.get_by_slug(slug).await
    }

    async fn resolve_by_id(&self, id: &Uuid) -> Result<Tenant> {
        self.get(id).await
    }

    async fn resolve_by_api_key(&self, _api_key: &str) -> Result<Tenant> {
        // In a real implementation, this would look up API key in a database
        Err(TenancyError::TenantNotFound("API key lookup not implemented".to_string()))
    }

    async fn resolve_from_jwt(&self, tenant_claim: &str) -> Result<Tenant> {
        // Try to parse as UUID first
        if let Ok(id) = Uuid::parse_str(tenant_claim) {
            return self.get(&id).await;
        }
        // Otherwise try as slug
        self.get_by_slug(tenant_claim).await
    }
}

/// Tenant isolation enforcement
pub struct TenantIsolation {
    current_context: Option<TenantContext>,
}

impl TenantIsolation {
    /// Create new isolation enforcer
    pub fn new() -> Self {
        Self {
            current_context: None,
        }
    }

    /// Set the current tenant context
    pub fn set_context(&mut self, context: TenantContext) {
        self.current_context = Some(context);
    }

    /// Clear the current context
    pub fn clear_context(&mut self) {
        self.current_context = None;
    }

    /// Get the current context
    pub fn get_context(&self) -> Result<&TenantContext> {
        self.current_context
            .as_ref()
            .ok_or(TenancyError::NoTenantContext)
    }

    /// Validate that access to a tenant's data is allowed
    pub fn validate_access(&self, target_tenant_id: &Uuid) -> Result<()> {
        let context = self.get_context()?;

        if &context.tenant_id != target_tenant_id {
            return Err(TenancyError::CrossTenantAccess {
                current: context.tenant_id.to_string(),
                target: target_tenant_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Default for TenantIsolation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_create_get() {
        let mut store = MemoryTenantStore::new();
        let tenant = Tenant::new(
            "Test Company".to_string(),
            "test-company".to_string(),
            "admin@test.com".to_string(),
        );
        let id = tenant.id;

        store.create(&tenant).await.unwrap();
        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.name, "Test Company");
    }

    #[tokio::test]
    async fn test_memory_store_duplicate() {
        let mut store = MemoryTenantStore::new();
        let tenant = Tenant::new(
            "Test Company".to_string(),
            "test-company".to_string(),
            "admin@test.com".to_string(),
        );

        store.create(&tenant).await.unwrap();
        let result = store.create(&tenant).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_resolver() {
        let mut store = MemoryTenantStore::new();
        let tenant = Tenant::new(
            "Test Company".to_string(),
            "test-company".to_string(),
            "admin@test.com".to_string(),
        );

        store.create(&tenant).await.unwrap();

        let resolved = store.resolve_by_subdomain("test-company").await.unwrap();
        assert_eq!(resolved.name, "Test Company");
    }

    #[test]
    fn test_tenant_isolation() {
        let mut isolation = TenantIsolation::new();
        let tenant_id = Uuid::new_v4();
        let other_tenant_id = Uuid::new_v4();

        let context = TenantContext::new(tenant_id, "test".to_string());
        isolation.set_context(context);

        // Same tenant should be allowed
        assert!(isolation.validate_access(&tenant_id).is_ok());

        // Different tenant should be denied
        assert!(isolation.validate_access(&other_tenant_id).is_err());
    }

    #[test]
    fn test_isolation_no_context() {
        let isolation = TenantIsolation::new();
        assert!(isolation.get_context().is_err());
    }
}
