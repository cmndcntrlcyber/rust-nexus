//! Framework definitions and registry
//!
//! Defines the core Framework struct and provides a registry
//! for managing all 20 supported compliance frameworks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::control::Control;

/// Framework identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameworkId(pub String);

impl FrameworkId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FrameworkId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for FrameworkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of compliance framework
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameworkCategory {
    SecurityStandards,
    UsGovernment,
    AuditTrust,
    Privacy,
    Healthcare,
    Financial,
    Emerging,
}

impl FrameworkCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SecurityStandards => "Security Standards",
            Self::UsGovernment => "US Government",
            Self::AuditTrust => "Audit/Trust",
            Self::Privacy => "Privacy",
            Self::Healthcare => "Healthcare",
            Self::Financial => "Financial",
            Self::Emerging => "Emerging Standards",
        }
    }
}

/// A compliance framework definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    /// Unique identifier (e.g., "nist_csf_2")
    pub id: FrameworkId,

    /// Display name (e.g., "NIST Cybersecurity Framework 2.0")
    pub name: String,

    /// Short description
    pub description: String,

    /// Version of the framework
    pub version: String,

    /// Category of the framework
    pub category: FrameworkCategory,

    /// Official URL or documentation link
    pub official_url: Option<String>,

    /// Controls defined in this framework
    pub controls: Vec<Control>,

    /// Framework domains/categories (e.g., NIST CSF has Identify, Protect, etc.)
    pub domains: Vec<FrameworkDomain>,

    /// Whether this framework is enabled for assessment
    pub enabled: bool,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Framework {
    /// Create a new framework
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: FrameworkCategory,
    ) -> Self {
        Self {
            id: FrameworkId::new(id),
            name: name.into(),
            description: String::new(),
            version: "1.0".to_string(),
            category,
            official_url: None,
            controls: Vec::new(),
            domains: Vec::new(),
            enabled: true,
            metadata: HashMap::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the official URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.official_url = Some(url.into());
        self
    }

    /// Add a control
    pub fn with_control(mut self, control: Control) -> Self {
        self.controls.push(control);
        self
    }

    /// Add controls
    pub fn with_controls(mut self, controls: Vec<Control>) -> Self {
        self.controls.extend(controls);
        self
    }

    /// Add a domain
    pub fn with_domain(mut self, domain: FrameworkDomain) -> Self {
        self.domains.push(domain);
        self
    }

    /// Get a control by ID
    pub fn get_control(&self, control_id: &str) -> Option<&Control> {
        self.controls.iter().find(|c| c.id.as_str() == control_id)
    }

    /// Get controls in a specific domain
    pub fn controls_in_domain(&self, domain_id: &str) -> Vec<&Control> {
        self.controls
            .iter()
            .filter(|c| c.domain_id.as_deref() == Some(domain_id))
            .collect()
    }

    /// Count total controls
    pub fn control_count(&self) -> usize {
        self.controls.len()
    }
}

/// A domain or category within a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkDomain {
    /// Domain identifier (e.g., "ID" for NIST CSF Identify)
    pub id: String,

    /// Domain name (e.g., "Identify")
    pub name: String,

    /// Description of the domain
    pub description: String,

    /// Ordering within the framework
    pub order: u32,
}

impl FrameworkDomain {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            order: 0,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// Registry of all available frameworks
#[derive(Debug, Default)]
pub struct FrameworkRegistry {
    frameworks: HashMap<FrameworkId, Framework>,
}

impl FrameworkRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all supported frameworks
    pub fn with_all_frameworks() -> Self {
        let mut registry = Self::new();

        // Load all framework definitions
        registry.register(crate::frameworks::iso27001::create_framework());
        registry.register(crate::frameworks::nist_csf::create_framework());
        registry.register(crate::frameworks::soc2::create_framework());
        registry.register(crate::frameworks::gdpr::create_framework());
        registry.register(crate::frameworks::pci_dss::create_framework());
        registry.register(crate::frameworks::hipaa::create_framework());
        registry.register(crate::frameworks::nist_800_53::create_framework());
        registry.register(crate::frameworks::cis_controls::create_framework());
        registry.register(crate::frameworks::fedramp::create_framework());
        registry.register(crate::frameworks::cmmc::create_framework());

        registry
    }

    /// Register a framework
    pub fn register(&mut self, framework: Framework) {
        self.frameworks.insert(framework.id.clone(), framework);
    }

    /// Get a framework by ID
    pub fn get(&self, id: &FrameworkId) -> Option<&Framework> {
        self.frameworks.get(id)
    }

    /// Get a framework by ID string
    pub fn get_by_id(&self, id: &str) -> Option<&Framework> {
        self.frameworks.get(&FrameworkId::new(id))
    }

    /// List all frameworks
    pub fn list(&self) -> Vec<&Framework> {
        self.frameworks.values().collect()
    }

    /// List frameworks by category
    pub fn list_by_category(&self, category: FrameworkCategory) -> Vec<&Framework> {
        self.frameworks
            .values()
            .filter(|f| f.category == category)
            .collect()
    }

    /// List enabled frameworks
    pub fn list_enabled(&self) -> Vec<&Framework> {
        self.frameworks.values().filter(|f| f.enabled).collect()
    }

    /// Get framework count
    pub fn count(&self) -> usize {
        self.frameworks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_creation() {
        let framework = Framework::new("test_fw", "Test Framework", FrameworkCategory::SecurityStandards)
            .with_description("A test framework")
            .with_version("2.0");

        assert_eq!(framework.id.as_str(), "test_fw");
        assert_eq!(framework.name, "Test Framework");
        assert_eq!(framework.version, "2.0");
    }

    #[test]
    fn test_framework_registry() {
        let mut registry = FrameworkRegistry::new();

        let framework = Framework::new("iso_27001", "ISO 27001", FrameworkCategory::SecurityStandards);
        registry.register(framework);

        assert!(registry.get_by_id("iso_27001").is_some());
        assert!(registry.get_by_id("nonexistent").is_none());
    }
}
