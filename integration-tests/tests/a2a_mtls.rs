//! v1.2 mTLS round-trip test (D-V1.2-mtls).
//!
//! Verifies the A2A plane can do a `GetAgentCard` over mTLS using certs
//! produced by `scripts/gen-certs.sh`. The test is `#[ignore]` by default
//! because it requires the `certs/` directory to exist; CI / `make demo`
//! provisions it first.

use std::time::Duration;

use nexus_a2a::mock::EchoShellHandler;
use nexus_a2a::{pb, tls, A2aClient, A2aServer};

fn agent_card() -> pb::AgentCard {
    pb::AgentCard {
        name: "v1.2-mtls".into(),
        description: "mTLS round-trip test".into(),
        version: "0.1.0".into(),
        skills: vec![],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

#[tokio::test]
#[ignore = "requires ./certs from scripts/gen-certs.sh and the NEXUS_* env vars"]
async fn mtls_round_trip() {
    let _ = tracing_subscriber::fmt::try_init();

    // Load server + client TLS configs from env vars.
    let server_tls = tls::load_server_config_from_env().expect("server TLS env");
    let client_tls = tls::load_client_config_from_env()
        .expect("client TLS env")
        .domain_name("localhost");

    // Bind ephemerally.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let url = format!("https://localhost:{}", addr.port());

    let server = A2aServer::new(agent_card(), EchoShellHandler);

    let server_task = tokio::spawn(async move {
        use tonic::transport::server::TcpIncoming;
        use tonic::transport::Server;
        let incoming = TcpIncoming::from(listener);
        Server::builder()
            .tls_config(server_tls)
            .expect("tls_config")
            .add_service(server.into_service())
            .serve_with_incoming(incoming)
            .await
            .expect("server");
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut client = A2aClient::connect_with_optional_tls(&url, false, Some(client_tls))
        .await
        .expect("connect mTLS");
    let card = client.get_agent_card().await.expect("get_agent_card");
    assert_eq!(card.name, "v1.2-mtls");

    server_task.abort();
}
