use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a tenant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantStatus {
    /// Tenant is active and can use the system
    Active,
    /// Tenant is in trial period
    Trial,
    /// Tenant is suspended (e.g., for non-payment)
    Suspended,
    /// Tenant is being provisioned
    Provisioning,
    /// Tenant is being deprovisioned
    Deprovisioning,
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "Active"),
            TenantStatus::Trial => write!(f, "Trial"),
            TenantStatus::Suspended => write!(f, "Suspended"),
            TenantStatus::Provisioning => write!(f, "Provisioning"),
            TenantStatus::Deprovisioning => write!(f, "Deprovisioning"),
        }
    }
}

/// Subscription tier for a tenant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    /// Free tier with limited features
    Free,
    /// Basic paid tier
    Basic,
    /// Professional tier with more features
    Professional,
    /// Enterprise tier with all features
    Enterprise,
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        SubscriptionTier::Free
    }
}

/// Resource quotas for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Maximum number of assets
    pub max_assets: u64,
    /// Maximum number of users
    pub max_users: u64,
    /// Maximum number of compliance scans per month
    pub max_scans_per_month: u64,
    /// Maximum evidence storage in bytes
    pub max_evidence_storage_bytes: u64,
    /// Maximum number of frameworks enabled
    pub max_frameworks: u64,
    /// Maximum retention period in days
    pub max_retention_days: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_assets: 100,
            max_users: 5,
            max_scans_per_month: 10,
            max_evidence_storage_bytes: 1024 * 1024 * 1024, // 1 GB
            max_frameworks: 3,
            max_retention_days: 365,
        }
    }
}

impl ResourceQuota {
    /// Create quotas for free tier
    pub fn free_tier() -> Self {
        Self::default()
    }

    /// Create quotas for basic tier
    pub fn basic_tier() -> Self {
        Self {
            max_assets: 500,
            max_users: 25,
            max_scans_per_month: 100,
            max_evidence_storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            max_frameworks: 10,
            max_retention_days: 730, // 2 years
        }
    }

    /// Create quotas for professional tier
    pub fn professional_tier() -> Self {
        Self {
            max_assets: 5000,
            max_users: 100,
            max_scans_per_month: 1000,
            max_evidence_storage_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
            max_frameworks: 20,
            max_retention_days: 2555, // 7 years
        }
    }

    /// Create quotas for enterprise tier (unlimited)
    pub fn enterprise_tier() -> Self {
        Self {
            max_assets: u64::MAX,
            max_users: u64::MAX,
            max_scans_per_month: u64::MAX,
            max_evidence_storage_bytes: u64::MAX,
            max_frameworks: 20,
            max_retention_days: 3650, // 10 years
        }
    }
}

/// Tenant-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Enabled compliance frameworks
    pub enabled_frameworks: Vec<String>,
    /// Default classification for evidence
    pub default_classification: String,
    /// Custom branding settings
    pub branding: Option<BrandingConfig>,
    /// SSO/SAML configuration
    pub sso_config: Option<SsoConfig>,
    /// Notification settings
    pub notifications: NotificationConfig,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            enabled_frameworks: vec!["nist_csf_2".to_string(), "iso_27001".to_string()],
            default_classification: "internal".to_string(),
            branding: None,
            sso_config: None,
            notifications: NotificationConfig::default(),
        }
    }
}

/// Custom branding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Company name
    pub company_name: String,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary color (hex)
    pub primary_color: Option<String>,
    /// Secondary color (hex)
    pub secondary_color: Option<String>,
}

/// SSO/SAML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// SSO provider (e.g., "okta", "azure_ad", "google")
    pub provider: String,
    /// Identity provider entity ID
    pub idp_entity_id: String,
    /// SSO login URL
    pub sso_url: String,
    /// Certificate for signature validation
    pub certificate: String,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Email notifications enabled
    pub email_enabled: bool,
    /// Slack integration enabled
    pub slack_enabled: bool,
    /// Slack webhook URL
    pub slack_webhook_url: Option<String>,
    /// Alert on compliance drift
    pub alert_on_drift: bool,
    /// Alert on scan failures
    pub alert_on_scan_failure: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            email_enabled: true,
            slack_enabled: false,
            slack_webhook_url: None,
            alert_on_drift: true,
            alert_on_scan_failure: true,
        }
    }
}

/// Core tenant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub id: Uuid,
    /// Tenant name/company name
    pub name: String,
    /// URL-friendly slug
    pub slug: String,
    /// Tenant status
    pub status: TenantStatus,
    /// Subscription tier
    pub tier: SubscriptionTier,
    /// Resource quotas
    pub quota: ResourceQuota,
    /// Tenant-specific configuration
    pub config: TenantConfig,
    /// Primary contact email
    pub contact_email: String,
    /// When tenant was created
    pub created_at: DateTime<Utc>,
    /// When tenant was last updated
    pub updated_at: DateTime<Utc>,
    /// When trial expires (if applicable)
    pub trial_expires_at: Option<DateTime<Utc>>,
}

