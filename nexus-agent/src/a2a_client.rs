//! Agent-side A2A bidi client (v1.2 — interactive shell sessions).
//!
//! Dials the C2's A2A `SendStreamingMessage` RPC, registers itself with an
//! `agent-register` first frame, then runs an inbound loop that:
//!
//! - Receives `shell-open` frames from the C2 (operator-initiated) and spawns
//!   a per-session PTY via [`crate::shell::ShellSession`].
//! - Forwards `shell-resize` to the appropriate PTY.
//! - Forwards `Part::file` byte chunks as PTY stdin.
//! - Forwards `shell-exit` (operator-side close) to kill the PTY.
//! - Pumps PTY output back to the C2 as `bytes_response` frames tagged with
//!   the same `task_id` the operator opened the session with.
//! - Emits `shell-exit` back to the C2 when the PTY exits.
//!
//! Multiple concurrent sessions are demultiplexed by `task_id`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::{pb, A2aClient};
use nexus_common::{NodeIdentity, OsKind};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

use crate::shell::{ShellSelect, ShellSession};

/// Configuration for the agent-side A2A bidi client.
#[derive(Debug, Clone)]
pub struct A2aClientConfig {
    /// C2 A2A gRPC URL (e.g. `"http://127.0.0.1:50052"`).
    pub c2_addr: String,
    /// Operator-supplied tag.
    pub tag: String,
    /// Allow non-loopback dials per D-V1-E.
    pub insecure_network: bool,
}

const SESSION_CMD_CAPACITY: usize = 64;

/// Probe the C2's A2A endpoint to verify connectivity. Returns the server's
/// `AgentCard.name` on success.
pub async fn probe_c2(cfg: &A2aClientConfig) -> Result<String> {
    let mut client = A2aClient::connect(&cfg.c2_addr, cfg.insecure_network)
        .await
        .with_context(|| format!("connect A2A {}", cfg.c2_addr))?;
    let card = client.get_agent_card().await.context("get_agent_card")?;
    Ok(card.name)
}

/// Dial the C2, register, and serve interactive shell sessions until the
/// stream closes or `shutdown` resolves.
pub async fn connect_and_serve(
    cfg: &A2aClientConfig,
    identity: &NodeIdentity,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> Result<()> {
    let mut client = A2aClient::connect(&cfg.c2_addr, cfg.insecure_network)
        .await
        .with_context(|| format!("connect A2A {}", cfg.c2_addr))?;

    let (tx, mut rx) = client
        .open_streaming_message()
        .await
        .context("open bidi stream")?;

    // Send agent-register as the first frame on the stream.
    let peer_id_hex = peer_id_hex(identity.peer_id());
    let register = ShellControl::AgentRegister {
        peer_id_hex,
        os: os_label(OsKind::detect()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        tag: if cfg.tag.is_empty() {
            None
        } else {
            Some(cfg.tag.clone())
        },
    };
    let register_msg = control_request("", &register).context("encode agent-register")?;
    tx.send(register_msg)
        .await
        .map_err(|_| anyhow!("send agent-register: C2 stream closed before handshake"))?;
    info!(addr = %cfg.c2_addr, "A2A agent-mode stream registered");

    // Session table: task_id → outbound command channel to the per-session task.
    let sessions: Arc<Mutex<HashMap<String, mpsc::Sender<SessionCmd>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("agent A2A loop received shutdown");
                break;
            }
            inbound = rx.message() => {
                match inbound {
                    Ok(Some(response)) => {
                        if let Err(err) = handle_inbound(response, &tx, Arc::clone(&sessions)).await {
                            warn!(error = %err, "inbound frame dispatch failed");
                        }
                    }
                    Ok(None) => {
                        info!("C2 closed the bidi stream cleanly");
                        break;
                    }
                    Err(err) => {
                        return Err(anyhow!("stream error: {err}"));
                    }
                }
            }
        }
    }

    // Kill any sessions still alive.
    let mut guard = sessions.lock().await;
    for (_, sender) in guard.drain() {
        let _ = sender.send(SessionCmd::Close).await;
    }

    Ok(())
}

