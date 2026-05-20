//! `A2aServer` — Tonic service host parameterized over a [`ShellHandler`].

use std::net::SocketAddr;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::server::TcpIncoming;
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

use crate::framing::ShellControl;
use crate::handler::{
    AgentLister, AgentRegisterParams, AgentRegistrationHandler, NullAgentRegistration, NullLister,
    ShellHandler, ShellOpenParams,
};
use crate::insecure;
use crate::pb;
use crate::pb::a2a_service_server::{A2aService, A2aServiceServer};

const STREAM_CHANNEL_CAPACITY: usize = 64;

/// A2A service implementation.
pub struct A2aServer<H: ShellHandler> {
    agent_card: pb::AgentCard,
    handler: Arc<H>,
    lister: Arc<dyn AgentLister>,
    agent_registration: Arc<dyn AgentRegistrationHandler>,
    /// v1.4.3 — optional broadcast tap on the audit sink. When set,
    /// `StreamAuditRecords` RPC delivers live records to gRPC subscribers.
    broadcast_audit: Option<Arc<crate::audit::BroadcastSink>>,
    /// v1.4.7 — server identity used to sign operator tokens (separate
    /// from the AgentCard signing identity by convention, though they
    /// can be the same `NodeIdentity`).
    server_identity: Option<Arc<nexus_common::NodeIdentity>>,
    /// v1.4.7 — max lifetime cap for operator tokens (default 24h).
    max_token_lifetime_seconds: u64,
}

impl<H: ShellHandler> A2aServer<H> {
    /// Build a new server with an empty default lister.
    pub fn new(agent_card: pb::AgentCard, handler: H) -> Self {
        Self {
            agent_card,
            handler: Arc::new(handler),
            lister: Arc::new(NullLister),
            agent_registration: Arc::new(NullAgentRegistration),
            broadcast_audit: None,
            server_identity: None,
            max_token_lifetime_seconds: crate::tokens::DEFAULT_LIFETIME_SECONDS,
        }
    }

    /// v1.4.3 — attach a [`BroadcastSink`] so `StreamAuditRecords`
    /// RPC subscribers see records as they're written.
    #[must_use]
    pub fn with_broadcast_audit(mut self, sink: Arc<crate::audit::BroadcastSink>) -> Self {
        self.broadcast_audit = Some(sink);
        self
    }

    /// v1.4.7 — attach the server's `NodeIdentity`; required for
    /// `IssueOperatorToken` RPC.
    #[must_use]
    pub fn with_server_identity(mut self, identity: Arc<nexus_common::NodeIdentity>) -> Self {
        self.server_identity = Some(identity);
        self
    }

    /// v1.4.7 — cap the lifetime any client may request via
    /// `IssueOperatorToken`. Defaults to [`crate::tokens::DEFAULT_LIFETIME_SECONDS`].
    #[must_use]
    pub fn with_max_token_lifetime(mut self, seconds: u64) -> Self {
        self.max_token_lifetime_seconds = seconds;
        self
    }

    /// Attach an [`AgentLister`] for `ListRegisteredAgents` RPCs.
    #[must_use]
    pub fn with_lister<L: AgentLister>(mut self, lister: L) -> Self {
        self.lister = Arc::new(lister);
        self
    }

    /// Attach an [`AgentRegistrationHandler`] to accept agent-mode bidi
    /// streams. Without this, streams whose first frame is `agent-register`
    /// are rejected with `Unimplemented`.
    #[must_use]
    pub fn with_agent_registration<R: AgentRegistrationHandler>(mut self, handler: R) -> Self {
        self.agent_registration = Arc::new(handler);
        self
    }

    /// Convert into a Tonic-ready service. v1.2 (Phase 1.2.7) applies the
    /// 4 MB message-size cap from [`crate::interceptors::MAX_MESSAGE_SIZE`]
    /// in both directions.
    pub fn into_service(self) -> A2aServiceServer<Self> {
        A2aServiceServer::new(self)
            .max_decoding_message_size(crate::interceptors::MAX_MESSAGE_SIZE)
            .max_encoding_message_size(crate::interceptors::MAX_MESSAGE_SIZE)
    }

