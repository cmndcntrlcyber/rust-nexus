use crate::error::Result;
use crate::types::Evidence;
use uuid::Uuid;

/// Trait for evidence storage backends
pub trait StorageBackend: Send + Sync {
    /// Store evidence
    fn store(&mut self, evidence: &Evidence) -> Result<()>;

    /// Retrieve evidence by ID
    fn retrieve(&self, id: &Uuid) -> Result<Evidence>;

    /// Update existing evidence
    fn update(&mut self, evidence: &Evidence) -> Result<()>;

    /// Delete evidence (if permitted by retention policy)
    fn delete(&mut self, id: &Uuid) -> Result<()>;

    /// List all evidence IDs (for iteration)
    fn list_ids(&self) -> Result<Vec<Uuid>>;

    /// Search evidence by metadata tags
    fn search_by_tags(&self, tags: &[String]) -> Result<Vec<Evidence>>;

    /// Search evidence by framework
    fn search_by_framework(&self, framework: &str) -> Result<Vec<Evidence>>;

    /// Search evidence by control ID
    fn search_by_control(&self, control_id: &str) -> Result<Vec<Evidence>>;
}

/// In-memory storage backend (for testing and development)
#[derive(Debug, Default)]
pub struct MemoryStorage {
    evidence: std::collections::HashMap<Uuid, Evidence>,
}

impl MemoryStorage {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        Self {
            evidence: std::collections::HashMap::new(),
        }
    }
}

impl StorageBackend for MemoryStorage {
    fn store(&mut self, evidence: &Evidence) -> Result<()> {
        if self.evidence.contains_key(&evidence.id) {
            return Err(crate::error::EvidenceError::AlreadyExists(
                evidence.id.to_string(),
            ));
        }
        self.evidence.insert(evidence.id, evidence.clone());
        Ok(())
    }

    fn retrieve(&self, id: &Uuid) -> Result<Evidence> {
        self.evidence
            .get(id)
            .cloned()
            .ok_or_else(|| crate::error::EvidenceError::NotFound(id.to_string()))
    }

    fn update(&mut self, evidence: &Evidence) -> Result<()> {
        if !self.evidence.contains_key(&evidence.id) {
            return Err(crate::error::EvidenceError::NotFound(
                evidence.id.to_string(),
            ));
        }
        self.evidence.insert(evidence.id, evidence.clone());
        Ok(())
    }

    fn delete(&mut self, id: &Uuid) -> Result<()> {
        self.evidence
            .remove(id)
            .ok_or_else(|| crate::error::EvidenceError::NotFound(id.to_string()))?;
        Ok(())
    }

    fn list_ids(&self) -> Result<Vec<Uuid>> {
        Ok(self.evidence.keys().copied().collect())
    }

    fn search_by_tags(&self, tags: &[String]) -> Result<Vec<Evidence>> {
        Ok(self
            .evidence
            .values()
            .filter(|e| tags.iter().any(|tag| e.metadata.tags.contains(tag)))
            .cloned()
            .collect())
    }

    fn search_by_framework(&self, framework: &str) -> Result<Vec<Evidence>> {
        Ok(self
            .evidence
            .values()
            .filter(|e| e.metadata.frameworks.contains(&framework.to_string()))
            .cloned()
            .collect())
    }

    fn search_by_control(&self, control_id: &str) -> Result<Vec<Evidence>> {
        Ok(self
            .evidence
            .values()
            .filter(|e| e.metadata.controls.contains(&control_id.to_string()))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EvidenceMetadata;

    #[test]
    fn test_memory_storage_store_retrieve() {
        let mut storage = MemoryStorage::new();
        let evidence = Evidence::new(
            EvidenceMetadata::default(),
            serde_json::json!({"test": "data"}),
        );
        let id = evidence.id;

        storage.store(&evidence).unwrap();
        let retrieved = storage.retrieve(&id).unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[test]
    fn test_memory_storage_search_by_tags() {
        let mut storage = MemoryStorage::new();
        let mut metadata = EvidenceMetadata::default();
        metadata.tags = vec!["test".to_string(), "demo".to_string()];

        let evidence = Evidence::new(metadata, serde_json::json!({"test": "data"}));
        storage.store(&evidence).unwrap();

        let results = storage.search_by_tags(&["test".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_memory_storage_duplicate_store() {
        let mut storage = MemoryStorage::new();
        let evidence = Evidence::new(
            EvidenceMetadata::default(),
            serde_json::json!({"test": "data"}),
        );

        storage.store(&evidence).unwrap();
        let result = storage.store(&evidence);
        assert!(result.is_err());
    }
}
