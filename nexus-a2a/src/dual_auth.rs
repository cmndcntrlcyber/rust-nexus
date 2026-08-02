//! Dual-auth gate — validates that a connecting console has authenticated
//! via both gRPC (mTLS + operator token) AND mesh (SealedEnvelope identity
//! verification) before allowing access to console tabs (WS1 Phase 1d).

use tracing::{debug, warn};

/// Result of a dual-auth validation.
#[derive(Debug, Clone)]
pub struct DualAuthResult {
    /// gRPC auth succeeded (mTLS client cert + valid operator token).
    pub grpc_authenticated: bool,
    /// Mesh auth succeeded (SealedEnvelope identity verified).
    pub mesh_authenticated: bool,
    /// Operator identity (from token or cert CN).
    pub operator_id: Option<String>,
    /// Peer ID from mesh identity verification (hex-encoded).
    pub mesh_peer_id: Option<String>,
}

impl DualAuthResult {
    /// Both auth paths must succeed for full access.
    pub fn is_fully_authenticated(&self) -> bool {
        self.grpc_authenticated && self.mesh_authenticated
    }
}

/// Gate that enforces both gRPC and mesh authentication before granting
/// console access to all 4 tabs.
pub struct DualAuthGate {
    require_dual: bool,
}

impl DualAuthGate {
    /// Create a gate. When `require_dual` is true, both gRPC and mesh
    /// authentication must succeed. When false, gRPC alone suffices
    /// (dev/loopback mode).
    pub fn new(require_dual: bool) -> Self {
        Self { require_dual }
    }

    /// Validate a connection attempt. Returns an error status if
    /// authentication requirements are not met.
    pub fn validate(&self, result: &DualAuthResult) -> Result<(), tonic::Status> {
        if !result.grpc_authenticated {
            warn!("dual-auth: gRPC authentication failed");
            return Err(tonic::Status::unauthenticated(
                "gRPC authentication required (mTLS client cert + operator token)",
            ));
        }

        if self.require_dual && !result.mesh_authenticated {
            warn!("dual-auth: mesh authentication failed");
            return Err(tonic::Status::unauthenticated(
                "mesh authentication required (SealedEnvelope identity verification)",
            ));
        }

        debug!(
            operator = ?result.operator_id,
            mesh_peer = ?result.mesh_peer_id,
            "dual-auth: fully authenticated"
        );
        Ok(())
    }

    /// Build a `DualAuthResult` from the constituent parts. Typically
    /// called by the console after completing both auth handshakes.
    pub fn build_result(
        grpc_ok: bool,
        operator_id: Option<String>,
        mesh_ok: bool,
        mesh_peer_id: Option<String>,
    ) -> DualAuthResult {
        DualAuthResult {
            grpc_authenticated: grpc_ok,
            mesh_authenticated: mesh_ok,
            operator_id,
            mesh_peer_id,
        }
    }
}

impl Default for DualAuthGate {
    fn default() -> Self {
        Self::new(true)
    }
}
