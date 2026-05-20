//! `OperatorRouter` — A2A `ShellHandler` impl that bridges an operator's A2A
//! `SendStreamingMessage` stream to a registered agent's A2A back-channel.
//!
//! For v1.1, agents that want to be reachable via interactive shells open
//! an A2A bidi stream back to the C2 (the agent-side `a2a_client` from
//! Phase 1.1.5 handles this). The agent's first frame is a special
//! `agent-register` text part identifying the peer-id; subsequent traffic
//! is shell-session bytes.
//!
//! Agents that registered only via the overlay's `RegisterAgent` (the
//! task-pull C2 path) appear in the listing but cannot host interactive
//! shells in v1.1. The router returns `FailedPrecondition` for those.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use nexus_a2a::capabilities::CapabilityCheck;
use nexus_a2a::framing::{bytes_response, control_response, ShellControl};
use nexus_a2a::handler::{
    AgentRegisterParams, AgentRegistrationHandler, ShellHandler, ShellOpenParams,
};
use nexus_a2a::pb as a2a_pb;
use tokio::sync::{mpsc, RwLock};
// `tonic_14` is the package-renamed Tonic 0.14 (see Cargo.toml). nexus-a2a's
// trait surface uses these types; we must match them exactly.
use tonic_14::{Status, Streaming};
use tracing::{debug, info, warn};

use crate::sessions::{SessionRecord, SessionRegistry};

/// One registered A2A-mode agent: a channel back to the agent's outbound
/// A2A stream (which the agent uses to send shell-output / shell-exit).
#[derive(Clone)]
pub struct AgentChannel {
    /// Sender that pushes operator-originated shell-session frames toward
    /// the agent. The agent's own A2A handler reads them from its inbound
    /// stream.
    pub tx: mpsc::Sender<Result<a2a_pb::StreamResponse, Status>>,
}

/// Concurrent-access table of A2A-mode agents.
#[derive(Default, Clone)]
pub struct AgentChannels {
    inner: Arc<RwLock<HashMap<[u8; 32], AgentChannel>>>,
}

impl AgentChannels {
    /// Empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an A2A-mode agent. Returns the previous entry if any (the
    /// caller usually wants to drop the old channel so the previous stream
    /// terminates).
    pub async fn register(&self, peer_id: [u8; 32], channel: AgentChannel) -> Option<AgentChannel> {
        self.inner.write().await.insert(peer_id, channel)
    }

    /// Look up an agent's channel.
    pub async fn lookup(&self, peer_id: &[u8; 32]) -> Option<AgentChannel> {
        self.inner.read().await.get(peer_id).cloned()
    }

    /// Remove an agent's entry.
    pub async fn remove(&self, peer_id: &[u8; 32]) -> Option<AgentChannel> {
        self.inner.write().await.remove(peer_id)
    }

    /// Snapshot of the registered peer-ids.
    pub async fn peer_ids(&self) -> Vec<[u8; 32]> {
        self.inner.read().await.keys().copied().collect()
    }

    /// Number of registered A2A-mode agents.
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// True when no agents are registered.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Pick the first registered agent (deterministic only when there's
    /// exactly one). Used as the default target when the operator doesn't
    /// supply `target_agent_id`.
    pub async fn first(&self) -> Option<([u8; 32], AgentChannel)> {
        self.inner
            .read()
            .await
            .iter()
            .next()
            .map(|(k, v)| (*k, v.clone()))
    }
}

/// A2A `ShellHandler` for the C2's operator-facing service.
pub struct OperatorRouter {
    agents: AgentChannels,
    sessions: SessionRegistry,
    capabilities: CapabilityCheck,
}

impl OperatorRouter {
    /// Construct with the shared registries. The capability check defaults
    /// to `allow_all`; use [`OperatorRouter::with_capability_check`] to
    /// gate operator → agent skill invocations in production.
    #[must_use]
    pub fn new(agents: AgentChannels, sessions: SessionRegistry) -> Self {
        Self {
            agents,
            sessions,
            capabilities: CapabilityCheck::allow_all(),
        }
    }

    /// Replace the capability check (Phase 1.2.5 / D-V1.2-caps).
    #[must_use]
    pub fn with_capability_check(mut self, check: CapabilityCheck) -> Self {
        self.capabilities = check;
        self
    }

