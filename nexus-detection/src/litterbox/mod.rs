//! LitterBox sandbox integration
//!
//! Client for interacting with LitterBox malware sandbox.
//! Provides automated sample submission and analysis retrieval.
//!
//! LitterBox: https://github.com/BlackSnufkin/LitterBox

pub mod deployment;

pub use deployment::{
    DeploymentConfig, DeploymentStatus, LitterBoxDeployer,
    PortMapping, VolumeMount, EnvVar, RestartPolicy, ResourceLimits,
};

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::types::{DetectionEvent, DetectionSource, Severity};
use crate::{DetectionError, Result};

/// LitterBox API client
pub struct LitterBoxClient {
    /// Base URL for LitterBox API
    base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// API key if required
    api_key: Option<String>,
    /// Request timeout
    timeout: Duration,
    /// Enable/disable flag
    enabled: bool,
}

/// Sample submission request
#[derive(Debug, Clone, Serialize)]
pub struct SubmissionRequest {
    /// File name
    pub filename: String,
    /// File content (base64 encoded)
    pub content_b64: String,
    /// Analysis timeout in seconds
    pub timeout: u32,
    /// Additional options
    pub options: SubmissionOptions,
}

/// Submission options
#[derive(Debug, Clone, Default, Serialize)]
pub struct SubmissionOptions {
    /// Enable network analysis
    pub network: bool,
    /// Enable memory analysis
    pub memory: bool,
    /// Target platform (windows, linux)
    pub platform: Option<String>,
}

/// Analysis result from LitterBox
#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisResult {
    /// Sample ID
    pub sample_id: String,
    /// Analysis status
    pub status: AnalysisStatus,
    /// Threat score (0-100)
    pub score: u32,
    /// Detected malware family
    pub malware_family: Option<String>,
    /// Behavioral signatures triggered
    pub signatures: Vec<String>,
    /// Network IOCs
    pub network_iocs: Vec<NetworkIOC>,
    /// File IOCs
    pub file_iocs: Vec<FileIOC>,
    /// MITRE ATT&CK techniques
    pub mitre_techniques: Vec<String>,
}

/// Analysis status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisStatus {
    /// Analysis pending
    Pending,
    /// Analysis in progress
    Running,
    /// Analysis complete
    Complete,
    /// Analysis failed
    Failed,
}

/// Network IOC from analysis
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkIOC {
    /// IOC type (ip, domain, url)
    pub ioc_type: String,
    /// IOC value
    pub value: String,
    /// Associated ports
    pub ports: Vec<u16>,
}

/// File IOC from analysis
#[derive(Debug, Clone, Deserialize)]
pub struct FileIOC {
    /// File path
    pub path: String,
    /// File hash (SHA256)
    pub sha256: String,
    /// Operation (created, modified, deleted)
    pub operation: String,
}

/// API response for sample submission
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SubmitResponse {
    sample_id: String,
    message: Option<String>,
}

/// API response for status check
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct StatusResponse {
    sample_id: String,
    status: String,
}

/// API response for analysis results
#[derive(Debug, Clone, Deserialize)]
struct ResultResponse {
    sample_id: String,
    status: String,
    score: u32,
    malware_family: Option<String>,
    signatures: Vec<String>,
    network_iocs: Vec<NetworkIOC>,
    file_iocs: Vec<FileIOC>,
    mitre_techniques: Vec<String>,
}

impl LitterBoxClient {
    /// Create a new LitterBox client
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url: base_url.into(),
            client,
            api_key: None,
            timeout: Duration::from_secs(300),
            enabled: true,
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build request with common headers
    fn build_request(&self, method: reqwest::Method, endpoint: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.request(method, &url);

        if let Some(ref api_key) = self.api_key {
            request = request.header("X-API-Key", api_key);
        }

        request = request.header("Content-Type", "application/json");
        request
    }

    /// Check API health
    pub async fn health_check(&self) -> Result<bool> {
        let response = self
            .build_request(reqwest::Method::GET, "/health")
            .send()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    /// Submit a sample for analysis
    pub async fn submit(&self, request: SubmissionRequest) -> Result<String> {
        if !self.enabled {
            return Err(DetectionError::LitterBoxError(
                "LitterBox client is disabled".to_string(),
            ));
        }

        let response = self
            .build_request(reqwest::Method::POST, "/api/v1/submit")
            .json(&request)
            .send()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Submit request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(DetectionError::LitterBoxError(format!(
                "Submit failed with status {}: {}",
                status, text
            )));
        }

        let submit_response: SubmitResponse = response
            .json()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to parse response: {}", e)))?;

        Ok(submit_response.sample_id)
    }

    /// Submit a file from path
    pub async fn submit_file(&self, file_path: &str, options: SubmissionOptions) -> Result<String> {
        use std::path::Path;

        let path = Path::new(file_path);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = tokio::fs::read(file_path)
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to read file: {}", e)))?;

        let content_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &content,
        );

        let request = SubmissionRequest {
            filename,
            content_b64,
            timeout: 300,
            options,
        };

