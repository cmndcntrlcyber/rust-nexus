//! `MeshNode` — libp2p Swarm wrapper.
//!
//! Composes Identify + Gossipsub + Ping over TCP + Noise + Yamux. Construct
//! via [`MeshNode::spawn`], which builds the swarm, spawns a background
//! tokio task driving it, and returns a [`MeshHandle`] you call
//! `publish` / `subscribe` / `dial` / `next_event` on.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use libp2p::core::transport::ListenerId;
use libp2p::gossipsub::{self, IdentTopic, MessageId, TopicHash};
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::ping;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{noise, tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use nexus_common::NodeIdentity;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, info, warn};

use crate::discovery::{build_kad, build_mdns};

const GOSSIPSUB_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(1000);
const CMD_CHANNEL_CAPACITY: usize = 64;
const EVENT_CHANNEL_CAPACITY: usize = 256;
const IDENTIFY_PROTOCOL: &str = "/nexus-mesh/identify/1.0.0";

/// Composite mesh behaviour.
///
/// v1.3 adds Kademlia DHT + mDNS LAN discovery. They're plain
/// components in the derive; events from them get matched on in
/// `handle_swarm_event` and either logged or surfaced via `MeshEvent`.
#[derive(NetworkBehaviour)]
struct MeshBehaviour {
    identify: identify::Behaviour,
    gossipsub: gossipsub::Behaviour,
    ping: ping::Behaviour,
    /// v1.3.4: Kademlia DHT for peer discovery (D-V1.3-H).
    kad: kad::Behaviour<MemoryStore>,
    /// v1.3.4: optional mDNS for LAN dev / testing. Wrapped in
    /// `Toggle` so production can disable it via the builder.
    mdns: libp2p::swarm::behaviour::toggle::Toggle<mdns::tokio::Behaviour>,
}

/// Builder/spawn entry point.
pub struct MeshNode;

/// Events surfaced from the swarm to the caller.
#[derive(Debug)]
pub enum MeshEvent {
    /// Local node started listening on `addr`.
    Listening(Multiaddr),
    /// A peer was identified via the Identify protocol.
    PeerIdentified {
        /// libp2p PeerId of the remote peer.
        peer: PeerId,
        /// Listen addresses advertised by the remote peer.
        listen_addrs: Vec<Multiaddr>,
    },
    /// A gossipsub message arrived on a subscribed topic.
    GossipMessage {
        /// libp2p PeerId of the publisher (as seen by gossipsub).
        from: Option<PeerId>,
        /// Topic hash the message was published to.
        topic: TopicHash,
        /// Raw bytes (the caller may further decode as a `SealedEnvelope`).
        data: Vec<u8>,
    },
    /// A ping round-trip completed.
    Ping {
        /// Remote peer.
        peer: PeerId,
        /// Round-trip time.
        rtt: Duration,
    },
    /// A new connection was established.
    ConnectionEstablished {
        /// Remote peer.
        peer: PeerId,
    },
}

/// Commands sent from the public-facing [`MeshHandle`] to the swarm task.
enum MeshCmd {
    Publish {
        topic: IdentTopic,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<MessageId>>,
    },
    Subscribe {
        topic: IdentTopic,
        reply: oneshot::Sender<Result<()>>,
    },
    Dial {
        addr: Multiaddr,
        reply: oneshot::Sender<Result<()>>,
    },
    ListenAddrs {
        reply: oneshot::Sender<Vec<Multiaddr>>,
    },
    /// v1.4.x-4: snapshot of the gossipsub mesh peers for `topic`.
    /// Used by the DTN store-and-forward path to decide whether a
    /// publish reached anyone before queuing.
    TopicSubscribers {
        topic: IdentTopic,
        reply: oneshot::Sender<usize>,
    },
}

/// Handle for interacting with a spawned mesh node.
pub struct MeshHandle {
    cmd_tx: mpsc::Sender<MeshCmd>,
    event_rx: Mutex<mpsc::Receiver<MeshEvent>>,
    local_peer_id: PeerId,
}