enum SessionCmd {
    Bytes(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Close,
}

async fn handle_inbound(
    response: pb::StreamResponse,
    tx: &mpsc::Sender<pb::Message>,
    sessions: Arc<Mutex<HashMap<String, mpsc::Sender<SessionCmd>>>>,
) -> Result<()> {
    let payload = response
        .payload
        .ok_or_else(|| anyhow!("StreamResponse missing payload"))?;
    let message = match payload {
        pb::stream_response::Payload::Message(m) => m,
        pb::stream_response::Payload::Task(_) => {
            debug!("agent ignored Task payload on shell-session stream");
            return Ok(());
        }
    };

    let task_id = message.task_id.clone();
    if task_id.is_empty() {
        return Err(anyhow!("inbound message missing task_id"));
    }

    for part in &message.parts {
        match ShellControl::try_from_part(part) {
            Ok(Some(ShellControl::ShellOpen {
                cols, rows, shell, ..
            })) => {
                let mut guard = sessions.lock().await;
                if guard.contains_key(&task_id) {
                    warn!(%task_id, "shell-open for existing session — ignoring");
                    continue;
                }
                let cmd = ShellSelect::for_host_with_override(shell.as_deref());
                let session = ShellSession::open(cmd, cols, rows)
                    .with_context(|| format!("open shell for session {task_id}"))?;
                let (cmd_tx, cmd_rx) = mpsc::channel::<SessionCmd>(SESSION_CMD_CAPACITY);
                guard.insert(task_id.clone(), cmd_tx);
                let tx_clone = tx.clone();
                let task_id_clone = task_id.clone();
                let sessions_clone = Arc::clone(&sessions);
                tokio::spawn(async move {
                    drive_session(session, cmd_rx, task_id_clone.clone(), tx_clone).await;
                    let mut g = sessions_clone.lock().await;
                    g.remove(&task_id_clone);
                });
                info!(%task_id, cols, rows, "opened agent-side shell session");
            }
            Ok(Some(ShellControl::ShellResize { cols, rows })) => {
                let guard = sessions.lock().await;
                if let Some(sender) = guard.get(&task_id) {
                    let _ = sender.send(SessionCmd::Resize { cols, rows }).await;
                }
            }
            Ok(Some(ShellControl::ShellExit { code })) => {
                debug!(%task_id, ?code, "operator-side close requested");
                let guard = sessions.lock().await;
                if let Some(sender) = guard.get(&task_id) {
                    let _ = sender.send(SessionCmd::Close).await;
                }
            }
            Ok(Some(other)) => {
                debug!(?other, "agent ignored control frame on agent-mode stream");
            }
            Ok(None) => {
                // Raw bytes — PTY stdin.
                if let Some(pb::part::Part::File(bytes)) = part.part.clone() {
                    let guard = sessions.lock().await;
                    if let Some(sender) = guard.get(&task_id) {
                        let _ = sender.send(SessionCmd::Bytes(bytes)).await;
                    } else {
                        debug!(%task_id, "stdin bytes for unknown session — dropping");
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, "bad control JSON on agent-mode stream");
            }
        }
    }
    Ok(())
}

async fn drive_session(
    mut session: ShellSession,
    mut cmd_rx: mpsc::Receiver<SessionCmd>,
    task_id: String,
    tx: mpsc::Sender<pb::Message>,
) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => match cmd {
                Some(SessionCmd::Bytes(bytes)) => {
                    if let Err(err) = session.write(&bytes).await {
                        warn!(%task_id, error = %err, "PTY write failed");
                        break;
                    }
                }
                Some(SessionCmd::Resize { cols, rows }) => {
                    if let Err(err) = session.resize(cols, rows).await {
                        warn!(%task_id, error = %err, "PTY resize failed");
                    }
                }
                Some(SessionCmd::Close) | None => {
                    let _ = session.kill();
                    break;
                }
            },
            chunk = session.recv_output() => match chunk {
                Some(bytes) => {
                    let mut msg = bytes_request(&task_id, bytes);
                    msg.role = "agent".to_string();
                    if tx.send(msg).await.is_err() {
                        debug!(%task_id, "C2 stream closed; ending session");
                        let _ = session.kill();
                        break;
                    }
                }
                None => {
                    debug!(%task_id, "PTY closed");
                    break;
                }
            },
        }
    }

    let exit = session.exit_code().flatten();
    if let Ok(part) = (ShellControl::ShellExit { code: exit }).to_text_part() {
        let mut msg = pb::Message {
            message_id: String::new(),
            role: "agent".into(),
            parts: vec![part],
            task_id: task_id.clone(),
        };
        // ensure the task_id is set even though pb::Message::default doesn't.
        msg.task_id = task_id;
        let _ = tx.send(msg).await;
    }
}

fn peer_id_hex(peer_id: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in peer_id {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn os_label(kind: OsKind) -> String {
    match kind {
        OsKind::Linux => "linux".to_string(),
        OsKind::Windows => "windows".to_string(),
        OsKind::MacOS => "macos".to_string(),
        OsKind::Other => "other".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_hex_round_trip() {
        let bytes = [0xabu8; 32];
        let hex = peer_id_hex(bytes);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
    }
}
