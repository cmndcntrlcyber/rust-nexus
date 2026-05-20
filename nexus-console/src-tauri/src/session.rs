//! Per-shell-session driver task.

use futures::StreamExt;
use nexus_a2a::framing::{control_request, ShellControl};
use nexus_a2a::pb as a2a_pb;
use nexus_a2a::A2aClient;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Tauri event name for shell stdout.
pub const SHELL_OUTPUT_EVENT: &str = "shell-output";
/// Tauri event name for shell exit.
pub const SHELL_EXIT_EVENT: &str = "shell-exit";
/// Tauri event name for shell errors.
pub const SHELL_ERROR_EVENT: &str = "shell-error";

/// `shell-output` payload.
#[derive(Debug, Clone, Serialize)]
pub struct ShellOutputPayload {
    /// Session id.
    pub session_id: u64,
    /// PTY bytes.
    pub bytes: Vec<u8>,
}

/// `shell-exit` payload.
#[derive(Debug, Clone, Serialize)]
pub struct ShellExitPayload {
    /// Session id.
    pub session_id: u64,
    /// Exit code.
    pub code: Option<i32>,
}

/// `shell-error` payload.
#[derive(Debug, Clone, Serialize)]
pub struct ShellErrorPayload {
    /// Session id.
    pub session_id: u64,
    /// Error message.
    pub message: String,
}

/// Outbound channel capacity.
pub const OUTBOUND_CAPACITY: usize = 64;

/// Spawn the driver task: send the initial `shell-open`, then run the bidi
/// loop until the operator closes the outbound side or the agent emits
/// `shell-exit`.
pub async fn open_session(
    app: AppHandle,
    mut client: A2aClient,
    session_id: u64,
    target_agent_id: Option<String>,
    cols: u16,
    rows: u16,
) -> anyhow::Result<(mpsc::Sender<a2a_pb::Message>, tokio::task::JoinHandle<()>)> {
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<a2a_pb::Message>(OUTBOUND_CAPACITY);

    let (client_tx, mut inbound) = client.open_streaming_message().await?;

    let open = ShellControl::ShellOpen {
        cols,
        rows,
        shell: None,
        target_agent_id,
    };
    let task_id = format!("console-{session_id}");
    let open_msg = control_request(&task_id, &open)?;
    client_tx
        .send(open_msg)
        .await
        .map_err(|_| anyhow::anyhow!("outbound channel closed before shell-open"))?;

    let app_for_task = app.clone();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                outbound = outbound_rx.recv() => {
                    let Some(msg) = outbound else {
                        drop(client_tx);
                        drain_inbound(&app_for_task, session_id, &mut inbound).await;
                        return;
                    };
                    if client_tx.send(msg).await.is_err() {
                        return;
                    }
                }
                inbound_item = inbound.next() => {
                    match inbound_item {
                        Some(Ok(resp)) => emit_response(&app_for_task, session_id, resp),
                        Some(Err(status)) => {
                            warn!(session_id, ?status, "shell stream status err");
                            let _ = app_for_task.emit(
                                SHELL_ERROR_EVENT,
                                ShellErrorPayload {
                                    session_id,
                                    message: format!("{status}"),
                                },
                            );
                            return;
                        }
                        None => {
                            debug!(session_id, "inbound stream closed");
                            return;
                        }
                    }
                }
            }
        }
    });

    Ok((outbound_tx, task))
}

async fn drain_inbound(
    app: &AppHandle,
    session_id: u64,
    inbound: &mut tonic::Streaming<a2a_pb::StreamResponse>,
) {
    while let Some(item) = inbound.next().await {
        match item {
            Ok(resp) => emit_response(app, session_id, resp),
            Err(status) => {
                let _ = app.emit(
                    SHELL_ERROR_EVENT,
                    ShellErrorPayload {
                        session_id,
                        message: format!("{status}"),
                    },
                );
                break;
            }
        }
    }
}

fn emit_response(app: &AppHandle, session_id: u64, resp: a2a_pb::StreamResponse) {
    let Some(a2a_pb::stream_response::Payload::Message(msg)) = resp.payload else {
        return;
    };
    for part in msg.parts {
        if let Ok(Some(ctrl)) = ShellControl::try_from_part(&part) {
            if let ShellControl::ShellExit { code } = ctrl {
                let _ = app.emit(SHELL_EXIT_EVENT, ShellExitPayload { session_id, code });
            }
            continue;
        }
        if let Some(a2a_pb::part::Part::File(bytes)) = part.part {
            if !bytes.is_empty() {
                let _ = app.emit(SHELL_OUTPUT_EVENT, ShellOutputPayload { session_id, bytes });
            }
        }
    }
}
