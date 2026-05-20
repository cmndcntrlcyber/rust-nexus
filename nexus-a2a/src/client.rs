//! `A2aClient` — thin async wrapper around the generated Tonic client.

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};
use tonic::Streaming;

use crate::insecure;
use crate::pb;
use crate::pb::a2a_service_client::A2aServiceClient;

/// Default mpsc channel capacity for outbound shell-session frames.
pub const CLIENT_TX_CAPACITY: usize = 64;

/// Thin wrapper.
#[derive(Clone)]
pub struct A2aClient {
    inner: A2aServiceClient<Channel>,
}

impl A2aClient {
    /// Connect to `addr` (e.g. `"http://127.0.0.1:50051"`). Enforces D-V1-E.
    pub async fn connect(addr: &str, insecure_network: bool) -> anyhow::Result<Self> {
        Self::connect_with_optional_tls(addr, insecure_network, None).await
    }

    /// Connect to `addr` with optional client TLS config (v1.2). Pass
    /// `Some(cfg)` for mTLS; `None` matches [`A2aClient::connect`].
    pub async fn connect_with_optional_tls(
        addr: &str,
        insecure_network: bool,
        tls: Option<tonic::transport::ClientTlsConfig>,
    ) -> anyhow::Result<Self> {
        let mut endpoint = Endpoint::from_shared(addr.to_string())?;
        let uri = endpoint.uri().clone();
        let host = uri
            .host()
            .ok_or_else(|| anyhow::anyhow!("missing host in {addr}"))?;
        let port = uri
            .port_u16()
            .ok_or_else(|| anyhow::anyhow!("missing port in {addr}"))?;
        let socket = format!("{host}:{port}");
        insecure::enforce(socket.as_str(), insecure_network)?;
        if let Some(t) = tls {
            endpoint = endpoint.tls_config(t)?;
        }
        let channel = endpoint.connect().await?;
        Ok(Self {
            inner: A2aServiceClient::new(channel),
        })
    }

    /// Wrap an existing channel.
    #[must_use]
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            inner: A2aServiceClient::new(channel),
        }
    }

    /// `GetAgentCard` RPC. Returns the card without verification — the
    /// caller can verify by passing it to [`crate::cards::verify`].
    pub async fn get_agent_card(&mut self) -> Result<pb::AgentCard, tonic::Status> {
        let response = self.inner.get_agent_card(pb::Empty {}).await?;
        Ok(response.into_inner())
    }

    /// `GetAgentCard` RPC, with Ed25519 signature verification (v1.2).
    /// Returns the verified card on success or `Status::permission_denied`
    /// with a clear reason if the signature is missing or invalid.
    pub async fn get_agent_card_verified(&mut self) -> Result<pb::AgentCard, tonic::Status> {
        let card = self.get_agent_card().await?;
        crate::cards::verify(&card)
            .map_err(|e| tonic::Status::permission_denied(format!("agent card signature: {e}")))?;
        Ok(card)
    }

    /// `ListRegisteredAgents` RPC.
    pub async fn list_registered_agents(
        &mut self,
    ) -> Result<Vec<pb::RegisteredAgent>, tonic::Status> {
        let response = self.inner.list_registered_agents(pb::Empty {}).await?;
        Ok(response.into_inner().agents)
    }

    /// `SendMessage` unary RPC.
    pub async fn send_message(
        &mut self,
        message: pb::Message,
    ) -> Result<pb::Message, tonic::Status> {
        let response = self.inner.send_message(message).await?;
        Ok(response.into_inner())
    }

    /// Open a `SendStreamingMessage` bidi stream.
    pub async fn open_streaming_message(
        &mut self,
    ) -> Result<(mpsc::Sender<pb::Message>, Streaming<pb::StreamResponse>), tonic::Status> {
        let (tx, rx) = mpsc::channel::<pb::Message>(CLIENT_TX_CAPACITY);
        let outbound = ReceiverStream::new(rx);
        let response = self.inner.send_streaming_message(outbound).await?;
        Ok((tx, response.into_inner()))
    }

    /// v1.4.3 — subscribe to `StreamAuditRecords` on the server.
    /// Returns the server-streaming `AuditRecordEvent` channel.
    pub async fn stream_audit_records(
        &mut self,
        request: pb::StreamAuditRecordsRequest,
    ) -> Result<Streaming<pb::AuditRecordEvent>, tonic::Status> {
        let response = self.inner.stream_audit_records(request).await?;
        Ok(response.into_inner())
    }

    /// v1.4.7 — request a signed operator token from the server.
    pub async fn issue_operator_token(
        &mut self,
        request: pb::IssueOperatorTokenRequest,
    ) -> Result<pb::OperatorTokenReply, tonic::Status> {
        let response = self.inner.issue_operator_token(request).await?;
        Ok(response.into_inner())
    }
}
