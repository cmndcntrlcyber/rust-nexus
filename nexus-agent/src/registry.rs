use nexus_common::{
    AttackTechnique, ExecutionContext, NexusError, Result, TechniqueParams, TechniqueResult,
};
use std::collections::HashMap;

/// Registry of ATT&CK techniques, populated from in-tree technique modules.
///
/// Replaces the monolithic match block in execution.rs with trait-object dispatch.
/// v3.8 WS4: techniques are now always built-in (consolidated from standalone crates).
pub struct TechniqueRegistry {
    /// Maps task_type string -> technique implementation
    techniques: HashMap<String, Box<dyn AttackTechnique>>,
}

impl TechniqueRegistry {
    /// Build the registry from all in-tree technique modules.
    ///
    /// Each technique module's `register()` returns one Box per technique.
    /// Each technique maps to its primary task_type for dispatch.
    pub fn build() -> Self {
        let mut techniques: HashMap<String, Box<dyn AttackTechnique>> = HashMap::new();

        // T1059 - Command and Scripting Interpreter (always available)
        for tech in nexus_agent::techniques::t1059::register() {
            let task_types = tech.task_types();
            if let Some(primary) = task_types.into_iter().next() {
                techniques.insert(primary, tech);
            }
        }

        // T1547 - Boot/Logon Autostart (platform-gated internally)
        for tech in nexus_agent::techniques::t1547::register() {
            let task_types = tech.task_types();
            if let Some(primary) = task_types.into_iter().next() {
                techniques.insert(primary, tech);
            }
        }

        // T1021.006 - WinRM (platform-gated internally)
        for tech in nexus_agent::techniques::t1021_006::register() {
            let task_types = tech.task_types();
            if let Some(primary) = task_types.into_iter().next() {
                techniques.insert(primary, tech);
            }
        }

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
        // T1059 shell interpreter is always registered (cross-platform)
        assert!(registry.has_technique("shell"));
        assert!(!registry.is_empty());

        // On Linux, systemd persistence should also be registered
        #[cfg(target_os = "linux")]
        assert!(registry.has_technique("systemd_persistence"));
    }
}
