//! `ShellHandler` + `AgentLister` traits — per-stream / per-listing entry
//! points the [`crate::A2aServer`] delegates to.

use async_trait::async_trait;
use tokio::sync::mpsc;
use tonic::Streaming;

use crate::pb;

/// Parameters extracted from the initial `shell-open` frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellOpenParams {
    /// Initial columns.
    pub cols: u16,
    /// Initial rows.
    pub rows: u16,
    /// Optional shell override.
    pub shell: Option<String>,
    /// Optional target-agent selector (hex peer id).
    pub target_agent_id: Option<String>,
    /// task_id echoed by the server for correlation.
    pub task_id: String,
    /// v1.3.5 — operator's mTLS client-cert CN, extracted by the
    /// server when the connection is mTLS-authenticated. `None` in
    /// plaintext-loopback dev runs; per-operator scoping
    /// (`CapabilityCheck::verify_with_operator`) only engages when
    /// this is `Some`.
    pub operator_cn: Option<String>,
    /// v1.4.7 — Ed25519-signed operator token (D-V1.4-D), extracted
    /// from the `x-nexus-operator-token` gRPC metadata header by the
    /// server. When `Some`, the OperatorRouter prefers token-based
    /// identity (decoupled from cert rotation) over `operator_cn`.
    pub operator_token: Option<Vec<u8>>,
}

/// Server-side per-stream shell handler.
#[async_trait]
pub trait ShellHandler: Send + Sync + 'static {
    /// Drive a single shell session through the supplied streams.
    async fn handle_stream(
        &self,
        open: ShellOpenParams,
        incoming: Streaming<pb::Message>,
        outgoing: mpsc::Sender<Result<pb::StreamResponse, tonic::Status>>,
    );
}

/// Operator-facing snapshot of one registered agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredAgentInfo {
    /// 32-byte BLAKE3 peer id.
    pub peer_id: [u8; 32],
    /// OS string (`"linux"`, `"windows"`, `"macos"`, `"other"`).
    pub os: String,
    /// Build version string.
    pub version: String,
    /// Operator-supplied tag.
    pub tag: String,
    /// Unix seconds at last seen.
    pub last_seen_unix: u64,
}

/// Supplies the operator console with the list of currently-registered
/// agents. Default impl (used when no lister is attached) returns empty.
#[async_trait]
pub trait AgentLister: Send + Sync + 'static {
    /// Snapshot of currently-registered agents.
    async fn list(&self) -> Vec<RegisteredAgentInfo>;
}

/// Null implementation used when [`crate::A2aServer`] is constructed without
/// a lister (loopback example, tests).
pub(crate) struct NullLister;

#[async_trait]
impl AgentLister for NullLister {
    async fn list(&self) -> Vec<RegisteredAgentInfo> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// v1.2 agent-mode bidi (D-V1.2-G)
// ---------------------------------------------------------------------------

/// Parameters extracted from the initial `agent-register` frame on an
/// agent-mode bidi stream. The C2 stores the bidi back-channel in
/// `AgentChannels` keyed by `peer_id` so operator sessions can be proxied
/// to this agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRegisterParams {
    /// 32-byte BLAKE3 peer id (decoded from the `peer_id_hex` field).
    pub peer_id: [u8; 32],
    /// OS label (linux / windows / macos / other).
    pub os: String,
    /// Optional build version string.
    pub version: Option<String>,
    /// Optional operator-facing tag.
    pub tag: Option<String>,
}

/// Server-side per-stream agent-registration handler. Stays alive for the
/// lifetime of the agent's persistent control channel; the implementation
/// reads operator-initiated frames on `incoming` and dispatches shell
/// sessions onto the agent, then forwards the agent's response stream back.
#[async_trait]
pub trait AgentRegistrationHandler: Send + Sync + 'static {
    /// Drive the persistent agent-mode bidi stream until either side closes.
    async fn handle_stream(
        &self,
        params: AgentRegisterParams,
        incoming: tonic::Streaming<pb::Message>,
        outgoing: mpsc::Sender<Result<pb::StreamResponse, tonic::Status>>,
    );
}

/// Null implementation used when [`crate::A2aServer`] is constructed without
/// an agent-registration handler (no agent-mode streams will be accepted).
pub(crate) struct NullAgentRegistration;

#[async_trait]
impl AgentRegistrationHandler for NullAgentRegistration {
    async fn handle_stream(
        &self,
        _params: AgentRegisterParams,
        _incoming: tonic::Streaming<pb::Message>,
        outgoing: mpsc::Sender<Result<pb::StreamResponse, tonic::Status>>,
    ) {
        let _ = outgoing
            .send(Err(tonic::Status::unimplemented(
                "agent registration not enabled on this A2A server",
            )))
            .await;
    }
}
