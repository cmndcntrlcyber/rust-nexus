use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationType {
    /// SIEM systems
    Siem,
    /// Ticketing systems
    Ticketing,
    /// Cloud providers
    CloudProvider,
    /// Version control
    VersionControl,
    /// CI/CD systems
    CiCd,
    /// Webhook
    Webhook,
    /// Custom integration
    Custom,
}

impl std::fmt::Display for IntegrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationType::Siem => write!(f, "SIEM"),
            IntegrationType::Ticketing => write!(f, "Ticketing"),
            IntegrationType::CloudProvider => write!(f, "Cloud Provider"),
            IntegrationType::VersionControl => write!(f, "Version Control"),
            IntegrationType::CiCd => write!(f, "CI/CD"),
            IntegrationType::Webhook => write!(f, "Webhook"),
            IntegrationType::Custom => write!(f, "Custom"),
        }
    }
}

/// Health status of an integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationStatus {
    /// Integration is healthy and connected
    Healthy,
    /// Integration is degraded (partial functionality)
    Degraded,
    /// Integration is unhealthy (not connected)
    Unhealthy,
    /// Integration is disabled
    Disabled,
    /// Integration status is unknown
    Unknown,
}

impl std::fmt::Display for IntegrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationStatus::Healthy => write!(f, "Healthy"),
            IntegrationStatus::Degraded => write!(f, "Degraded"),
            IntegrationStatus::Unhealthy => write!(f, "Unhealthy"),
            IntegrationStatus::Disabled => write!(f, "Disabled"),
            IntegrationStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Integration ID
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Integration type
    pub integration_type: IntegrationType,
    /// Provider (e.g., "splunk", "jira", "aws")
    pub provider: String,
    /// Base URL/endpoint
    pub endpoint: String,
    /// Authentication method
    pub auth: AuthConfig,
    /// Whether integration is enabled
    pub enabled: bool,
    /// Custom configuration options
    pub options: serde_json::Value,
    /// When config was created
    pub created_at: DateTime<Utc>,
    /// When config was last updated
    pub updated_at: DateTime<Utc>,
}

impl IntegrationConfig {
    /// Create a new integration config
    pub fn new(
        name: &str,
        integration_type: IntegrationType,
        provider: &str,
        endpoint: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            integration_type,
            provider: provider.to_string(),
            endpoint: endpoint.to_string(),
            auth: AuthConfig::None,
            enabled: true,
            options: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set authentication
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// No authentication
    None,
    /// API key authentication
    ApiKey {
        key: String,
        header: String,
    },
    /// Bearer token authentication
    Bearer {
        token: String,
    },
    /// Basic authentication
    Basic {
        username: String,
        password: String,
    },
    /// OAuth2 authentication
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
        scopes: Vec<String>,
    },
}

/// Health check result for an integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationHealth {
    /// Integration ID
    pub integration_id: Uuid,
    /// Current status
    pub status: IntegrationStatus,
    /// Status message
    pub message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Last successful check
    pub last_success: Option<DateTime<Utc>>,
    /// When this check was performed
    pub checked_at: DateTime<Utc>,
}

impl IntegrationHealth {
    /// Create a healthy status
    pub fn healthy(integration_id: Uuid, response_time_ms: u64) -> Self {
        let now = Utc::now();
        Self {
            integration_id,
            status: IntegrationStatus::Healthy,
            message: None,
            response_time_ms: Some(response_time_ms),
            last_success: Some(now),
            checked_at: now,
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(integration_id: Uuid, message: &str) -> Self {
        Self {
            integration_id,
            status: IntegrationStatus::Unhealthy,
            message: Some(message.to_string()),
            response_time_ms: None,
            last_success: None,
            checked_at: Utc::now(),
        }
    }
}

/// Event collected from an integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationEvent {
    /// Event ID
    pub id: Uuid,
    /// Source integration ID
    pub integration_id: Uuid,
    /// Event type
    pub event_type: String,
    /// Event data
    pub data: serde_json::Value,
    /// When event occurred at source
    pub source_timestamp: Option<DateTime<Utc>>,
    /// When event was collected
    pub collected_at: DateTime<Utc>,
}

impl IntegrationEvent {
    /// Create a new event
    pub fn new(integration_id: Uuid, event_type: &str, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            integration_id,
            event_type: event_type.to_string(),
            data,
            source_timestamp: None,
            collected_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_config() {
        let config = IntegrationConfig::new(
            "Splunk SIEM",
            IntegrationType::Siem,
            "splunk",
            "https://splunk.example.com",
        )
        .with_auth(AuthConfig::Bearer {
            token: "secret".to_string(),
        });

        assert_eq!(config.name, "Splunk SIEM");
        assert_eq!(config.integration_type, IntegrationType::Siem);
        assert!(config.enabled);
    }

    #[test]
    fn test_integration_health() {
        let id = Uuid::new_v4();
        let healthy = IntegrationHealth::healthy(id, 150);
        assert_eq!(healthy.status, IntegrationStatus::Healthy);
        assert_eq!(healthy.response_time_ms, Some(150));

        let unhealthy = IntegrationHealth::unhealthy(id, "Connection refused");
        assert_eq!(unhealthy.status, IntegrationStatus::Unhealthy);
    }

    #[test]
    fn test_integration_event() {
        let id = Uuid::new_v4();
        let event = IntegrationEvent::new(
            id,
            "security_alert",
            serde_json::json!({"severity": "high"}),
        );

        assert_eq!(event.integration_id, id);
        assert_eq!(event.event_type, "security_alert");
    }

    #[test]
    fn test_type_display() {
        assert_eq!(IntegrationType::Siem.to_string(), "SIEM");
        assert_eq!(IntegrationStatus::Healthy.to_string(), "Healthy");
    }
}
