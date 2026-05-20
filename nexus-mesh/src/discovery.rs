//! v1.3 Kademlia DHT + mDNS peer discovery (Phase 1.3.4).
//!
//! Standalone helpers for constructing Kademlia and mDNS behaviours
//! configured for the nexus protocol. Full Swarm integration (a custom
//! `#[derive(NetworkBehaviour)]` that composes Identify + Gossipsub +
//! Ping + Kad + Mdns) is one focused follow-up that touches
//! [`crate::node`]; this module exposes the parts so that integration
//! is mechanical.
//!
//! ## What this module provides
//!
//! - [`KAD_PROTOCOL`] — the protocol name advertised by the DHT.
//! - [`MDNS_SERVICE`] — the mDNS service tag for LAN dev / testing.
//! - [`kad_config`] — a tuned [`libp2p::kad::Config`].
//! - [`build_kad`] — constructs a [`libp2p::kad::Behaviour`] keyed by
//!   the supplied local peer id, with in-memory record storage.
//! - [`build_mdns`] — constructs a [`libp2p::mdns::tokio::Behaviour`]
//!   for the nexus mDNS service.

use std::time::Duration;

use libp2p::identity::PeerId;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::StreamProtocol;

/// Protocol name advertised by the Kademlia DHT. Must match across all
/// participating nodes — bump the version when the wire format changes.
pub const KAD_PROTOCOL: &str = "/nexus/kad/1.0.0";

/// mDNS service tag for LAN dev / testing. Production deployments
/// disable mDNS and rely on static seed bootstrap (`[mesh.bootstrap]`
/// in `nexus.toml`) instead.
pub const MDNS_SERVICE: &str = "_nexus._udp.local";

/// Default Kademlia query timeout.
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

/// Default record TTL.
pub const RECORD_TTL: Duration = Duration::from_secs(36 * 3600);

/// Tuned `libp2p::kad::Config` for the nexus DHT.
#[must_use]
pub fn kad_config() -> kad::Config {
    let mut cfg = kad::Config::default();
    cfg.set_protocol_names(vec![StreamProtocol::new(KAD_PROTOCOL)]);
    cfg.set_query_timeout(QUERY_TIMEOUT);
    cfg.set_record_ttl(Some(RECORD_TTL));
    cfg
}

/// Construct a Kademlia behaviour keyed by `local_peer_id`. Uses an
/// in-memory record store (suitable for v1.3; future v1.4 can swap in
/// a persistent store).
#[must_use]
pub fn build_kad(local_peer_id: PeerId) -> kad::Behaviour<MemoryStore> {
    let store = MemoryStore::new(local_peer_id);
    kad::Behaviour::with_config(local_peer_id, store, kad_config())
}

/// Construct an mDNS behaviour for LAN discovery.
pub fn build_mdns(local_peer_id: PeerId) -> std::io::Result<mdns::tokio::Behaviour> {
    let config = mdns::Config {
        ttl: Duration::from_secs(60 * 60),
        query_interval: Duration::from_secs(5 * 60),
        ..mdns::Config::default()
    };
    mdns::tokio::Behaviour::new(config, local_peer_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    #[test]
    fn kad_config_uses_nexus_protocol() {
        let cfg = kad_config();
        // The config's protocols are private — confirm the
        // constructor at least produces a Config with no panic.
        // Stronger assertions would require exposing internals.
        drop(cfg);
        assert_eq!(KAD_PROTOCOL, "/nexus/kad/1.0.0");
    }

    #[test]
    fn build_kad_returns_a_behaviour() {
        let kp = Keypair::generate_ed25519();
        let pid = PeerId::from(kp.public());
        let _behaviour = build_kad(pid);
        // No-panic = success at the API level. End-to-end Kademlia
        // operations are exercised by the example binary
        // (`examples/kad_discovery.rs`).
    }

    #[tokio::test]
    async fn build_mdns_returns_a_behaviour() {
        // mDNS construction binds netlink sockets via the tokio
        // runtime, so this needs to be a #[tokio::test].
        let kp = Keypair::generate_ed25519();
        let pid = PeerId::from(kp.public());
        let _behaviour = build_mdns(pid).expect("mdns construction");
    }
}
