//! Compliance dashboard routes
//!
//! This module contains all routes for the compliance monitoring dashboard.

use crate::{
    models::*, AlertLevel, DashboardEvent, DashboardState,
};
use serde::{Deserialize, Serialize};
use warp::{Filter, Reply};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Pagination parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: usize, page: usize, per_page: usize) -> Self {
        let total_pages = (total + per_page - 1) / per_page;
        Self {
            data,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

/// Control filter parameters
#[derive(Debug, Clone, Deserialize)]
pub struct ControlFilterParams {
    pub status: Option<String>,
    pub owner: Option<String>,
}

/// Asset filter parameters
#[derive(Debug, Clone, Deserialize)]
pub struct AssetFilterParams {
    pub asset_type: Option<String>,
    pub environment: Option<String>,
    pub criticality: Option<String>,
}

/// Report generation request
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReportRequest {
    pub report_type: String,
    pub framework_ids: Vec<String>,
    pub requested_by: String,
}

/// Evidence submission request
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitEvidenceRequest {
    pub control_id: String,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub collected_by: String,
}

// ============================================================================
// Route Builders
// ============================================================================

/// Build all compliance API routes
pub fn compliance_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let frameworks = framework_routes(state.clone());
    let controls = control_routes(state.clone());
    let evidence = evidence_routes(state.clone());
    let assets = asset_routes(state.clone());
    let scores = score_routes(state.clone());
    let reports = report_routes(state);

    frameworks
        .or(controls)
        .or(evidence)
        .or(assets)
        .or(scores)
        .or(reports)
}

// ============================================================================
// Framework Routes
// ============================================================================

fn framework_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let list = warp::path!("api" / "compliance" / "frameworks")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_frameworks);

    let get = warp::path!("api" / "compliance" / "frameworks" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_framework);

    let score = warp::path!("api" / "compliance" / "frameworks" / String / "score")
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_framework_score);

    list.or(get).or(score)
}

async fn list_frameworks(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let frameworks = state.frameworks.read().await;
    let response = ApiResponse::success(frameworks.clone());
    Ok(warp::reply::json(&response))
}

async fn get_framework(
    framework_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let frameworks = state.frameworks.read().await;

    if let Some(framework) = frameworks.iter().find(|f| f.id == framework_id) {
        let response = ApiResponse::success(framework.clone());
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<Framework> = ApiResponse::error("Framework not found");
        Ok(warp::reply::json(&response))
    }
}

async fn get_framework_score(
    framework_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let scores = state.compliance_scores.read().await;

    if let Some(score) = scores.get(&framework_id) {
        let response = ApiResponse::success(score.clone());
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<ComplianceScore> = ApiResponse::error("Score not found");
        Ok(warp::reply::json(&response))
    }
}

// ============================================================================
// Control Routes
// ============================================================================

fn control_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let list = warp::path!("api" / "compliance" / "frameworks" / String / "controls")
        .and(warp::get())
        .and(warp::query::<ControlFilterParams>())
        .and(with_state(state.clone()))
        .and_then(list_controls);

    let get = warp::path!("api" / "compliance" / "controls" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_control);

    let update_status = warp::path!("api" / "compliance" / "controls" / String / "status")
        .and(warp::put())
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(update_control_status);

    list.or(get).or(update_status)
}

