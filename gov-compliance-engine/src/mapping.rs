//! Cross-framework control mappings
//!
//! Provides the ability to map controls between different frameworks,
//! enabling unified compliance views and efficient multi-framework compliance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::framework::FrameworkId;
use crate::control::ControlId;

/// Strength of the mapping between controls
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappingStrength {
    /// Exact or near-exact mapping (>90% overlap)
    Strong,
    /// Substantial overlap (60-90%)
    Moderate,
    /// Partial overlap (30-60%)
    Weak,
    /// Related but distinct (<30%)
    Related,
}

impl MappingStrength {
    /// Get a numeric weight for scoring purposes
    pub fn weight(&self) -> f64 {
        match self {
            Self::Strong => 1.0,
            Self::Moderate => 0.75,
            Self::Weak => 0.5,
            Self::Related => 0.25,
        }
    }
}

/// A mapping from one control to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMapping {
    /// Target framework ID
    pub target_framework: FrameworkId,

    /// Target control ID
    pub target_control_id: String,

    /// Target control name (for display)
    pub target_control_name: String,

    /// Strength of the mapping
    pub strength: MappingStrength,

    /// Notes about the mapping
    pub notes: Option<String>,
}

impl ControlMapping {
    /// Create a new control mapping
    pub fn new(
        target_framework: impl Into<String>,
        target_control_id: impl Into<String>,
        target_control_name: impl Into<String>,
        strength: MappingStrength,
    ) -> Self {
        Self {
            target_framework: FrameworkId::new(target_framework),
            target_control_id: target_control_id.into(),
            target_control_name: target_control_name.into(),
            strength,
            notes: None,
        }
    }

    /// Add notes to the mapping
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Registry of all cross-framework mappings
#[derive(Debug, Default)]
pub struct MappingRegistry {
    /// Mappings indexed by source framework -> source control -> target mappings
    mappings: HashMap<FrameworkId, HashMap<ControlId, Vec<ControlMapping>>>,
}

impl MappingRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mapping
    pub fn add_mapping(
        &mut self,
        source_framework: FrameworkId,
        source_control: ControlId,
        mapping: ControlMapping,
    ) {
        self.mappings
            .entry(source_framework)
            .or_default()
            .entry(source_control)
            .or_default()
            .push(mapping);
    }

    /// Get mappings for a specific control
    pub fn get_mappings(
        &self,
        source_framework: &FrameworkId,
        source_control: &ControlId,
    ) -> Option<&Vec<ControlMapping>> {
        self.mappings
            .get(source_framework)
            .and_then(|fw| fw.get(source_control))
    }

    /// Get all mappings to a specific framework
    pub fn get_mappings_to_framework(
        &self,
        source_framework: &FrameworkId,
        target_framework: &FrameworkId,
    ) -> Vec<(&ControlId, &ControlMapping)> {
        let mut result = Vec::new();
        if let Some(fw_mappings) = self.mappings.get(source_framework) {
            for (control_id, mappings) in fw_mappings {
                for mapping in mappings {
                    if &mapping.target_framework == target_framework {
                        result.push((control_id, mapping));
                    }
                }
            }
        }
        result
    }

