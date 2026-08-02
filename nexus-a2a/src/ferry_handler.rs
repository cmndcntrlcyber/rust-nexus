//! `HarnessFerryHandler` trait — server-side entry point for ferry RPCs.
//!
//! Follows the [`ShellHandler`](crate::handler::ShellHandler) pattern:
//! the [`A2aServer`](crate::A2aServer) delegates incoming `HarnessTask`
//! messages to an implementor that maps them to local task execution.

use async_trait::async_trait;

use crate::pb;

/// Server-side handler for `SubmitHarnessTask` unary RPCs.
///
/// Implementors receive a [`pb::HarnessTask`], dispatch the tool invocation
/// to the local `TaskExecutor`, and return a [`pb::HarnessTaskResult`].
#[async_trait]
pub trait HarnessFerryHandler: Send + Sync + 'static {
    /// Execute a single harness task and return the result.
    async fn handle_task(
        &self,
        task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, tonic::Status>;
}

/// Null implementation used when no ferry handler is attached.
pub(crate) struct NullFerryHandler;

#[async_trait]
impl HarnessFerryHandler for NullFerryHandler {
    async fn handle_task(
        &self,
        _task: pb::HarnessTask,
    ) -> Result<pb::HarnessTaskResult, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "harness ferry not enabled on this A2A server",
        ))
    }
}