impl MeshHandle {
    /// libp2p PeerId of this node.
    #[must_use]
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Publish raw bytes to `topic`.
    pub async fn publish(&self, topic: &IdentTopic, data: Vec<u8>) -> Result<MessageId> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(MeshCmd::Publish {
                topic: topic.clone(),
                data,
                reply: tx,
            })
            .await
            .map_err(|_| anyhow!("mesh swarm task is gone"))?;
        rx.await.map_err(|_| anyhow!("publish reply dropped"))?
    }

    /// Subscribe to `topic`.
    pub async fn subscribe(&self, topic: &IdentTopic) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(MeshCmd::Subscribe {
                topic: topic.clone(),
                reply: tx,
            })
            .await
            .map_err(|_| anyhow!("mesh swarm task is gone"))?;
        rx.await.map_err(|_| anyhow!("subscribe reply dropped"))?
    }

    /// Dial `addr`.
    pub async fn dial(&self, addr: Multiaddr) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(MeshCmd::Dial { addr, reply: tx })
            .await
            .map_err(|_| anyhow!("mesh swarm task is gone"))?;
        rx.await.map_err(|_| anyhow!("dial reply dropped"))?
    }

    /// Snapshot of current listen addresses.
    pub async fn listen_addrs(&self) -> Vec<Multiaddr> {
        let (tx, rx) = oneshot::channel();
        if self
            .cmd_tx
            .send(MeshCmd::ListenAddrs { reply: tx })
            .await
            .is_err()
        {
            return Vec::new();
        }
        rx.await.unwrap_or_default()
    }

    /// v1.4.x-4: number of gossipsub mesh peers currently subscribed to
    /// `topic`. Used by the DTN store-and-forward decision: gossipsub
    /// `publish` can succeed with zero subscribers, so callers needing
    /// "delivered to *someone*" semantics check this before deciding
    /// whether to enqueue into [`crate::dtn::DtnQueue`].
    ///
    /// Returns 0 if the swarm task is gone.
    pub async fn topic_subscribers(&self, topic: &IdentTopic) -> usize {
        let (tx, rx) = oneshot::channel();
        if self
            .cmd_tx
            .send(MeshCmd::TopicSubscribers {
                topic: topic.clone(),
                reply: tx,
            })
            .await
            .is_err()
        {
            return 0;
        }
        rx.await.unwrap_or(0)
    }

    /// Receive the next event.
    pub async fn next_event(&self) -> Option<MeshEvent> {
        self.event_rx.lock().await.recv().await
    }
}

impl MeshNode {
    /// Spawn a new mesh node listening on `listen`.
    pub fn spawn(identity: &NodeIdentity, listen: Multiaddr) -> Result<MeshHandle> {
        let kp = build_libp2p_keypair(identity)?;
        let local_peer_id = kp.public().to_peer_id();
        info!(?local_peer_id, ?listen, "mesh: spawning node");

        let mut swarm = SwarmBuilder::with_existing_identity(kp)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| anyhow!("tcp transport: {e}"))?
            .with_behaviour(build_behaviour)
            .map_err(|e| anyhow!("behaviour: {e}"))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let _listener_id: ListenerId = swarm.listen_on(listen)?;

        let (cmd_tx, cmd_rx) = mpsc::channel::<MeshCmd>(CMD_CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel::<MeshEvent>(EVENT_CHANNEL_CAPACITY);

        tokio::spawn(run_swarm(swarm, cmd_rx, event_tx));

        Ok(MeshHandle {
            cmd_tx,
            event_rx: Mutex::new(event_rx),
            local_peer_id,
        })
    }
}

fn build_libp2p_keypair(identity: &NodeIdentity) -> Result<libp2p::identity::Keypair> {
    let mut seed = identity.ed25519_seed();
    let kp = libp2p::identity::Keypair::ed25519_from_bytes(&mut seed)
        .map_err(|e| anyhow!("derive libp2p keypair: {e}"))?;
    Ok(kp)
}

fn build_behaviour(key: &libp2p::identity::Keypair) -> MeshBehaviour {
    let peer_id = key.public().to_peer_id();
    let identify = identify::Behaviour::new(
        identify::Config::new(IDENTIFY_PROTOCOL.into(), key.public())
            .with_agent_version(format!("nexus-mesh/{}", env!("CARGO_PKG_VERSION"))),
    );

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(GOSSIPSUB_HEARTBEAT_INTERVAL)
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .expect("gossipsub config");
    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )
    .expect("gossipsub behaviour");

    let ping = ping::Behaviour::new(ping::Config::new());

    // v1.3.4 — Kademlia + mDNS via `crate::discovery` helpers.
    let kad = build_kad(peer_id);
    // mDNS construction can fail when the netlink socket can't be
    // bound (e.g. in some sandbox environments). Wrap in Toggle so a
    // failure degrades gracefully to "no LAN discovery" rather than
    // crashing the node.
    let mdns_inner: Option<mdns::tokio::Behaviour> = match build_mdns(peer_id) {
        Ok(b) => Some(b),
        Err(err) => {
            warn!(error = %err, "mesh: mDNS construction failed; disabling LAN discovery");
            None
        }
    };
    let mdns = libp2p::swarm::behaviour::toggle::Toggle::from(mdns_inner);

    MeshBehaviour {
        identify,
        gossipsub,
        ping,
        kad,
        mdns,
    }
}