async fn list_controls(
    framework_id: String,
    filters: ControlFilterParams,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let controls = state.controls.read().await;

    if let Some(framework_controls) = controls.get(&framework_id) {
        let filtered: Vec<Control> = framework_controls
            .iter()
            .filter(|c| {
                if let Some(ref status) = filters.status {
                    if c.status.to_string().to_lowercase() != status.to_lowercase() {
                        return false;
                    }
                }
                if let Some(ref owner) = filters.owner {
                    if !c.owner.to_lowercase().contains(&owner.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let response = ApiResponse::success(filtered);
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<Vec<Control>> = ApiResponse::success(vec![]);
        Ok(warp::reply::json(&response))
    }
}

async fn get_control(
    control_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let controls = state.controls.read().await;

    for framework_controls in controls.values() {
        if let Some(control) = framework_controls.iter().find(|c| c.id == control_id) {
            let response = ApiResponse::success(control.clone());
            return Ok(warp::reply::json(&response));
        }
    }

    let response: ApiResponse<Control> = ApiResponse::error("Control not found");
    Ok(warp::reply::json(&response))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

async fn update_control_status(
    control_id: String,
    request: UpdateStatusRequest,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let mut controls = state.controls.write().await;

    for framework_controls in controls.values_mut() {
        if let Some(control) = framework_controls.iter_mut().find(|c| c.id == control_id) {
            let old_status = control.status;
            let new_status = match request.status.to_lowercase().as_str() {
                "pass" => ControlStatus::Pass,
                "fail" => ControlStatus::Fail,
                "pending" => ControlStatus::Pending,
                "notapplicable" | "not_applicable" => ControlStatus::NotApplicable,
                _ => {
                    let response: ApiResponse<()> = ApiResponse::error("Invalid status");
                    return Ok(warp::reply::json(&response));
                }
            };

            control.status = new_status;
            control.updated_at = chrono::Utc::now();

            // Broadcast the change
            state.broadcast(DashboardEvent::ControlStatusChanged {
                control_id: control_id.clone(),
                framework_id: control.framework_id.clone(),
                old_status,
                new_status,
            });

            let response = ApiResponse::success(control.clone());
            return Ok(warp::reply::json(&response));
        }
    }

    let response: ApiResponse<Control> = ApiResponse::error("Control not found");
    Ok(warp::reply::json(&response))
}

// ============================================================================
// Evidence Routes
// ============================================================================

fn evidence_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let list = warp::path!("api" / "compliance" / "controls" / String / "evidence")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_evidence);

    let get = warp::path!("api" / "compliance" / "evidence" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_evidence);

    let submit = warp::path!("api" / "compliance" / "evidence")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(submit_evidence);

    list.or(get).or(submit)
}

async fn list_evidence(
    control_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let evidence = state.evidence.read().await;

    let control_evidence = evidence.get(&control_id).cloned().unwrap_or_default();
    let response = ApiResponse::success(control_evidence);
    Ok(warp::reply::json(&response))
}

async fn get_evidence(
    evidence_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let evidence = state.evidence.read().await;

    for control_evidence in evidence.values() {
        if let Some(ev) = control_evidence.iter().find(|e| e.id == evidence_id) {
            let response = ApiResponse::success(ev.clone());
            return Ok(warp::reply::json(&response));
        }
    }

    let response: ApiResponse<Evidence> = ApiResponse::error("Evidence not found");
    Ok(warp::reply::json(&response))
}

async fn submit_evidence(
    request: SubmitEvidenceRequest,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let evidence_type = match request.evidence_type.to_lowercase().as_str() {
        "screenshot" => EvidenceType::Screenshot,
        "logfile" | "log_file" => EvidenceType::LogFile,
        "configfile" | "config_file" => EvidenceType::ConfigFile,
        "scanreport" | "scan_report" => EvidenceType::ScanReport,
        "policydocument" | "policy_document" => EvidenceType::PolicyDocument,
        "attestation" => EvidenceType::Attestation,
        _ => {
            let response: ApiResponse<()> = ApiResponse::error("Invalid evidence type");
            return Ok(warp::reply::json(&response));
        }
    };

    let mut new_evidence = Evidence::new(
        &uuid::Uuid::new_v4().to_string(),
        &request.control_id,
        evidence_type,
        &request.title,
    );
    new_evidence.collected_by = request.collected_by.clone();
    if let Some(desc) = request.description {
        new_evidence.description = desc;
    }

    let evidence_id = new_evidence.id.clone();
    let control_id = new_evidence.control_id.clone();

    let mut evidence = state.evidence.write().await;
    evidence
        .entry(request.control_id.clone())
        .or_insert_with(Vec::new)
        .push(new_evidence.clone());
    drop(evidence);

    // Broadcast the event
    state.broadcast(DashboardEvent::EvidenceCollected {
        evidence_id,
        control_id,
    });

    let response = ApiResponse::success(new_evidence);
    Ok(warp::reply::json(&response))
}

// ============================================================================
// Asset Routes
// ============================================================================

fn asset_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let list = warp::path!("api" / "compliance" / "assets")
        .and(warp::get())
        .and(warp::query::<AssetFilterParams>())
        .and(with_state(state.clone()))
        .and_then(list_assets);

    let get = warp::path!("api" / "compliance" / "assets" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_asset);

    let compliance = warp::path!("api" / "compliance" / "assets" / String / "compliance")
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_asset_compliance);

    list.or(get).or(compliance)
}

async fn list_assets(
    filters: AssetFilterParams,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let assets = state.assets.read().await;

    let filtered: Vec<Asset> = assets
        .iter()
        .filter(|a| {
            if let Some(ref asset_type) = filters.asset_type {
                if a.asset_type.to_string().to_lowercase() != asset_type.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref criticality) = filters.criticality {
                let c = match a.criticality {
                    Criticality::Critical => "critical",
                    Criticality::High => "high",
                    Criticality::Medium => "medium",
                    Criticality::Low => "low",
                };
                if c != criticality.to_lowercase() {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    let response = ApiResponse::success(filtered);
    Ok(warp::reply::json(&response))
}

async fn get_asset(
    asset_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let assets = state.assets.read().await;

    if let Some(asset) = assets.iter().find(|a| a.id == asset_id) {
        let response = ApiResponse::success(asset.clone());
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<Asset> = ApiResponse::error("Asset not found");
        Ok(warp::reply::json(&response))
    }
}

async fn get_asset_compliance(
    asset_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let assets = state.assets.read().await;

    if let Some(asset) = assets.iter().find(|a| a.id == asset_id) {
        let total_frameworks = asset.compliance_status.len();
        let compliant_frameworks = asset.compliance_status.values().filter(|&&v| v).count();
        let non_compliant_frameworks = total_frameworks - compliant_frameworks;
        let compliance_percentage = if total_frameworks > 0 {
            (compliant_frameworks as f64 / total_frameworks as f64) * 100.0
        } else {
            0.0
        };

        let view = AssetComplianceView {
            asset: asset.clone(),
            total_frameworks,
            compliant_frameworks,
            non_compliant_frameworks,
            compliance_percentage,
            failing_controls: vec![], // Would be populated from actual data
        };

        let response = ApiResponse::success(view);
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<AssetComplianceView> = ApiResponse::error("Asset not found");
        Ok(warp::reply::json(&response))
    }
}

// ============================================================================
// Compliance Score Routes
// ============================================================================

fn score_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let dashboard = warp::path!("api" / "compliance" / "dashboard")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_compliance_dashboard);

    let all_scores = warp::path!("api" / "compliance" / "scores")
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_all_scores);

    dashboard.or(all_scores)
}

/// Dashboard summary response
#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub total_frameworks: usize,
    pub total_controls: usize,
    pub total_assets: usize,
    pub overall_score: f64,
    pub scores_by_framework: Vec<ComplianceScore>,
    pub recent_events: Vec<String>,
    pub alerts: Vec<DashboardAlert>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardAlert {
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

async fn get_compliance_dashboard(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let frameworks = state.frameworks.read().await;
    let controls = state.controls.read().await;
    let assets = state.assets.read().await;
    let scores = state.compliance_scores.read().await;

    let total_frameworks = frameworks.len();
    let total_controls: usize = controls.values().map(|c| c.len()).sum();
    let total_assets = assets.len();

    let scores_list: Vec<ComplianceScore> = scores.values().cloned().collect();
    let overall_score = if !scores_list.is_empty() {
        scores_list.iter().map(|s| s.percentage).sum::<f64>() / scores_list.len() as f64
    } else {
        0.0
    };

    // Generate alerts for low scores
    let mut alerts = Vec::new();
    for score in &scores_list {
        if score.percentage < 50.0 {
            alerts.push(DashboardAlert {
                level: AlertLevel::Critical,
                message: format!("Framework {} has critical compliance score: {:.1}%",
                    score.framework_id, score.percentage),
                timestamp: chrono::Utc::now(),
            });
        } else if score.percentage < 70.0 {
            alerts.push(DashboardAlert {
                level: AlertLevel::Warning,
                message: format!("Framework {} has low compliance score: {:.1}%",
                    score.framework_id, score.percentage),
                timestamp: chrono::Utc::now(),
            });
        }
    }

    let summary = DashboardSummary {
        total_frameworks,
        total_controls,
        total_assets,
        overall_score,
        scores_by_framework: scores_list,
        recent_events: vec![],
        alerts,
    };

    let response = ApiResponse::success(summary);
    Ok(warp::reply::json(&response))
}

async fn get_all_scores(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let scores = state.compliance_scores.read().await;
    let scores_list: Vec<ComplianceScore> = scores.values().cloned().collect();
    let response = ApiResponse::success(scores_list);
    Ok(warp::reply::json(&response))
}

// ============================================================================
// Report Routes
// ============================================================================

fn report_routes(
    state: DashboardState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let list = warp::path!("api" / "compliance" / "reports")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_reports);

    let generate = warp::path!("api" / "compliance" / "reports" / "generate")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(generate_report);

    let get = warp::path!("api" / "compliance" / "reports" / String)
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_report);

    list.or(generate).or(get)
}

async fn list_reports(
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let reports = state.report_jobs.read().await;
    let response = ApiResponse::success(reports.clone());
    Ok(warp::reply::json(&response))
}

async fn generate_report(
    request: GenerateReportRequest,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let report_type = match request.report_type.to_lowercase().as_str() {
        "fullaudit" | "full_audit" => ReportType::FullAudit,
        "executivesummary" | "executive_summary" => ReportType::ExecutiveSummary,
        "controlmatrix" | "control_matrix" => ReportType::ControlMatrix,
        "evidencepackage" | "evidence_package" => ReportType::EvidencePackage,
        _ => {
            let response: ApiResponse<()> = ApiResponse::error("Invalid report type");
            return Ok(warp::reply::json(&response));
        }
    };

    let job = ReportJob::new(
        &uuid::Uuid::new_v4().to_string(),
        report_type,
        request.framework_ids,
        &request.requested_by,
    );

    let job_id = job.id.clone();

    let mut reports = state.report_jobs.write().await;
    reports.push(job.clone());
    drop(reports);

    // Broadcast the event
    state.broadcast(DashboardEvent::ReportStatusUpdate {
        job_id,
        status: ReportStatus::Queued,
        progress: Some(0),
    });

    let response = ApiResponse::success(job);
    Ok(warp::reply::json(&response))
}

async fn get_report(
    report_id: String,
    state: DashboardState,
) -> Result<impl Reply, warp::Rejection> {
    let reports = state.report_jobs.read().await;

    if let Some(report) = reports.iter().find(|r| r.id == report_id) {
        let response = ApiResponse::success(report.clone());
        Ok(warp::reply::json(&response))
    } else {
        let response: ApiResponse<ReportJob> = ApiResponse::error("Report not found");
        Ok(warp::reply::json(&response))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn with_state(
    state: DashboardState,
) -> impl Filter<Extract = (DashboardState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> = ApiResponse::error("Something went wrong");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3, 4, 5];
        let response = PaginatedResponse::new(data, 100, 1, 5);

        assert_eq!(response.total, 100);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 5);
        assert_eq!(response.total_pages, 20);
    }

    #[test]
    fn test_dashboard_summary_serialization() {
        let summary = DashboardSummary {
            total_frameworks: 5,
            total_controls: 100,
            total_assets: 50,
            overall_score: 85.5,
            scores_by_framework: vec![],
            recent_events: vec![],
            alerts: vec![],
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_frameworks\":5"));
        assert!(json.contains("\"overall_score\":85.5"));
    }

    #[test]
    fn test_generate_report_request() {
        let json = r#"{
            "report_type": "full_audit",
            "framework_ids": ["nist-csf", "iso-27001"],
            "requested_by": "admin@example.com"
        }"#;

        let request: GenerateReportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.report_type, "full_audit");
        assert_eq!(request.framework_ids.len(), 2);
        assert_eq!(request.requested_by, "admin@example.com");
    }

    #[test]
    fn test_submit_evidence_request() {
        let json = r#"{
            "control_id": "AC-1",
            "evidence_type": "screenshot",
            "title": "Access Control Screenshot",
            "description": "Screenshot showing access control configuration",
            "collected_by": "auditor@example.com"
        }"#;

        let request: SubmitEvidenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.control_id, "AC-1");
        assert_eq!(request.evidence_type, "screenshot");
        assert!(request.description.is_some());
    }
}
