use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::types::{ApiResponse, HealthResponse, Pagination, PaginatedResponse, ComponentHealth};

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    /// Service version
    pub version: String,
    /// Start time for uptime calculation
    pub start_time: std::time::Instant,
}

impl ApiState {
    /// Create new API state
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            start_time: std::time::Instant::now(),
        }
    }
}

/// Create the API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
        // Framework endpoints
        .route("/api/v1/frameworks", get(list_frameworks))
        .route("/api/v1/frameworks/:id", get(get_framework))
        // Control endpoints
        .route("/api/v1/controls", get(list_controls))
        .route("/api/v1/controls/:id", get(get_control))
        // Asset endpoints
        .route("/api/v1/assets", get(list_assets))
        .route("/api/v1/assets/:id", get(get_asset))
        // Evidence endpoints
        .route("/api/v1/evidence", get(list_evidence).post(create_evidence))
        .route("/api/v1/evidence/:id", get(get_evidence))
        // Report endpoints
        .route("/api/v1/reports", get(list_reports).post(generate_report))
        .route("/api/v1/reports/:id", get(get_report))
        // Compliance endpoints
        .route("/api/v1/compliance/score", get(get_compliance_score))
        .route("/api/v1/compliance/drift", get(get_drift_status))
        .with_state(state)
}

// === Health Endpoints ===

async fn health_check(State(state): State<ApiState>) -> Json<HealthResponse> {
    let mut components = std::collections::HashMap::new();
    components.insert(
        "api".to_string(),
        ComponentHealth {
            status: "healthy".to_string(),
            message: None,
            response_time_ms: Some(1),
        },
    );

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        components,
    })
}

async fn readiness_check() -> Json<ApiResponse<()>> {
    Json(ApiResponse::message("ready"))
}

async fn liveness_check() -> Json<ApiResponse<()>> {
    Json(ApiResponse::message("alive"))
}

// === Framework Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameworkSummary {
    pub id: String,
    pub name: String,
    pub category: String,
    pub version: String,
    pub control_count: usize,
}

async fn list_frameworks(
    Query(pagination): Query<Pagination>,
) -> Result<Json<ApiResponse<PaginatedResponse<FrameworkSummary>>>> {
    // Placeholder - would query actual frameworks
    let frameworks = vec![
        FrameworkSummary {
            id: "nist_csf_2".to_string(),
            name: "NIST Cybersecurity Framework 2.0".to_string(),
            category: "Security Standards".to_string(),
            version: "2.0".to_string(),
            control_count: 106,
        },
        FrameworkSummary {
            id: "iso_27001".to_string(),
            name: "ISO 27001:2022".to_string(),
            category: "Security Standards".to_string(),
            version: "2022".to_string(),
            control_count: 93,
        },
    ];

    let total = frameworks.len() as u64;
    let response = PaginatedResponse::new(frameworks, total, &pagination);
    Ok(Json(ApiResponse::success(response)))
}

async fn get_framework(Path(id): Path<String>) -> Result<Json<ApiResponse<FrameworkSummary>>> {
    // Placeholder - would query actual framework
    if id == "nist_csf_2" {
        Ok(Json(ApiResponse::success(FrameworkSummary {
            id: "nist_csf_2".to_string(),
            name: "NIST Cybersecurity Framework 2.0".to_string(),
            category: "Security Standards".to_string(),
            version: "2.0".to_string(),
            control_count: 106,
        })))
    } else {
        Err(ApiError::NotFound(format!("Framework {} not found", id)))
    }
}

// === Control Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlSummary {
    pub id: String,
    pub name: String,
    pub framework_id: String,
    pub domain: String,
    pub status: String,
}

async fn list_controls(
    Query(pagination): Query<Pagination>,
) -> Result<Json<ApiResponse<PaginatedResponse<ControlSummary>>>> {
    let controls = vec![
        ControlSummary {
            id: "GV.OC-01".to_string(),
            name: "Organizational Context".to_string(),
            framework_id: "nist_csf_2".to_string(),
            domain: "Govern".to_string(),
            status: "Implemented".to_string(),
        },
    ];

    let total = controls.len() as u64;
    let response = PaginatedResponse::new(controls, total, &pagination);
    Ok(Json(ApiResponse::success(response)))
}

async fn get_control(Path(id): Path<String>) -> Result<Json<ApiResponse<ControlSummary>>> {
    Ok(Json(ApiResponse::success(ControlSummary {
        id: id.clone(),
        name: "Sample Control".to_string(),
        framework_id: "nist_csf_2".to_string(),
        domain: "Govern".to_string(),
        status: "Implemented".to_string(),
    })))
}

// === Asset Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetSummary {
    pub id: Uuid,
    pub hostname: String,
    pub asset_type: String,
    pub os: String,
    pub status: String,
    pub compliance_score: f64,
}

async fn list_assets(
    Query(pagination): Query<Pagination>,
) -> Result<Json<ApiResponse<PaginatedResponse<AssetSummary>>>> {
    let assets = vec![
        AssetSummary {
            id: Uuid::new_v4(),
            hostname: "web-prod-01".to_string(),
            asset_type: "Server".to_string(),
            os: "Ubuntu 22.04".to_string(),
            status: "Online".to_string(),
            compliance_score: 95.5,
        },
    ];

    let total = assets.len() as u64;
    let response = PaginatedResponse::new(assets, total, &pagination);
    Ok(Json(ApiResponse::success(response)))
}