async fn run_swarm(
    mut swarm: Swarm<MeshBehaviour>,
    mut cmd_rx: mpsc::Receiver<MeshCmd>,
    event_tx: mpsc::Sender<MeshEvent>,
) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break };
                handle_cmd(&mut swarm, cmd).await;
            }
            event = swarm.select_next_some() => {
                handle_swarm_event(&mut swarm, event, &event_tx).await;
            }
        }
    }
    debug!("mesh: swarm task ending");
}

async fn handle_cmd(swarm: &mut Swarm<MeshBehaviour>, cmd: MeshCmd) {
    match cmd {
        MeshCmd::Publish { topic, data, reply } => {
            let result = swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic, data)
                .map_err(|e| anyhow!("gossipsub publish: {e}"));
            let _ = reply.send(result);
        }
        MeshCmd::Subscribe { topic, reply } => {
            let result = swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .map(|_| ())
                .context("gossipsub subscribe");
            let _ = reply.send(result);
        }
        MeshCmd::Dial { addr, reply } => {
            let result = swarm.dial(addr).context("dial");
            let _ = reply.send(result);
        }
        MeshCmd::ListenAddrs { reply } => {
            let addrs: Vec<Multiaddr> = swarm.listeners().cloned().collect();
            let _ = reply.send(addrs);
        }
        MeshCmd::TopicSubscribers { topic, reply } => {
            let hash = topic.hash();
            let count = swarm.behaviour_mut().gossipsub.mesh_peers(&hash).count();
            let _ = reply.send(count);
        }
    }
}

async fn handle_swarm_event(
    swarm: &mut Swarm<MeshBehaviour>,
    event: SwarmEvent<MeshBehaviourEvent>,
    event_tx: &mpsc::Sender<MeshEvent>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            info!(addr = %address, "mesh: listening");
            let _ = event_tx.send(MeshEvent::Listening(address)).await;
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            debug!(peer = %peer_id, "mesh: connection established");
            let _ = event_tx
                .send(MeshEvent::ConnectionEstablished { peer: peer_id })
                .await;
        }
        SwarmEvent::Behaviour(MeshBehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            let _ = event_tx
                .send(MeshEvent::PeerIdentified {
                    peer: peer_id,
                    listen_addrs: info.listen_addrs,
                })
                .await;
        }
        SwarmEvent::Behaviour(MeshBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message,
            ..
        })) => {
            let _ = event_tx
                .send(MeshEvent::GossipMessage {
                    from: Some(propagation_source),
                    topic: message.topic,
                    data: message.data,
                })
                .await;
        }
        SwarmEvent::Behaviour(MeshBehaviourEvent::Ping(ping::Event {
            peer,
            connection: _,
            result: Ok(rtt),
        })) => {
            let _ = event_tx.send(MeshEvent::Ping { peer, rtt }).await;
        }
        SwarmEvent::OutgoingConnectionError { error, peer_id, .. } => {
            warn!(?peer_id, ?error, "mesh: outgoing connection error");
        }
        // v1.3.4 — Kademlia discovery events.
        SwarmEvent::Behaviour(MeshBehaviourEvent::Kad(kad_event)) => {
            debug!(?kad_event, "mesh: kad event");
        }
        // v1.3.4 — mDNS discovers a peer on the LAN: bootstrap a
        // dial + register the address with Kademlia so the routing
        // table fills out.
        SwarmEvent::Behaviour(MeshBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
            for (peer_id, addr) in list {
                debug!(?peer_id, %addr, "mesh: mDNS discovered");
                swarm.behaviour_mut().kad.add_address(&peer_id, addr);
            }
        }
        SwarmEvent::Behaviour(MeshBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
            for (peer_id, _addr) in list {
                debug!(?peer_id, "mesh: mDNS peer expired");
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::PublicKey;

    #[test]
    fn libp2p_peer_id_matches_ed25519_pubkey() {
        let id = NodeIdentity::from_seed(&[42u8; 32]);
        let kp = build_libp2p_keypair(&id).expect("kp");
        let pk: PublicKey = kp.public();
        let ed = pk
            .clone()
            .try_into_ed25519()
            .expect("ed25519 conversion")
            .to_bytes();
        assert_eq!(ed, id.ed25519_public());
    }
}
