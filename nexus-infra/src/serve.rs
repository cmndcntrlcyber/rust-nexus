//! v1.1 top-level server wiring.
//!
//! Hosts the new A2A service (Tonic 0.14) on its own port. The existing
//! `nexus-infra::grpc_server::GrpcServer` (Tonic 0.10) continues to run on
//! its own config-driven port, untouched. The two services share an
//! `Arc<RwLock<HashMap<String, AgentSession>>>` agent registry (via
//! [`crate::a2a_lister::RegistryLister`]) so the operator console can list
//! every registered agent.
//!
//! Per D-V1.1-A "additive integration": this module is new; nothing here
//! modifies the overlay's gRPC server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use nexus_a2a::pb as a2a_pb;
use nexus_a2a::A2aServer;
use nexus_common::NodeIdentity;
use tokio::sync::RwLock;
use tracing::info;

use crate::a2a_lister::RegistryLister;
use crate::a2a_router::{AgentChannels, AgentRegistrar, OperatorRouter};
use crate::grpc_server::AgentSession;
use crate::sessions::SessionRegistry;

/// Default A2A AgentCard returned to operators.
#[must_use]
pub fn default_agent_card() -> a2a_pb::AgentCard {
    a2a_pb::AgentCard {
        name: "nexus-infra".into(),
        description: "rust-nexus C2 server (v1.1 simple-mesh integration)".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        skills: vec![a2a_pb::AgentSkill {
            id: "shell-session".into(),
            name: "shell-session".into(),
            description: "Interactive PTY-backed shell routed to an A2A-mode agent.".into(),
            tags: vec!["v1.1".into(), "routed".into()],
        }],
        signature: Vec::new(),
        signer_peer_id: Vec::new(),
    }
}

/// Config for the A2A side. The existing nexus.proto side is configured
/// via `nexus.toml [grpc_server]` and the overlay's own runner.
#[derive(Debug, Clone)]
pub struct A2aServeOptions {
    /// A2A gRPC bind address. Typically distinct from the existing
    /// NexusC2 port (which Tonic 0.10 owns).
    pub bind: SocketAddr,
    /// Allow non-loopback binds per D-V1-E.
    pub insecure_network: bool,
}

/// Shared state passed into [`run_a2a`] so the existing C2's registry and
/// the new A2A side see the same agents.
#[derive(Clone)]
pub struct A2aSharedState {
    /// The overlay's agent registry. Sourced from `GrpcServer`.
    pub agents_view: Arc<RwLock<HashMap<String, AgentSession>>>,
    /// Table of A2A-mode agents (populated by Phase 1.1.5's agent-side
    /// `a2a_client`).
    pub a2a_agents: AgentChannels,
    /// Per-session operator routing.
    pub sessions: SessionRegistry,
    /// v1.2: server identity for signing the AgentCard (D-V1.2-cards).
    /// When `None`, the card ships unsigned (v1.1 wire compat); callers
    /// that want signed cards should provision a persistent identity via
    /// [`NodeIdentity::load_or_create`].
    pub server_identity: Option<Arc<NodeIdentity>>,
}

impl A2aSharedState {
    /// Build state with empty A2A-mode tables, bound to the overlay's
    /// agent registry.
    #[must_use]
    pub fn new(agents_view: Arc<RwLock<HashMap<String, AgentSession>>>) -> Self {
        Self {
            agents_view,
            a2a_agents: AgentChannels::new(),
            sessions: SessionRegistry::new(),
            server_identity: None,
        }
    }

    /// Attach a server identity. The AgentCard will be Ed25519-signed at
    /// the start of [`run_a2a`].
    #[must_use]
    pub fn with_server_identity(mut self, identity: Arc<NodeIdentity>) -> Self {
        self.server_identity = Some(identity);
        self
    }
}

/// Run the A2A gRPC server until `shutdown` resolves.
pub async fn run_a2a(
    opts: A2aServeOptions,
    state: A2aSharedState,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    // `run_a2a` does not own a TLS config — pass tls_configured=false. For
    // production mTLS, the `nexus-server` binary calls
    // `A2aServer::serve_with_optional_tls` directly so the gate sees
    // tls_configured=true and accepts non-loopback binds.
    nexus_a2a::insecure::enforce(opts.bind, opts.insecure_network, false)
        .context("loopback gate (A2A)")?;

    info!(
        a2a_bind = ?opts.bind,
        insecure_network = opts.insecure_network,
        "v1.1 A2A service starting"
    );

    let router = OperatorRouter::new(state.a2a_agents.clone(), state.sessions.clone());
    let registrar = AgentRegistrar::new(state.a2a_agents.clone(), state.sessions.clone());
    let lister = RegistryLister::new(state.agents_view.clone());

    let mut card = default_agent_card();
    if let Some(identity) = state.server_identity.as_ref() {
        nexus_a2a::cards::sign(&mut card, identity);
        info!("A2A AgentCard signed with server identity");
    } else {
        info!("A2A AgentCard ships unsigned (no server identity configured)");
    }

    let server = A2aServer::new(card, router)
        .with_lister(lister)
        .with_agent_registration(registrar);

    server
        .serve(opts.bind, opts.insecure_network, shutdown)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_agent_card_advertises_shell_session_skill() {
        let card = default_agent_card();
        assert_eq!(card.name, "nexus-infra");
        assert!(card.skills.iter().any(|s| s.id == "shell-session"));
    }
}
