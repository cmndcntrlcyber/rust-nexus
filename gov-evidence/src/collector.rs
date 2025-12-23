use crate::error::Result;
use crate::storage::StorageBackend;
use crate::types::{CustodyEntry, CustodyState, Evidence, EvidenceMetadata};
use uuid::Uuid;

/// Main evidence collector interface
pub struct EvidenceCollector {
    storage: Box<dyn StorageBackend>,
}

impl EvidenceCollector {
    /// Create a new evidence collector with a storage backend
    pub fn new(storage: Box<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Collect new evidence
    pub fn collect(
        &mut self,
        metadata: EvidenceMetadata,
        data: serde_json::Value,
        actor: String,
    ) -> Result<Uuid> {
        let mut evidence = Evidence::new(metadata, data);

        // Add creation custody entry
        let custody_entry = CustodyEntry::new(
            actor,
            CustodyState::Created,
            evidence.hash.clone(),
        );
        evidence.add_custody_entry(custody_entry);

        let id = evidence.id;
        self.storage.store(&evidence)?;
        Ok(id)
    }

    /// Access evidence (adds Accessed custody entry)
    pub fn access(&mut self, id: &Uuid, actor: String) -> Result<Evidence> {
        let mut evidence = self.storage.retrieve(id)?;

        let custody_entry = CustodyEntry::new(
            actor,
            CustodyState::Accessed,
            evidence.hash.clone(),
        );
        evidence.add_custody_entry(custody_entry);

        self.storage.update(&evidence)?;
        Ok(evidence)
    }

    /// Modify evidence (adds Modified custody entry)
    pub fn modify(
        &mut self,
        id: &Uuid,
        data: serde_json::Value,
        actor: String,
        reason: String,
    ) -> Result<()> {
        let mut evidence = self.storage.retrieve(id)?;

        // Update data and recompute hash
        evidence.data = data;
        evidence.hash = Evidence::compute_hash(&evidence.data);

        let custody_entry = CustodyEntry::with_reason(
            actor,
            CustodyState::Modified,
            evidence.hash.clone(),
            reason,
        );
        evidence.add_custody_entry(custody_entry);

        self.storage.update(&evidence)?;
        Ok(())
    }

    /// Approve evidence (adds Approved custody entry)
    pub fn approve(&mut self, id: &Uuid, actor: String, reason: Option<String>) -> Result<()> {
        let mut evidence = self.storage.retrieve(id)?;

        let custody_entry = if let Some(r) = reason {
            CustodyEntry::with_reason(
                actor,
                CustodyState::Approved,
                evidence.hash.clone(),
                r,
            )
        } else {
            CustodyEntry::new(actor, CustodyState::Approved, evidence.hash.clone())
        };

        evidence.add_custody_entry(custody_entry);
        self.storage.update(&evidence)?;
        Ok(())
    }

    /// Export evidence (adds Exported custody entry)
    pub fn export(&mut self, id: &Uuid, actor: String, destination: String) -> Result<Evidence> {
        let mut evidence = self.storage.retrieve(id)?;

        let custody_entry = CustodyEntry::with_reason(
            actor,
            CustodyState::Exported,
            evidence.hash.clone(),
            format!("Exported to: {}", destination),
        );
        evidence.add_custody_entry(custody_entry);

        self.storage.update(&evidence)?;
        Ok(evidence)
    }

    /// Verify evidence integrity
    pub fn verify(&self, id: &Uuid) -> Result<bool> {
        let evidence = self.storage.retrieve(id)?;
        Ok(evidence.verify_hash())
    }

    /// Get evidence by ID without adding custody entry
    pub fn get(&self, id: &Uuid) -> Result<Evidence> {
        self.storage.retrieve(id)
    }

    /// Search evidence by tags
    pub fn search_by_tags(&self, tags: &[String]) -> Result<Vec<Evidence>> {
        self.storage.search_by_tags(tags)
    }

    /// Search evidence by framework
    pub fn search_by_framework(&self, framework: &str) -> Result<Vec<Evidence>> {
        self.storage.search_by_framework(framework)
    }

    /// Search evidence by control
    pub fn search_by_control(&self, control_id: &str) -> Result<Vec<Evidence>> {
        self.storage.search_by_control(control_id)
    }

    /// List all evidence
    pub fn list_all(&self) -> Result<Vec<Evidence>> {
        let ids = self.storage.list_ids()?;
        ids.iter()
            .map(|id| self.storage.retrieve(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn test_collect_evidence() {
        let storage = Box::new(MemoryStorage::new());
        let mut collector = EvidenceCollector::new(storage);

        let metadata = EvidenceMetadata {
            title: "Test Evidence".to_string(),
            ..Default::default()
        };
        let data = serde_json::json!({"test": "data"});

        let id = collector.collect(metadata, data, "test@example.com".to_string()).unwrap();
        let evidence = collector.get(&id).unwrap();

        assert_eq!(evidence.chain_of_custody.len(), 1);
        assert_eq!(evidence.current_state(), Some(CustodyState::Created));
    }

    #[test]
    fn test_access_evidence() {
        let storage = Box::new(MemoryStorage::new());
        let mut collector = EvidenceCollector::new(storage);

        let id = collector
            .collect(
                EvidenceMetadata::default(),
                serde_json::json!({"test": "data"}),
                "creator@example.com".to_string(),
            )
            .unwrap();

        let evidence = collector.access(&id, "accessor@example.com".to_string()).unwrap();
        assert_eq!(evidence.chain_of_custody.len(), 2);
        assert_eq!(evidence.current_state(), Some(CustodyState::Accessed));
    }

    #[test]
    fn test_modify_evidence() {
        let storage = Box::new(MemoryStorage::new());
        let mut collector = EvidenceCollector::new(storage);

        let id = collector
            .collect(
                EvidenceMetadata::default(),
                serde_json::json!({"test": "data"}),
                "creator@example.com".to_string(),
            )
            .unwrap();

        collector
            .modify(
                &id,
                serde_json::json!({"test": "modified"}),
                "modifier@example.com".to_string(),
                "Updated data".to_string(),
            )
            .unwrap();

        let evidence = collector.get(&id).unwrap();
        assert_eq!(evidence.chain_of_custody.len(), 2);
        assert_eq!(evidence.current_state(), Some(CustodyState::Modified));
        assert_eq!(evidence.data["test"], "modified");
    }

    #[test]
    fn test_approve_evidence() {
        let storage = Box::new(MemoryStorage::new());
        let mut collector = EvidenceCollector::new(storage);

        let id = collector
            .collect(
                EvidenceMetadata::default(),
                serde_json::json!({"test": "data"}),
                "creator@example.com".to_string(),
            )
            .unwrap();

        collector
            .approve(&id, "approver@example.com".to_string(), Some("Looks good".to_string()))
            .unwrap();

        let evidence = collector.get(&id).unwrap();
        assert_eq!(evidence.current_state(), Some(CustodyState::Approved));
    }

    #[test]
    fn test_verify_integrity() {
        let storage = Box::new(MemoryStorage::new());
        let mut collector = EvidenceCollector::new(storage);

        let id = collector
            .collect(
                EvidenceMetadata::default(),
                serde_json::json!({"test": "data"}),
                "creator@example.com".to_string(),
            )
            .unwrap();

        assert!(collector.verify(&id).unwrap());
    }
}