        self.submit(request).await
    }

    /// Get analysis status
    pub async fn get_status(&self, sample_id: &str) -> Result<AnalysisStatus> {
        if !self.enabled {
            return Err(DetectionError::LitterBoxError(
                "LitterBox client is disabled".to_string(),
            ));
        }

        let endpoint = format!("/api/v1/status/{}", sample_id);
        let response = self
            .build_request(reqwest::Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Status request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(DetectionError::LitterBoxError(format!(
                "Status check failed with status {}: {}",
                status, text
            )));
        }

        let status_response: StatusResponse = response
            .json()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to parse response: {}", e)))?;

        match status_response.status.to_lowercase().as_str() {
            "pending" => Ok(AnalysisStatus::Pending),
            "running" | "processing" => Ok(AnalysisStatus::Running),
            "complete" | "completed" => Ok(AnalysisStatus::Complete),
            "failed" | "error" => Ok(AnalysisStatus::Failed),
            _ => Ok(AnalysisStatus::Pending),
        }
    }

    /// Get analysis result
    pub async fn get_result(&self, sample_id: &str) -> Result<AnalysisResult> {
        if !self.enabled {
            return Err(DetectionError::LitterBoxError(
                "LitterBox client is disabled".to_string(),
            ));
        }

        let endpoint = format!("/api/v1/result/{}", sample_id);
        let response = self
            .build_request(reqwest::Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Result request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(DetectionError::LitterBoxError(format!(
                "Result fetch failed with status {}: {}",
                status, text
            )));
        }

        let result_response: ResultResponse = response
            .json()
            .await
            .map_err(|e| DetectionError::LitterBoxError(format!("Failed to parse response: {}", e)))?;

        let status = match result_response.status.to_lowercase().as_str() {
            "pending" => AnalysisStatus::Pending,
            "running" | "processing" => AnalysisStatus::Running,
            "complete" | "completed" => AnalysisStatus::Complete,
            "failed" | "error" => AnalysisStatus::Failed,
            _ => AnalysisStatus::Pending,
        };

        Ok(AnalysisResult {
            sample_id: result_response.sample_id,
            status,
            score: result_response.score,
            malware_family: result_response.malware_family,
            signatures: result_response.signatures,
            network_iocs: result_response.network_iocs,
            file_iocs: result_response.file_iocs,
            mitre_techniques: result_response.mitre_techniques,
        })
    }

    /// Submit and wait for analysis completion
    pub async fn analyze(&self, request: SubmissionRequest) -> Result<AnalysisResult> {
        let sample_id = self.submit(request).await?;

        // Poll for completion
        let poll_interval = Duration::from_secs(10);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                return Err(DetectionError::LitterBoxError(
                    "Analysis timeout".to_string(),
                ));
            }

            let status = self.get_status(&sample_id).await?;

            match status {
                AnalysisStatus::Complete => {
                    return self.get_result(&sample_id).await;
                }
                AnalysisStatus::Failed => {
                    return Err(DetectionError::LitterBoxError(
                        "Analysis failed".to_string(),
                    ));
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// Analyze a file and return detection events
    pub async fn analyze_file(&self, file_path: &str) -> Result<Vec<DetectionEvent>> {
        let options = SubmissionOptions {
            network: true,
            memory: true,
            platform: None,
        };

        let sample_id = self.submit_file(file_path, options).await?;

        // Wait for analysis
        let poll_interval = Duration::from_secs(10);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                return Err(DetectionError::LitterBoxError(
                    "Analysis timeout".to_string(),
                ));
            }

            let status = self.get_status(&sample_id).await?;

            match status {
                AnalysisStatus::Complete => {
                    let result = self.get_result(&sample_id).await?;
                    return Ok(self.result_to_events(&result));
                }
                AnalysisStatus::Failed => {
                    return Err(DetectionError::LitterBoxError(
                        "Analysis failed".to_string(),
                    ));
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// Convert analysis result to detection events
    pub fn result_to_events(&self, result: &AnalysisResult) -> Vec<DetectionEvent> {
        let mut events = Vec::new();

        // Create main detection event based on score
        let severity = match result.score {
            0..=25 => Severity::Low,
            26..=50 => Severity::Medium,
            51..=75 => Severity::High,
            _ => Severity::Critical,
        };

        let description = if let Some(ref family) = result.malware_family {
            format!("Malware detected: {} (score: {})", family, result.score)
        } else {
            format!("Suspicious sample (score: {})", result.score)
        };

        let mut event = DetectionEvent::new(
            DetectionSource::Sandbox,
            severity,
            format!("LB-{}", result.sample_id),
            description,
        );

        // Add MITRE techniques
        for technique in &result.mitre_techniques {
            event = event.with_mitre(technique);
        }

        events.push(event);

        // Add events for high-confidence signatures
        for sig in &result.signatures {
            events.push(DetectionEvent::new(
                DetectionSource::Sandbox,
                Severity::Medium,
                format!("LB-SIG-{}", result.sample_id),
                format!("Sandbox signature: {}", sig),
            ));
        }

        events
    }

    /// Check if client is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the client
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the client
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Default for LitterBoxClient {
    fn default() -> Self {
        Self::new("http://localhost:8000")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = LitterBoxClient::new("http://sandbox.local:8000");
        assert!(client.is_enabled());
        assert_eq!(client.base_url(), "http://sandbox.local:8000");
    }

    #[test]
    fn test_result_to_events() {
        let client = LitterBoxClient::default();
        let result = AnalysisResult {
            sample_id: "test-123".to_string(),
            status: AnalysisStatus::Complete,
            score: 85,
            malware_family: Some("Emotet".to_string()),
            signatures: vec!["persistence".to_string(), "network_beacon".to_string()],
            network_iocs: vec![],
            file_iocs: vec![],
            mitre_techniques: vec!["T1547".to_string(), "T1071".to_string()],
        };

        let events = client.result_to_events(&result);
        assert!(!events.is_empty());
        assert_eq!(events[0].severity, Severity::Critical);
        assert!(events[0].description.contains("Emotet"));
    }
}
