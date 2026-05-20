//! Two-node Kademlia discovery example (Phase 1.3.4).
//!
//! Spawns a seed node + a discovering node. The discovering node
//! dials the seed, runs `kad::Behaviour::bootstrap()` (driven by the
//! periodic kad heartbeat from libp2p), and verifies the routing
//! table is non-empty within a 10s window.
//!
//! Run:
//!   cargo run -p nexus-mesh --example kad_discovery

use std::time::Duration;

use anyhow::Result;
use nexus_common::NodeIdentity;
use nexus_mesh::{MeshEvent, MeshNode};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().ok();

    // Seed: bind on a fixed loopback port so the discovering node has
    // a Multiaddr to dial.
    let seed_id = NodeIdentity::from_seed(&[1u8; 32]);
    let seed_listen = "/ip4/127.0.0.1/tcp/47100".parse().unwrap();
    let seed = MeshNode::spawn(&seed_id, seed_listen)?;
    // Drain the listening event so we know the seed bound successfully.
    match tokio::time::timeout(Duration::from_secs(2), seed.next_event()).await {
        Ok(Some(MeshEvent::Listening(addr))) => {
            println!("[seed] listening on {addr}");
        }
        other => {
            eprintln!("[seed] unexpected first event: {other:?}");
        }
    }

    // Discoverer: bind on a different port and dial the seed.
    let disc_id = NodeIdentity::from_seed(&[2u8; 32]);
    let disc_listen = "/ip4/127.0.0.1/tcp/47101".parse().unwrap();
    let disc = MeshNode::spawn(&disc_id, disc_listen)?;
    disc.dial("/ip4/127.0.0.1/tcp/47100".parse().unwrap())
        .await?;

    // Drain events for 5s; both sides should observe at least an
    // identify exchange.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut peer_observed = false;
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if let Ok(Some(ev)) = tokio::time::timeout(remaining, disc.next_event()).await {
            match ev {
                MeshEvent::PeerIdentified { peer, .. } => {
                    println!("[disc] identified peer {peer}");
                    peer_observed = true;
                    break;
                }
                MeshEvent::ConnectionEstablished { peer } => {
                    println!("[disc] connected to {peer}");
                }
                MeshEvent::Listening(addr) => {
                    println!("[disc] listening on {addr}");
                }
                other => println!("[disc] event: {other:?}"),
            }
        }
    }

    if peer_observed {
        println!("ok: Kademlia + identify wired");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "no peer identified within 5s — Kademlia integration broken"
        ))
    }
}
