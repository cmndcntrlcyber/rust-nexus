//! `two_node_ping` — spin up two `MeshNode`s on loopback and verify they
//! reach each other via libp2p Ping. Exits 0 if both nodes observe a
//! `MeshEvent::Ping` within 10s.

use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use nexus_common::NodeIdentity;
use nexus_mesh::{MeshEvent, MeshHandle, MeshNode};
use tokio::time::{timeout, Instant};
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();

    match run().await {
        Ok(()) => {
            println!("[two-node-ping] OK");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("[two-node-ping] FAILED: {err:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<()> {
    let id_a = NodeIdentity::from_seed(&[1u8; 32]);
    let id_b = NodeIdentity::from_seed(&[2u8; 32]);

    let node_a = MeshNode::spawn(&id_a, "/ip4/127.0.0.1/tcp/0".parse()?)?;
    let node_b = MeshNode::spawn(&id_b, "/ip4/127.0.0.1/tcp/0".parse()?)?;

    let addr_a = wait_for_listen(&node_a).await?;
    info!(addr = %addr_a, "node A listening");

    node_b.dial(addr_a).await?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut a_pinged = false;
    let mut b_pinged = false;

    while !(a_pinged && b_pinged) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("timeout waiting for ping events (a={a_pinged}, b={b_pinged})");
        }
        tokio::select! {
            ev = timeout(remaining, node_a.next_event()) => {
                if let Ok(Some(MeshEvent::Ping { peer, rtt })) = ev {
                    info!(peer = %peer, ?rtt, "node A: ping");
                    a_pinged = true;
                }
            }
            ev = timeout(remaining, node_b.next_event()) => {
                if let Ok(Some(MeshEvent::Ping { peer, rtt })) = ev {
                    info!(peer = %peer, ?rtt, "node B: ping");
                    b_pinged = true;
                }
            }
        }
    }
    Ok(())
}

async fn wait_for_listen(node: &MeshHandle) -> Result<libp2p::Multiaddr> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("listen timeout");
        }
        match timeout(remaining, node.next_event()).await {
            Ok(Some(MeshEvent::Listening(addr))) => return Ok(addr),
            Ok(Some(_)) => continue,
            Ok(None) => anyhow::bail!("event stream closed"),
            Err(_) => anyhow::bail!("listen timeout"),
        }
    }
}