    /// Find equivalent controls across all frameworks
    pub fn find_equivalent_controls(
        &self,
        source_framework: &FrameworkId,
        source_control: &ControlId,
    ) -> Vec<&ControlMapping> {
        self.get_mappings(source_framework, source_control)
            .map(|mappings| {
                mappings
                    .iter()
                    .filter(|m| m.strength == MappingStrength::Strong)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Unified control that aggregates mappings across frameworks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedControl {
    /// Primary identifier
    pub id: String,

    /// Unified name
    pub name: String,

    /// Description
    pub description: String,

    /// Mappings to each framework's control
    pub framework_mappings: HashMap<String, String>,

    /// Common test that applies to all mapped controls
    pub unified_tests: Vec<String>,
}

impl UnifiedControl {
    /// Create a new unified control
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            framework_mappings: HashMap::new(),
            unified_tests: Vec::new(),
        }
    }

    /// Add a framework mapping
    pub fn add_mapping(&mut self, framework_id: &str, control_id: &str) {
        self.framework_mappings
            .insert(framework_id.to_string(), control_id.to_string());
    }

    /// Check if this unified control covers a specific framework
    pub fn covers_framework(&self, framework_id: &str) -> bool {
        self.framework_mappings.contains_key(framework_id)
    }
}

/// Common cross-framework mapping builder
pub struct CommonMappings;

impl CommonMappings {
    /// Create NIST CSF to ISO 27001 mappings
    pub fn nist_csf_to_iso27001() -> Vec<(String, ControlMapping)> {
        vec![
            // Identify
            (
                "ID.AM-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.8.1.1",
                    "Inventory of assets",
                    MappingStrength::Strong,
                ),
            ),
            (
                "ID.AM-2".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.8.1.1",
                    "Inventory of assets",
                    MappingStrength::Strong,
                ),
            ),
            // Protect
            (
                "PR.AC-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.9.2.1",
                    "User registration and de-registration",
                    MappingStrength::Strong,
                ),
            ),
            (
                "PR.AC-3".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.6.2.2",
                    "Teleworking",
                    MappingStrength::Moderate,
                ),
            ),
            (
                "PR.DS-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.8.2.3",
                    "Handling of assets",
                    MappingStrength::Strong,
                ),
            ),
            // Detect
            (
                "DE.CM-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.12.4.1",
                    "Event logging",
                    MappingStrength::Strong,
                ),
            ),
            // Respond
            (
                "RS.RP-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.16.1.5",
                    "Response to information security incidents",
                    MappingStrength::Strong,
                ),
            ),
            // Recover
            (
                "RC.RP-1".to_string(),
                ControlMapping::new(
                    "iso_27001",
                    "A.17.1.2",
                    "Implementing information security continuity",
                    MappingStrength::Strong,
                ),
            ),
        ]
    }

    /// Create NIST CSF to SOC 2 mappings
    pub fn nist_csf_to_soc2() -> Vec<(String, ControlMapping)> {
        vec![
            (
                "ID.AM-1".to_string(),
                ControlMapping::new(
                    "soc2",
                    "CC6.1",
                    "Logical and Physical Access Controls",
                    MappingStrength::Strong,
                ),
            ),
            (
                "PR.AC-1".to_string(),
                ControlMapping::new(
                    "soc2",
                    "CC6.1",
                    "Logical and Physical Access Controls",
                    MappingStrength::Strong,
                ),
            ),
            (
                "PR.DS-1".to_string(),
                ControlMapping::new(
                    "soc2",
                    "CC6.6",
                    "Logical Access Security Measures",
                    MappingStrength::Strong,
                ),
            ),
            (
                "DE.CM-1".to_string(),
                ControlMapping::new(
                    "soc2",
                    "CC7.2",
                    "Security Monitoring",
                    MappingStrength::Strong,
                ),
            ),
            (
                "RS.RP-1".to_string(),
                ControlMapping::new(
                    "soc2",
                    "CC7.4",
                    "Incident Response",
                    MappingStrength::Strong,
                ),
            ),
        ]
    }

    /// Create ISO 27001 to PCI DSS mappings
    pub fn iso27001_to_pci_dss() -> Vec<(String, ControlMapping)> {
        vec![
            (
                "A.9.2.1".to_string(),
                ControlMapping::new(
                    "pci_dss",
                    "7.1",
                    "Limit access to system components and cardholder data",
                    MappingStrength::Strong,
                ),
            ),
            (
                "A.9.4.3".to_string(),
                ControlMapping::new(
                    "pci_dss",
                    "8.2",
                    "Proper user authentication management",
                    MappingStrength::Strong,
                ),
            ),
            (
                "A.10.1.1".to_string(),
                ControlMapping::new(
                    "pci_dss",
                    "3.4",
                    "Render PAN unreadable anywhere it is stored",
                    MappingStrength::Strong,
                ),
            ),
            (
                "A.12.4.1".to_string(),
                ControlMapping::new(
                    "pci_dss",
                    "10.2",
                    "Implement automated audit trails",
                    MappingStrength::Strong,
                ),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_mapping() {
        let mapping = ControlMapping::new(
            "iso_27001",
            "A.9.2.1",
            "User registration",
            MappingStrength::Strong,
        );

        assert_eq!(mapping.target_framework.as_str(), "iso_27001");
        assert_eq!(mapping.strength.weight(), 1.0);
    }

    #[test]
    fn test_mapping_registry() {
        let mut registry = MappingRegistry::new();

        let mapping = ControlMapping::new(
            "iso_27001",
            "A.9.2.1",
            "User registration",
            MappingStrength::Strong,
        );

        registry.add_mapping(
            FrameworkId::new("nist_csf"),
            ControlId::new("PR.AC-1"),
            mapping,
        );

        let mappings = registry.get_mappings(
            &FrameworkId::new("nist_csf"),
            &ControlId::new("PR.AC-1"),
        );

        assert!(mappings.is_some());
        assert_eq!(mappings.unwrap().len(), 1);
    }
}
