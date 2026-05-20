//! In-process loopback test using the `EchoShellHandler` mock.

use std::time::Duration;

use futures::StreamExt;
use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::mock::EchoShellHandler;
use nexus_a2a::pb;
use nexus_a2a::{A2aClient, A2aServer};
use tokio::time::timeout;

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "test-agent".into(),
        description: "Echo handler for the loopback test".into(),
        version: "0.1.0".into(),
        skills: vec![pb::AgentSkill {
            id: "shell-session".into(),
            name: "shell-session".into(),
            description: "echoes inbound bytes (mock)".into(),
            tags: vec!["v1.1".into()],
        }],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

// v1.4.7 finish — gRPC metadata token extraction at dispatch time.

#[tokio::test]
async fn v1_4_7_operator_token_metadata_extracted_at_dispatch() {
    use nexus_a2a::framing::{control_request, ShellControl};
    use nexus_a2a::tokens::{OperatorToken, TOKEN_METADATA_KEY};
    use nexus_common::NodeIdentity;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    // Server with a NodeIdentity so the IssueOperatorToken/dispatch
    // path can verify tokens.
    let identity = Arc::new(NodeIdentity::from_seed(&[33u8; 32]));
    let server =
        A2aServer::new(agent_card(), EchoShellHandler).with_server_identity(identity.clone());
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
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Issue a token directly (skipping the IssueOperatorToken RPC for
    // brevity — the v1.4 round-2 test already covers that surface).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (token_bytes, _) = OperatorToken::issue(&identity, [0xab; 16], now, 3600);
    let token_hex: String = token_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Send a streaming-message request with the token in metadata.
    // The dispatch path should accept it (well-formed token) and
    // proceed to the shell-open echo loop.
    let channel = tonic::transport::Channel::from_shared(url.clone())
        .expect("endpoint")
        .connect()
        .await
        .expect("connect");
    let mut client = pb::a2a_service_client::A2aServiceClient::new(channel);

    let (tx, rx) = tokio::sync::mpsc::channel::<pb::Message>(8);
    let outbound = tokio_stream::wrappers::ReceiverStream::new(rx);

    let mut req = tonic::Request::new(outbound);
    req.metadata_mut().insert(
        TOKEN_METADATA_KEY,
        token_hex.parse().expect("metadata value"),
    );
    let response = client.send_streaming_message(req).await.expect("send");
    let mut inbound = response.into_inner();

    // First-frame shell-open + echo.
    let open = ShellControl::ShellOpen {
        cols: 80,
        rows: 24,
        shell: None,
        target_agent_id: None,
    };
    tx.send(control_request("op-v1.4.7", &open).expect("encode"))
        .await
        .expect("send");

    // Bytes round-trip via the echo handler.
    tx.send(nexus_a2a::framing::bytes_request(
        "op-v1.4.7",
        b"v1.4.7-marker".to_vec(),
    ))
    .await
    .expect("send bytes");

    use futures::StreamExt as _;
    let mut accumulated = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while accumulated.len() < b"v1.4.7-marker".len() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let response = tokio::time::timeout(remaining, inbound.next())
            .await
            .expect("timeout")
            .expect("closed")
            .expect("status");
        if let Some(pb::stream_response::Payload::Message(msg)) = response.payload {
            for part in msg.parts {
                if let Some(pb::part::Part::File(bytes)) = part.part {
                    accumulated.extend_from_slice(&bytes);
                }
            }
        }
    }
    assert_eq!(&accumulated, b"v1.4.7-marker");

    server_task.abort();
}

// v1.4.3 / v1.4.7 — audit streaming + operator-token issuance.

