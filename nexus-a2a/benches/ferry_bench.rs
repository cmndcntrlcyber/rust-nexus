//! WS1 Phase 1b: ferry latency benchmark.
//!
//! Measures round-trip latency of `SubmitHarnessTask` over gRPC loopback
//! with a no-op ferry handler to establish baseline transport overhead.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nexus_a2a::mock::EchoShellHandler;
use nexus_a2a::pb;
use nexus_a2a::{A2aClient, A2aServer, HarnessFerryHandler};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "ferry-bench".into(),
        description: "Ferry latency benchmark".into(),
        version: "0.1.0".into(),
        skills: vec![],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

struct NoOpFerryHandler;

#[async_trait::async_trait]
impl HarnessFerryHandler for NoOpFerryHandler {
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, tonic::Status> {
        Ok(pb::HarnessTaskResult {
            task_id: task.task_id,
            output: "ok".into(),
            is_error: false,
            execution_duration_ms: 0,
            bytes_sent: 0,
            bytes_recv: 0,
            commands_run: 1,
            kernel_context: None,
        })
    }
}

fn bench_ferry_grpc_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let (url, _server_task) = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let url = format!("http://{addr}");

        let server = A2aServer::new(agent_card(), EchoShellHandler)
            .with_ferry_handler(NoOpFerryHandler);

        let task = tokio::spawn(async move {
            use tonic::transport::server::TcpIncoming;
            use tonic::transport::Server;
            let incoming = TcpIncoming::from(listener);
            Server::builder()
                .add_service(server.into_service())
                .serve_with_incoming(incoming)
                .await
                .expect("server");
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        (url, task)
    });

    let mut client = rt.block_on(A2aClient::connect(&url, false)).expect("connect");

    let counter = AtomicU64::new(0);

    c.bench_function("ferry_grpc_unary_round_trip", |b| {
        b.iter(|| {
            let i = counter.fetch_add(1, Ordering::Relaxed);
            let task = pb::HarnessTask {
                task_id: format!("bench-{i}"),
                tool_name: "noop".into(),
                json_arguments: "{}".into(),
                session_id: String::new(),
                engagement_scope_hash: String::new(),
                operator_signature: String::new(),
                target_agent_id: String::new(),
            };
            let result = rt
                .block_on(client.submit_harness_task(black_box(task)))
                .expect("submit");
            assert!(!result.is_error);
        })
    });

    c.bench_function("ferry_grpc_unary_1kb_payload", |b| {
        let big_args = format!("{{\"data\":\"{}\"}}", "x".repeat(1024));
        b.iter(|| {
            let i = counter.fetch_add(1, Ordering::Relaxed);
            let task = pb::HarnessTask {
                task_id: format!("bench-1k-{i}"),
                tool_name: "noop".into(),
                json_arguments: big_args.clone(),
                session_id: String::new(),
                engagement_scope_hash: String::new(),
                operator_signature: String::new(),
                target_agent_id: String::new(),
            };
            let result = rt
                .block_on(client.submit_harness_task(black_box(task)))
                .expect("submit");
            assert!(!result.is_error);
        })
    });
}

criterion_group!(benches, bench_ferry_grpc_latency);
criterion_main!(benches);
