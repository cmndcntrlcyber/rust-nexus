//! `ChatDispatcher` trait — server-side entry point for `StreamChat` RPCs.
//!
//! Follows the [`HarnessFerryHandler`](crate::ferry_handler::HarnessFerryHandler)
//! pattern: the [`A2aServer`](crate::A2aServer) delegates incoming `ChatRequest`
//! messages to an implementor that routes them to LLM inference.
//!
//! Chat and ferry are separate traits because ferry is unary (tool execution:
//! one request → one response) while chat is streaming (LLM token generation:
//! one request → stream of `ChatChunk` deltas).

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::pb;

/// Server-side dispatcher for `StreamChat` streaming RPCs.
///
/// Implementors receive a [`pb::ChatRequest`], route it to the appropriate
/// LLM inference backend based on `ChatMode`, and stream [`pb::ChatChunk`]
/// deltas back through the provided `tx` channel. The final chunk MUST have
/// `done = true`.
///
/// rust-nexus has zero LLM code — the concrete implementation lives in
/// nexus-harness (`HarnessChatDispatcher`) which wraps the `Provider::chat_stream()`
/// from `crates/nexus-llm/`.
#[async_trait]
pub trait ChatDispatcher: Send + Sync + 'static {
    /// Handle a streaming chat request, sending `ChatChunk` deltas through `tx`.
    async fn dispatch_chat(
        &self,
        request: pb::ChatRequest,
        tx: mpsc::Sender<Result<pb::ChatChunk, tonic::Status>>,
    ) -> Result<(), tonic::Status>;
}

/// Null implementation for deployments without nexus-harness co-location.
pub struct NullChatDispatcher;

#[async_trait]
impl ChatDispatcher for NullChatDispatcher {
    async fn dispatch_chat(
        &self,
        request: pb::ChatRequest,
        tx: mpsc::Sender<Result<pb::ChatChunk, tonic::Status>>,
    ) -> Result<(), tonic::Status> {
        let _ = tx
            .send(Ok(pb::ChatChunk {
                text: "Chat dispatcher not configured. \
                       Connect nexus-harness to enable LLM inference."
                    .into(),
                done: true,
                session_id: request.session_id,
                meta: None,
            }))
            .await;
        Ok(())
    }
}
