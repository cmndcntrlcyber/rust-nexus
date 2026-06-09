//! `GrpcTransport` — agent-side A2A transport.

use async_trait::async_trait;
use nexus_web_comms::{ShutdownFuture, Transport, TransportContext, TransportKind};
use tracing::info;

use crate::a2a_client::{connect_and_serve, A2aClientConfig};

/// gRPC agent transport.
#[derive(Debug, Clone)]
pub struct GrpcTransport {
    /// C2 A2A URL.
    pub c2_addr: String,
    /// Allow non-loopback addresses per D-V1-E.
    pub insecure_network: bool,
}

impl GrpcTransport {
    /// New transport pointed at `c2_addr`.
    #[must_use]
    pub fn new(c2_addr: impl Into<String>) -> Self {
        Self {
            c2_addr: c2_addr.into(),
            insecure_network: false,
        }
    }
}

#[async_trait]
impl Transport for GrpcTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Grpc
    }

    async fn run(
        self: Box<Self>,
        ctx: TransportContext,
        shutdown: ShutdownFuture,
    ) -> anyhow::Result<()> {
        info!(
            transport = %TransportKind::Grpc,
            c2 = %self.c2_addr,
            "agent transport: starting"
        );

        let cfg = A2aClientConfig {
            c2_addr: self.c2_addr.clone(),
            tag: ctx.tag,
            insecure_network: self.insecure_network,
        };
        connect_and_serve(&cfg, &ctx.identity, shutdown).await
    }
}
