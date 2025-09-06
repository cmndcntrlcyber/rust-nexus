//! Web UI request handlers

use warp::Reply;
use serde_json::json;
use crate::{WebUIState, TaskExecutionRequest, TaskExecutionResponse};

/// List all connected agents
pub async fn list_agents(state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    let agents = state.agent_connections.read().await;
    Ok(warp::reply::json(&*agents))
}

/// Get agent details by ID
pub async fn get_agent_details(agent_id: String, state: WebUIState) -> Result<Box<dyn Reply>, warp::Rejection> {
    let agents = state.agent_connections.read().await;
    
    if let Some(agent) = agents.get(&agent_id) {
        Ok(Box::new(warp::reply::json(agent)))
    } else {
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Agent not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )))
    }
}

/// Execute task on agent
pub async fn execute_task(
    agent_id: String,
    request: TaskExecutionRequest,
    state: WebUIState,
) -> Result<impl Reply, warp::Rejection> {
    // TODO: Integrate with gRPC client to send task to agent
    let response = TaskExecutionResponse {
        task_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        result: None,
        error: None,
        timestamp: chrono::Utc::now(),
    };
    
    Ok(warp::reply::json(&response))
}

/// List managed domains
pub async fn list_domains(state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    // TODO: Integrate with domain manager
    Ok(warp::reply::json(&json!({"domains": []})))
}

/// Trigger domain rotation
pub async fn rotate_domain(state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    // TODO: Integrate with domain manager
    Ok(warp::reply::json(&json!({"status": "rotation_initiated"})))
}

/// Get system information
pub async fn get_system_info(state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    let info = json!({
        "service": "nexus-webui",
        "version": "0.1.0",
        "agents_connected": state.agent_connections.read().await.len(),
        "timestamp": chrono::Utc::now()
    });
    
    Ok(warp::reply::json(&info))
}
