# New Gov-Nexus Crates - Baby Steps Implementation Plan

**Document Version:** 1.0
**Created:** 2025-12-21
**Project:** gov-nexus Compliance Platform
**Phase:** 5 - New Crate Development

## Executive Summary

This document provides a detailed, step-by-step implementation plan for creating six new governance and compliance crates for the gov-nexus platform. Each crate adds specific compliance-focused capabilities, building the foundation for automated governance, risk management, and compliance (GRC) operations.

**Principle:** The Baby Steps™ Methodology - Each action must be the smallest possible meaningful change, with validation after every step. The process is the product.

## Table of Contents

1. [Crates Overview](#crates-overview)
2. [Pre-Implementation Planning](#pre-implementation-planning)
3. [Implementation Sequence](#implementation-sequence)
4. [Per-Crate Implementation Steps](#per-crate-implementation-steps)
5. [Validation Procedures](#validation-procedures)
6. [Integration Steps](#integration-steps)
7. [Success Criteria](#success-criteria)

---

## Crates Overview

### 1. gov-evidence
**Purpose:** Evidence collection with chain of custody for audit compliance

**Core Capabilities:**
- Evidence struct with cryptographic hash and digital signature
- Chain of custody tracking with timestamps
- Evidence collector with pluggable storage backends
- Custody entry states: Created, Accessed, Modified, Approved, Exported
- Evidence metadata and tagging
- Evidence search and retrieval
- Evidence integrity verification

**Dependencies:**
- serde, serde_json (serialization)
- uuid (evidence identifiers)
- chrono (timestamps)
- sha2 (cryptographic hashing)
- ed25519-dalek (digital signatures)
- thiserror (error handling)

**Key Types:**
- `Evidence` - Core evidence structure
- `EvidenceCollector` - Main evidence collection interface
- `CustodyEntry` - Chain of custody record
- `CustodyState` - Enum for custody states
- `StorageBackend` - Trait for storage implementations
- `EvidenceMetadata` - Evidence classification and tags

---

### 2. gov-policy
**Purpose:** Policy engine for drift detection and enforcement

**Core Capabilities:**
- Policy definition DSL (Domain-Specific Language)
- Baseline configuration snapshots
- Drift detection by comparing baseline vs current state
- Policy rule evaluation engine
- Remediation suggestion generation
- Policy versioning and history
- Policy templates per framework (NIST, ISO, PCI, etc.)

**Dependencies:**
- serde, serde_json (policy serialization)
- serde_yaml (YAML policy definitions)
- uuid (policy identifiers)
- chrono (timestamps)
- regex (pattern matching in rules)
- thiserror (error handling)

**Key Types:**
- `Policy` - Policy definition
- `PolicyRule` - Individual rule within policy
- `DriftDetector` - Detects configuration drift
- `BaselineSnapshot` - Captured baseline state
- `DriftReport` - Detected drift results
- `PolicyEnforcer` - Enforcement engine
- `RemediationSuggestion` - Auto-generated remediation steps

---

### 3. gov-reporting
**Purpose:** Audit-ready report generation

**Core Capabilities:**
- Report templates per compliance framework
- PDF and DOCX output generation
- Evidence compilation and aggregation
- Control mapping and coverage analysis
- Executive summary generation
- Chart and graph generation for metrics
- Report scheduling and automation

**Dependencies:**
- serde, serde_json (data serialization)
- uuid (report identifiers)
- chrono (timestamps)
- printpdf (PDF generation)
- docx-rs (DOCX generation)
- plotters (chart generation)
- thiserror (error handling)

**Key Types:**
- `ReportTemplate` - Framework-specific templates
- `ReportGenerator` - Main report generation engine
- `EvidenceCompiler` - Aggregates evidence for controls
- `ControlCoverage` - Control implementation status
- `ReportSection` - Individual report sections
- `ReportMetadata` - Report classification and distribution

---

### 4. gov-api
**Purpose:** REST API for integrations and external access

**Core Capabilities:**
- RESTful HTTP server
- Routes for frameworks, controls, assets, evidence, reports
- Authentication and authorization
- API versioning
- OpenAPI/Swagger specification generation
- Request validation and rate limiting
- Audit logging for all API calls

**Dependencies:**
- axum (async HTTP framework)
- tokio (async runtime)
- tower (middleware)
- serde, serde_json (JSON serialization)
- uuid (request IDs)
- jsonwebtoken (JWT authentication)
- utoipa (OpenAPI generation)
- tower-http (CORS, compression, etc.)
- thiserror (error handling)

**Key Types:**
- `ApiServer` - Main API server
- `ApiConfig` - Server configuration
- `ApiRoutes` - Route definitions
- `ApiError` - Standardized error responses
- `ApiResponse<T>` - Standardized success responses
- `AuthContext` - Authentication context
- `RateLimiter` - Rate limiting enforcement

---

### 5. gov-tenancy
**Purpose:** Multi-tenant SaaS support

**Core Capabilities:**
- Tenant isolation and data partitioning
- Tenant context propagation through requests
- Tenant-specific configuration
- Resource quotas and limits per tenant
- Tenant provisioning and deprovisioning
- Cross-tenant data access prevention
- Tenant-aware caching

**Dependencies:**
- serde, serde_json (serialization)
- uuid (tenant identifiers)
- tokio (async runtime)
- async-trait (async traits)
- thiserror (error handling)

**Key Types:**
- `Tenant` - Tenant definition
- `TenantContext` - Request-scoped tenant context
- `TenantResolver` - Resolves tenant from request
- `TenantConfig` - Tenant-specific configuration
- `ResourceQuota` - Tenant resource limits
- `TenantIsolation` - Data isolation enforcement
- `MultiTenantStore<T>` - Tenant-aware data store wrapper

---

### 6. gov-integrations
**Purpose:** Third-party integrations for compliance data collection

**Core Capabilities:**
- SIEM integration (Splunk, Azure Sentinel, Elastic)
- Ticketing system integration (Jira, ServiceNow)
- Cloud provider integration (AWS Config, Azure Policy, GCP SCC)
- Version control integration (GitHub, GitLab)
- CI/CD integration (Jenkins, GitHub Actions)
- Generic webhook support
- Integration health monitoring

**Dependencies:**
- reqwest (HTTP client)
- serde, serde_json (serialization)
- tokio (async runtime)
- async-trait (async traits)
- oauth2 (OAuth authentication)
- thiserror (error handling)

**Key Types:**
- `Integration` - Generic integration interface
- `SiemIntegration` - SIEM-specific integration
- `TicketingIntegration` - Ticketing system integration
- `CloudProviderIntegration` - Cloud provider integration
- `IntegrationConfig` - Integration configuration
- `IntegrationHealth` - Health check results
- `EventCollector` - Collects events from integrations

---

## Pre-Implementation Planning

### Overall Strategy

**Sequence of Implementation:**
1. **gov-evidence** - Foundation for all compliance tracking
2. **gov-policy** - Depends on evidence for validation
3. **gov-reporting** - Depends on evidence and policy
4. **gov-tenancy** - Cross-cutting concern, implement early
5. **gov-api** - Exposes functionality from other crates
6. **gov-integrations** - Connects to external systems

**Why This Order:**
- Evidence is the foundational data structure
- Policy builds on evidence for enforcement
- Reporting consumes evidence and policy data
- Tenancy can be developed in parallel with evidence
- API comes later to expose mature functionality
- Integrations connect the mature platform to external systems

### Directory Structure

Each crate follows this structure:
```
gov-<name>/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          # Public API and module declarations
│   ├── error.rs        # Error types
│   ├── types.rs        # Core types and structs
│   ├── traits.rs       # Trait definitions (if needed)
│   └── [modules]/      # Feature-specific modules
├── tests/
│   └── integration_tests.rs
└── examples/
    └── basic_usage.rs
```

### Common Patterns

**Error Handling:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrateError {
    #[error("Description: {0}")]
    VariantName(String),
}

pub type Result<T> = std::result::Result<T, CrateError>;
```

**Configuration:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateConfig {
    // Configuration fields
}

impl Default for CrateConfig {
    fn default() -> Self {
        Self {
            // Sensible defaults
        }
    }
}
```

**Async Traits:**
```rust
use async_trait::async_trait;

#[async_trait]
pub trait SomeTrait {
    async fn some_method(&self) -> Result<()>;
}
```

---

## Implementation Sequence

### Phase 1: gov-evidence (Steps 1-15)
Foundation crate - implement first

### Phase 2: gov-tenancy (Steps 16-25)
Cross-cutting concern - implement early for architectural integration

### Phase 3: gov-policy (Steps 26-35)
Business logic - depends on evidence

### Phase 4: gov-reporting (Steps 36-45)
Reporting - depends on evidence and policy

### Phase 5: gov-api (Steps 46-55)
API layer - exposes functionality

### Phase 6: gov-integrations (Steps 56-65)
External connectivity - depends on evidence and API

### Phase 7: Integration and Validation (Steps 66-75)
Workspace integration and final validation

---

## Per-Crate Implementation Steps

## Phase 1: gov-evidence (Steps 1-15)

### Step 1: Create gov-evidence Crate Structure
**Objective:** Initialize the crate with basic structure

**Actions:**
```bash
cd /home/cmndcntrl/code/rust-nexus
mkdir -p gov-evidence/src
mkdir -p gov-evidence/tests
mkdir -p gov-evidence/examples
```

**Validation:**
```bash
test -d /home/cmndcntrl/code/rust-nexus/gov-evidence
test -d /home/cmndcntrl/code/rust-nexus/gov-evidence/src
```

**Expected Outcome:** Directory structure created

---

### Step 2: Create gov-evidence Cargo.toml
**Objective:** Define crate metadata and dependencies

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/Cargo.toml`:

```toml
[package]
name = "gov-evidence"
version = "0.1.0"
edition = "2021"
authors = ["Gov-Nexus Team"]
description = "Evidence collection and chain of custody for compliance auditing"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/rust-nexus"
keywords = ["compliance", "evidence", "audit", "governance", "custody"]
categories = ["authentication", "cryptography"]

[dependencies]
# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Identifiers and time
uuid = { workspace = true }
chrono = { workspace = true }

# Cryptography
sha2 = "0.10"
ed25519-dalek = { version = "2.1", features = ["serde"] }
rand = { workspace = true }

# Error handling
thiserror = { workspace = true }

# Async runtime (for future storage backends)
tokio = { workspace = true, optional = true }
async-trait = { version = "0.1", optional = true }

[dev-dependencies]
tempfile = "3.8"

[features]
default = []
async = ["tokio", "async-trait"]
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo check --all-features
```

**Expected Outcome:** Cargo.toml parses correctly

**Commit Point:**
```bash
git add gov-evidence/Cargo.toml
git commit -m "Initialize gov-evidence crate structure

- Created Cargo.toml with dependencies for evidence collection
- Dependencies: serde, uuid, chrono, sha2, ed25519-dalek
- Optional async feature for async storage backends
- Part of new crates creation for gov-nexus"
```

---

### Step 3: Create gov-evidence Error Types
**Objective:** Define error handling for the crate

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/src/error.rs`:

```rust
use thiserror::Error;

/// Errors that can occur in evidence collection and management
#[derive(Error, Debug)]
pub enum EvidenceError {
    #[error("Evidence not found: {0}")]
    NotFound(String),

    #[error("Invalid evidence hash: expected {expected}, got {actual}")]
    InvalidHash { expected: String, actual: String },

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Chain of custody violation: {0}")]
    CustodyViolation(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid evidence state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Evidence already exists: {0}")]
    AlreadyExists(String),
}

/// Result type alias for evidence operations
pub type Result<T> = std::result::Result<T, EvidenceError>;
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo check
```

**Expected Outcome:** Error types compile

**Commit Point:**
```bash
git add gov-evidence/src/error.rs
git commit -m "Add error types for gov-evidence

- Defined EvidenceError enum with comprehensive error variants
- Created Result type alias
- Covers: not found, hash/signature validation, custody violations, storage
- Part of new crates creation for gov-nexus"
```

---

### Step 4: Create gov-evidence Core Types
**Objective:** Define core data structures

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/src/types.rs`:

```rust
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
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
            hash: hash.clone(),
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

    /// Sign evidence with a signing key
    pub fn sign(&mut self, signing_key: &SigningKey) {
        use ed25519_dalek::Signer;

        let message = self.hash.as_bytes();
        let signature = signing_key.sign(message);
        self.signature = Some(base64::encode(signature.to_bytes()));
    }

    /// Verify signature
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> bool {
        use ed25519_dalek::Verifier;

        if let Some(sig_b64) = &self.signature {
            if let Ok(sig_bytes) = base64::decode(sig_b64) {
                if let Ok(signature) = Signature::from_bytes(&sig_bytes.try_into().unwrap_or([0u8; 64])) {
                    let message = self.hash.as_bytes();
                    return verifying_key.verify(message, &signature).is_ok();
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
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo test
```

**Expected Outcome:** All tests pass

**Commit Point:**
```bash
git add gov-evidence/src/types.rs
git commit -m "Add core types for gov-evidence

- Implemented Evidence struct with hash and signature
- Added CustodyEntry and CustodyState for chain of custody
- Implemented EvidenceMetadata for classification and tagging
- Added hash verification and digital signature support
- Includes comprehensive unit tests
- Part of new crates creation for gov-nexus"
```

---

### Step 5: Create gov-evidence Storage Trait
**Objective:** Define storage backend interface

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/src/storage.rs`:

```rust
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
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo test
```

**Expected Outcome:** All tests pass

**Commit Point:**
```bash
git add gov-evidence/src/storage.rs
git commit -m "Add storage backend trait for gov-evidence

- Defined StorageBackend trait for pluggable storage
- Implemented MemoryStorage for testing and development
- Support for search by tags, framework, and control ID
- Includes comprehensive unit tests
- Part of new crates creation for gov-nexus"
```

---

### Step 6: Create gov-evidence Collector
**Objective:** Implement main evidence collection interface

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/src/collector.rs`:

```rust
use crate::error::{EvidenceError, Result};
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
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo test
```

**Expected Outcome:** All tests pass

**Commit Point:**
```bash
git add gov-evidence/src/collector.rs
git commit -m "Add evidence collector for gov-evidence

- Implemented EvidenceCollector with full custody tracking
- Methods: collect, access, modify, approve, export
- Automatic custody entry creation for all operations
- Search capabilities by tags, framework, and control
- Includes comprehensive unit tests
- Part of new crates creation for gov-nexus"
```

---

### Step 7: Create gov-evidence lib.rs
**Objective:** Define public API

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/src/lib.rs`:

```rust
//! # gov-evidence
//!
//! Evidence collection and chain of custody for compliance auditing.
//!
//! This crate provides cryptographically-secured evidence collection with
//! comprehensive chain of custody tracking for compliance and audit purposes.
//!
//! ## Features
//!
//! - Evidence storage with SHA-256 hashing
//! - Digital signatures using Ed25519
//! - Chain of custody with detailed audit trail
//! - Pluggable storage backends
//! - Evidence search and retrieval
//! - Metadata tagging and classification
//!
//! ## Example
//!
//! ```rust
//! use gov_evidence::{EvidenceCollector, EvidenceMetadata, MemoryStorage};
//!
//! let storage = Box::new(MemoryStorage::new());
//! let mut collector = EvidenceCollector::new(storage);
//!
//! let metadata = EvidenceMetadata {
//!     title: "Server Configuration".to_string(),
//!     frameworks: vec!["NIST-800-53".to_string()],
//!     controls: vec!["CM-2".to_string()],
//!     ..Default::default()
//! };
//!
//! let data = serde_json::json!({
//!     "hostname": "web-server-01",
//!     "os": "Ubuntu 22.04",
//!     "patch_level": "2024-01"
//! });
//!
//! let evidence_id = collector.collect(
//!     metadata,
//!     data,
//!     "automation@example.com".to_string()
//! ).unwrap();
//!
//! // Evidence is now stored with chain of custody
//! let evidence = collector.get(&evidence_id).unwrap();
//! assert!(evidence.verify_hash());
//! ```

pub mod collector;
pub mod error;
pub mod storage;
pub mod types;

// Re-export main types
pub use collector::EvidenceCollector;
pub use error::{EvidenceError, Result};
pub use storage::{MemoryStorage, StorageBackend};
pub use types::{CustodyEntry, CustodyState, Evidence, EvidenceMetadata};
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus/gov-evidence
cargo doc --no-deps --open
cargo test
```

**Expected Outcome:** Documentation builds, tests pass

**Commit Point:**
```bash
git add gov-evidence/src/lib.rs
git commit -m "Add public API for gov-evidence

- Created lib.rs with module declarations
- Re-exported main types for easy access
- Added comprehensive crate documentation with example
- Part of new crates creation for gov-nexus"
```

---

### Step 8: Create gov-evidence README
**Objective:** Document the crate

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/README.md`:

```markdown
# gov-evidence

Evidence collection and chain of custody for compliance auditing.

## Overview

`gov-evidence` provides cryptographically-secured evidence collection with comprehensive chain of custody tracking. It's designed for compliance and audit scenarios where evidence integrity and provenance are critical.

## Features

- **Cryptographic Integrity**: SHA-256 hashing and Ed25519 digital signatures
- **Chain of Custody**: Detailed audit trail of all evidence access and modifications
- **Pluggable Storage**: Trait-based storage backends (in-memory, file system, database)
- **Search & Retrieval**: Search by tags, frameworks, and control IDs
- **Metadata Management**: Rich metadata with tagging and classification
- **Custody States**: Created, Accessed, Modified, Approved, Exported

## Quick Start

```rust
use gov_evidence::{EvidenceCollector, EvidenceMetadata, MemoryStorage};

// Create collector with storage backend
let storage = Box::new(MemoryStorage::new());
let mut collector = EvidenceCollector::new(storage);

// Define evidence metadata
let metadata = EvidenceMetadata {
    title: "Firewall Configuration".to_string(),
    frameworks: vec!["NIST-800-53".to_string()],
    controls: vec!["SC-7".to_string()],
    tags: vec!["network".to_string(), "security".to_string()],
    classification: "confidential".to_string(),
    ..Default::default()
};

// Collect evidence
let data = serde_json::json!({
    "rules": ["deny all inbound", "allow established"]
});

let evidence_id = collector.collect(
    metadata,
    data,
    "automation@example.com".to_string()
)?;

// Access evidence (adds custody entry)
let evidence = collector.access(&evidence_id, "auditor@example.com".to_string())?;

// Verify integrity
assert!(evidence.verify_hash());
```

## Chain of Custody

Every interaction with evidence creates a custody entry:

```rust
// Collect evidence
let id = collector.collect(metadata, data, "creator@example.com".to_string())?;
// Custody: [Created]

// Access evidence
collector.access(&id, "viewer@example.com".to_string())?;
// Custody: [Created, Accessed]

// Modify evidence
collector.modify(&id, new_data, "editor@example.com".to_string(), "Updated".to_string())?;
// Custody: [Created, Accessed, Modified]

// Approve evidence
collector.approve(&id, "approver@example.com".to_string(), Some("Verified".to_string()))?;
// Custody: [Created, Accessed, Modified, Approved]

// Export evidence
collector.export(&id, "exporter@example.com".to_string(), "audit-report.pdf".to_string())?;
// Custody: [Created, Accessed, Modified, Approved, Exported]
```

## Storage Backends

Implement the `StorageBackend` trait for custom storage:

```rust
use gov_evidence::{StorageBackend, Evidence, Result};

struct DatabaseStorage {
    // Your database connection
}

impl StorageBackend for DatabaseStorage {
    fn store(&mut self, evidence: &Evidence) -> Result<()> {
        // Store to database
    }

    fn retrieve(&self, id: &Uuid) -> Result<Evidence> {
        // Retrieve from database
    }

    // Implement other trait methods...
}
```

## License

MIT OR Apache-2.0
```

**Validation:**
```bash
test -f /home/cmndcntrl/code/rust-nexus/gov-evidence/README.md
```

**Expected Outcome:** README created

**Commit Point:**
```bash
git add gov-evidence/README.md
git commit -m "Add README for gov-evidence

- Comprehensive documentation with examples
- Chain of custody demonstration
- Custom storage backend guidance
- Part of new crates creation for gov-nexus"
```

---

### Step 9: Add gov-evidence to Workspace
**Objective:** Integrate crate into workspace

**Action:**
Edit `/home/cmndcntrl/code/rust-nexus/Cargo.toml`:

Add to workspace members:
```toml
members = [
    "nexus-agent",
    "nexus-common",
    "nexus-infra",
    "nexus-webui",
    "nexus-recon",
    "nexus-hybrid-exec",
    "gov-evidence",  # Add this line
]
```

Add workspace dependencies:
```toml
[workspace.dependencies]
# ... existing dependencies ...

# Cryptographic hashing
sha2 = "0.10"

# Digital signatures
ed25519-dalek = { version = "2.1", features = ["serde"] }

# Async traits
async-trait = "0.1"

# YAML parsing
serde_yaml = "0.9"

# Regular expressions
regex = "1.10"
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus
cargo check --workspace
cargo test -p gov-evidence
```

**Expected Outcome:** Workspace builds, gov-evidence tests pass

**Commit Point:**
```bash
git add Cargo.toml
git commit -m "Add gov-evidence to workspace

- Added gov-evidence to workspace members
- Added workspace dependencies: sha2, ed25519-dalek, async-trait, serde_yaml, regex
- Workspace builds successfully
- Part of new crates creation for gov-nexus"
```

---

### Step 10: Create gov-evidence Example
**Objective:** Provide working example

**Action:**
Create `/home/cmndcntrl/code/rust-nexus/gov-evidence/examples/basic_usage.rs`:

```rust
//! Basic usage example for gov-evidence

use gov_evidence::{CustodyState, EvidenceCollector, EvidenceMetadata, MemoryStorage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Gov-Evidence Basic Usage Example ===\n");

    // Create evidence collector with in-memory storage
    let storage = Box::new(MemoryStorage::new());
    let mut collector = EvidenceCollector::new(storage);

    // Define evidence metadata
    let metadata = EvidenceMetadata {
        title: "Web Server Configuration Audit".to_string(),
        description: Some("Configuration snapshot of production web server".to_string()),
        frameworks: vec!["NIST-800-53".to_string(), "ISO-27001".to_string()],
        controls: vec!["CM-2".to_string(), "CM-6".to_string()],
        tags: vec![
            "configuration".to_string(),
            "web-server".to_string(),
            "production".to_string(),
        ],
        classification: "confidential".to_string(),
        retention_days: Some(2555), // 7 years
    };

    // Evidence data
    let config_data = serde_json::json!({
        "hostname": "web-prod-01",
        "os": "Ubuntu 22.04 LTS",
        "kernel": "5.15.0-91-generic",
        "services": {
            "nginx": {
                "version": "1.24.0",
                "status": "running",
                "config": "/etc/nginx/nginx.conf"
            },
            "ssh": {
                "version": "OpenSSH 8.9p1",
                "status": "running",
                "port": 22
            }
        },
        "security": {
            "firewall": "ufw enabled",
            "last_patch": "2024-01-15",
            "users": ["admin", "deploy"]
        }
    });

    // Collect evidence
    println!("1. Collecting evidence...");
    let evidence_id = collector.collect(
        metadata,
        config_data,
        "automation-scanner@example.com".to_string(),
    )?;
    println!("   Evidence ID: {}", evidence_id);

    // Access evidence (simulating auditor review)
    println!("\n2. Auditor accessing evidence...");
    let evidence = collector.access(&evidence_id, "auditor@example.com".to_string())?;
    println!("   Title: {}", evidence.metadata.title);
    println!("   Hash: {}", evidence.hash);
    println!("   Custody entries: {}", evidence.chain_of_custody.len());

    // Verify integrity
    println!("\n3. Verifying evidence integrity...");
    let is_valid = collector.verify(&evidence_id)?;
    println!("   Integrity check: {}", if is_valid { "✓ PASS" } else { "✗ FAIL" });

    // Approve evidence
    println!("\n4. Security team approving evidence...");
    collector.approve(
        &evidence_id,
        "security-lead@example.com".to_string(),
        Some("Configuration meets security baseline requirements".to_string()),
    )?;

    // Export evidence for report
    println!("\n5. Exporting evidence for compliance report...");
    let final_evidence = collector.export(
        &evidence_id,
        "compliance-automation@example.com".to_string(),
        "Q1-2024-Compliance-Report.pdf".to_string(),
    )?;

    // Display chain of custody
    println!("\n6. Chain of Custody:");
    for (i, entry) in final_evidence.chain_of_custody.iter().enumerate() {
        println!("   {}. {} - {} by {}",
            i + 1,
            entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            entry.state,
            entry.actor
        );
        if let Some(reason) = &entry.reason {
            println!("      Reason: {}", reason);
        }
    }

    // Search example
    println!("\n7. Searching evidence by framework...");
    let nist_evidence = collector.search_by_framework("NIST-800-53")?;
    println!("   Found {} evidence items for NIST-800-53", nist_evidence.len());

    println!("\n8. Searching evidence by control...");
    let cm2_evidence = collector.search_by_control("CM-2")?;
    println!("   Found {} evidence items for control CM-2", cm2_evidence.len());

    println!("\n=== Example Complete ===");
    Ok(())
}
```

**Validation:**
```bash
cd /home/cmndcntrl/code/rust-nexus
cargo run --example basic_usage --package gov-evidence
```

**Expected Outcome:** Example runs successfully

**Commit Point:**
```bash
git add gov-evidence/examples/basic_usage.rs
git commit -m "Add basic usage example for gov-evidence

- Comprehensive example demonstrating all key features
- Shows evidence collection, access, approval, and export
- Displays chain of custody tracking
- Demonstrates search capabilities
- Part of new crates creation for gov-nexus"
```

---

### Steps 11-15: Gov-Evidence Polish and Documentation

Due to space constraints, I'll provide a summary:

**Step 11:** Add integration tests
**Step 12:** Add digital signature example
**Step 13:** Add file system storage backend (optional feature)
**Step 14:** Benchmark tests for performance validation
**Step 15:** Final validation and documentation polish

**Summary Commit:**
```bash
git add gov-evidence/
git commit -m "Complete gov-evidence crate implementation

- Added integration tests for full workflow
- Added digital signature examples
- Optional file system storage backend
- Performance benchmarks
- Complete documentation
- Ready for use in gov-nexus platform
- Part of new crates creation for gov-nexus"
```

---

## Phases 2-6: Remaining Crates

Due to the length of this document, I'll provide the high-level structure for the remaining crates. Each would follow the same baby-steps pattern:

### Phase 2: gov-tenancy (Steps 16-25)
- Step 16-20: Core types, traits, and tenant resolver
- Step 21-25: Multi-tenant storage wrappers and middleware

### Phase 3: gov-policy (Steps 26-35)
- Step 26-30: Policy DSL, rule engine, and drift detector
- Step 31-35: Policy templates and remediation engine

### Phase 4: gov-reporting (Steps 36-45)
- Step 36-40: Report templates and generation engine
- Step 41-45: PDF/DOCX output and chart generation

### Phase 5: gov-api (Steps 46-55)
- Step 46-50: Axum server setup and route definitions
- Step 51-55: OpenAPI generation and middleware

### Phase 6: gov-integrations (Steps 56-65)
- Step 56-60: Integration trait and SIEM connectors
- Step 61-65: Cloud provider and ticketing integrations

### Phase 7: Final Integration (Steps 66-75)
- Step 66-70: Workspace integration and dependency resolution
- Step 71-75: End-to-end testing and documentation

---

## Validation Procedures

### After Each Step
1. **Compile check:** `cargo check -p <crate-name>`
2. **Run tests:** `cargo test -p <crate-name>`
3. **Review diff:** `git diff`
4. **Commit atomically:** Clear, focused commit message

### After Each Phase
1. **Full crate build:** `cargo build -p <crate-name> --all-features`
2. **Documentation:** `cargo doc -p <crate-name> --no-deps`
3. **Examples:** `cargo run --example <example> -p <crate-name>`
4. **Integration tests:** `cargo test -p <crate-name> --test '*'`

### Final Validation
1. **Workspace build:** `cargo build --workspace --all-features`
2. **All tests:** `cargo test --workspace`
3. **Clippy:** `cargo clippy --workspace -- -D warnings`
4. **Format:** `cargo fmt --all -- --check`
5. **Documentation:** `cargo doc --workspace --no-deps`

---

## Integration Steps

### Step 66: Update Root Cargo.toml
Add all new crates to workspace members

### Step 67: Cross-Crate Dependencies
Wire up dependencies between crates (e.g., gov-api depends on gov-evidence)

### Step 68: Create End-to-End Example
Example using multiple crates together

### Step 69: Update Main Documentation
Update root README.md with new crates

### Step 70: Performance Testing
Benchmark key operations across crates

---

## Success Criteria

### Per-Crate Completion
- [ ] Cargo.toml with all dependencies
- [ ] Core types and traits defined
- [ ] Error handling implemented
- [ ] Primary functionality implemented
- [ ] Unit tests passing (>80% coverage)
- [ ] Integration tests passing
- [ ] Documentation complete (rustdoc)
- [ ] README with examples
- [ ] At least one working example
- [ ] Integrated into workspace

### Phase 7 Completion
- [ ] All 6 crates implemented
- [ ] Workspace builds cleanly
- [ ] All tests pass
- [ ] Documentation complete
- [ ] Examples demonstrate key features
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Ready for production use

---

## Timeline Estimate

| Phase | Crate | Steps | Estimated Time |
|-------|-------|-------|----------------|
| 1 | gov-evidence | 1-15 | 8-12 hours |
| 2 | gov-tenancy | 16-25 | 6-8 hours |
| 3 | gov-policy | 26-35 | 10-14 hours |
| 4 | gov-reporting | 36-45 | 12-16 hours |
| 5 | gov-api | 46-55 | 10-14 hours |
| 6 | gov-integrations | 56-65 | 14-18 hours |
| 7 | Integration | 66-75 | 8-10 hours |
| **Total** | **6 crates** | **75 steps** | **68-92 hours** |

**Realistic estimate:** 10-12 working days with careful baby-steps execution

---

## Appendix A: Quick Reference Commands

```bash
# Create new crate
mkdir -p gov-<name>/src gov-<name>/tests gov-<name>/examples

# Check single crate
cargo check -p gov-<name>

# Test single crate
cargo test -p gov-<name>

# Run example
cargo run --example <example> -p gov-<name>

# Build documentation
cargo doc -p gov-<name> --no-deps --open

# Check workspace
cargo check --workspace

# Test workspace
cargo test --workspace

# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace -- -D warnings
```

---

## Appendix B: Dependency Matrix

| Crate | Depends On |
|-------|------------|
| gov-evidence | (none) |
| gov-tenancy | (none) |
| gov-policy | gov-evidence |
| gov-reporting | gov-evidence, gov-policy |
| gov-api | gov-evidence, gov-policy, gov-reporting, gov-tenancy |
| gov-integrations | gov-evidence, gov-api |

---

## Document Maintenance

This implementation plan is a living document.

**Update when:**
- New dependencies discovered
- Implementation challenges arise
- Timeline estimates prove inaccurate
- Architecture decisions change

**Version History:**
- v1.0 (2025-12-21): Initial comprehensive implementation plan created

---

**End of Implementation Plan**

**Status:** Ready for execution
**Next Step:** Begin Phase 1, Step 1 - Create gov-evidence crate structure
