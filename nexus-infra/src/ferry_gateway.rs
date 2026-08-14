//! v3.10 WS1 — REST ferry gateway.
//!
//! Axum HTTP router that proxies REST calls to the A2A gRPC ferry RPCs.
//! Co-hosted with the Prometheus `/metrics` endpoint on port 9100.
//! RTPI calls these endpoints with plain `fetch()` — no gRPC client needed.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
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
use nexus_a2a::A2aClient;
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
// v1.7 JSON request/response types — operator chat, steering, approvals
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatRequestJson {
    #[serde(default)]
    pub mode: i32,
    pub content: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub engagement_id: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ChatChunkJson {
    pub text: String,
    pub done: bool,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ChatChunkMetaJson>,
}

#[derive(Debug, Serialize)]
pub struct ChatChunkMetaJson {
    pub model_used: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tools_invoked: Vec<String>,
    pub task_id: String,
    pub latency_ms: f32,
}

#[derive(Debug, Deserialize)]
pub struct SteerRequestJson {
    pub agent_id: String,
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub action: i32,
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub operator_id: String,
}

#[derive(Debug, Serialize)]
pub struct SteerResponseJson {
    pub success: bool,
    pub message: String,
    pub previous_state: String,
    pub new_state: String,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalDecisionJson {
    pub approval_id: String,
    pub approved: bool,
    #[serde(default)]
    pub operator_id: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub decided_at_unix: u64,
}

#[derive(Debug, Serialize)]
pub struct ApprovalRequestJson {
    pub approval_id: String,
    pub skill_name: String,
    pub target: String,
    pub risk_level: String,
    pub requesting_agent: String,
    pub description: String,
    pub techniques: Vec<String>,
    pub created_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Debug, Serialize)]
pub struct ApprovalQueueResponseJson {
    pub pending: Vec<ApprovalRequestJson>,
    pub total_pending: u32,
    pub approved_today: u32,
    pub denied_today: u32,
}

#[derive(Debug, Serialize)]
pub struct OperatorNotificationJson {
    pub id: String,
    pub severity: i32,
    pub title: String,
    pub body: String,
    pub source: String,
    pub timestamp_unix: u64,
    pub requires_action: bool,
    pub action_ref: String,
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
    /// v1.7 — A2A client for operator chat, steering, and approval RPCs.
    /// Clone per-request (tonic `Channel` multiplexes internally).
    pub a2a_client: Option<A2aClient>,
}

impl std::fmt::Debug for FerryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FerryState")
            .field("agent_card_name", &self.agent_card_name)
            .field("agent_card_version", &self.agent_card_version)
            .field("a2a_client", &self.a2a_client.is_some())
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
// v1.7 route handlers — operator chat, steering, approvals
// ---------------------------------------------------------------------------

async fn post_ferry_chat(
    State(state): State<FerryState>,
    Json(req): Json<ChatRequestJson>,
) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    let grpc_req = pb::ChatRequest {
        mode: req.mode,
        content: req.content,
        session_id: req.session_id,
        engagement_id: req.engagement_id,
        metadata: req.metadata,
    };

    let mut grpc_stream = match client.stream_chat(grpc_req).await {
        Ok(s) => s,
        Err(status) => {
            warn!(error = %status, "stream_chat failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": format!("{status}"),
                    "code": status.code() as i32,
                })),
            )
                .into_response();
        }
    };

    let stream = async_stream::stream! {
        loop {
            match grpc_stream.message().await {
                Ok(Some(chunk)) => {
                    let json = ChatChunkJson {
                        text: chunk.text,
                        done: chunk.done,
                        session_id: chunk.session_id,
                        meta: chunk.meta.map(|m| ChatChunkMetaJson {
                            model_used: m.model_used,
                            input_tokens: m.input_tokens,
                            output_tokens: m.output_tokens,
                            tools_invoked: m.tools_invoked,
                            task_id: m.task_id,
                            latency_ms: m.latency_ms,
                        }),
                    };
                    let data = serde_json::to_string(&json).unwrap_or_default();
                    yield Ok::<_, Infallible>(Event::default().data(data));
                }
                Ok(None) => break,
                Err(status) => {
                    let err = serde_json::json!({
                        "error": format!("{status}"),
                        "code": status.code() as i32,
                    });
                    yield Ok::<_, Infallible>(
                        Event::default()
                            .event("error")
                            .data(serde_json::to_string(&err).unwrap_or_default()),
                    );
                    break;
                }
            }
        }
    };

    Sse::new(stream).into_response()
}

