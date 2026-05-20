//! v1.1 integration: operator → A2A `SendStreamingMessage` → `EchoShellHandler`
//! → operator round-trip. Proves the A2A wire path works inside the
//! integrated workspace alongside the overlay's existing `NexusC2` service.

use std::time::Duration;

use futures::StreamExt;
use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::mock::EchoShellHandler;
use nexus_a2a::pb;
use nexus_a2a::{A2aClient, A2aServer};
use tokio::time::timeout;

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "v1.1-integration".into(),
        description: "v1.1 simple-mesh integration loopback".into(),
        version: "0.1.0".into(),
        skills: vec![pb::AgentSkill {
            id: "shell-session".into(),
            name: "shell-session".into(),
            description: "Echo-mode loopback for v1.1 verification.".into(),
            tags: vec!["v1.1".into()],
        }],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

#[tokio::test]
async fn echo_round_trip_with_overlay_workspace() {
    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    let server = A2aServer::new(agent_card(), EchoShellHandler);
    let server_task = tokio::spawn(async move {
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
    let card = client.get_agent_card().await.expect("get_agent_card");
    assert_eq!(card.name, "v1.1-integration");

    let (tx, mut rx) = client.open_streaming_message().await.expect("open stream");
    let task_id = "v1.1-loopback";

    let open = ShellControl::ShellOpen {
        cols: 80,
        rows: 24,
        shell: None,
        target_agent_id: None,
    };
    tx.send(control_request(task_id, &open).expect("control"))
        .await
        .expect("send open");

    tx.send(bytes_request(task_id, b"hello".to_vec()))
        .await
        .expect("send hello");
    tx.send(bytes_request(task_id, b" v1.1".to_vec()))
        .await
        .expect("send v1.1");

    let mut accumulated = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while accumulated.len() < b"hello v1.1".len() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let item = timeout(remaining, rx.next())
            .await
            .expect("recv timeout")
            .expect("stream closed early")
            .expect("status");
        if let Some(pb::stream_response::Payload::Message(msg)) = item.payload {
            for part in msg.parts {
                if let Some(pb::part::Part::File(bytes)) = part.part {
                    accumulated.extend_from_slice(&bytes);
                }
            }
        }
    }
    assert_eq!(accumulated, b"hello v1.1");

    server_task.abort();
}

#[tokio::test]
async fn list_registered_agents_works_with_null_lister() {
    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    let server = A2aServer::new(agent_card(), EchoShellHandler);
    let server_task = tokio::spawn(async move {
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
    let agents = client
        .list_registered_agents()
        .await
        .expect("list_registered_agents");
    assert!(agents.is_empty(), "default lister returns empty list");

    server_task.abort();
}
