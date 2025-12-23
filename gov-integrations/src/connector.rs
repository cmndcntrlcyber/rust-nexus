use crate::error::{IntegrationError, Result};
use crate::types::{AuthConfig, IntegrationConfig, IntegrationEvent, IntegrationHealth};
use async_trait::async_trait;
use std::time::Instant;

/// Trait for integration connectors
#[async_trait]
pub trait Connector: Send + Sync {
    /// Get the integration configuration
    fn config(&self) -> &IntegrationConfig;

    /// Test connection and return health status
    async fn health_check(&self) -> Result<IntegrationHealth>;

    /// Collect events from the integration
    async fn collect_events(&self, since: Option<chrono::DateTime<chrono::Utc>>) -> Result<Vec<IntegrationEvent>>;

    /// Send data to the integration (if supported)
    async fn send(&self, event_type: &str, data: serde_json::Value) -> Result<()> {
        let _ = event_type;
        let _ = data;
        Err(IntegrationError::InvalidConfiguration(
            "Send not supported for this integration".to_string(),
        ))
    }
}

/// Generic HTTP-based connector
pub struct HttpConnector {
    config: IntegrationConfig,
    client: reqwest::Client,
}

impl HttpConnector {
    /// Create a new HTTP connector
    pub fn new(config: IntegrationConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { config, client })
    }

    /// Build request with authentication
    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);

        request = match &self.config.auth {
            AuthConfig::None => request,
            AuthConfig::ApiKey { key, header } => request.header(header.as_str(), key.as_str()),
            AuthConfig::Bearer { token } => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            AuthConfig::Basic { username, password } => request.basic_auth(username, Some(password)),
            AuthConfig::OAuth2 { .. } => {
                // OAuth2 would need token refresh logic
                request
            }
        };

        request
    }
}

#[async_trait]
impl Connector for HttpConnector {
    fn config(&self) -> &IntegrationConfig {
        &self.config
    }

    async fn health_check(&self) -> Result<IntegrationHealth> {
        let start = Instant::now();
        let url = format!("{}/health", self.config.endpoint);

        match self.build_request(&url).send().await {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                if response.status().is_success() {
                    Ok(IntegrationHealth::healthy(self.config.id, elapsed))
                } else {
                    Ok(IntegrationHealth::unhealthy(
                        self.config.id,
                        &format!("HTTP {}", response.status()),
                    ))
                }
            }
            Err(e) => Ok(IntegrationHealth::unhealthy(
                self.config.id,
                &e.to_string(),
            )),
        }
    }

    async fn collect_events(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<IntegrationEvent>> {
        // Generic implementation would fetch from API
        // Specific integrations would override this
        Ok(Vec::new())
    }

    async fn send(&self, event_type: &str, data: serde_json::Value) -> Result<()> {
        let url = format!("{}/events", self.config.endpoint);

        let payload = serde_json::json!({
            "type": event_type,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ApiError {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }
}

/// Webhook connector for receiving/sending webhooks
pub struct WebhookConnector {
    config: IntegrationConfig,
    client: reqwest::Client,
}

impl WebhookConnector {
    /// Create a new webhook connector
    pub fn new(config: IntegrationConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        Ok(Self { config, client })
    }

    /// Send a webhook
    pub async fn send_webhook(&self, payload: serde_json::Value) -> Result<()> {
        let response = self
            .client
            .post(&self.config.endpoint)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ApiError {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }
}

#[async_trait]
impl Connector for WebhookConnector {
    fn config(&self) -> &IntegrationConfig {
        &self.config
    }

    async fn health_check(&self) -> Result<IntegrationHealth> {
        // For webhooks, we can only verify the endpoint is reachable
        let start = Instant::now();

        match self.client.head(&self.config.endpoint).send().await {
            Ok(_) => {
                let elapsed = start.elapsed().as_millis() as u64;
                Ok(IntegrationHealth::healthy(self.config.id, elapsed))
            }
            Err(e) => Ok(IntegrationHealth::unhealthy(
                self.config.id,
                &e.to_string(),
            )),
        }
    }

    async fn collect_events(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<IntegrationEvent>> {
        // Webhooks are push-based, not pull-based
        Ok(Vec::new())
    }

    async fn send(&self, event_type: &str, data: serde_json::Value) -> Result<()> {
        let payload = serde_json::json!({
            "event_type": event_type,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.send_webhook(payload).await
    }
}

/// Integration registry for managing multiple connectors
pub struct IntegrationRegistry {
    connectors: std::collections::HashMap<uuid::Uuid, Box<dyn Connector>>,
}

impl IntegrationRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            connectors: std::collections::HashMap::new(),
        }
    }

    /// Register a connector
    pub fn register(&mut self, connector: Box<dyn Connector>) {
        let id = connector.config().id;
        self.connectors.insert(id, connector);
    }

    /// Get a connector by ID
    pub fn get(&self, id: &uuid::Uuid) -> Option<&dyn Connector> {
        self.connectors.get(id).map(|c| c.as_ref())
    }

    /// Check health of all connectors
    pub async fn health_check_all(&self) -> Vec<IntegrationHealth> {
        let mut results = Vec::new();
        for connector in self.connectors.values() {
            match connector.health_check().await {
                Ok(health) => results.push(health),
                Err(e) => {
                    results.push(IntegrationHealth::unhealthy(
                        connector.config().id,
                        &e.to_string(),
                    ));
                }
            }
        }
        results
    }

    /// List all integration IDs
    pub fn list_ids(&self) -> Vec<uuid::Uuid> {
        self.connectors.keys().copied().collect()
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::IntegrationType;

    #[test]
    fn test_registry() {
        let registry = IntegrationRegistry::new();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_http_connector_creation() {
        let config = IntegrationConfig::new(
            "Test",
            IntegrationType::Webhook,
            "generic",
            "https://example.com",
        );

        let connector = HttpConnector::new(config);
        assert!(connector.is_ok());
    }
}
