# Dashboard Transformation Plan: nexus-webui to gov-dashboard

**Document Version:** 1.0
**Created:** 2025-12-21
**Status:** Planning Phase
**Methodology:** Baby Steps Implementation

---

## Executive Summary

Transform the existing C2 operator dashboard (`nexus-webui`) into a compliance monitoring dashboard (`gov-dashboard`) that provides real-time visibility into compliance status across 20+ regulatory frameworks.

**Core Principle:** Each step must be the smallest possible meaningful change. The process is the product.

---

## Current State Analysis

### Existing Infrastructure
**File:** `/home/cmndcntrl/code/rust-nexus/nexus-webui/src/lib.rs`

**Current Capabilities:**
- Warp-based HTTP server (lines 1-329)
- WebSocket support for real-time updates (lines 226-234)
- Agent connection management (lines 59-88)
- Task execution interface (lines 279-296)
- CORS-enabled REST API (lines 143-224)
- Broadcast event system (lines 89-99, 244-248)

**Existing Routes:**
```
GET  /health
GET  /api/agents
GET  /api/agents/:id
POST /api/agents/:id/tasks
GET  /api/domains
POST /api/domains/rotate
GET  /api/system
WS   /ws
```

**Existing State Structure:**
```rust
pub struct WebUIState {
    pub config: WebUIConfig,
    pub grpc_client: Arc<GrpcClient>,
    pub domain_manager: Arc<DomainManager>,
    pub agent_connections: Arc<RwLock<HashMap<String, AgentConnection>>>,
    pub broadcast_tx: broadcast::Sender<WebUIEvent>,
}
```

---

## Target State Definition

### New Dashboard Identity: gov-dashboard

**Purpose:** Compliance monitoring and audit reporting across multiple regulatory frameworks.

**Target Routes (New):**
```
GET  /api/frameworks                    # List all 20 frameworks with compliance scores
GET  /api/frameworks/:id                # Framework details (name, version, control count)
GET  /api/frameworks/:id/controls       # Controls for framework with status
GET  /api/frameworks/:id/compliance-score  # Current compliance percentage
GET  /api/controls/:id                  # Control details (requirement, status, owner)
GET  /api/controls/:id/evidence         # Evidence for control with chain of custody
GET  /api/assets                        # Asset inventory with compliance status
GET  /api/assets/:id                    # Asset details (type, owner, location)
GET  /api/assets/:id/compliance         # Asset compliance status across frameworks
POST /api/reports/generate              # Generate audit report (async job)
GET  /api/reports/:id                   # Retrieve generated report
GET  /api/reports/:id/download          # Download report PDF/JSON
WS   /ws/compliance-stream              # Real-time compliance score updates
WS   /ws/drift-alerts                   # Configuration drift alerts
```

**Target State Structure:**
```rust
pub struct DashboardState {
    pub config: DashboardConfig,
    pub frameworks: Arc<RwLock<HashMap<String, Framework>>>,
    pub controls: Arc<RwLock<HashMap<String, Control>>>,
    pub assets: Arc<RwLock<HashMap<String, Asset>>>,
    pub evidence: Arc<RwLock<HashMap<String, Evidence>>>,
    pub compliance_scores: Arc<RwLock<HashMap<String, ComplianceScore>>>,
    pub broadcast_tx: broadcast::Sender<DashboardEvent>,
    pub report_jobs: Arc<RwLock<HashMap<String, ReportJob>>>,
}
```

---

## Data Model Transformation

### Phase 1: Core Data Structures

**Baby Step 1.1:** Create `Framework` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    pub id: String,
    pub name: String,              // "NIST 800-53", "SOC 2", etc.
    pub version: String,           // "Rev 5", "2017", etc.
    pub description: String,
    pub control_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Baby Step 1.2:** Create `Control` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    pub id: String,
    pub framework_id: String,
    pub control_number: String,    // "AC-1", "CC6.1", etc.
    pub title: String,
    pub requirement: String,       // Full text of requirement
    pub status: ControlStatus,
    pub owner: String,             // Person/team responsible
    pub last_assessed: Option<DateTime<Utc>>,
    pub next_assessment: Option<DateTime<Utc>>,
    pub evidence_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlStatus {
    Pass,
    Fail,
    Pending,
    NotApplicable,
}
```

**Baby Step 1.3:** Create `Asset` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub description: String,
    pub owner: String,
    pub location: String,          // Physical/cloud location
    pub environment: Environment,
    pub criticality: Criticality,
    pub compliance_status: HashMap<String, bool>, // framework_id -> compliant
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Server,
    Database,
    Application,
    Network,
    Storage,
    Endpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Environment {
    Production,
    Staging,
    Development,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Criticality {
    Critical,
    High,
    Medium,
    Low,
}
```