    async fn pick_target(
        &self,
        target_hex: Option<&str>,
    ) -> Result<([u8; 32], AgentChannel), Status> {
        if let Some(hex) = target_hex {
            let peer_id = parse_hex_peer_id(hex)?;
            if let Some(channel) = self.agents.lookup(&peer_id).await {
                return Ok((peer_id, channel));
            }
            return Err(Status::failed_precondition(format!(
                "agent {hex} has no live A2A back-channel; v1.1 interactive shells require the \
                 agent to connect via the new A2A path (overlay nexus.proto agents do not yet \
                 support interactive shells)"
            )));
        }
        self.agents
            .first()
            .await
            .ok_or_else(|| Status::failed_precondition("no A2A-mode agents registered"))
    }
}

#[async_trait]
impl ShellHandler for OperatorRouter {
    async fn handle_stream(
        &self,
        open: ShellOpenParams,
        mut incoming: Streaming<a2a_pb::Message>,
        outgoing: mpsc::Sender<Result<a2a_pb::StreamResponse, Status>>,
    ) {
        let (peer_id, agent_channel) = match self.pick_target(open.target_agent_id.as_deref()).await
        {
            Ok(pair) => pair,
            Err(status) => {
                let _ = outgoing.send(Err(status)).await;
                return;
            }
        };

        // Phase 1.2.5 capability gate (D-V1.2-caps), v1.3.5-extended with
        // per-operator scoping (D-V1.3-G). When `open.operator_cn` is set
        // (mTLS peer cert CN extracted by the v1.3 server), the gate runs
        // `verify_with_operator`; otherwise falls back to the agent-only
        // check so pre-v1.3 deployments keep working.
        let target_hex: String = peer_id.iter().map(|b| format!("{:02x}", b)).collect();
        let cap_result = match open.operator_cn.as_deref() {
            Some(cn) => self
                .capabilities
                .verify_with_operator(cn, &target_hex, "shell-session"),
            None => self.capabilities.verify(&target_hex, "shell-session"),
        };
        if let Err(err) = cap_result {
            warn!(target = %target_hex, "{err}");
            let _ = outgoing
                .send(Err(Status::permission_denied(err.to_string())))
                .await;
            return;
        }

        // Allocate session id; register the operator's outbound channel.
        let session_id = self.sessions.next_session_id();
        let record = SessionRecord {
            agent_peer_id: peer_id,
            operator_tx: outgoing.clone(),
        };
        self.sessions.insert(session_id, record).await;

        // Push the shell-open through to the agent. We re-frame as a
        // StreamResponse with a session-id-tagged task_id so the agent
        // knows which session this is.
        let task_id_for_agent = format!("op-session:{session_id}");
        let open_frame = ShellControl::ShellOpen {
            cols: open.cols,
            rows: open.rows,
            shell: open.shell.clone(),
            target_agent_id: None,
        };
        let open_response = match control_response("server", &task_id_for_agent, &open_frame) {
            Ok(r) => r,
            Err(err) => {
                let _ = outgoing
                    .send(Err(Status::internal(format!("encode open: {err}"))))
                    .await;
                self.sessions.remove(session_id).await;
                return;
            }
        };
        if agent_channel.tx.send(Ok(open_response)).await.is_err() {
            let _ = outgoing
                .send(Err(Status::failed_precondition(
                    "target agent's A2A channel closed",
                )))
                .await;
            self.sessions.remove(session_id).await;
            return;
        }

        debug!(
            session = session_id,
            target = ?peer_id,
            "operator session opened (proxying)"
        );

        // Forward operator's inbound frames to the agent.
        while let Some(item) = incoming.next().await {
            let msg = match item {
                Ok(m) => m,
                Err(err) => {
                    debug!(error = %err, "operator inbound: stream error");
                    break;
                }
            };
            for part in msg.parts {
                if let Ok(Some(ctrl)) = ShellControl::try_from_part(&part) {
                    let response = control_response("server", &task_id_for_agent, &ctrl);
                    if let Ok(r) = response {
                        if agent_channel.tx.send(Ok(r)).await.is_err() {
                            warn!("operator inbound: agent channel closed mid-session");
                            break;
                        }
                    }
                    continue;
                }
                if let Some(a2a_pb::part::Part::File(bytes)) = part.part {
                    let r = bytes_response("server", &task_id_for_agent, bytes);
                    if agent_channel.tx.send(Ok(r)).await.is_err() {
                        warn!("operator inbound: agent channel closed mid-session");
                        break;
                    }
                }
            }
        }

        // Operator closed inbound — push a shell-exit to the agent and
        // clean up.
        let exit = ShellControl::ShellExit { code: Some(0) };
        if let Ok(r) = control_response("server", &task_id_for_agent, &exit) {
            let _ = agent_channel.tx.send(Ok(r)).await;
        }
        self.sessions.remove(session_id).await;
        debug!(session = session_id, "operator session closed");
    }
}

