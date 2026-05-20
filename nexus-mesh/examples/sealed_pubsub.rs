//! `sealed_pubsub` — 3 nodes; A seals a message for C; B sees the gossip
//! frame but cannot decrypt; C decrypts cleanly.

use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use libp2p::Multiaddr;
use nexus_common::{NodeIdentity, SealedEnvelope};
use nexus_mesh::topics;
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
            println!("[sealed-pubsub] OK");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("[sealed-pubsub] FAILED: {err:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<()> {
    let id_a = NodeIdentity::from_seed(&[1u8; 32]);
    let id_b = NodeIdentity::from_seed(&[2u8; 32]);
    let id_c = NodeIdentity::from_seed(&[3u8; 32]);

    let node_a = MeshNode::spawn(&id_a, "/ip4/127.0.0.1/tcp/0".parse()?)?;
    let node_b = MeshNode::spawn(&id_b, "/ip4/127.0.0.1/tcp/0".parse()?)?;
    let node_c = MeshNode::spawn(&id_c, "/ip4/127.0.0.1/tcp/0".parse()?)?;

    let addr_a = wait_for_listen(&node_a).await?;
    let _addr_b = wait_for_listen(&node_b).await?;
    let _addr_c = wait_for_listen(&node_c).await?;
    info!(addr = %addr_a, "node A listening");

    node_b.dial(addr_a.clone()).await?;
    node_c.dial(addr_a.clone()).await?;

    let topic = topics::op_broadcast("v1.1-mesh-demo");
    node_a.subscribe(&topic).await?;
    node_b.subscribe(&topic).await?;
    node_c.subscribe(&topic).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let envelope = SealedEnvelope::seal(&id_a, &id_c.x25519_public(), b"top secret from A")?;
    let wire = envelope.to_bincode()?;
    info!(bytes = wire.len(), "A publishing sealed envelope for C");
    node_a.publish(&topic, wire).await?;

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut c_ok = false;
    let mut b_observed = false;

    while !(c_ok && b_observed) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!(
                "timeout waiting for both B (observed={b_observed}) and C (decrypted={c_ok})"
            );
        }
        tokio::select! {
            ev = timeout(remaining, node_b.next_event()) => {
                if let Ok(Some(MeshEvent::GossipMessage { data, .. })) = ev {
                    let env = SealedEnvelope::from_bincode(&data)?;
                    let err = env.open(&id_b).expect_err("B must NOT decrypt");
                    info!(error = %err, "B observed but cannot decrypt (expected)");
                    b_observed = true;
                }
            }
            ev = timeout(remaining, node_c.next_event()) => {
                if let Ok(Some(MeshEvent::GossipMessage { data, .. })) = ev {
                    let env = SealedEnvelope::from_bincode(&data)?;
                    let plaintext = env.open(&id_c).expect("C must decrypt");
                    assert_eq!(plaintext, b"top secret from A");
                    info!(plaintext = %String::from_utf8_lossy(&plaintext), "C decrypted");
                    c_ok = true;
                }
            }
        }
    }
    Ok(())
}

async fn wait_for_listen(node: &MeshHandle) -> Result<Multiaddr> {
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
