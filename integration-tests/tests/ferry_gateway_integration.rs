//! Integration test: REST ferry gateway HTTP round-trip.
//!
//! Spins up the axum ferry router with a test ferry handler, then exercises
//! each endpoint via reqwest to prove the REST surface works end-to-end
//! without requiring a live gRPC backend.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use nexus_a2a::gml::GmlAdjustmentLayer;
use nexus_a2a::pb;
use nexus_a2a::situational_awareness::SituationalAwareness;
use nexus_a2a::HarnessFerryHandler;
use nexus_infra::ferry_gateway::{ferry_router, FerryState};
use serde_json::Value;
use tokio::sync::Mutex;

/// A test ferry handler that echoes back the task's tool_name and arguments.
struct TestFerryHandler;

#[async_trait]
impl HarnessFerryHandler for TestFerryHandler {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, tonic::Status> {
        Ok(pb::HarnessTaskResult {
            task_id: task.task_id,
            output: format!("echo:{}:{}", task.tool_name, task.json_arguments),
            is_error: false,
            execution_duration_ms: 1,
            bytes_sent: 0,
            bytes_recv: task.json_arguments.len() as u64,
            commands_run: 1,
            kernel_context: None,
            progress_percent: None,
            stage: None,
            intermediate_output: None,
        })
    }
}

/// Build a FerryState with test defaults and spawn the gateway on a random port.
/// Returns the base URL (e.g. `http://127.0.0.1:12345`).
async fn spawn_gateway() -> String {
    let sa = Arc::new(SituationalAwareness::new());
    let gml = Arc::new(Mutex::new(GmlAdjustmentLayer::new()));
    let handler: Arc<dyn HarnessFerryHandler> = Arc::new(TestFerryHandler);

    let state = FerryState {
        ferry_handler: handler,
        situational_awareness: sa,
        gml,
        agent_card_name: "test-server".into(),
        agent_card_version: "0.1.0".into(),
        a2a_client: None,
    };

    let app = ferry_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    // Give the server a moment to start accepting connections.
    tokio::time::sleep(Duration::from_millis(100)).await;

    base
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let base = tokio::time::timeout(Duration::from_secs(10), spawn_gateway())
        .await
        .expect("gateway startup timed out");

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ferry/health"))
        .send()
        .await
        .expect("health request");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("health json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["server_name"], "test-server");
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["agents_connected"], 0);
}

#[tokio::test]
async fn task_endpoint_echoes_back() {
    let base = tokio::time::timeout(Duration::from_secs(10), spawn_gateway())
        .await
        .expect("gateway startup timed out");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/ferry/task"))
        .json(&serde_json::json!({
            "task_id": "t1",
            "tool_name": "echo",
            "json_arguments": "{}"
        }))
        .send()
        .await
        .expect("task request");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("task json");
    assert_eq!(body["task_id"], "t1");
    assert_eq!(body["is_error"], false);
    assert!(
        body["output"].as_str().unwrap().contains("echo:echo:{}"),
        "expected echo output, got: {}",
        body["output"]
    );
    assert_eq!(body["execution_duration_ms"], 1);
    assert_eq!(body["commands_run"], 1);
}

#[tokio::test]
async fn agents_endpoint_returns_array() {
    let base = tokio::time::timeout(Duration::from_secs(10), spawn_gateway())
        .await
        .expect("gateway startup timed out");

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ferry/agents"))
        .send()
        .await
        .expect("agents request");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("agents json");
    assert!(body["agents"].is_array(), "expected agents array");
    // No agents registered in the test fixture, so the array is empty.
    assert_eq!(body["agents"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn anomaly_endpoint_returns_barometer() {
    let base = tokio::time::timeout(Duration::from_secs(10), spawn_gateway())
        .await
        .expect("gateway startup timed out");

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ferry/anomaly"))
        .send()
        .await
        .expect("anomaly request");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("anomaly json");
    assert!(body.get("barometer").is_some(), "expected barometer field");
    assert_eq!(body["barometer"], 0.0, "initial barometer should be 0.0");
    assert_eq!(body["throttling_active"], false);
}

#[tokio::test]
async fn situational_endpoint_streams_sse() {
    let base = tokio::time::timeout(Duration::from_secs(10), spawn_gateway())
        .await
        .expect("gateway startup timed out");

    let client = reqwest::Client::new();

    // The SSE endpoint loops forever (5s between events), so we read just the
    // first chunk and verify it contains the expected event type.
    let mut resp = client
        .get(format!("{base}/ferry/situational"))
        .send()
        .await
        .expect("situational request");

    assert_eq!(resp.status(), 200);

    // Read the first chunk within a timeout (the first event is emitted
    // immediately before the 5-second sleep).
    let chunk = tokio::time::timeout(Duration::from_secs(5), resp.chunk())
        .await
        .expect("chunk read timed out")
        .expect("chunk read error")
        .expect("expected non-empty first chunk");

    let text = String::from_utf8_lossy(&chunk);
    assert!(
        text.contains("event: situational-update"),
        "expected SSE event header 'event: situational-update', got: {text}"
    );
    // The data payload is JSON containing agent_count and barometer.
    assert!(
        text.contains("agent_count"),
        "expected agent_count in SSE data, got: {text}"
    );
}
