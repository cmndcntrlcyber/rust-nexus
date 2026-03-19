use nexus_common::{
    AttackTechnique, ExecutionContext, NexusError, Result, TechniqueParams, TechniqueResult,
};
use std::collections::HashMap;

/// Registry of ATT&CK techniques, populated at build time via feature flags.
///
/// Replaces the monolithic match block in execution.rs with trait-object dispatch.
pub struct TechniqueRegistry {
    /// Maps task_type string -> technique implementation
    techniques: HashMap<String, Box<dyn AttackTechnique>>,
}

impl TechniqueRegistry {
    /// Build the registry from all feature-gated technique crates.
    ///
    /// Each technique crate's `register()` returns one Box per technique.
    /// Each technique maps to its primary task_type for dispatch.
    pub fn build() -> Self {
        let mut techniques: HashMap<String, Box<dyn AttackTechnique>> = HashMap::new();

        #[cfg(feature = "t1059")]
        {
            for tech in nexus_t1059_command_scripting::register() {
                // Each technique handles one primary task_type
                let task_types = tech.task_types();
                if let Some(primary) = task_types.into_iter().next() {
                    techniques.insert(primary, tech);
                }
            }
        }

        #[cfg(feature = "t1547")]
        {
            for tech in nexus_t1547_boot_logon_autostart::register() {
                let task_types = tech.task_types();
                if let Some(primary) = task_types.into_iter().next() {
                    techniques.insert(primary, tech);
                }
            }
        }

        #[cfg(feature = "t1021-006")]
        {
            for tech in nexus_t1021_006_winrm::register() {
                let task_types = tech.task_types();
                if let Some(primary) = task_types.into_iter().next() {
                    techniques.insert(primary, tech);
                }
            }
        }

        // Future technique crates register here:
        // #[cfg(feature = "t1055")]
        // for tech in nexus_t1055_process_injection::register() { ... }

        Self { techniques }
    }

    /// Check if a task type is handled by a registered technique
    pub fn has_technique(&self, task_type: &str) -> bool {
        self.techniques.contains_key(task_type)
    }

    /// Dispatch a task to the appropriate technique
    pub async fn dispatch(
        &self,
        ctx: &ExecutionContext,
        task_type: &str,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        let technique = self
            .techniques
            .get(task_type)
            .ok_or_else(|| NexusError::UnknownTechnique(task_type.to_string()))?;

        technique.validate(&params)?;
        technique.execute(ctx, params).await
    }

    /// Get all capabilities from registered techniques
    pub fn capabilities(&self) -> Vec<String> {
        let mut caps: Vec<String> = self
            .techniques
            .values()
            .flat_map(|t| t.capabilities())
            .collect();
        caps.sort();
        caps.dedup();
        caps
    }

    /// Get metadata about all registered techniques
    pub fn list_techniques(&self) -> Vec<(&str, &str)> {
        self.techniques
            .values()
            .map(|t| (t.technique_id(), t.name()))
            .collect()
    }

    /// Number of registered task type handlers
    pub fn len(&self) -> usize {
        self.techniques.len()
    }

    pub fn is_empty(&self) -> bool {
        self.techniques.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_build() {
        let registry = TechniqueRegistry::build();
        #[cfg(feature = "t1059")]
        {
            assert!(registry.has_technique("shell"));
            assert!(!registry.is_empty());
        }
        #[cfg(not(feature = "t1059"))]
        {
            assert!(registry.is_empty());
        }
    }
}