    /// Bind to `addr` and serve until `shutdown` resolves. Enforces D-V1-E.
    pub async fn serve(
        self,
        addr: SocketAddr,
        insecure_network: bool,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<()> {
        self.serve_with_optional_tls(addr, insecure_network, None, shutdown)
            .await
    }

    /// Bind to `addr` and serve over mTLS (v1.2). `tls` carries the
    /// server identity + (optional) client-CA. Without a TLS config,
    /// behaves identically to [`A2aServer::serve`].
    pub async fn serve_with_optional_tls(
        self,
        addr: SocketAddr,
        insecure_network: bool,
        tls: Option<tonic::transport::ServerTlsConfig>,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<()> {
        insecure::enforce(addr, insecure_network)?;
        let with_tls = tls.is_some();
        info!(
            ?addr,
            insecure_network,
            mtls = with_tls,
            "A2A server starting"
        );
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let incoming = TcpIncoming::from(listener);
        let mut builder = Server::builder();
        if let Some(t) = tls {
            builder = builder.tls_config(t)?;
        }
        builder
            .add_service(self.into_service())
            .serve_with_incoming_shutdown(incoming, shutdown)
            .await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl<H: ShellHandler> A2aService for A2aServer<H> {
    async fn send_message(
        &self,
        request: Request<pb::Message>,
    ) -> Result<Response<pb::Message>, Status> {
        let mut msg = request.into_inner();
        msg.role = "agent".into();
        Ok(Response::new(msg))
    }

    type SendStreamingMessageStream = ReceiverStream<Result<pb::StreamResponse, Status>>;

    async fn send_streaming_message(
        &self,
        request: Request<Streaming<pb::Message>>,
    ) -> Result<Response<Self::SendStreamingMessageStream>, Status> {
        // v1.4.7 finish — pull the operator token from gRPC metadata
        // BEFORE consuming the request into its stream. If the token
        // verifies against the server's NodeIdentity public key, the
        // dispatch task receives `Some(token_bytes)` for the
        // OperatorRouter to use during capability lookup.
        let extracted_token = self
            .server_identity
            .as_ref()
            .and_then(|id| extract_operator_token(request.metadata(), &id.ed25519_public()));

        // Return the response stream immediately so the gRPC bidi handshake
        // completes; do first-frame parsing in a spawned task.
        let incoming = request.into_inner();
        let (tx, rx) = mpsc::channel::<Result<pb::StreamResponse, Status>>(STREAM_CHANNEL_CAPACITY);
        let handler = Arc::clone(&self.handler);
        let agent_registration = Arc::clone(&self.agent_registration);
        tokio::spawn(dispatch_stream(
            handler,
            agent_registration,
            incoming,
            tx,
            extracted_token,
        ));
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_agent_card(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::AgentCard>, Status> {
        Ok(Response::new(self.agent_card.clone()))
    }

    async fn list_registered_agents(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::RegisteredAgents>, Status> {
        let entries = self.lister.list().await;
        let agents = entries
            .into_iter()
            .map(|e| pb::RegisteredAgent {
                peer_id: e.peer_id.to_vec(),
                os: e.os,
                version: e.version,
                tag: e.tag,
                last_seen_unix: e.last_seen_unix,
            })
            .collect();
        Ok(Response::new(pb::RegisteredAgents { agents }))
    }

    // ---------------------------------------------------------------
    // v1.3 — Unimplemented stubs for upstream A2A v0.3 RPC surface
    // (D-V1.3-A). Returning `Unimplemented` rather than failing
    // method-not-found because the proto file *does* declare these
    // methods; the server is honest about the v1.3 implementation
    // gap.
    // ---------------------------------------------------------------

    async fn get_task(
        &self,
        _request: Request<pb::GetTaskRequest>,
    ) -> Result<Response<pb::Task>, Status> {
        Err(Status::unimplemented(
            "GetTask: full upstream A2A task surface lands in v1.4",
        ))
    }

    async fn cancel_task(
        &self,
        _request: Request<pb::CancelTaskRequest>,
    ) -> Result<Response<pb::Task>, Status> {
        Err(Status::unimplemented(
            "CancelTask: full upstream A2A task surface lands in v1.4",
        ))
    }

    type TaskSubscriptionStream = ReceiverStream<Result<pb::StreamResponse, Status>>;

    async fn task_subscription(
        &self,
        _request: Request<pb::TaskSubscriptionRequest>,
    ) -> Result<Response<Self::TaskSubscriptionStream>, Status> {
        Err(Status::unimplemented(
            "TaskSubscription: full upstream A2A task surface lands in v1.4",
        ))
    }

    async fn create_task_push_notification_config(
        &self,
        _request: Request<pb::CreateTaskPushNotificationConfigRequest>,
    ) -> Result<Response<pb::PushNotificationConfig>, Status> {
        Err(Status::unimplemented(
            "CreateTaskPushNotificationConfig: full upstream A2A task surface lands in v1.4",
        ))
    }

    async fn list_task(
        &self,
        _request: Request<pb::ListTaskRequest>,
    ) -> Result<Response<pb::ListTaskResponse>, Status> {
        Err(Status::unimplemented(
            "ListTask: full upstream A2A task surface lands in v1.4",
        ))
    }

    async fn get_authenticated_extended_agent_card(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::AgentCard>, Status> {
        // v1.3 ships only the standard card (signed via D-V1.2-cards).
        // The "extended" / authenticated variant adds per-operator
        // capability metadata; that requires plumbing the operator CN
        // through `Request::peer_certs()` and is queued for v1.4.
        Ok(Response::new(self.agent_card.clone()))
    }

    // ---------------------------------------------------------------
    // v1.4.3 — audit-log streaming (D-V1.4 / Phase 1.4.3).
    // ---------------------------------------------------------------

    type StreamAuditRecordsStream = ReceiverStream<Result<pb::AuditRecordEvent, Status>>;

    async fn stream_audit_records(
        &self,
        request: Request<pb::StreamAuditRecordsRequest>,
    ) -> Result<Response<Self::StreamAuditRecordsStream>, Status> {
        let Some(broadcast) = self.broadcast_audit.as_ref() else {
            return Err(Status::unavailable(
                "StreamAuditRecords requires the server to be configured with a BroadcastSink \
                 — see `A2aServer::with_broadcast_audit`",
            ));
        };

        let req = request.into_inner();
        let filter = crate::audit::AuditFilter {
            actor: req.actor_filter,
            action: req.action_filter,
            since_unix: req.since_unix,
        };

        let mut rx = broadcast.subscribe();
        let (tx, out_rx) = mpsc::channel::<Result<pb::AuditRecordEvent, Status>>(64);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(record) => {
                        if !filter.matches(&record) {
                            continue;
                        }
                        let event = pb::AuditRecordEvent {
                            timestamp_unix: record.timestamp_unix,
                            actor: record.actor,
                            action: record.action,
                            resource: record.resource,
                            prev_hash: record.prev_hash,
                            record_hash: record.record_hash,
                        };
                        if tx.send(Ok(event)).await.is_err() {
                            // Subscriber gone.
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        let _ = tx
                            .send(Err(Status::data_loss(format!(
                                "audit broadcast lagged {skipped} records"
                            ))))
                            .await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(out_rx)))
    }

    // ---------------------------------------------------------------
    // v1.4.7 — operator token issuance (Phase 1.4.7).
    // ---------------------------------------------------------------

    async fn issue_operator_token(
        &self,
        request: Request<pb::IssueOperatorTokenRequest>,
    ) -> Result<Response<pb::OperatorTokenReply>, Status> {
        let Some(identity) = self.server_identity.as_ref() else {
            return Err(Status::failed_precondition(
                "IssueOperatorToken requires the server to be configured with a NodeIdentity \
                 — see `A2aServer::with_server_identity`",
            ));
        };

        let req = request.into_inner();
        if req.operator_id.len() != 16 {
            return Err(Status::invalid_argument(format!(
                "operator_id must be 16 bytes, got {}",
                req.operator_id.len()
            )));
        }
        let mut operator_id = [0u8; 16];
        operator_id.copy_from_slice(&req.operator_id);

        // Cap to the configured max lifetime.
        let lifetime = req
            .lifetime_seconds
            .min(self.max_token_lifetime_seconds)
            .max(60); // floor at 60s to avoid trivially-expired tokens

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Status::internal(format!("system time: {e}")))?
            .as_secs();

        let (bytes, token) =
            crate::tokens::OperatorToken::issue(identity, operator_id, now, lifetime);

        Ok(Response::new(pb::OperatorTokenReply {
            token: bytes.to_vec(),
            issued_unix: token.issued_unix,
            expires_unix: token.expires_unix,
        }))
    }
}

/// First-frame dispatch shape. v1.2 splits operator-mode (shell-open) and
/// agent-mode (agent-register) streams onto separate handlers.
enum FirstFrame {
    ShellOpen {
        cols: u16,
        rows: u16,
        shell: Option<String>,
        target_agent_id: Option<String>,
    },
    AgentRegister(AgentRegisterParams),
}

async fn dispatch_stream<H: ShellHandler>(
    handler: Arc<H>,
    agent_registration: Arc<dyn AgentRegistrationHandler>,
    mut incoming: Streaming<pb::Message>,
    outgoing: mpsc::Sender<Result<pb::StreamResponse, Status>>,
    operator_token: Option<Vec<u8>>,
) {
    let first = match incoming.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(err)) => {
            let _ = outgoing.send(Err(err)).await;
            return;
        }
        None => {
            let _ = outgoing
                .send(Err(Status::invalid_argument(
                    "first frame missing; client closed stream",
                )))
                .await;
            return;
        }
    };

    let parsed = match parse_first_frame(&first) {
        Ok(parsed) => parsed,
        Err(status) => {
            let _ = outgoing.send(Err(status)).await;
            return;
        }
    };

    match parsed {
        FirstFrame::ShellOpen {
            cols,
            rows,
            shell,
            target_agent_id,
        } => {
            let task_id = if first.task_id.is_empty() {
                uuid_like()
            } else {
                first.task_id.clone()
            };
            let open = ShellOpenParams {
                cols,
                rows,
                shell,
                target_agent_id,
                task_id,
                // v1.3.5: real CN extraction from `Request::peer_certs()`
                // is wired by the server config layer (Phase 1.3.6+).
                // For now we ship `None` and the OperatorRouter falls
                // back to the agent-only capability check.
                operator_cn: None,
                // v1.4.7 — operator token (if any) was extracted +
                // verified by `send_streaming_message` from the
                // `x-nexus-operator-token` gRPC metadata header. The
                // OperatorRouter prefers this over the cert-CN path.
                operator_token: operator_token.clone(),
            };
            handler.handle_stream(open, incoming, outgoing).await;
        }
        FirstFrame::AgentRegister(params) => {
            agent_registration
                .handle_stream(params, incoming, outgoing)
                .await;
        }
    }
}

fn parse_first_frame(msg: &pb::Message) -> Result<FirstFrame, Status> {
    for part in &msg.parts {
        match ShellControl::try_from_part(part) {
            Ok(Some(ShellControl::ShellOpen {
                cols,
                rows,
                shell,
                target_agent_id,
            })) => {
                return Ok(FirstFrame::ShellOpen {
                    cols,
                    rows,
                    shell,
                    target_agent_id,
                });
            }
            Ok(Some(ShellControl::AgentRegister {
                peer_id_hex,
                os,
                version,
                tag,
            })) => {
                let peer_id = decode_peer_id(&peer_id_hex)
                    .map_err(|e| Status::invalid_argument(format!("bad peer_id_hex: {e}")))?;
                return Ok(FirstFrame::AgentRegister(AgentRegisterParams {
                    peer_id,
                    os,
                    version,
                    tag,
                }));
            }
            Ok(Some(other)) => warn!(?other, "first frame had unexpected control payload"),
            Ok(None) => {}
            Err(err) => {
                return Err(Status::invalid_argument(format!(
                    "first frame: bad JSON: {err}"
                )));
            }
        }
    }
    Err(Status::invalid_argument(
        "first frame must contain a shell-open or agent-register control part",
    ))
}

fn decode_peer_id(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 {
        return Err(format!("expected 64-char hex peer id, got {}", s.len()));
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let hex = &s[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// v1.4.7 finish — extract + verify the operator token from gRPC
/// metadata header `x-nexus-operator-token`. Returns `Some(token_bytes)`
/// only when the token decodes cleanly against `server_pubkey` and is
/// not expired. Verification failures or missing header → `None` (the
/// dispatch falls back to cert-CN scoping).
fn extract_operator_token(
    metadata: &tonic::metadata::MetadataMap,
    server_pubkey: &[u8; 32],
) -> Option<Vec<u8>> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let value = metadata.get(crate::tokens::TOKEN_METADATA_KEY)?;
    let token_str = value.to_str().ok()?;
    // Tokens are sent as hex in the metadata header to avoid
    // binary-data quirks. Decode hex → bytes.
    let token_bytes = hex_decode(token_str)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    crate::tokens::OperatorToken::decode_verified(&token_bytes, server_pubkey, now)
        .ok()
        .map(|_| token_bytes)
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("task-{now_nanos:032x}-{counter:08x}")
}
