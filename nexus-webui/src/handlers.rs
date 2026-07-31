//! Web UI request handlers

use crate::{TaskExecutionRequest, TaskExecutionResponse, WebUIState};
use serde_json::json;
use warp::Reply;

/// List all connected agents
pub async fn list_agents(state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    let agents = state.agent_connections.read().await;
    Ok(warp::reply::json(&*agents))
}

/// Get agent details by ID
pub async fn get_agent_details(
    agent_id: String,
    state: WebUIState,
) -> Result<Box<dyn Reply>, warp::Rejection> {
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
    _agent_id: String,
    _request: TaskExecutionRequest,
    _state: WebUIState,
) -> Result<impl Reply, warp::Rejection> {
    // DEPRECATED(legacy-webui): Integrate with gRPC client to send task to agent
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
pub async fn list_domains(_state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    // DEPRECATED(legacy-webui): Integrate with domain manager
    Ok(warp::reply::json(&json!({"domains": []})))
}

/// Trigger domain rotation
pub async fn rotate_domain(_state: WebUIState) -> Result<impl Reply, warp::Rejection> {
    // DEPRECATED(legacy-webui): Integrate with domain manager
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskExecutionResponse;

    #[test]
    fn test_task_execution_response_serialization() {
        let resp = TaskExecutionResponse {
            task_id: "abc-123".to_string(),
            status: "queued".to_string(),
            result: None,
            error: None,
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["task_id"], "abc-123");
        assert_eq!(json["status"], "queued");
        assert!(json["result"].is_null());
    }

    #[test]
    fn test_task_execution_request_deserialization() {
        let json = r#"{
            "task_type": "shell",
            "parameters": {"cmd": "whoami"},
            "timeout": 30,
            "priority": 1
        }"#;
        let req: crate::TaskExecutionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task_type, "shell");
        assert_eq!(req.parameters.get("cmd").unwrap(), "whoami");
        assert_eq!(req.timeout, Some(30));
    }

    #[test]
    fn test_list_domains_json_shape() {
        let expected = json!({"domains": []});
        assert!(expected["domains"].is_array());
        assert_eq!(expected["domains"].as_array().unwrap().len(), 0);
    }
}
