//! v3.10 WS1 — REST ferry gateway.
//!
//! Axum HTTP router that proxies REST calls to the A2A gRPC ferry RPCs.
//! Co-hosted with the Prometheus `/metrics` endpoint on port 9100.
//! RTPI calls these endpoints with plain `fetch()` — no gRPC client needed.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

use nexus_a2a::ferry_handler::HarnessFerryHandler;
use nexus_a2a::gml::GmlAdjustmentLayer;
use nexus_a2a::pb;
use nexus_a2a::situational_awareness::SituationalAwareness;
use nexus_mesh::telemetry::TelemetrySnapshot;

// ---------------------------------------------------------------------------
// JSON request/response types mirroring proto messages
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct FerryTaskRequest {
    pub task_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub json_arguments: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub target_agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct FerryTaskResponse {
    pub task_id: String,
    pub output: String,
    pub is_error: bool,
    pub execution_duration_ms: u64,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub commands_run: u32,
}

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub peer_id: String,
    pub os: String,
    pub version: String,
    pub tag: String,
    pub last_seen_unix: u64,
}

#[derive(Debug, Serialize)]
pub struct AgentsResponse {
    pub agents: Vec<AgentInfo>,
}

#[derive(Debug, Serialize)]
pub struct AnomalyResponse {
    pub barometer: f64,
    pub agent_attributions: HashMap<String, f64>,
    pub throttling_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct TelemetryRequest {
    pub window_start: u64,
    pub window_end: u64,
    #[serde(default)]
    pub node_features: HashMap<String, NodeFeatures>,
}

#[derive(Debug, Deserialize)]
pub struct NodeFeatures {
    pub peer_id: String,
    #[serde(default)]
    pub message_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct RateAdjustRequest {
    pub agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct RateAdjustResponse {
    pub agent_id: String,
    pub multiplier: f64,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub server_name: String,
    pub version: String,
    pub agents_connected: usize,
}

// ---------------------------------------------------------------------------
// Shared state for the gateway handlers
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct FerryState {
    pub ferry_handler: Arc<dyn HarnessFerryHandler>,
    pub situational_awareness: Arc<SituationalAwareness>,
    pub gml: Arc<Mutex<GmlAdjustmentLayer>>,
    pub agent_card_name: String,
    pub agent_card_version: String,
}

impl std::fmt::Debug for FerryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FerryState")
            .field("agent_card_name", &self.agent_card_name)
            .field("agent_card_version", &self.agent_card_version)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn post_ferry_task(
    State(state): State<FerryState>,
    Json(req): Json<FerryTaskRequest>,
) -> impl IntoResponse {
    let task = pb::HarnessTask {
        task_id: req.task_id,
        tool_name: req.tool_name,
        json_arguments: req.json_arguments,
        session_id: req.session_id,
        engagement_scope_hash: Vec::new(),
        operator_signature: Vec::new(),
        target_agent_id: req.target_agent_id,
    };

    match state.ferry_handler.handle_task(task).await {
        Ok(result) => Json(FerryTaskResponse {
            task_id: result.task_id,
            output: result.output,
            is_error: result.is_error,
            execution_duration_ms: result.execution_duration_ms,
            bytes_sent: result.bytes_sent,
            bytes_recv: result.bytes_recv,
            commands_run: result.commands_run,
        })
        .into_response(),
        Err(status) => {
            warn!(error = %status, "ferry task failed");
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": format!("{status}"),
                    "code": status.code() as i32,
                })),
            )
                .into_response()
        }
    }
}

async fn get_ferry_agents(State(state): State<FerryState>) -> impl IntoResponse {
    let snapshots = state.situational_awareness.all_agents().await;
    let agents: Vec<AgentInfo> = snapshots
        .into_iter()
        .map(|a| AgentInfo {
            peer_id: a.peer_id_hex,
            os: a.os,
            version: a.version,
            tag: a.tag,
            last_seen_unix: a.last_seen_unix,
        })
        .collect();
    Json(AgentsResponse { agents })
}

async fn get_ferry_anomaly(State(state): State<FerryState>) -> impl IntoResponse {
    let gml = state.gml.lock().await;
    let attributions: HashMap<String, f64> = gml
        .agent_attributions_iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    Json(AnomalyResponse {
        barometer: gml.barometer(),
        agent_attributions: attributions,
        throttling_active: gml.should_throttle(),
    })
}

async fn post_ferry_telemetry(
    State(state): State<FerryState>,
    Json(req): Json<TelemetryRequest>,
) -> impl IntoResponse {
    let mut features = HashMap::new();
    for (k, v) in req.node_features {
        features.insert(
            k,
            nexus_mesh::telemetry::NodeFeatures {
                peer_id: v.peer_id,
                message_count: v.message_count,
                bytes_sent: 0,
                bytes_recv: 0,
                task_count: 0,
                error_rate: 0.0,
                total_syscalls: 0,
                total_memory_allocated: 0,
                total_processes_spawned: 0,
                total_files_touched: 0,
                total_network_connections: 0,
                sandbox_violations: 0,
            },
        );
    }
    let snapshot = TelemetrySnapshot {
        window_start: req.window_start,
        window_end: req.window_end,
        node_features: features,
        edge_features: Vec::new(),
    };
    let mut gml = state.gml.lock().await;
    gml.ingest_snapshot(&snapshot);
    StatusCode::ACCEPTED
}

async fn post_ferry_rate_adjust(
    State(state): State<FerryState>,
    Json(req): Json<RateAdjustRequest>,
) -> impl IntoResponse {
    let gml = state.gml.lock().await;
    let multiplier = gml.rate_adjustment(&req.agent_id);
    Json(RateAdjustResponse {
        agent_id: req.agent_id,
        multiplier,
    })
}

async fn get_ferry_health(State(state): State<FerryState>) -> impl IntoResponse {
    let agent_count = state.situational_awareness.agent_count().await;
    Json(HealthResponse {
        status: "ok".into(),
        server_name: state.agent_card_name.clone(),
        version: state.agent_card_version.clone(),
        agents_connected: agent_count,
    })
}

// ---------------------------------------------------------------------------
// Router constructor
// ---------------------------------------------------------------------------

pub fn ferry_router(state: FerryState) -> Router {
    Router::new()
        .route("/ferry/task", post(post_ferry_task))
        .route("/ferry/agents", get(get_ferry_agents))
        .route("/ferry/anomaly", get(get_ferry_anomaly))
        .route("/ferry/telemetry", post(post_ferry_telemetry))
        .route("/ferry/rate-adjust", post(post_ferry_rate_adjust))
        .route("/ferry/health", get(get_ferry_health))
        .with_state(state)
}