async fn get_asset(Path(id): Path<Uuid>) -> Result<Json<ApiResponse<AssetSummary>>> {
    Ok(Json(ApiResponse::success(AssetSummary {
        id,
        hostname: "web-prod-01".to_string(),
        asset_type: "Server".to_string(),
        os: "Ubuntu 22.04".to_string(),
        status: "Online".to_string(),
        compliance_score: 95.5,
    })))
}

// === Evidence Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub id: Uuid,
    pub title: String,
    pub control_id: String,
    pub collected_at: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvidenceRequest {
    pub title: String,
    pub control_id: String,
    pub data: serde_json::Value,
}

async fn list_evidence(
    Query(pagination): Query<Pagination>,
) -> Result<Json<ApiResponse<PaginatedResponse<EvidenceSummary>>>> {
    let evidence = vec![
        EvidenceSummary {
            id: Uuid::new_v4(),
            title: "SSH Configuration".to_string(),
            control_id: "AC-17".to_string(),
            collected_at: chrono::Utc::now().to_rfc3339(),
            status: "Approved".to_string(),
        },
    ];

    let total = evidence.len() as u64;
    let response = PaginatedResponse::new(evidence, total, &pagination);
    Ok(Json(ApiResponse::success(response)))
}

async fn get_evidence(Path(id): Path<Uuid>) -> Result<Json<ApiResponse<EvidenceSummary>>> {
    Ok(Json(ApiResponse::success(EvidenceSummary {
        id,
        title: "SSH Configuration".to_string(),
        control_id: "AC-17".to_string(),
        collected_at: chrono::Utc::now().to_rfc3339(),
        status: "Approved".to_string(),
    })))
}

async fn create_evidence(
    Json(request): Json<CreateEvidenceRequest>,
) -> Result<Json<ApiResponse<EvidenceSummary>>> {
    Ok(Json(ApiResponse::success_with_message(
        EvidenceSummary {
            id: Uuid::new_v4(),
            title: request.title,
            control_id: request.control_id,
            collected_at: chrono::Utc::now().to_rfc3339(),
            status: "Created".to_string(),
        },
        "Evidence created successfully",
    )))
}

// === Report Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: Uuid,
    pub title: String,
    pub framework: String,
    pub generated_at: String,
    pub score: f64,
}

#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub title: String,
    pub framework: String,
}

async fn list_reports(
    Query(pagination): Query<Pagination>,
) -> Result<Json<ApiResponse<PaginatedResponse<ReportSummary>>>> {
    let reports = vec![
        ReportSummary {
            id: Uuid::new_v4(),
            title: "Q4 2024 Compliance Report".to_string(),
            framework: "NIST-800-53".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            score: 87.5,
        },
    ];

    let total = reports.len() as u64;
    let response = PaginatedResponse::new(reports, total, &pagination);
    Ok(Json(ApiResponse::success(response)))
}

async fn get_report(Path(id): Path<Uuid>) -> Result<Json<ApiResponse<ReportSummary>>> {
    Ok(Json(ApiResponse::success(ReportSummary {
        id,
        title: "Q4 2024 Compliance Report".to_string(),
        framework: "NIST-800-53".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        score: 87.5,
    })))
}

async fn generate_report(
    Json(request): Json<GenerateReportRequest>,
) -> Result<Json<ApiResponse<ReportSummary>>> {
    Ok(Json(ApiResponse::success_with_message(
        ReportSummary {
            id: Uuid::new_v4(),
            title: request.title,
            framework: request.framework,
            generated_at: chrono::Utc::now().to_rfc3339(),
            score: 0.0, // Would be calculated
        },
        "Report generation started",
    )))
}

// === Compliance Endpoints ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceScoreResponse {
    pub overall_score: f64,
    pub grade: String,
    pub frameworks: std::collections::HashMap<String, f64>,
    pub trend: String,
}

async fn get_compliance_score() -> Result<Json<ApiResponse<ComplianceScoreResponse>>> {
    let mut frameworks = std::collections::HashMap::new();
    frameworks.insert("nist_csf_2".to_string(), 92.5);
    frameworks.insert("iso_27001".to_string(), 88.0);
    frameworks.insert("soc2".to_string(), 95.0);

    Ok(Json(ApiResponse::success(ComplianceScoreResponse {
        overall_score: 91.8,
        grade: "A".to_string(),
        frameworks,
        trend: "Improving".to_string(),
    })))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriftStatusResponse {
    pub has_drift: bool,
    pub drift_count: usize,
    pub critical_count: usize,
    pub last_checked: String,
}

async fn get_drift_status() -> Result<Json<ApiResponse<DriftStatusResponse>>> {
    Ok(Json(ApiResponse::success(DriftStatusResponse {
        has_drift: false,
        drift_count: 0,
        critical_count: 0,
        last_checked: chrono::Utc::now().to_rfc3339(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_state() {
        let state = ApiState::new("1.0.0");
        assert_eq!(state.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = ApiState::new("1.0.0");
        let response = health_check(State(state)).await;
        assert_eq!(response.status, "healthy");
    }

    #[tokio::test]
    async fn test_readiness() {
        let response = readiness_check().await;
        assert!(response.success);
    }
}
