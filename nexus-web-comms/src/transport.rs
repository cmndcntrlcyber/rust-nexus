//! v1.1 `Transport` trait — pluggable agent ↔ C2 lifeline abstraction.
//!
//! The concrete implementations live in `nexus-agent` (`GrpcTransport`,
//! `MeshTransport`, plus the overlay's existing TCP/HTTP/WS fallback paths
//! once they're adapted). This crate just defines the trait + supporting
//! types so the agent binary can compose either path without
//! `nexus-web-comms` depending on `nexus-agent` (which would be circular).
//!
//! Coexists with the overlay's `WebCommsConfig`-driven HTTP/WS fallback
//! code; not a replacement. Per D-V1.1-A "additive integration".

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use nexus_common::NodeIdentity;
use serde::{Deserialize, Serialize};

/// Boxed-and-pinned future passed into [`Transport::run`] to signal
/// shutdown. Resolves when the transport should stop.
pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Convenience constructor for a [`ShutdownFuture`].
#[must_use]
pub fn shutdown_from<F>(f: F) -> ShutdownFuture
where
    F: Future<Output = ()> + Send + 'static,
{
    Box::pin(f)
}

/// Identity material + metadata handed to the transport on `run`.
pub struct TransportContext {
    /// Agent's persistent cryptographic identity (Ed25519 + X25519 from
    /// `nexus_common::identity::NodeIdentity`).
    pub identity: NodeIdentity,
    /// Operator-supplied human-friendly tag.
    pub tag: String,
}

/// The agent ↔ C2 transport contract.
#[async_trait]
pub trait Transport: Send + 'static {
    /// Stable identifier for logs / status surfaces.
    fn kind(&self) -> TransportKind;

    /// Run the transport to completion.
    async fn run(
        self: Box<Self>,
        ctx: TransportContext,
        shutdown: ShutdownFuture,
    ) -> anyhow::Result<()>;
}

/// Stable identifier for each transport flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    /// Tonic gRPC over `nexus-a2a` (v1.1 default for interactive shells).
    Grpc,
    /// libp2p mesh (experimental in v1.1; no server-side wiring yet).
    Mesh,
    /// Overlay's existing TCP socket / HTTP fallback (v1.0 overlay path).
    Legacy,
}

impl TransportKind {
    /// Lowercase string suitable for config files and logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Grpc => "grpc",
            Self::Mesh => "mesh",
            Self::Legacy => "legacy",
        }
    }

    /// Whether this transport is safe to recommend for v1.1 deployments.
    #[must_use]
    pub const fn is_production_ready(self) -> bool {
        matches!(self, Self::Grpc | Self::Legacy)
    }
}

impl std::str::FromStr for TransportKind {
    type Err = UnknownTransport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "grpc" => Ok(Self::Grpc),
            "mesh" => Ok(Self::Mesh),
            "legacy" => Ok(Self::Legacy),
            other => Err(UnknownTransport(other.to_string())),
        }
    }
}

impl std::fmt::Display for TransportKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an unknown `transport.kind` string is encountered.
#[derive(Debug, thiserror::Error)]
#[error("unknown transport kind {0:?}; expected 'grpc', 'mesh', or 'legacy'")]
pub struct UnknownTransport(pub String);

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn kind_round_trips_from_str() {
        assert_eq!(
            TransportKind::from_str("grpc").unwrap(),
            TransportKind::Grpc
        );
        assert_eq!(
            TransportKind::from_str("mesh").unwrap(),
            TransportKind::Mesh
        );
        assert_eq!(
            TransportKind::from_str("legacy").unwrap(),
            TransportKind::Legacy
        );
    }

    #[test]
    fn kind_rejects_unknown() {
        let err = TransportKind::from_str("smtp").expect_err("must fail");
        assert!(err.to_string().contains("smtp"));
    }

    #[test]
    fn production_readiness() {
        assert!(TransportKind::Grpc.is_production_ready());
        assert!(TransportKind::Legacy.is_production_ready());
        assert!(!TransportKind::Mesh.is_production_ready());
    }

    #[test]
    fn kind_serde_lowercase() {
        let json = serde_json::to_string(&TransportKind::Grpc).expect("ser");
        assert_eq!(json, "\"grpc\"");
    }
}
