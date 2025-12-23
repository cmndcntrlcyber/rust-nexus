use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// State of evidence in the chain of custody
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustodyState {
    /// Evidence was created
    Created,
    /// Evidence was accessed/viewed
    Accessed,
    /// Evidence was modified
    Modified,
    /// Evidence was approved by authority
    Approved,
    /// Evidence was exported/shared
    Exported,
}

impl std::fmt::Display for CustodyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustodyState::Created => write!(f, "Created"),
            CustodyState::Accessed => write!(f, "Accessed"),
            CustodyState::Modified => write!(f, "Modified"),
            CustodyState::Approved => write!(f, "Approved"),
            CustodyState::Exported => write!(f, "Exported"),
        }
    }
}

/// Entry in the chain of custody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEntry {
    /// Unique identifier for this custody entry
    pub id: Uuid,
    /// When this custody event occurred
    pub timestamp: DateTime<Utc>,
    /// Who performed this action (user ID, service account, etc.)
    pub actor: String,
    /// What state change occurred
    pub state: CustodyState,
    /// Optional reason or description
    pub reason: Option<String>,
    /// Hash of the evidence at this point in time
    pub evidence_hash: String,
}

impl CustodyEntry {
    /// Create a new custody entry
    pub fn new(actor: String, state: CustodyState, evidence_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor,
            state,
            reason: None,
            evidence_hash,
        }
    }

    /// Create a custody entry with a reason
    pub fn with_reason(
        actor: String,
        state: CustodyState,
        evidence_hash: String,
        reason: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor,
            state,
            reason: Some(reason),
            evidence_hash,
        }
    }
}

/// Metadata for evidence classification and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMetadata {
    /// Evidence title/name
    pub title: String,
    /// Detailed description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Compliance framework(s) this evidence supports
    pub frameworks: Vec<String>,
    /// Control ID(s) this evidence satisfies
    pub controls: Vec<String>,
    /// Classification level (e.g., "public", "internal", "confidential")
    pub classification: String,
    /// Retention period in days (None = indefinite)
    pub retention_days: Option<u32>,
}

impl Default for EvidenceMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: None,
            tags: Vec::new(),
            frameworks: Vec::new(),
            controls: Vec::new(),
            classification: "internal".to_string(),
            retention_days: Some(2555), // ~7 years default retention
        }
    }
}

/// Core evidence structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique evidence identifier
    pub id: Uuid,
    /// When evidence was created
    pub created_at: DateTime<Utc>,
    /// Evidence metadata
    pub metadata: EvidenceMetadata,
    /// Actual evidence data (JSON-serializable)
    pub data: serde_json::Value,
    /// SHA-256 hash of the evidence data
    pub hash: String,
    /// Digital signature (base64-encoded)
    pub signature: Option<String>,
    /// Chain of custody records
    pub chain_of_custody: Vec<CustodyEntry>,
}

impl Evidence {
    /// Create new evidence with data
    pub fn new(metadata: EvidenceMetadata, data: serde_json::Value) -> Self {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let hash = Self::compute_hash(&data);

        Self {
            id,
            created_at,
            metadata,
            data,
            hash,
            signature: None,
            chain_of_custody: Vec::new(),
        }
    }

    /// Compute SHA-256 hash of data
    pub fn compute_hash(data: &serde_json::Value) -> String {
        let json_str = serde_json::to_string(data).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify hash integrity
    pub fn verify_hash(&self) -> bool {
        let computed = Self::compute_hash(&self.data);
        computed == self.hash
    }

    /// Sign evidence with a signing key (Ed25519)
    #[cfg(feature = "signing")]
    pub fn sign(&mut self, signing_key: &ed25519_dalek::SigningKey) {
        use ed25519_dalek::Signer;

        let message = self.hash.as_bytes();
        let signature = signing_key.sign(message);
        self.signature = Some(base64::encode(signature.to_bytes()));
    }

    /// Verify signature
    #[cfg(feature = "signing")]
    pub fn verify_signature(&self, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
        use ed25519_dalek::{Signature, Verifier};

        if let Some(sig_b64) = &self.signature {
            if let Ok(sig_bytes) = base64::decode(sig_b64) {
                if let Ok(sig_array) = sig_bytes.try_into() {
                    if let Ok(signature) = Signature::from_bytes(&sig_array) {
                        let message = self.hash.as_bytes();
                        return verifying_key.verify(message, &signature).is_ok();
                    }
                }
            }
        }
        false
    }

    /// Add custody entry to chain
    pub fn add_custody_entry(&mut self, entry: CustodyEntry) {
        self.chain_of_custody.push(entry);
    }

    /// Get latest custody state
    pub fn current_state(&self) -> Option<CustodyState> {
        self.chain_of_custody.last().map(|e| e.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_creation() {
        let metadata = EvidenceMetadata {
            title: "Test Evidence".to_string(),
            ..Default::default()
        };
        let data = serde_json::json!({"test": "data"});
        let evidence = Evidence::new(metadata, data);

        assert!(!evidence.id.is_nil());
        assert!(evidence.verify_hash());
        assert_eq!(evidence.metadata.title, "Test Evidence");
    }

    #[test]
    fn test_hash_verification() {
        let metadata = EvidenceMetadata::default();
        let data = serde_json::json!({"test": "data"});
        let mut evidence = Evidence::new(metadata, data);

        // Original hash should verify
        assert!(evidence.verify_hash());

        // Modify data
        evidence.data = serde_json::json!({"test": "modified"});

        // Hash should no longer match
        assert!(!evidence.verify_hash());
    }

    #[test]
    fn test_custody_chain() {
        let metadata = EvidenceMetadata::default();
        let data = serde_json::json!({"test": "data"});
        let mut evidence = Evidence::new(metadata, data);

        let entry = CustodyEntry::new(
            "user@example.com".to_string(),
            CustodyState::Created,
            evidence.hash.clone(),
        );
        evidence.add_custody_entry(entry);

        assert_eq!(evidence.chain_of_custody.len(), 1);
        assert_eq!(evidence.current_state(), Some(CustodyState::Created));
    }

    #[test]
    fn test_custody_state_display() {
        assert_eq!(CustodyState::Created.to_string(), "Created");
        assert_eq!(CustodyState::Approved.to_string(), "Approved");
    }
}