impl Tenant {
    /// Create a new tenant
    pub fn new(name: String, slug: String, contact_email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            status: TenantStatus::Provisioning,
            tier: SubscriptionTier::Free,
            quota: ResourceQuota::free_tier(),
            config: TenantConfig::default(),
            contact_email,
            created_at: now,
            updated_at: now,
            trial_expires_at: None,
        }
    }

    /// Create a tenant with trial period
    pub fn new_trial(name: String, slug: String, contact_email: String, trial_days: i64) -> Self {
        let now = Utc::now();
        let trial_expires = now + chrono::Duration::days(trial_days);
        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            status: TenantStatus::Trial,
            tier: SubscriptionTier::Professional, // Give full access during trial
            quota: ResourceQuota::professional_tier(),
            config: TenantConfig::default(),
            contact_email,
            created_at: now,
            updated_at: now,
            trial_expires_at: Some(trial_expires),
        }
    }

    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, TenantStatus::Active | TenantStatus::Trial)
    }

    /// Check if trial has expired
    pub fn is_trial_expired(&self) -> bool {
        if let Some(expires) = self.trial_expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Activate the tenant
    pub fn activate(&mut self) {
        self.status = TenantStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Suspend the tenant
    pub fn suspend(&mut self) {
        self.status = TenantStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Update subscription tier
    pub fn set_tier(&mut self, tier: SubscriptionTier) {
        self.tier = tier;
        self.quota = match tier {
            SubscriptionTier::Free => ResourceQuota::free_tier(),
            SubscriptionTier::Basic => ResourceQuota::basic_tier(),
            SubscriptionTier::Professional => ResourceQuota::professional_tier(),
            SubscriptionTier::Enterprise => ResourceQuota::enterprise_tier(),
        };
        self.updated_at = Utc::now();
    }
}

/// Request-scoped tenant context
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Current tenant ID
    pub tenant_id: Uuid,
    /// Current tenant slug
    pub tenant_slug: String,
    /// Current user ID (if authenticated)
    pub user_id: Option<String>,
    /// User's roles within the tenant
    pub roles: Vec<String>,
}

impl TenantContext {
    /// Create a new tenant context
    pub fn new(tenant_id: Uuid, tenant_slug: String) -> Self {
        Self {
            tenant_id,
            tenant_slug,
            user_id: None,
            roles: Vec::new(),
        }
    }

    /// Create context with user
    pub fn with_user(mut self, user_id: String, roles: Vec<String>) -> Self {
        self.user_id = Some(user_id);
        self.roles = roles;
        self
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.has_role("admin") || self.has_role("owner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new(
            "Test Company".to_string(),
            "test-company".to_string(),
            "admin@test.com".to_string(),
        );

        assert!(!tenant.id.is_nil());
        assert_eq!(tenant.name, "Test Company");
        assert_eq!(tenant.status, TenantStatus::Provisioning);
        assert_eq!(tenant.tier, SubscriptionTier::Free);
    }

    #[test]
    fn test_tenant_trial() {
        let tenant = Tenant::new_trial(
            "Trial Company".to_string(),
            "trial-company".to_string(),
            "admin@trial.com".to_string(),
            14,
        );

        assert_eq!(tenant.status, TenantStatus::Trial);
        assert_eq!(tenant.tier, SubscriptionTier::Professional);
        assert!(tenant.trial_expires_at.is_some());
        assert!(!tenant.is_trial_expired());
    }

    #[test]
    fn test_tenant_activation() {
        let mut tenant = Tenant::new(
            "Test Company".to_string(),
            "test-company".to_string(),
            "admin@test.com".to_string(),
        );

        assert!(!tenant.is_active());
        tenant.activate();
        assert!(tenant.is_active());
    }

    #[test]
    fn test_tenant_context() {
        let context = TenantContext::new(Uuid::new_v4(), "test".to_string())
            .with_user("user123".to_string(), vec!["admin".to_string(), "viewer".to_string()]);

        assert!(context.is_admin());
        assert!(context.has_role("viewer"));
        assert!(!context.has_role("editor"));
    }

    #[test]
    fn test_resource_quotas() {
        let free = ResourceQuota::free_tier();
        let enterprise = ResourceQuota::enterprise_tier();

        assert!(enterprise.max_assets > free.max_assets);
        assert!(enterprise.max_users > free.max_users);
    }
}
