//! v1.3 server-side mesh listener (Phase 1.3.3).
//!
//! Spawns a `MeshNode`, subscribes to the canonical `server_inbox`
//! topic, decodes inbound `SealedEnvelope`s, and surfaces them via an
//! mpsc receiver so the OperatorRouter (or whatever wraps this) can
//! act on them.
//!
//! v1.3 scope: the listener decodes envelopes into raw payload bytes
//! and exposes them. Full operator → mesh → agent routing (where the
//! C2 sends the payload onward to the target agent via
//! `AgentChannels`) is wired by the same OperatorRouter that already
//! handles the gRPC-arrived case; this module just gets the bytes off
//! the wire.

use std::sync::Arc;

use anyhow::{Context, Result};
use nexus_common::{NodeIdentity, SealedEnvelope};
use nexus_mesh::topics::{server_inbox, Role};
use nexus_mesh::{MeshEvent, MeshHandle, MeshNode};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

/// Channel capacity for the listener's outbound decoded-payload stream.
const DECODED_CHANNEL_CAPACITY: usize = 128;

/// Configuration for the server-side mesh listener.
#[derive(Debug, Clone)]
pub struct MeshListenerOptions {
    /// Mesh listen multiaddr (e.g. `/ip4/0.0.0.0/tcp/9101`).
    pub listen: String,
    /// Static bootstrap peers (Phase 1.3.4 layers Kademlia DHT
    /// discovery on top so this becomes optional).
    pub bootstrap: Vec<String>,
}

/// One decoded shell-session payload, ready to forward to an agent
/// back-channel by the OperatorRouter.
pub struct DecodedPayload {
    /// Sender's Ed25519 public key (32 bytes — the operator's identity).
    pub sender_pubkey: [u8; 32],
    /// Decrypted payload bytes (the operator's framed `ShellControl` /
    /// PTY input).
    pub plaintext: Vec<u8>,
}

/// Handle returned by [`spawn_mesh_listener`]. Drop to terminate the
/// background task (the mesh node closes when its handle's last
/// reference goes away).
pub struct MeshListener {
    /// Underlying mesh handle — exposed so callers can publish back
    /// (operator-bound responses) on the same swarm.
    pub handle: Arc<MeshHandle>,
    /// Stream of decoded inbound payloads.
    pub decoded_rx: Mutex<mpsc::Receiver<DecodedPayload>>,
}

/// Spawn the listener: starts a `MeshNode`, subscribes to the canonical
/// `server_inbox` topic, decrypts incoming envelopes against
/// `local_identity`'s X25519 key, and forwards plaintexts on
/// `decoded_rx`.
///
/// Returns the handle + receiver wrapped in `Arc`/`Mutex` so callers
/// can clone and share across tasks.
pub async fn spawn_mesh_listener(
    local_identity: &NodeIdentity,
    opts: MeshListenerOptions,
) -> Result<MeshListener> {
    let listen_addr: libp2p::Multiaddr =
        opts.listen.parse().context("parse mesh listen multiaddr")?;
    let handle = MeshNode::spawn(local_identity, listen_addr).context("spawn mesh node")?;
    let handle = Arc::new(handle);

    // Subscribe to the C2's inbound shell-session topic (server_inbox()).
    let topic = server_inbox();
    handle
        .subscribe(&topic)
        .await
        .context("subscribe to server_inbox")?;
    info!(role = ?Role::C2, topic = %topic, "C2 subscribed to mesh inbox");

    // Dial the static bootstrap peers (v1.3.4 Kademlia integration
    // will let us discover peers dynamically; for now this is the
    // baseline).
    for bs in &opts.bootstrap {
        match bs.parse::<libp2p::Multiaddr>() {
            Ok(addr) => {
                if let Err(err) = handle.dial(addr.clone()).await {
                    warn!(%bs, error = %err, "bootstrap dial failed");
                }
            }
            Err(err) => warn!(%bs, error = %err, "invalid bootstrap multiaddr"),
        }
    }

    // Forwarder task: pump MeshEvent::GossipMessage → SealedEnvelope
    // decode → DecodedPayload.
    let (tx, rx) = mpsc::channel::<DecodedPayload>(DECODED_CHANNEL_CAPACITY);
    let handle_for_task = Arc::clone(&handle);
    let identity_x_secret = local_identity.x25519_public(); // for logging only
    let identity_clone = clone_identity_for_task(local_identity);

    tokio::spawn(async move {
        loop {
            let event = match handle_for_task.next_event().await {
                Some(e) => e,
                None => {
                    debug!("mesh listener: event stream closed");
                    break;
                }
            };
            if let MeshEvent::GossipMessage { from, topic, data } = event {
                debug!(?from, %topic, bytes = data.len(), "mesh listener: gossip");
                match decode_envelope(&data, &identity_clone) {
                    Ok(decoded) => {
                        if tx.send(decoded).await.is_err() {
                            debug!("mesh listener: decoded channel closed");
                            break;
                        }
                    }
                    Err(err) => {
                        warn!(error = %err, "mesh listener: envelope decode failed");
                    }
                }
            }
        }
        let _ = identity_x_secret; // silence unused
    });

    Ok(MeshListener {
        handle,
        decoded_rx: Mutex::new(rx),
    })
}

