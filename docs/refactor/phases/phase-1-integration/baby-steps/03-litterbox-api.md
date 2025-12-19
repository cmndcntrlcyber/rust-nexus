# 🐾 Baby Step 1.3: LitterBox API Integration

> Implement LitterBox deployment and API client.

**STATUS: ✅ COMPLETE**

## 📋 Objective

Create automated LitterBox deployment using existing infrastructure and implement API client for analysis submission.

## ✅ Prerequisites

- [x] Baby Step 1.1 complete (scaffold)
- [x] Understand nexus-infra DomainManager
- [x] Understand nexus-infra CertManager
- [x] Review LitterBox API documentation

## 🔧 Implementation (Completed)

### Deployment Module (deployment.rs)

```rust
pub struct LitterBoxDeployer {
    config: DeploymentConfig,
    client: reqwest::Client,
}

pub struct DeploymentConfig {
    pub image: String,              // Docker image
    pub container_name: String,     // Container name
    pub ports: Vec<PortMapping>,    // Port mappings
    pub volumes: Vec<VolumeMount>,  // Volume mounts
    pub environment: Vec<EnvVar>,   // Environment vars
    pub network: Option<String>,    // Docker network
    pub restart_policy: RestartPolicy,
    pub resource_limits: Option<ResourceLimits>,
}

impl LitterBoxDeployer {
    pub async fn deploy(&self) -> Result<DeploymentStatus>
    pub async fn stop(&self) -> Result<()>
    pub async fn remove(&self) -> Result<()>
    pub async fn get_logs(&self, lines: Option<usize>) -> Result<String>
    pub async fn get_status(&self) -> Result<DeploymentStatus>
    pub async fn health_check(&self) -> Result<bool>
}
```

### API Client (mod.rs)

```rust
pub struct LitterBoxClient {
    base_url: String,
    client: reqwest::Client,
    api_key: Option<String>,
    timeout: Duration,
    enabled: bool,
}

impl LitterBoxClient {
    // Sample submission
    pub async fn submit(&self, request: SubmissionRequest) -> Result<String>
    pub async fn submit_file(&self, file_path: &str, options: SubmissionOptions) -> Result<String>

    // Analysis retrieval
    pub async fn get_status(&self, sample_id: &str) -> Result<AnalysisStatus>
    pub async fn get_result(&self, sample_id: &str) -> Result<AnalysisResult>

    // Complete workflows
    pub async fn analyze(&self, request: SubmissionRequest) -> Result<AnalysisResult>
    pub async fn analyze_file(&self, file_path: &str) -> Result<Vec<DetectionEvent>>

    // Health monitoring
    pub async fn health_check(&self) -> Result<bool>

    // Event conversion
    pub fn result_to_events(&self, result: &AnalysisResult) -> Vec<DetectionEvent>
}
```

### Analysis Result Types

```rust
pub struct AnalysisResult {
    pub sample_id: String,
    pub status: AnalysisStatus,
    pub score: u32,              // Threat score (0-100)
    pub malware_family: Option<String>,
    pub signatures: Vec<String>,
    pub network_iocs: Vec<NetworkIOC>,
    pub file_iocs: Vec<FileIOC>,
    pub mitre_techniques: Vec<String>,
}

pub enum AnalysisStatus {
    Pending, Running, Complete, Failed
}
```

## ✅ Verification Checklist

- [x] LitterBox deployment automation implemented
- [x] Docker container orchestration via CLI
- [x] API client can upload samples (submit, submit_file)
- [x] Analysis results retrievable (get_status, get_result)
- [x] Health monitoring implemented
- [x] Result to DetectionEvent conversion
- [ ] Live LitterBox instance testing (requires deployment)

## 📤 Output

- `litterbox/deployment.rs` - Docker deployment automation
- `litterbox/mod.rs` - Full LitterBox API client
- Async operations with reqwest
- Base64 encoding for file uploads

## ➡️ Next Step

[04-event-pipeline.md](04-event-pipeline.md)

---
**Completed**: 2024-12-19
**Assigned To**: Infrastructure Agent