#[tokio::test]
async fn v1_4_audit_streaming_and_token_issuance() {
    use nexus_a2a::audit::{make_record, AuditSink, BroadcastSink, MemSink};
    use nexus_common::NodeIdentity;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt::try_init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    // Server with both a broadcast audit sink and a NodeIdentity.
    let primary = Arc::new(MemSink::new()) as Arc<dyn AuditSink>;
    let broadcast = Arc::new(BroadcastSink::new(primary.clone()));
    let identity = Arc::new(NodeIdentity::from_seed(&[7u8; 32]));
    let server_pubkey = identity.ed25519_public();

    let server = A2aServer::new(agent_card(), EchoShellHandler)
        .with_broadcast_audit(broadcast.clone())
        .with_server_identity(identity.clone());

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
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut client = pb::a2a_service_client::A2aServiceClient::connect(url.clone())
        .await
        .expect("connect");

    // -- Issue an operator token.
    let resp = client
        .issue_operator_token(tonic::Request::new(pb::IssueOperatorTokenRequest {
            operator_id: vec![0xab; 16],
            lifetime_seconds: 3600,
        }))
        .await
        .expect("issue token")
        .into_inner();
    assert_eq!(resp.token.len(), 97); // TOKEN_LEN
    assert!(resp.expires_unix > resp.issued_unix);

    // Verify the token decodes against the server's public key.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let decoded =
        nexus_a2a::tokens::OperatorToken::decode_verified(&resp.token, &server_pubkey, now + 1)
            .expect("decode");
    assert_eq!(decoded.operator_id, [0xab; 16]);

    // -- StreamAuditRecords: open a subscription, then emit a record on
    //    the server-side broadcast sink and confirm we observe it.
    let mut stream = client
        .stream_audit_records(tonic::Request::new(pb::StreamAuditRecordsRequest {
            actor_filter: String::new(),
            action_filter: String::new(),
            since_unix: 0,
        }))
        .await
        .expect("subscribe")
        .into_inner();

    // Give the subscription a moment to register.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    broadcast
        .append(make_record("test-actor", "shell_session_open", "ab12"))
        .await;

    let event = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        futures::StreamExt::next(&mut stream),
    )
    .await
    .expect("recv timeout")
    .expect("stream closed")
    .expect("status");
    assert_eq!(event.actor, "test-actor");
    assert_eq!(event.action, "shell_session_open");
    assert_eq!(event.resource, "ab12");

    server_task.abort();
}

// v1.3.1 — confirm the new upstream-style RPCs are reachable on the
// wire and return Unimplemented (D-V1.3-A).

#[tokio::test]
async fn v1_3_upstream_stubs_return_unimplemented() {
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
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Direct gRPC client (instead of A2aClient, which doesn't yet
    // expose the v1.3 RPC stubs as ergonomic methods).
    let mut client = pb::a2a_service_client::A2aServiceClient::connect(url.clone())
        .await
        .expect("connect");

    let err = client
        .get_task(tonic::Request::new(pb::GetTaskRequest {
            task_id: "test".into(),
        }))
        .await
        .expect_err("GetTask must be Unimplemented");
    assert_eq!(err.code(), tonic::Code::Unimplemented);

    let err = client
        .cancel_task(tonic::Request::new(pb::CancelTaskRequest {
            task_id: "test".into(),
        }))
        .await
        .expect_err("CancelTask must be Unimplemented");
    assert_eq!(err.code(), tonic::Code::Unimplemented);

    let err = client
        .list_task(tonic::Request::new(pb::ListTaskRequest {
            context_id: "ctx".into(),
            page_size: 10,
            page_token: String::new(),
        }))
        .await
        .expect_err("ListTask must be Unimplemented");
    assert_eq!(err.code(), tonic::Code::Unimplemented);

    // GetAuthenticatedExtendedAgentCard returns the regular card.
    let card = client
        .get_authenticated_extended_agent_card(tonic::Request::new(pb::Empty {}))
        .await
        .expect("ExtendedAgentCard returns the standard card")
        .into_inner();
    assert_eq!(card.name, "test-agent");

    server_task.abort();
}

#[tokio::test]
async fn echo_round_trip() {
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
    assert_eq!(card.name, "test-agent");

    let (tx, mut rx) = client.open_streaming_message().await.expect("open stream");
    let task_id = "test-session";

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
    tx.send(bytes_request(task_id, b" world".to_vec()))
        .await
        .expect("send world");

    let mut accumulated = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while accumulated.len() < b"hello world".len() {
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
    assert_eq!(accumulated, b"hello world");

    drop(tx);

    let mut saw_exit = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while !saw_exit {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match timeout(remaining, rx.next()).await {
            Ok(Some(Ok(resp))) => {
                if let Some(pb::stream_response::Payload::Message(msg)) = resp.payload {
                    for part in msg.parts {
                        if let Ok(Some(ShellControl::ShellExit { code })) =
                            ShellControl::try_from_part(&part)
                        {
                            assert_eq!(code, Some(0));
                            saw_exit = true;
                            break;
                        }
                    }
                }
            }
            Ok(Some(Err(err))) => panic!("status err: {err:?}"),
            Ok(None) => panic!("stream ended before shell-exit"),
            Err(_) => panic!("timeout before shell-exit"),
        }
    }

    server_task.abort();
}

#[tokio::test]
async fn shell_open_missing_returns_invalid_argument() {
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
    let (tx, mut rx) = client.open_streaming_message().await.expect("open stream");

    tx.send(bytes_request("foo", b"hello".to_vec()))
        .await
        .expect("send");

    let item = timeout(Duration::from_secs(2), rx.next())
        .await
        .expect("timeout")
        .expect("stream end")
        .expect_err("must be Status err");
    assert_eq!(item.code(), tonic::Code::InvalidArgument);

    server_task.abort();
}