async fn post_ferry_steer(
    State(state): State<FerryState>,
    Json(req): Json<SteerRequestJson>,
) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    let grpc_req = pb::SteerRequest {
        agent_id: req.agent_id,
        task_id: req.task_id,
        action: req.action,
        instruction: req.instruction,
        operator_id: req.operator_id,
    };

    match client.steer_agent(grpc_req).await {
        Ok(resp) => Json(SteerResponseJson {
            success: resp.success,
            message: resp.message,
            previous_state: resp.previous_state,
            new_state: resp.new_state,
        })
        .into_response(),
        Err(status) => {
            warn!(error = %status, "steer_agent failed");
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

async fn get_ferry_approvals(State(state): State<FerryState>) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    match client.get_approval_queue().await {
        Ok(resp) => {
            let pending: Vec<ApprovalRequestJson> = resp
                .pending
                .into_iter()
                .map(|a| ApprovalRequestJson {
                    approval_id: a.approval_id,
                    skill_name: a.skill_name,
                    target: a.target,
                    risk_level: a.risk_level,
                    requesting_agent: a.requesting_agent,
                    description: a.description,
                    techniques: a.techniques,
                    created_at_unix: a.created_at_unix,
                    expires_at_unix: a.expires_at_unix,
                })
                .collect();
            Json(ApprovalQueueResponseJson {
                pending,
                total_pending: resp.total_pending,
                approved_today: resp.approved_today,
                denied_today: resp.denied_today,
            })
            .into_response()
        }
        Err(status) => {
            warn!(error = %status, "get_approval_queue failed");
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

async fn post_ferry_approve(
    State(state): State<FerryState>,
    Json(req): Json<ApprovalDecisionJson>,
) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    let decision = pb::ApprovalDecision {
        approval_id: req.approval_id,
        approved: req.approved,
        operator_id: req.operator_id,
        reason: req.reason,
        decided_at_unix: req.decided_at_unix,
    };

    match client.submit_approval(decision).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(status) => {
            warn!(error = %status, "submit_approval failed");
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

async fn get_ferry_approvals_stream(State(state): State<FerryState>) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    let mut grpc_stream = match client.stream_approvals().await {
        Ok(s) => s,
        Err(status) => {
            warn!(error = %status, "stream_approvals failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": format!("{status}"),
                    "code": status.code() as i32,
                })),
            )
                .into_response();
        }
    };

    let stream = async_stream::stream! {
        loop {
            match grpc_stream.message().await {
                Ok(Some(req)) => {
                    let json = ApprovalRequestJson {
                        approval_id: req.approval_id,
                        skill_name: req.skill_name,
                        target: req.target,
                        risk_level: req.risk_level,
                        requesting_agent: req.requesting_agent,
                        description: req.description,
                        techniques: req.techniques,
                        created_at_unix: req.created_at_unix,
                        expires_at_unix: req.expires_at_unix,
                    };
                    let data = serde_json::to_string(&json).unwrap_or_default();
                    yield Ok::<_, Infallible>(Event::default().data(data));
                }
                Ok(None) => break,
                Err(status) => {
                    let err = serde_json::json!({
                        "error": format!("{status}"),
                        "code": status.code() as i32,
                    });
                    yield Ok::<_, Infallible>(
                        Event::default()
                            .event("error")
                            .data(serde_json::to_string(&err).unwrap_or_default()),
                    );
                    break;
                }
            }
        }
    };

    Sse::new(stream).into_response()
}

async fn get_ferry_notifications(State(state): State<FerryState>) -> impl IntoResponse {
    let Some(ref a2a) = state.a2a_client else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A2A client not configured"})),
        )
            .into_response();
    };
    let mut client = a2a.clone();

    let mut grpc_stream = match client.stream_notifications().await {
        Ok(s) => s,
        Err(status) => {
            warn!(error = %status, "stream_notifications failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": format!("{status}"),
                    "code": status.code() as i32,
                })),
            )
                .into_response();
        }
    };

    let stream = async_stream::stream! {
        loop {
            match grpc_stream.message().await {
                Ok(Some(notif)) => {
                    let json = OperatorNotificationJson {
                        id: notif.id,
                        severity: notif.severity,
                        title: notif.title,
                        body: notif.body,
                        source: notif.source,
                        timestamp_unix: notif.timestamp_unix,
                        requires_action: notif.requires_action,
                        action_ref: notif.action_ref,
                    };
                    let data = serde_json::to_string(&json).unwrap_or_default();
                    yield Ok::<_, Infallible>(Event::default().data(data));
                }
                Ok(None) => break,
                Err(status) => {
                    let err = serde_json::json!({
                        "error": format!("{status}"),
                        "code": status.code() as i32,
                    });
                    yield Ok::<_, Infallible>(
                        Event::default()
                            .event("error")
                            .data(serde_json::to_string(&err).unwrap_or_default()),
                    );
                    break;
                }
            }
        }
    };

    Sse::new(stream).into_response()
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
        // v1.7 — operator chat, steering, approvals
        .route("/ferry/chat", post(post_ferry_chat))
        .route("/ferry/steer", post(post_ferry_steer))
        .route("/ferry/approvals", get(get_ferry_approvals))
        .route("/ferry/approve", post(post_ferry_approve))
        .route("/ferry/approvals/stream", get(get_ferry_approvals_stream))
        .route("/ferry/notifications", get(get_ferry_notifications))
        .with_state(state)
}