/// C2-side `AgentRegistrationHandler`. Records the agent's outbound channel
/// into [`AgentChannels`] so [`OperatorRouter::pick_target`] can find it,
/// then pumps the agent's inbound replies back to the right operator
/// session.
pub struct AgentRegistrar {
    agents: AgentChannels,
    sessions: SessionRegistry,
}

impl AgentRegistrar {
    /// Construct.
    #[must_use]
    pub fn new(agents: AgentChannels, sessions: SessionRegistry) -> Self {
        Self { agents, sessions }
    }
}

#[async_trait]
impl AgentRegistrationHandler for AgentRegistrar {
    async fn handle_stream(
        &self,
        params: AgentRegisterParams,
        mut incoming: Streaming<a2a_pb::Message>,
        outgoing: mpsc::Sender<Result<a2a_pb::StreamResponse, Status>>,
    ) {
        let peer_id = params.peer_id;
        let channel = AgentChannel {
            tx: outgoing.clone(),
        };
        // If an old channel exists, drop it so its previous stream
        // terminates cleanly.
        if let Some(prev) = self.agents.register(peer_id, channel).await {
            drop(prev);
            warn!(
                ?peer_id,
                "evicted previous A2A back-channel for this agent (re-registration)"
            );
        }
        info!(
            ?peer_id,
            os = %params.os,
            version = ?params.version,
            tag = ?params.tag,
            "A2A agent registered"
        );

        // Pump agent → C2 replies. Each inbound Message carries
        // `task_id = "op-session:{session_id}"`. We extract the
        // session_id, look up the operator's outbound channel in the
        // SessionRegistry, and forward the Message (re-wrapped as a
        // StreamResponse) to the operator.
        while let Some(item) = incoming.next().await {
            let msg = match item {
                Ok(m) => m,
                Err(err) => {
                    debug!(error = %err, "agent inbound: stream error");
                    break;
                }
            };

            let Some(session_id) = parse_session_id(&msg.task_id) else {
                debug!(task_id = %msg.task_id, "agent reply task_id not a session id; ignoring");
                continue;
            };

            let Some(record) = self.sessions.get(session_id).await else {
                debug!(session_id, "agent reply for unknown session; ignoring");
                continue;
            };

            let response = a2a_pb::StreamResponse {
                payload: Some(a2a_pb::stream_response::Payload::Message(a2a_pb::Message {
                    message_id: msg.message_id,
                    role: "agent".into(),
                    parts: msg.parts,
                    task_id: msg.task_id,
                })),
            };

            if record.operator_tx.send(Ok(response)).await.is_err() {
                debug!(session_id, "operator stream closed; dropping reply");
            }
        }

        // Agent stream closed — deregister.
        self.agents.remove(&peer_id).await;
        info!(?peer_id, "A2A agent deregistered");
    }
}

fn parse_session_id(task_id: &str) -> Option<u64> {
    task_id.strip_prefix("op-session:")?.parse().ok()
}

fn parse_hex_peer_id(hex: &str) -> Result<[u8; 32], Status> {
    if hex.len() != 64 {
        return Err(Status::invalid_argument(format!(
            "target_agent_id must be 64 hex chars, got {}",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)
            .map_err(|_| Status::invalid_argument("non-ASCII in target_agent_id"))?;
        out[i] = u8::from_str_radix(s, 16)
            .map_err(|_| Status::invalid_argument(format!("bad hex at byte {i}")))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_peer_id_round_trip() {
        let bytes = [0xABu8; 32];
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let parsed = parse_hex_peer_id(&hex).expect("parse");
        assert_eq!(parsed, bytes);
    }

    #[test]
    fn parse_hex_peer_id_rejects_short() {
        let err = parse_hex_peer_id("ab").expect_err("must fail");
        assert_eq!(err.code(), tonic_14::Code::InvalidArgument);
    }
}
