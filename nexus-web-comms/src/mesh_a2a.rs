//! D-XLINK-A: mesh ↔ A2A transport boundary (v1.5 interop checkpoint).
//!
//! ## Decision record (D-XLINK-A)
//!
//! | Transport | Role | Status |
//! |-----------|------|--------|
//! | A2A gRPC (`nexus-a2a`) | Primary control plane | production-ready |
//! | libp2p mesh (`nexus-mesh`) | Fallback when gRPC is blocked | experimental |
//! | A2A-over-mesh tunnel | Optional Phase 5 adapter | deferred |
//!
//! The two transports are independent; no shared state except the agent's
//! `NodeIdentity` (Ed25519 keypair from `nexus-common::identity`).
//!
//! ## What this module provides (v1.5)
//!
//! * [`TransportPriority`] — ordered list expressing the preference: gRPC
//!   first, mesh fallback, legacy last.
//! * [`MeshA2aBridge`] — zero-cost marker struct that records the resolved
//!   D-XLINK-A boundary in code.  A full `Transport`-trait impl (Phase 5)
//!   is deferred; the struct gives import targets for the A2A-over-mesh
//!   path so call sites can be stubbed now and filled in later without
//!   changing signatures.
//! * [`select_transport`] — picks the highest-priority transport kind
//!   that is available (i.e., its binary exists / port is reachable).

use crate::transport::TransportKind;

/// Ordered transport preference for a v1.5 agent.
///
/// The agent tries each kind in slice order and uses the first one that
/// connects successfully.
pub const TRANSPORT_PRIORITY: &[TransportKind] =
    &[TransportKind::Grpc, TransportKind::Mesh, TransportKind::Legacy];

/// Selects the highest-priority transport from a set of available ones.
///
/// `available` contains the `TransportKind` values that have passed a
/// basic connectivity probe.  Returns the first entry of
/// [`TRANSPORT_PRIORITY`] that appears in `available`, or `None` if
/// nothing is reachable.
pub fn select_transport(available: &[TransportKind]) -> Option<TransportKind> {
    TRANSPORT_PRIORITY
        .iter()
        .find(|k| available.contains(k))
        .copied()
}

/// Boundary marker for the A2A-over-mesh tunnel path (D-XLINK-A Phase 5,
/// currently deferred).
///
/// This struct exists so that code paths that will eventually use the
/// tunnel can be written against a concrete type today.  When Phase 5 is
/// implemented this becomes a real `Transport` impl.
pub struct MeshA2aBridge {
    /// The gRPC endpoint that is the primary A2A address.
    pub grpc_addr: String,
    /// The libp2p multiaddr used as the mesh fallback.
    pub mesh_multiaddr: String,
}

impl MeshA2aBridge {
    pub fn new(grpc_addr: impl Into<String>, mesh_multiaddr: impl Into<String>) -> Self {
        Self {
            grpc_addr: grpc_addr.into(),
            mesh_multiaddr: mesh_multiaddr.into(),
        }
    }

    /// Returns the `TransportKind` that should be used first.
    ///
    /// In v1.5 this always returns `Grpc`; when Phase 5 lands the
    /// implementation will probe both and return based on reachability.
    pub fn preferred_kind(&self) -> TransportKind {
        TransportKind::Grpc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grpc_wins_when_available() {
        let available = vec![TransportKind::Grpc, TransportKind::Mesh];
        assert_eq!(select_transport(&available), Some(TransportKind::Grpc));
    }

    #[test]
    fn mesh_fallback_when_grpc_absent() {
        let available = vec![TransportKind::Mesh, TransportKind::Legacy];
        assert_eq!(select_transport(&available), Some(TransportKind::Mesh));
    }

    #[test]
    fn none_when_nothing_available() {
        assert_eq!(select_transport(&[]), None);
    }

    #[test]
    fn bridge_preferred_kind_is_grpc() {
        let bridge = MeshA2aBridge::new("localhost:50051", "/ip4/127.0.0.1/tcp/9000");
        assert_eq!(bridge.preferred_kind(), TransportKind::Grpc);
    }
}
