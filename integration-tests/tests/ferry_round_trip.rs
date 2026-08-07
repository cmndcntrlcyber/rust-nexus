//! WS1 Phase 1b integration test: ferry round-trip via gRPC loopback.
//!
//! Proves: HarnessTask → SubmitHarnessTask RPC → ferry handler → TaskExecutor
//! → HarnessTaskResult round-trip over loopback gRPC.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nexus_a2a::mock::EchoShellHandler;
use nexus_a2a::pb;
use nexus_a2a::{A2aClient, A2aServer, HarnessFerryHandler};
use tokio::time::timeout;

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "ferry-test".into(),
        description: "Ferry round-trip integration test".into(),
        version: "0.1.0".into(),
        skills: vec![pb::AgentSkill {
            id: "harness-tool".into(),
            name: "harness-tool".into(),
            description: "Ferry tool dispatch".into(),
            tags: vec!["v1.5".into()],
        }],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

/// A test ferry handler that echoes back the task's tool_name and arguments.
struct TestFerryHandler {
    invocation_count: AtomicU64,
}

impl TestFerryHandler {
    fn new() -> Self {
        Self {
            invocation_count: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl HarnessFerryHandler for TestFerryHandler {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, tonic::Status> {
        self.invocation_count.fetch_add(1, Ordering::Relaxed);
        Ok(pb::HarnessTaskResult {
            task_id: task.task_id,
            output: format!("echo:{}:{}", task.tool_name, task.json_arguments),
            is_error: false,
            execution_duration_ms: 1,
            bytes_sent: 0,
            bytes_recv: task.json_arguments.len() as u64,
            commands_run: 1,
            kernel_context: None,
        })
    }
}

#[tokio::test]
async fn ferry_grpc_round_trip_unary() {
    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    let handler = TestFerryHandler::new();
    let server = A2aServer::new(agent_card(), EchoShellHandler)
        .with_ferry_handler(handler);

    let _server_task = tokio::spawn(async move {
        use tonic::transport::server::TcpIncoming;
        use tonic::transport::Server;
        let incoming = TcpIncoming::from(listener);
        Server::builder()
            .add_service(server.into_service())
            .serve_with_incoming(incoming)
            .await
            .expect("server");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = A2aClient::connect(&url, false).await.expect("connect");

    let task = pb::HarnessTask {
        task_id: "test-001".into(),
        tool_name: "shell".into(),
        json_arguments: r#"{"command":"whoami"}"#.into(),
        session_id: "sess-001".into(),
        engagement_scope_hash: Vec::new(),
        operator_signature: Vec::new(),
        target_agent_id: "agent-001".into(),
    };

    let result = timeout(Duration::from_secs(5), client.submit_harness_task(task))
        .await
        .expect("timeout")
        .expect("submit_harness_task");

    assert_eq!(result.task_id, "test-001");
    assert!(!result.is_error);
    assert!(result.output.starts_with("echo:shell:"));
    assert!(result.output.contains("whoami"));
    assert!(result.execution_duration_ms >= 0);
}

#[tokio::test]
async fn ferry_grpc_round_trip_streaming() {
    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    let server = A2aServer::new(agent_card(), EchoShellHandler)
        .with_ferry_handler(TestFerryHandler::new());

    let _server_task = tokio::spawn(async move {
        use tonic::transport::server::TcpIncoming;
        use tonic::transport::Server;
        let incoming = TcpIncoming::from(listener);
        Server::builder()
            .add_service(server.into_service())
            .serve_with_incoming(incoming)
            .await
            .expect("server");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = A2aClient::connect(&url, false).await.expect("connect");

    let (tx, mut rx) = client.stream_harness_task().await.expect("stream open");

    for i in 0..3 {
        let task = pb::HarnessTask {
            task_id: format!("stream-{i:03}"),
            tool_name: "process_list".into(),
            json_arguments: "{}".into(),
            session_id: "sess-stream".into(),
            engagement_scope_hash: Vec::new(),
            operator_signature: Vec::new(),
            target_agent_id: "agent-002".into(),
        };
        tx.send(task).await.expect("send");
    }

    for i in 0..3 {
        let result = timeout(Duration::from_secs(5), rx.next())
            .await
            .expect("timeout")
            .expect("stream item")
            .expect("grpc status");

        assert_eq!(result.task_id, format!("stream-{i:03}"));
        assert!(!result.is_error);
    }
}

#[tokio::test]
async fn ferry_error_propagation() {
    let _ = tracing_subscriber::fmt::try_init();

    struct FailingHandler;

    #[async_trait::async_trait]
    impl HarnessFerryHandler for FailingHandler {
        async fn handle_task(
            &self,
            task: pb::HarnessTask,
        ) -> Result<pb::HarnessTaskResult, tonic::Status> {
            Ok(pb::HarnessTaskResult {
                task_id: task.task_id,
                output: "simulated failure".into(),
                is_error: true,
                execution_duration_ms: 0,
                bytes_sent: 0,
                bytes_recv: 0,
                commands_run: 0,
                kernel_context: None,
            })
        }
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    let server = A2aServer::new(agent_card(), EchoShellHandler)
        .with_ferry_handler(FailingHandler);

    let _server_task = tokio::spawn(async move {
        use tonic::transport::server::TcpIncoming;
        use tonic::transport::Server;
        let incoming = TcpIncoming::from(listener);
        Server::builder()
            .add_service(server.into_service())
            .serve_with_incoming(incoming)
            .await
            .expect("server");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = A2aClient::connect(&url, false).await.expect("connect");

    let task = pb::HarnessTask {
        task_id: "fail-001".into(),
        tool_name: "bad_tool".into(),
        json_arguments: "{}".into(),
        session_id: String::new(),
        engagement_scope_hash: Vec::new(),
        operator_signature: Vec::new(),
        target_agent_id: String::new(),
    };

    let result = timeout(Duration::from_secs(5), client.submit_harness_task(task))
        .await
        .expect("timeout")
        .expect("submit");

    assert!(result.is_error);
    assert_eq!(result.output, "simulated failure");
}

use futures::StreamExt;
