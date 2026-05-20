//! v1.2 integration: operator → C2 (`OperatorRouter` + `AgentRegistrar`)
//! → agent (`nexus_agent::a2a_client::connect_and_serve`) → real PTY →
//! response round-trip.
//!
//! Proves D-V1.2-G: an A2A bidi stream whose first frame is `agent-register`
//! gets routed to the agent-registration handler, populating
//! [`AgentChannels`], so a subsequent operator stream targeting that
//! agent's `peer_id_hex` runs an interactive shell session end-to-end.

use std::time::Duration;

use futures::StreamExt;
use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::{pb, A2aClient, A2aServer};
use nexus_agent::a2a_client::{connect_and_serve, A2aClientConfig};
use nexus_common::NodeIdentity;
use nexus_infra::a2a_router::{AgentChannels, AgentRegistrar, OperatorRouter};
use nexus_infra::sessions::SessionRegistry;
use tokio::time::timeout;

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "v1.2-bidi-test".into(),
        description: "v1.2 agent-side bidi loopback".into(),
        version: "0.1.0".into(),
        skills: vec![pb::AgentSkill {
            id: "shell-session".into(),
            name: "shell-session".into(),
            description: "Interactive PTY routed via OperatorRouter.".into(),
            tags: vec!["v1.2".into()],
        }],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

#[tokio::test]
async fn agent_round_trip_via_a2a_bidi() {
    let _ = tracing_subscriber::fmt::try_init();

    // Bind the C2's A2A server to an ephemeral port.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind C2");
    let addr = listener.local_addr().expect("addr");
    let url = format!("http://{addr}");

    let agents = AgentChannels::new();
    let sessions = SessionRegistry::new();
    let router = OperatorRouter::new(agents.clone(), sessions.clone());
    let registrar = AgentRegistrar::new(agents, sessions);
    let server = A2aServer::new(agent_card(), router).with_agent_registration(registrar);

    let server_task = tokio::spawn(async move {
        use tonic::transport::server::TcpIncoming;
        use tonic::transport::Server;
        let incoming = TcpIncoming::from(listener);
        Server::builder()
            .add_service(server.into_service())
            .serve_with_incoming(incoming)
            .await
            .expect("C2 server");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn the agent, which dials the C2 and registers. NodeIdentity
    // intentionally doesn't impl Clone (it holds a private key); construct
    // it inside the spawn so the caller can derive the peer-id from a
    // throwaway copy on the same seed.
    let seed = [42u8; 32];
    let agent_peer_id_hex = peer_id_hex(NodeIdentity::from_seed(&seed).peer_id());
    let agent_cfg = A2aClientConfig {
        c2_addr: url.clone(),
        tag: "test-agent".into(),
        insecure_network: false,
    };
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let agent_task = tokio::spawn(async move {
        let identity = NodeIdentity::from_seed(&seed);
        let shutdown = async move {
            let _ = stop_rx.await;
        };
        let _ = connect_and_serve(&agent_cfg, &identity, shutdown).await;
    });

    // Give the agent a moment to register.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Operator opens a shell session targeting this agent.
    let mut operator = A2aClient::connect(&url, false)
        .await
        .expect("operator connect");
    let (op_tx, mut op_rx) = operator
        .open_streaming_message()
        .await
        .expect("operator stream");
    let operator_task_id = "operator-1";
    let shell_open = ShellControl::ShellOpen {
        cols: 80,
        rows: 24,
        shell: None,
        target_agent_id: Some(agent_peer_id_hex.clone()),
    };
    op_tx
        .send(control_request(operator_task_id, &shell_open).expect("encode open"))
        .await
        .expect("send open");

    // Send `echo probe-marker\n` to the PTY.
    let probe: &[u8] = if cfg!(target_os = "windows") {
        b"Write-Host probe-marker\r\n"
    } else {
        b"echo probe-marker\n"
    };
    op_tx
        .send(bytes_request(operator_task_id, probe.to_vec()))
        .await
        .expect("send probe");

    // Read until we see the marker, with a generous timeout.
    let mut accumulated: Vec<u8> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!(
                "timeout waiting for marker; collected: {:?}",
                String::from_utf8_lossy(&accumulated)
            );
        }
        match timeout(remaining, op_rx.next()).await {
            Ok(Some(Ok(response))) => {
                if let Some(pb::stream_response::Payload::Message(msg)) = response.payload {
                    for part in msg.parts {
                        if let Some(pb::part::Part::File(bytes)) = part.part {
                            accumulated.extend_from_slice(&bytes);
                        }
                    }
                }
                if String::from_utf8_lossy(&accumulated).contains("probe-marker") {
                    break;
                }
            }
            Ok(Some(Err(status))) => panic!("operator stream status error: {status}"),
            Ok(None) => panic!(
                "operator stream closed; collected: {:?}",
                String::from_utf8_lossy(&accumulated)
            ),
            Err(_) => panic!(
                "outer timeout; collected: {:?}",
                String::from_utf8_lossy(&accumulated)
            ),
        }
    }

    // Cleanup.
    let _ = stop_tx.send(());
    drop(op_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), agent_task).await;
    server_task.abort();
}

fn peer_id_hex(peer_id: [u8; 32]) -> String {
    peer_id.iter().map(|b| format!("{:02x}", b)).collect()
}
