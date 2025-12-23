//! # gov-integrations
//!
//! Third-party integrations for compliance data collection.
//!
//! This crate provides connectors for integrating with external systems
//! such as SIEMs, ticketing systems, and cloud providers.
//!
//! ## Features
//!
//! - Generic HTTP connector
//! - Webhook sender/receiver
//! - Integration health monitoring
//! - Event collection and forwarding
//! - Multiple authentication methods
//!
//! ## Supported Integration Types
//!
//! - SIEM (Splunk, Azure Sentinel, Elastic)
//! - Ticketing (Jira, ServiceNow)
//! - Cloud Providers (AWS, Azure, GCP)
//! - Version Control (GitHub, GitLab)
//! - CI/CD (Jenkins, GitHub Actions)
//! - Webhooks
//!
//! ## Example
//!
//! ```rust
//! use gov_integrations::{
//!     HttpConnector, IntegrationConfig, IntegrationType, AuthConfig, Connector
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create integration config
//!     let config = IntegrationConfig::new(
//!         "Splunk SIEM",
//!         IntegrationType::Siem,
//!         "splunk",
//!         "https://splunk.example.com/api",
//!     ).with_auth(AuthConfig::Bearer {
//!         token: "your-token".to_string(),
//!     });
//!
//!     // Create connector
//!     let connector = HttpConnector::new(config)?;
//!
//!     // Check health
//!     let health = connector.health_check().await?;
//!     println!("Status: {}", health.status);
//!
//!     Ok(())
//! }
//! ```

pub mod connector;
pub mod error;
pub mod types;

// Re-export main types
pub use connector::{Connector, HttpConnector, IntegrationRegistry, WebhookConnector};
pub use error::{IntegrationError, Result};
pub use types::{
    AuthConfig, IntegrationConfig, IntegrationEvent, IntegrationHealth, IntegrationStatus,
    IntegrationType,
};