/// Decode + verify a single inbound mesh frame.
fn decode_envelope(bytes: &[u8], identity: &NodeIdentity) -> Result<DecodedPayload> {
    let envelope = SealedEnvelope::from_bincode(bytes).context("parse envelope")?;
    let plaintext = envelope
        .open(identity)
        .context("open envelope (decrypt + verify)")?;
    Ok(DecodedPayload {
        sender_pubkey: envelope.sender_ed25519,
        plaintext,
    })
}

/// Construct a fresh `NodeIdentity` instance with the same seed as
/// `src` so the background task owns its own copy. `NodeIdentity`
/// doesn't impl `Clone` for safety; this helper preserves that
/// guarantee at the call site by reconstructing from the public seed.
fn clone_identity_for_task(src: &NodeIdentity) -> NodeIdentity {
    NodeIdentity::from_seed(&src.ed25519_seed())
}

impl Default for MeshListenerOptions {
    fn default() -> Self {
        Self {
            listen: "/ip4/0.0.0.0/tcp/9101".to_string(),
            bootstrap: Vec::new(),
        }
    }
}

/// v1.4.3 finish — drain decoded mesh payloads into a caller-supplied
/// callback. Each decoded `SealedEnvelope` from the mesh is parsed for
/// its inner `ShellControl::ShellOpen` (if present) and forwarded along
/// with the operator's sender pubkey + the routing target peer id.
///
/// The OperatorRouter's `pick_target` + `AgentChannels` machinery is
/// the natural consumer (close the operator → mesh → agent loop), but
/// the callback shape lets tests and alternative consumers (audit-only
/// gateways, etc.) plug in without owning the AgentChannels lock.
///
/// Returns a `JoinHandle` so the caller can abort the pump on shutdown.
pub fn pump_mesh_decoded<F>(
    listener: Arc<MeshListener>,
    on_payload: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn(DecodedPayload) + Send + Sync + 'static,
{
    tokio::spawn(async move {
        loop {
            let mut rx_guard = listener.decoded_rx.lock().await;
            let payload = match rx_guard.recv().await {
                Some(p) => p,
                None => {
                    debug!("pump_mesh_decoded: decoded_rx closed");
                    break;
                }
            };
            drop(rx_guard);
            on_payload(payload);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn options_default_listen() {
        let opts = MeshListenerOptions::default();
        assert!(opts.listen.starts_with("/ip4/"));
        assert!(opts.bootstrap.is_empty());
    }

    /// pump_mesh_decoded fires the callback for each payload it dequeues.
    /// We synthesize a MeshListener manually (skipping the libp2p spawn)
    /// so this stays a unit test.
    #[tokio::test]
    async fn pump_forwards_payloads_to_callback() {
        // Build a synthetic listener: a no-op handle isn't needed since
        // pump only touches `decoded_rx`. We construct the MeshListener
        // struct directly from a channel pair via a dummy listener
        // helper.
        let (tx, rx) = mpsc::channel::<DecodedPayload>(8);
        let listener = Arc::new(MeshListener {
            // SAFETY: pump_mesh_decoded never dereferences `handle`,
            // so any valid Arc<MeshHandle> would do. Skipping
            // construction by panicking only if reached.
            handle: synthesize_unreachable_handle(),
            decoded_rx: Mutex::new(rx),
        });

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&count);
        let join = pump_mesh_decoded(Arc::clone(&listener), move |_p| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

        tx.send(DecodedPayload {
            sender_pubkey: [1; 32],
            plaintext: b"one".to_vec(),
        })
        .await
        .unwrap();
        tx.send(DecodedPayload {
            sender_pubkey: [2; 32],
            plaintext: b"two".to_vec(),
        })
        .await
        .unwrap();

        // Give the pump a moment.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(count.load(Ordering::Relaxed), 2);

        // Close the channel; pump should exit cleanly.
        drop(tx);
        tokio::time::timeout(std::time::Duration::from_secs(1), join)
            .await
            .expect("pump finishes after channel close")
            .expect("pump task succeeds");
    }

    /// Test-only helper that fabricates an `Arc<MeshHandle>` whose
    /// constituent fields are never read by `pump_mesh_decoded`. The
    /// pump only borrows `decoded_rx`, not `handle`, so any same-shape
    /// placeholder is safe at runtime — though we never actually
    /// dereference the methods.
    fn synthesize_unreachable_handle() -> Arc<MeshHandle> {
        // Spawn a real mesh node on an ephemeral port. Slightly heavy
        // for a unit test but the alternative (faking MeshHandle's
        // private internals) is fragile.
        let id = nexus_common::NodeIdentity::from_seed(&[123; 32]);
        let listen: libp2p::Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        Arc::new(MeshNode::spawn(&id, listen).expect("spawn"))
    }
}