**Baby Step 1.4:** Create `Evidence` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub control_id: String,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub collected_by: String,
    pub collected_at: DateTime<Utc>,
    pub chain_of_custody: Vec<CustodyEvent>,
    pub hash: String,              // SHA-256 of evidence file
    pub status: EvidenceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    Screenshot,
    LogFile,
    ConfigFile,
    ScanReport,
    PolicyDocument,
    Attestation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEvent {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,            // "collected", "reviewed", "approved"
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceStatus {
    Collected,
    UnderReview,
    Approved,
    Rejected,
}
```

**Baby Step 1.5:** Create `ComplianceScore` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    pub framework_id: String,
    pub total_controls: usize,
    pub passing_controls: usize,
    pub failing_controls: usize,
    pub pending_controls: usize,
    pub na_controls: usize,
    pub percentage: f64,           // (passing / (total - na)) * 100
    pub last_calculated: DateTime<Utc>,
    pub trend: ScoreTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoreTrend {
    Improving,
    Declining,
    Stable,
}
```

**Baby Step 1.6:** Create `ReportJob` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportJob {
    pub id: String,
    pub report_type: ReportType,
    pub framework_ids: Vec<String>,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub status: ReportStatus,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportType {
    FullAudit,
    ExecutiveSummary,
    ControlMatrix,
    EvidencePackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportStatus {
    Queued,
    Generating,
    Completed,
    Failed,
}
```

---

## Implementation Phases

### PHASE 1: Foundation - Data Models (6 steps)

**Objective:** Create all core data structures without breaking existing code.

**Baby Step 1.1:** Create new file `nexus-webui/src/models.rs`
- Add module declaration
- No code yet, just module structure
- Verify compilation succeeds

**Baby Step 1.2:** Add `Framework` struct to models.rs
- Copy struct definition exactly as specified
- Add required imports (serde, chrono)
- Verify compilation succeeds

**Baby Step 1.3:** Add `Control` and `ControlStatus` to models.rs
- Copy struct definition exactly as specified
- Add ControlStatus enum
- Verify compilation succeeds

**Baby Step 1.4:** Add `Asset` with related enums to models.rs
- Copy Asset struct
- Add AssetType, Environment, Criticality enums
- Verify compilation succeeds

**Baby Step 1.5:** Add `Evidence` with related structs to models.rs
- Copy Evidence struct
- Add EvidenceType, CustodyEvent, EvidenceStatus enums
- Verify compilation succeeds

**Baby Step 1.6:** Add `ComplianceScore` and `ReportJob` to models.rs
- Copy both structs
- Add ScoreTrend, ReportType, ReportStatus enums
- Verify compilation succeeds
- Export models from lib.rs: `pub mod models;`

**Success Criteria:**
- All data structures compile without errors
- No existing functionality is broken
- Models are properly exported

---

### PHASE 2: State Transformation (8 steps)

**Objective:** Transform WebUIState to DashboardState incrementally.

**Baby Step 2.1:** Create DashboardState struct in lib.rs
- Add new struct alongside existing WebUIState
- Copy WebUIConfig as DashboardConfig (rename only)
- Don't use it yet, just define it
- Verify compilation succeeds

**Baby Step 2.2:** Add frameworks field to DashboardState
- `pub frameworks: Arc<RwLock<HashMap<String, Framework>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.3:** Add controls field to DashboardState
- `pub controls: Arc<RwLock<HashMap<String, Control>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.4:** Add assets field to DashboardState
- `pub assets: Arc<RwLock<HashMap<String, Asset>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.5:** Add evidence field to DashboardState
- `pub evidence: Arc<RwLock<HashMap<String, Evidence>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.6:** Add compliance_scores field to DashboardState
- `pub compliance_scores: Arc<RwLock<HashMap<String, ComplianceScore>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.7:** Add report_jobs field to DashboardState
- `pub report_jobs: Arc<RwLock<HashMap<String, ReportJob>>>`
- Initialize as empty HashMap in constructor
- Verify compilation succeeds

**Baby Step 2.8:** Add broadcast_tx for DashboardEvent
- Create DashboardEvent enum (minimal for now)
- Add `pub broadcast_tx: broadcast::Sender<DashboardEvent>`
- Initialize in constructor
- Verify compilation succeeds

**Success Criteria:**
- DashboardState struct is fully defined
- All fields are properly initialized
- Existing WebUIState still works
- No runtime errors

---

### PHASE 3: Event System Transformation (4 steps)

**Objective:** Replace WebUIEvent with DashboardEvent.

**Baby Step 3.1:** Create DashboardEvent enum in lib.rs
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DashboardEvent {
    // Compliance events
    ComplianceScoreUpdate {
        framework_id: String,
        score: ComplianceScore,
    },
    ControlStatusChange {
        control_id: String,
        old_status: ControlStatus,
        new_status: ControlStatus,
    },
    // Drift detection
    DriftDetected {
        asset_id: String,
        control_id: String,
        severity: String,
        message: String,
    },
    // Evidence events
    EvidenceAdded {
        control_id: String,
        evidence_id: String,
    },
    // Report events
    ReportJobCreated {
        job_id: String,
    },
    ReportJobCompleted {
        job_id: String,
        file_path: String,
    },
    ReportJobFailed {
        job_id: String,
        error: String,
    },
    // System events
    SystemAlert {
        level: String,
        message: String,
    },
}
```
- Add enum definition
- Verify compilation succeeds

**Baby Step 3.2:** Add DashboardEvent to websocket.rs
- Update handle_websocket_connection to accept DashboardEvent
- Keep WebUIEvent handling for backward compatibility
- Verify compilation succeeds

**Baby Step 3.3:** Create broadcast helper methods on DashboardState
```rust
impl DashboardState {
    pub async fn broadcast_event(&self, event: DashboardEvent) {
        if let Err(e) = self.broadcast_tx.send(event) {
            warn!("Failed to broadcast event: {}", e);
        }
    }
}
```
- Add implementation block
- Verify compilation succeeds

**Baby Step 3.4:** Test event broadcasting
- Create simple test that sends DashboardEvent
- Verify events can be sent and received
- Verify compilation and tests succeed

**Success Criteria:**
- DashboardEvent enum is complete
- Events can be broadcast through the system
- WebSocket can handle new events
- Tests pass

---

### PHASE 4: Framework Management Routes (6 steps)

**Objective:** Implement framework listing and detail routes.

**Baby Step 4.1:** Create governance_handlers.rs module
- New file: `nexus-webui/src/governance_handlers.rs`
- Add module declaration in lib.rs: `pub mod governance_handlers;`
- Add placeholder for list_frameworks function
- Verify compilation succeeds

**Baby Step 4.2:** Implement GET /api/frameworks handler
```rust
pub async fn list_frameworks(
    state: DashboardState
) -> Result<impl Reply, warp::Rejection> {
    let frameworks = state.frameworks.read().await;
    let frameworks_vec: Vec<&Framework> = frameworks.values().collect();
    Ok(warp::reply::json(&frameworks_vec))
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 4.3:** Wire up /api/frameworks route in lib.rs
- Add route to build_api_routes method
- Test route returns empty array
- Verify compilation and manual test succeed

**Baby Step 4.4:** Implement GET /api/frameworks/:id handler
```rust
pub async fn get_framework_details(
    framework_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let frameworks = state.frameworks.read().await;

    if let Some(framework) = frameworks.get(&framework_id) {
        Ok(Box::new(warp::reply::json(framework)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Framework not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 4.5:** Wire up /api/frameworks/:id route
- Add route to build_api_routes method
- Test route returns 404 for non-existent framework
- Verify compilation and manual test succeed

**Baby Step 4.6:** Add sample framework data loader
```rust
impl DashboardState {
    pub async fn load_sample_frameworks(&self) {
        let mut frameworks = self.frameworks.write().await;

        let nist = Framework {
            id: "nist-800-53".to_string(),
            name: "NIST 800-53".to_string(),
            version: "Rev 5".to_string(),
            description: "Security and Privacy Controls".to_string(),
            control_count: 1100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        frameworks.insert(nist.id.clone(), nist);
        // Add more frameworks...
    }
}
```
- Add helper method
- Call from DashboardServer initialization
- Test route returns actual framework data
- Verify compilation and manual test succeed

**Success Criteria:**
- GET /api/frameworks returns list of frameworks
- GET /api/frameworks/:id returns framework details
- 404 handling works correctly
- Sample data loads properly

---

### PHASE 5: Control Management Routes (6 steps)

**Objective:** Implement control listing and detail routes.

**Baby Step 5.1:** Implement GET /api/frameworks/:id/controls handler
```rust
pub async fn get_framework_controls(
    framework_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let controls = state.controls.read().await;

    let framework_controls: Vec<&Control> = controls
        .values()
        .filter(|c| c.framework_id == framework_id)
        .collect();

    Ok(Box::new(warp::reply::json(&framework_controls)))
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 5.2:** Wire up /api/frameworks/:id/controls route
- Add route to build_api_routes method
- Test route returns empty array
- Verify compilation and manual test succeed

**Baby Step 5.3:** Implement GET /api/controls/:id handler
```rust
pub async fn get_control_details(
    control_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let controls = state.controls.read().await;

    if let Some(control) = controls.get(&control_id) {
        Ok(Box::new(warp::reply::json(control)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Control not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 5.4:** Wire up /api/controls/:id route
- Add route to build_api_routes method
- Test route returns 404 for non-existent control
- Verify compilation and manual test succeed

**Baby Step 5.5:** Add sample control data loader
```rust
impl DashboardState {
    pub async fn load_sample_controls(&self) {
        let mut controls = self.controls.write().await;

        let ac1 = Control {
            id: "nist-ac-1".to_string(),
            framework_id: "nist-800-53".to_string(),
            control_number: "AC-1".to_string(),
            title: "Policy and Procedures".to_string(),
            requirement: "Develop, document, and disseminate...".to_string(),
            status: ControlStatus::Pass,
            owner: "Security Team".to_string(),
            last_assessed: Some(Utc::now() - chrono::Duration::days(30)),
            next_assessment: Some(Utc::now() + chrono::Duration::days(335)),
            evidence_count: 3,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        controls.insert(ac1.id.clone(), ac1);
        // Add more controls...
    }
}
```
- Add helper method
- Call from DashboardServer initialization
- Test routes return actual control data
- Verify compilation and manual test succeed

**Baby Step 5.6:** Create ControlStatusView aggregate
```rust
#[derive(Debug, Serialize)]
pub struct ControlStatusView {
    pub control: Control,
    pub evidence_count: usize,
    pub latest_evidence: Option<Evidence>,
    pub days_since_assessment: Option<i64>,
    pub days_until_next: Option<i64>,
}

pub async fn get_control_status_view(
    control_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let controls = state.controls.read().await;
    let evidence = state.evidence.read().await;

    if let Some(control) = controls.get(&control_id) {
        let control_evidence: Vec<&Evidence> = evidence
            .values()
            .filter(|e| e.control_id == control_id)
            .collect();

        let view = ControlStatusView {
            control: control.clone(),
            evidence_count: control_evidence.len(),
            latest_evidence: control_evidence.first().map(|e| (*e).clone()),
            days_since_assessment: control.last_assessed.map(|d| (Utc::now() - d).num_days()),
            days_until_next: control.next_assessment.map(|d| (d - Utc::now()).num_days()),
        };

        Ok(Box::new(warp::reply::json(&view)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Control not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add ControlStatusView struct to models.rs
- Add handler function
- Wire up route
- Verify compilation and manual test succeed

**Success Criteria:**
- GET /api/frameworks/:id/controls returns filtered controls
- GET /api/controls/:id returns control details
- ControlStatusView provides aggregated data
- Sample data loads properly

---

### PHASE 6: Evidence Management Routes (5 steps)

**Objective:** Implement evidence listing and detail routes.

**Baby Step 6.1:** Implement GET /api/controls/:id/evidence handler
```rust
pub async fn get_control_evidence(
    control_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let evidence = state.evidence.read().await;

    let control_evidence: Vec<&Evidence> = evidence
        .values()
        .filter(|e| e.control_id == control_id)
        .collect();

    Ok(warp::reply::json(&control_evidence))
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 6.2:** Wire up /api/controls/:id/evidence route
- Add route to build_api_routes method
- Test route returns empty array
- Verify compilation and manual test succeed

**Baby Step 6.3:** Implement POST /api/controls/:id/evidence handler
```rust
#[derive(Debug, Deserialize)]
pub struct AddEvidenceRequest {
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub collected_by: String,
}

pub async fn add_control_evidence(
    control_id: String,
    request: AddEvidenceRequest,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let evidence_id = uuid::Uuid::new_v4().to_string();

    let new_evidence = Evidence {
        id: evidence_id.clone(),
        control_id: control_id.clone(),
        evidence_type: request.evidence_type,
        title: request.title,
        description: request.description,
        file_path: request.file_path.clone(),
        collected_by: request.collected_by.clone(),
        collected_at: Utc::now(),
        chain_of_custody: vec![CustodyEvent {
            timestamp: Utc::now(),
            actor: request.collected_by,
            action: "collected".to_string(),
            notes: None,
        }],
        hash: request.file_path
            .map(|p| format!("sha256:{}", uuid::Uuid::new_v4())) // Placeholder
            .unwrap_or_default(),
        status: EvidenceStatus::Collected,
    };

    let mut evidence = state.evidence.write().await;
    evidence.insert(evidence_id.clone(), new_evidence.clone());
    drop(evidence);

    // Broadcast event
    state.broadcast_event(DashboardEvent::EvidenceAdded {
        control_id,
        evidence_id: evidence_id.clone(),
    }).await;

    Ok(warp::reply::json(&new_evidence))
}
```
- Add AddEvidenceRequest struct
- Add handler function
- Verify compilation succeeds

**Baby Step 6.4:** Wire up POST /api/controls/:id/evidence route
- Add route to build_api_routes method
- Test route creates evidence
- Verify WebSocket receives EvidenceAdded event
- Verify compilation and manual test succeed

**Baby Step 6.5:** Add sample evidence data loader
```rust
impl DashboardState {
    pub async fn load_sample_evidence(&self) {
        let mut evidence = self.evidence.write().await;

        let ev1 = Evidence {
            id: "ev-001".to_string(),
            control_id: "nist-ac-1".to_string(),
            evidence_type: EvidenceType::PolicyDocument,
            title: "Access Control Policy v2.1".to_string(),
            description: "Current access control policy document".to_string(),
            file_path: Some("/evidence/policies/ac-policy-v2.1.pdf".to_string()),
            collected_by: "security@example.com".to_string(),
            collected_at: Utc::now() - chrono::Duration::days(10),
            chain_of_custody: vec![
                CustodyEvent {
                    timestamp: Utc::now() - chrono::Duration::days(10),
                    actor: "security@example.com".to_string(),
                    action: "collected".to_string(),
                    notes: None,
                },
                CustodyEvent {
                    timestamp: Utc::now() - chrono::Duration::days(5),
                    actor: "auditor@example.com".to_string(),
                    action: "reviewed".to_string(),
                    notes: Some("Policy meets requirements".to_string()),
                },
            ],
            hash: "sha256:abc123...".to_string(),
            status: EvidenceStatus::Approved,
        };

        evidence.insert(ev1.id.clone(), ev1);
        // Add more evidence...
    }
}
```
- Add helper method
- Call from DashboardServer initialization
- Test routes return actual evidence data
- Verify compilation and manual test succeed

**Success Criteria:**
- GET /api/controls/:id/evidence returns evidence list
- POST /api/controls/:id/evidence creates new evidence
- Chain of custody is properly initialized
- Events are broadcast correctly

---

### PHASE 7: Asset Management Routes (7 steps)

**Objective:** Implement asset inventory and compliance status routes.

**Baby Step 7.1:** Implement GET /api/assets handler
```rust
pub async fn list_assets(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let assets = state.assets.read().await;
    let assets_vec: Vec<&Asset> = assets.values().collect();
    Ok(warp::reply::json(&assets_vec))
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 7.2:** Wire up /api/assets route
- Add route to build_api_routes method
- Test route returns empty array
- Verify compilation and manual test succeed

**Baby Step 7.3:** Implement GET /api/assets/:id handler
```rust
pub async fn get_asset_details(
    asset_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let assets = state.assets.read().await;

    if let Some(asset) = assets.get(&asset_id) {
        Ok(Box::new(warp::reply::json(asset)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Asset not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 7.4:** Wire up /api/assets/:id route
- Add route to build_api_routes method
- Test route returns 404 for non-existent asset
- Verify compilation and manual test succeed

**Baby Step 7.5:** Create AssetComplianceView aggregate
```rust
#[derive(Debug, Serialize)]
pub struct AssetComplianceView {
    pub asset: Asset,
    pub total_frameworks: usize,
    pub compliant_frameworks: usize,
    pub non_compliant_frameworks: usize,
    pub compliance_percentage: f64,
    pub failing_controls: Vec<String>, // control_ids
}
```
- Add struct to models.rs
- Verify compilation succeeds

**Baby Step 7.6:** Implement GET /api/assets/:id/compliance handler
```rust
pub async fn get_asset_compliance(
    asset_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let assets = state.assets.read().await;

    if let Some(asset) = assets.get(&asset_id) {
        let total = asset.compliance_status.len();
        let compliant = asset.compliance_status.values().filter(|&&v| v).count();
        let percentage = if total > 0 {
            (compliant as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let view = AssetComplianceView {
            asset: asset.clone(),
            total_frameworks: total,
            compliant_frameworks: compliant,
            non_compliant_frameworks: total - compliant,
            compliance_percentage: percentage,
            failing_controls: vec![], // TODO: Calculate from controls
        };

        Ok(Box::new(warp::reply::json(&view)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Asset not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Wire up route
- Verify compilation and manual test succeed

**Baby Step 7.7:** Add sample asset data loader
```rust
impl DashboardState {
    pub async fn load_sample_assets(&self) {
        let mut assets = self.assets.write().await;

        let mut compliance_status = HashMap::new();
        compliance_status.insert("nist-800-53".to_string(), true);
        compliance_status.insert("soc2".to_string(), false);

        let server1 = Asset {
            id: "asset-001".to_string(),
            name: "prod-web-01".to_string(),
            asset_type: AssetType::Server,
            description: "Production web server".to_string(),
            owner: "engineering@example.com".to_string(),
            location: "AWS us-east-1".to_string(),
            environment: Environment::Production,
            criticality: Criticality::Critical,
            compliance_status,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assets.insert(server1.id.clone(), server1);
        // Add more assets...
    }
}
```
- Add helper method
- Call from DashboardServer initialization
- Test routes return actual asset data
- Verify compilation and manual test succeed

**Success Criteria:**
- GET /api/assets returns asset inventory
- GET /api/assets/:id returns asset details
- GET /api/assets/:id/compliance returns compliance view
- Sample data loads properly

---

### PHASE 8: Compliance Score Routes (4 steps)

**Objective:** Implement compliance score calculation and retrieval.

**Baby Step 8.1:** Implement compliance score calculation logic
```rust
impl DashboardState {
    pub async fn calculate_compliance_score(&self, framework_id: &str) -> ComplianceScore {
        let controls = self.controls.read().await;

        let framework_controls: Vec<&Control> = controls
            .values()
            .filter(|c| c.framework_id == framework_id)
            .collect();

        let total = framework_controls.len();
        let passing = framework_controls.iter().filter(|c| matches!(c.status, ControlStatus::Pass)).count();
        let failing = framework_controls.iter().filter(|c| matches!(c.status, ControlStatus::Fail)).count();
        let pending = framework_controls.iter().filter(|c| matches!(c.status, ControlStatus::Pending)).count();
        let na = framework_controls.iter().filter(|c| matches!(c.status, ControlStatus::NotApplicable)).count();

        let applicable = total - na;
        let percentage = if applicable > 0 {
            (passing as f64 / applicable as f64) * 100.0
        } else {
            0.0
        };

        ComplianceScore {
            framework_id: framework_id.to_string(),
            total_controls: total,
            passing_controls: passing,
            failing_controls: failing,
            pending_controls: pending,
            na_controls: na,
            percentage,
            last_calculated: Utc::now(),
            trend: ScoreTrend::Stable, // TODO: Calculate from history
        }
    }
}
```
- Add method to DashboardState impl block
- Verify compilation succeeds

**Baby Step 8.2:** Implement GET /api/frameworks/:id/compliance-score handler
```rust
pub async fn get_framework_compliance_score(
    framework_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let score = state.calculate_compliance_score(&framework_id).await;

    // Cache the score
    let mut scores = state.compliance_scores.write().await;
    scores.insert(framework_id.clone(), score.clone());

    Ok(warp::reply::json(&score))
}
```
- Add function to governance_handlers.rs
- Verify compilation succeeds

**Baby Step 8.3:** Wire up /api/frameworks/:id/compliance-score route
- Add route to build_api_routes method
- Test route calculates score correctly
- Verify compilation and manual test succeed

**Baby Step 8.4:** Add compliance score caching and auto-update
```rust
impl DashboardState {
    pub async fn update_compliance_scores(&self) {
        let frameworks = self.frameworks.read().await;

        for framework_id in frameworks.keys() {
            let score = self.calculate_compliance_score(framework_id).await;

            let mut scores = self.compliance_scores.write().await;
            scores.insert(framework_id.clone(), score.clone());
            drop(scores);

            // Broadcast update
            self.broadcast_event(DashboardEvent::ComplianceScoreUpdate {
                framework_id: framework_id.clone(),
                score,
            }).await;
        }
    }

    pub async fn start_score_updater(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                state.update_compliance_scores().await;
            }
        });
    }
}
```
- Add updater methods
- Call start_score_updater from DashboardServer::start
- Verify scores update automatically
- Verify WebSocket receives score updates
- Verify compilation and manual test succeed

**Success Criteria:**
- GET /api/frameworks/:id/compliance-score returns calculated score
- Scores are cached in state
- Automatic updates broadcast to WebSocket clients
- Score calculation is accurate

---

### PHASE 9: Report Generation (8 steps)

**Objective:** Implement async report generation with job tracking.

**Baby Step 9.1:** Create report generator module
- New file: `nexus-webui/src/report_generator.rs`
- Add module declaration in lib.rs: `pub mod report_generator;`
- Add placeholder struct
- Verify compilation succeeds

**Baby Step 9.2:** Implement report generation logic (stub)
```rust
use crate::models::*;
use std::path::PathBuf;

pub struct ReportGenerator;

impl ReportGenerator {
    pub async fn generate_full_audit(
        framework_ids: Vec<String>,
        output_dir: PathBuf,
    ) -> Result<PathBuf, String> {
        // Stub implementation
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(output_dir.join("audit_report.json"))
    }

    pub async fn generate_executive_summary(
        framework_ids: Vec<String>,
        output_dir: PathBuf,
    ) -> Result<PathBuf, String> {
        // Stub implementation
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        Ok(output_dir.join("executive_summary.json"))
    }
}
```
- Add ReportGenerator struct and methods
- Verify compilation succeeds

**Baby Step 9.3:** Implement POST /api/reports/generate handler
```rust
#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub report_type: ReportType,
    pub framework_ids: Vec<String>,
    pub requested_by: String,
}

pub async fn generate_report(
    request: GenerateReportRequest,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let job_id = uuid::Uuid::new_v4().to_string();

    let job = ReportJob {
        id: job_id.clone(),
        report_type: request.report_type.clone(),
        framework_ids: request.framework_ids.clone(),
        requested_by: request.requested_by,
        requested_at: Utc::now(),
        status: ReportStatus::Queued,
        file_path: None,
        error: None,
    };

    let mut jobs = state.report_jobs.write().await;
    jobs.insert(job_id.clone(), job.clone());
    drop(jobs);

    // Broadcast event
    state.broadcast_event(DashboardEvent::ReportJobCreated {
        job_id: job_id.clone(),
    }).await;

    // Start async generation
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Update status to Generating
        {
            let mut jobs = state_clone.report_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = ReportStatus::Generating;
            }
        }

        // Generate report (stub)
        let result = match request.report_type {
            ReportType::FullAudit => {
                ReportGenerator::generate_full_audit(
                    request.framework_ids,
                    PathBuf::from("/tmp/reports"),
                ).await
            },
            ReportType::ExecutiveSummary => {
                ReportGenerator::generate_executive_summary(
                    request.framework_ids,
                    PathBuf::from("/tmp/reports"),
                ).await
            },
            _ => Err("Report type not implemented".to_string()),
        };

        // Update job with result
        let mut jobs = state_clone.report_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            match result {
                Ok(path) => {
                    job.status = ReportStatus::Completed;
                    job.file_path = Some(path.to_string_lossy().to_string());

                    state_clone.broadcast_event(DashboardEvent::ReportJobCompleted {
                        job_id: job_id.clone(),
                        file_path: job.file_path.clone().unwrap(),
                    }).await;
                },
                Err(e) => {
                    job.status = ReportStatus::Failed;
                    job.error = Some(e.clone());

                    state_clone.broadcast_event(DashboardEvent::ReportJobFailed {
                        job_id: job_id.clone(),
                        error: e,
                    }).await;
                }
            }
        }
    });

    Ok(warp::reply::json(&job))
}
```
- Add GenerateReportRequest struct
- Add handler function
- Verify compilation succeeds

**Baby Step 9.4:** Wire up POST /api/reports/generate route
- Add route to build_api_routes method
- Test route creates job and returns immediately
- Verify compilation and manual test succeed

**Baby Step 9.5:** Implement GET /api/reports/:id handler
```rust
pub async fn get_report_job(
    job_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let jobs = state.report_jobs.read().await;

    if let Some(job) = jobs.get(&job_id) {
        Ok(Box::new(warp::reply::json(job)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Report job not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Wire up route
- Verify compilation and manual test succeed

**Baby Step 9.6:** Implement GET /api/reports/:id/download handler (stub)
```rust
pub async fn download_report(
    job_id: String,
    state: DashboardState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
    let jobs = state.report_jobs.read().await;

    if let Some(job) = jobs.get(&job_id) {
        if job.status == ReportStatus::Completed {
            if let Some(file_path) = &job.file_path {
                // TODO: Serve actual file
                Ok(Box::new(warp::reply::json(&json!({
                    "status": "ready",
                    "download_url": format!("/downloads/{}", job_id)
                }))))
            } else {
                Ok(Box::new(warp::reply::with_status(
                    warp::reply::json(&json!({"error": "Report file not available"})),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                )))
            }
        } else {
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Report not ready", "status": job.status})),
                warp::http::StatusCode::CONFLICT,
            )))
        }
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Report job not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}
```
- Add function to governance_handlers.rs
- Wire up route
- Verify compilation and manual test succeed

**Baby Step 9.7:** Test full report generation workflow
- Create report job via POST
- Poll GET /api/reports/:id until completed
- Verify status transitions: Queued -> Generating -> Completed
- Verify WebSocket events are received
- Verify compilation and manual test succeed

**Baby Step 9.8:** Add GET /api/reports route to list all jobs
```rust
pub async fn list_report_jobs(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let jobs = state.report_jobs.read().await;
    let jobs_vec: Vec<&ReportJob> = jobs.values().collect();
    Ok(warp::reply::json(&jobs_vec))
}
```
- Add function to governance_handlers.rs
- Wire up route
- Verify compilation and manual test succeed

**Success Criteria:**
- POST /api/reports/generate creates async job
- GET /api/reports/:id tracks job status
- GET /api/reports/:id/download returns download URL when ready
- GET /api/reports lists all jobs
- WebSocket receives job status updates
- Report generation runs asynchronously

---

### PHASE 10: WebSocket Specialization (3 steps)

**Objective:** Create specialized WebSocket endpoints for different event types.

**Baby Step 10.1:** Implement /ws/compliance-stream handler
```rust
pub async fn handle_compliance_websocket(
    ws: warp::ws::Ws,
    state: DashboardState,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |websocket| handle_compliance_connection(websocket, state)))
}

async fn handle_compliance_connection(
    ws: warp::ws::WebSocket,
    state: DashboardState,
) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Send initial compliance scores
    {
        let scores = state.compliance_scores.read().await;
        for score in scores.values() {
            let event = DashboardEvent::ComplianceScoreUpdate {
                framework_id: score.framework_id.clone(),
                score: score.clone(),
            };
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = ws_tx.send(warp::ws::Message::text(json)).await;
            }
        }
    }

    // Stream only compliance-related events
    while let Ok(event) = broadcast_rx.recv().await {
        let should_send = matches!(
            event,
            DashboardEvent::ComplianceScoreUpdate { .. } |
            DashboardEvent::ControlStatusChange { .. }
        );

        if should_send {
            if let Ok(json) = serde_json::to_string(&event) {
                if ws_tx.send(warp::ws::Message::text(json)).await.is_err() {
                    break;
                }
            }
        }
    }
}
```
- Add handler to websocket.rs
- Wire up route: `warp::path!("ws" / "compliance-stream")`
- Test WebSocket receives only compliance events
- Verify compilation and manual test succeed

**Baby Step 10.2:** Implement /ws/drift-alerts handler
```rust
pub async fn handle_drift_websocket(
    ws: warp::ws::Ws,
    state: DashboardState,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |websocket| handle_drift_connection(websocket, state)))
}

async fn handle_drift_connection(
    ws: warp::ws::WebSocket,
    state: DashboardState,
) {
    let (mut ws_tx, _ws_rx) = ws.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Stream only drift detection events
    while let Ok(event) = broadcast_rx.recv().await {
        let should_send = matches!(event, DashboardEvent::DriftDetected { .. });

        if should_send {
            if let Ok(json) = serde_json::to_string(&event) {
                if ws_tx.send(warp::ws::Message::text(json)).await.is_err() {
                    break;
                }
            }
        }
    }
}
```
- Add handler to websocket.rs
- Wire up route: `warp::path!("ws" / "drift-alerts")`
- Test WebSocket receives only drift events
- Verify compilation and manual test succeed

**Baby Step 10.3:** Add test drift event generator
```rust
impl DashboardState {
    pub async fn simulate_drift_alert(&self, asset_id: &str, control_id: &str) {
        self.broadcast_event(DashboardEvent::DriftDetected {
            asset_id: asset_id.to_string(),
            control_id: control_id.to_string(),
            severity: "medium".to_string(),
            message: "Configuration drift detected: firewall rule modified".to_string(),
        }).await;
    }
}
```
- Add helper method
- Create test endpoint to trigger drift alert
- Verify /ws/drift-alerts receives event
- Verify compilation and manual test succeed

**Success Criteria:**
- /ws/compliance-stream filters compliance events
- /ws/drift-alerts filters drift events
- Each WebSocket only receives relevant events
- Initial state is sent on connection (for compliance stream)

---

### PHASE 11: Configuration and Initialization (5 steps)

**Objective:** Transform server initialization from WebUI to Dashboard.

**Baby Step 11.1:** Create DashboardConfig struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub bind_address: String,
    pub port: u16,
    pub enable_websockets: bool,
    pub static_files_path: Option<String>,
    pub cors_origins: Vec<String>,
    pub report_output_dir: String,
    pub score_update_interval_secs: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            enable_websockets: true,
            static_files_path: None,
            cors_origins: vec!["*".to_string()],
            report_output_dir: "/tmp/reports".to_string(),
            score_update_interval_secs: 300,
        }
    }
}
```
- Replace WebUIConfig with DashboardConfig
- Add new configuration fields
- Verify compilation succeeds

**Baby Step 11.2:** Create DashboardServer struct
```rust
pub struct DashboardServer {
    state: DashboardState,
}

impl DashboardServer {
    pub fn new(config: DashboardConfig) -> Result<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);

        let state = DashboardState {
            config,
            frameworks: Arc::new(RwLock::new(HashMap::new())),
            controls: Arc::new(RwLock::new(HashMap::new())),
            assets: Arc::new(RwLock::new(HashMap::new())),
            evidence: Arc::new(RwLock::new(HashMap::new())),
            compliance_scores: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            report_jobs: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(Self { state })
    }

    pub async fn initialize(&self) -> Result<()> {
        // Load sample data
        self.state.load_sample_frameworks().await;
        self.state.load_sample_controls().await;
        self.state.load_sample_assets().await;
        self.state.load_sample_evidence().await;

        // Calculate initial scores
        self.state.update_compliance_scores().await;

        // Start background tasks
        self.state.start_score_updater().await;

        Ok(())
    }
}
```
- Add DashboardServer struct
- Add initialization method
- Verify compilation succeeds

**Baby Step 11.3:** Implement DashboardServer::start method
```rust
impl DashboardServer {
    pub async fn start(self) -> Result<()> {
        let state = self.state.clone();

        info!("Starting Governance Dashboard on {}:{}",
              state.config.bind_address, state.config.port);

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&json!({
                    "status": "healthy",
                    "service": "gov-dashboard",
                    "timestamp": Utc::now().to_rfc3339()
                }))
            });

        // Build routes
        let api_routes = self.build_api_routes().await;
        let ws_routes = self.build_websocket_routes().await;
        let static_routes = self.build_static_routes().await;

        // CORS
        let cors = warp::cors()
            .allow_origins(state.config.cors_origins.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec!["content-type", "authorization"])
            .build();

        // Combine routes
        let routes = health
            .or(api_routes)
            .or(ws_routes)
            .or(static_routes)
            .with(cors)
            .with(warp::log("gov-dashboard"));

        // Start server
        let addr = format!("{}:{}", state.config.bind_address, state.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| NexusError::ConfigurationError(format!("Invalid bind address: {}", e)))?;

        warp::serve(routes).run(addr).await;

        Ok(())
    }
}
```
- Add start method
- Copy route building logic from WebUIServer
- Update service name to "gov-dashboard"
- Verify compilation succeeds

**Baby Step 11.4:** Update build_api_routes with all new routes
```rust
async fn build_api_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = self.state.clone();

    // Framework routes
    let frameworks_list = warp::path!("api" / "frameworks")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::list_frameworks);

    let framework_details = warp::path!("api" / "frameworks" / String)
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_framework_details);

    let framework_controls = warp::path!("api" / "frameworks" / String / "controls")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_framework_controls);

    let framework_compliance = warp::path!("api" / "frameworks" / String / "compliance-score")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_framework_compliance_score);

    // Control routes
    let control_details = warp::path!("api" / "controls" / String)
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_control_details);

    let control_evidence = warp::path!("api" / "controls" / String / "evidence")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_control_evidence);

    let add_evidence = warp::path!("api" / "controls" / String / "evidence")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::add_control_evidence);

    // Asset routes
    let assets_list = warp::path!("api" / "assets")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::list_assets);

    let asset_details = warp::path!("api" / "assets" / String)
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_asset_details);

    let asset_compliance = warp::path!("api" / "assets" / String / "compliance")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_asset_compliance);

    // Report routes
    let reports_list = warp::path!("api" / "reports")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::list_report_jobs);

    let generate_report = warp::path!("api" / "reports" / "generate")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::generate_report);

    let report_job = warp::path!("api" / "reports" / String)
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::get_report_job);

    let download_report = warp::path!("api" / "reports" / String / "download")
        .and(warp::get())
        .and(with_dashboard_state(state.clone()))
        .and_then(governance_handlers::download_report);

    frameworks_list
        .or(framework_details)
        .or(framework_controls)
        .or(framework_compliance)
        .or(control_details)
        .or(control_evidence)
        .or(add_evidence)
        .or(assets_list)
        .or(asset_details)
        .or(asset_compliance)
        .or(reports_list)
        .or(generate_report)
        .or(report_job)
        .or(download_report)
        .boxed()
}

fn with_dashboard_state(
    state: DashboardState,
) -> impl Filter<Extract = (DashboardState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}
```
- Update build_api_routes method
- Wire up all governance routes
- Remove old C2-related routes
- Verify compilation succeeds

**Baby Step 11.5:** Update build_websocket_routes with specialized endpoints
```rust
async fn build_websocket_routes(&self) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
    let state = self.state.clone();

    let compliance_stream = warp::path!("ws" / "compliance-stream")
        .and(warp::ws())
        .and(with_dashboard_state(state.clone()))
        .and_then(websocket::handle_compliance_websocket);

    let drift_alerts = warp::path!("ws" / "drift-alerts")
        .and(warp::ws())
        .and(with_dashboard_state(state.clone()))
        .and_then(websocket::handle_drift_websocket);

    compliance_stream
        .or(drift_alerts)
        .boxed()
}
```
- Update build_websocket_routes method
- Wire up specialized WebSocket endpoints
- Remove generic /ws endpoint
- Verify compilation succeeds

**Success Criteria:**
- DashboardServer initializes properly
- All routes are registered
- Sample data loads on startup
- Background score updater starts
- Server starts without errors

---

### PHASE 12: Testing and Validation (6 steps)

**Objective:** Comprehensive testing of all dashboard functionality.

**Baby Step 12.1:** Create integration test module
- New file: `nexus-webui/tests/dashboard_tests.rs`
- Add basic test structure
- Verify test compilation succeeds

**Baby Step 12.2:** Test framework routes
```rust
#[tokio::test]
async fn test_list_frameworks() {
    let config = DashboardConfig::default();
    let server = DashboardServer::new(config).unwrap();
    server.initialize().await.unwrap();

    // Test GET /api/frameworks
    // Verify response contains frameworks
}

#[tokio::test]
async fn test_get_framework_details() {
    // Test GET /api/frameworks/:id
    // Verify framework details are returned
}

#[tokio::test]
async fn test_framework_compliance_score() {
    // Test GET /api/frameworks/:id/compliance-score
    // Verify score calculation is correct
}
```
- Add framework route tests
- Verify tests pass

**Baby Step 12.3:** Test control routes
```rust
#[tokio::test]
async fn test_list_framework_controls() {
    // Test GET /api/frameworks/:id/controls
    // Verify controls are filtered by framework
}

#[tokio::test]
async fn test_get_control_details() {
    // Test GET /api/controls/:id
    // Verify control details are returned
}

#[tokio::test]
async fn test_control_evidence() {
    // Test GET /api/controls/:id/evidence
    // Verify evidence is returned
}
```
- Add control route tests
- Verify tests pass

**Baby Step 12.4:** Test asset routes
```rust
#[tokio::test]
async fn test_list_assets() {
    // Test GET /api/assets
    // Verify assets are returned
}

#[tokio::test]
async fn test_asset_compliance() {
    // Test GET /api/assets/:id/compliance
    // Verify compliance view is calculated correctly
}
```
- Add asset route tests
- Verify tests pass

**Baby Step 12.5:** Test report generation
```rust
#[tokio::test]
async fn test_generate_report() {
    // Test POST /api/reports/generate
    // Verify job is created
}

#[tokio::test]
async fn test_report_job_status() {
    // Test GET /api/reports/:id
    // Verify job status transitions
}
```
- Add report route tests
- Verify tests pass

**Baby Step 12.6:** Test WebSocket functionality
```rust
#[tokio::test]
async fn test_compliance_stream_websocket() {
    // Test /ws/compliance-stream
    // Verify compliance events are received
}

#[tokio::test]
async fn test_drift_alerts_websocket() {
    // Test /ws/drift-alerts
    // Verify drift events are received
}
```
- Add WebSocket tests
- Verify tests pass

**Success Criteria:**
- All route tests pass
- WebSocket tests pass
- No compilation warnings
- Test coverage is comprehensive

---

### PHASE 13: Documentation and Migration (4 steps)

**Objective:** Document the transformation and create migration guide.

**Baby Step 13.1:** Update module documentation
- Update lib.rs module-level docs to reflect gov-dashboard purpose
- Update handler module docs
- Update model module docs
- Verify documentation builds correctly

**Baby Step 13.2:** Create API documentation
- Document all API endpoints with examples
- Create OpenAPI/Swagger spec (optional)
- Add request/response examples
- Verify documentation is accurate

**Baby Step 13.3:** Create migration guide
- Document steps to migrate from WebUI to Dashboard
- List breaking changes
- Provide upgrade path
- Include configuration changes

**Baby Step 13.4:** Update README
- Update project description
- Add dashboard screenshots (when UI is ready)
- Document getting started steps
- Add API usage examples

**Success Criteria:**
- All modules have up-to-date documentation
- API documentation is comprehensive
- Migration guide is clear
- README reflects new purpose

---

### PHASE 14: Cleanup and Deprecation (5 steps)

**Objective:** Remove or deprecate old C2-related code.

**Baby Step 14.1:** Mark old types as deprecated
```rust
#[deprecated(since = "0.2.0", note = "Use DashboardState instead")]
pub struct WebUIState { ... }

#[deprecated(since = "0.2.0", note = "Use DashboardEvent instead")]
pub enum WebUIEvent { ... }
```
- Add deprecation warnings
- Verify compilation with warnings

**Baby Step 14.2:** Create compatibility layer (optional)
- Add type aliases for backward compatibility
- Document deprecation timeline
- Verify existing code can still compile

**Baby Step 14.3:** Remove agent-related code
- Remove AgentConnection struct (or deprecate)
- Remove agent management routes
- Update state structure
- Verify compilation succeeds

**Baby Step 14.4:** Remove domain rotation code
- Remove domain management routes
- Remove DomainManager references
- Clean up related handlers
- Verify compilation succeeds

**Baby Step 14.5:** Final cleanup
- Remove unused imports
- Fix all clippy warnings
- Run cargo fmt
- Verify all tests pass

**Success Criteria:**
- No unused code remains
- All deprecations are documented
- Code is clean and formatted
- Tests pass

---

## Implementation Summary

### Total Baby Steps: 79

**Phase Breakdown:**
- Phase 1: Foundation - Data Models (6 steps)
- Phase 2: State Transformation (8 steps)
- Phase 3: Event System (4 steps)
- Phase 4: Framework Routes (6 steps)
- Phase 5: Control Routes (6 steps)
- Phase 6: Evidence Routes (5 steps)
- Phase 7: Asset Routes (7 steps)
- Phase 8: Compliance Scores (4 steps)
- Phase 9: Report Generation (8 steps)
- Phase 10: WebSocket Specialization (3 steps)
- Phase 11: Configuration (5 steps)
- Phase 12: Testing (6 steps)
- Phase 13: Documentation (4 steps)
- Phase 14: Cleanup (5 steps)

### Validation Checkpoints

After each phase:
1. Run `cargo build` - Must succeed
2. Run `cargo test` - All tests must pass
3. Run `cargo clippy` - No warnings
4. Manual API testing - All endpoints work
5. WebSocket testing - Events flow correctly

### Sample Data Requirements

**Minimum 20 Frameworks:**
1. NIST 800-53 Rev 5
2. SOC 2 Type II
3. ISO 27001:2013
4. PCI DSS 4.0
5. HIPAA
6. GDPR
7. CCPA
8. FedRAMP
9. FISMA
10. CIS Controls v8
11. NIST Cybersecurity Framework
12. COBIT 2019
13. ISO 27017 (Cloud)
14. ISO 27018 (Privacy)
15. HITRUST CSF
16. CSA CCM v4
17. CMMC Level 3
18. ISO 22301 (Business Continuity)
19. SOX (Sarbanes-Oxley)
20. GLBA (Gramm-Leach-Bliley)

### File Structure After Transformation

```
nexus-webui/
├── src/
│   ├── lib.rs                    # DashboardServer, DashboardState
│   ├── models.rs                 # All data structures (NEW)
│   ├── governance_handlers.rs    # API handlers (NEW)
│   ├── report_generator.rs       # Report generation (NEW)
│   ├── websocket.rs              # WebSocket handlers (UPDATED)
│   ├── handlers.rs               # Legacy handlers (DEPRECATED)
│   ├── static_files.rs           # Static file serving
│   └── templates.rs              # HTML templates
├── tests/
│   └── dashboard_tests.rs        # Integration tests (NEW)
└── Cargo.toml
```

---

## Risk Mitigation

### Potential Issues and Solutions

**Issue 1: Breaking existing integrations**
- Solution: Maintain backward compatibility layer during transition
- Deprecate gradually over multiple versions

**Issue 2: Performance with large control sets**
- Solution: Implement pagination for control listings
- Cache compliance scores
- Use database instead of in-memory HashMap for production

**Issue 3: Report generation blocking**
- Solution: Use async job system (implemented in Phase 9)
- Consider using dedicated worker pool for heavy reports

**Issue 4: WebSocket connection limits**
- Solution: Implement connection pooling
- Set maximum connections per client
- Add rate limiting

---

## Next Steps After Completion

1. **Database Integration:** Replace in-memory HashMaps with PostgreSQL/SQLite
2. **UI Development:** Build React/Vue frontend consuming the API
3. **Authentication:** Add JWT-based auth for multi-user access
4. **RBAC:** Implement role-based access control
5. **Audit Logging:** Track all changes to controls and evidence
6. **File Storage:** Implement S3/local storage for evidence files
7. **Real Evidence Collection:** Integrate with scanning tools
8. **Drift Detection:** Implement actual configuration drift detection
9. **Notification System:** Add email/Slack alerts for compliance changes
10. **Advanced Reports:** PDF generation with charts and graphs

---

## Conclusion

This plan follows the Baby Steps methodology by breaking down the transformation into 79 atomic, testable steps. Each step is the smallest possible meaningful change, ensuring that:

1. The process is visible and trackable
2. Each change can be validated independently
3. Rollback is possible at any point
4. Progress is incremental and demonstrable
5. The transformation is methodical and low-risk

**Remember: The process is the product.**

By following this plan step-by-step, the transformation from `nexus-webui` (C2 dashboard) to `gov-dashboard` (compliance monitoring) will be complete, tested, and production-ready.
