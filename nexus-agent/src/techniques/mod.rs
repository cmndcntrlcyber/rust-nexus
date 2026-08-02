//! Consolidated ATT&CK technique implementations (v3.8 WS4).
//!
//! Replaces the standalone `nexus-t1059-command-scripting`,
//! `nexus-t1547-boot-logon-autostart`, and `nexus-t1021-006-winrm`
//! crates with in-tree modules under `nexus-agent`.

pub mod t1021_006;
pub mod t1059;
pub mod t1547;

use nexus_common::AttackTechnique;
use std::sync::Arc;

/// Registry of available ATT&CK technique implementations.
///
/// Techniques are registered at startup and looked up by ID
/// during task dispatch.
pub struct TechniqueRegistry {
    techniques: Vec<Arc<dyn AttackTechnique>>,
}

impl TechniqueRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            techniques: Vec::new(),
        }
    }

    /// Register a technique implementation.
    pub fn register(&mut self, technique: Arc<dyn AttackTechnique>) {
        self.techniques.push(technique);
    }

    /// Look up a technique by its MITRE ATT&CK ID (e.g. `"T1059"`).
    pub fn find_by_id(&self, technique_id: &str) -> Option<&Arc<dyn AttackTechnique>> {
        self.techniques
            .iter()
            .find(|t| t.technique_id() == technique_id)
    }

    /// Return all registered techniques.
    pub fn all(&self) -> &[Arc<dyn AttackTechnique>] {
        &self.techniques
    }
}

impl Default for TechniqueRegistry {
    fn default() -> Self {
        Self::new()
    }
}
