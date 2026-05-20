//! In-memory `EchoShellHandler` for tests. Echoes every inbound bytes-frame
//! back; doesn't spawn a PTY.

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use tonic::Streaming;
use tracing::debug;

use crate::framing::{bytes_response, control_response, ShellControl};
use crate::handler::{ShellHandler, ShellOpenParams};
use crate::pb;

/// Echoes every inbound bytes-frame back. Used by tests so the A2A round
/// trip does not depend on `portable-pty`.
pub struct EchoShellHandler;

#[async_trait]
impl ShellHandler for EchoShellHandler {
    async fn handle_stream(
        &self,
        open: ShellOpenParams,
        mut incoming: Streaming<pb::Message>,
        outgoing: mpsc::Sender<Result<pb::StreamResponse, tonic::Status>>,
    ) {
        debug!(?open, "EchoShellHandler: stream opened");

        while let Some(item) = incoming.next().await {
            let msg = match item {
                Ok(m) => m,
                Err(err) => {
                    debug!(error = %err, "EchoShellHandler: inbound error");
                    break;
                }
            };
            for part in msg.parts {
                if let Some(pb::part::Part::File(bytes)) = part.part {
                    if outgoing
                        .send(Ok(bytes_response("agent", &open.task_id, bytes)))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }

        let exit = ShellControl::ShellExit { code: Some(0) };
        if let Ok(resp) = control_response("agent", &open.task_id, &exit) {
            let _ = outgoing.send(Ok(resp)).await;
        }
    }
}
