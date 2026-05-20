//! `ShellSession` — PTY-backed remote shell.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::shell::select::ShellCommand;

/// Default PTY read chunk.
pub const DEFAULT_READ_CHUNK_BYTES: usize = 65_536;

const CMD_CHANNEL_CAPACITY: usize = 64;
const OUTPUT_CHANNEL_CAPACITY: usize = 128;

enum Cmd {
    Write(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

/// A live PTY-backed shell session.
pub struct ShellSession {
    cmd_tx: mpsc::Sender<Cmd>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    child_killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
    exit_status: Arc<Mutex<Option<Option<i32>>>>,
}

impl ShellSession {
    /// Spawn `cmd` under a new PTY.
    pub fn open(cmd: ShellCommand, cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let mut builder = CommandBuilder::new(&cmd.program);
        for arg in &cmd.args {
            builder.arg(arg);
        }
        for (k, v) in std::env::vars_os() {
            builder.env(k, v);
        }
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            builder.cwd(home);
        }

        let child = pair.slave.spawn_command(builder).context("spawn shell")?;
        let killer = child.clone_killer();
        drop(pair.slave);

        let reader = pair.master.try_clone_reader().context("clone PTY reader")?;
        let writer = pair.master.take_writer().context("take PTY writer")?;
        let master = pair.master;

        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(CMD_CHANNEL_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>(OUTPUT_CHANNEL_CAPACITY);

        spawn_reader_thread(reader, output_tx);
        spawn_writer_thread(writer, master, cmd_rx);

        let exit_status = Arc::new(Mutex::new(None));
        spawn_waiter_thread(child, Arc::clone(&exit_status));

        Ok(Self {
            cmd_tx,
            output_rx,
            child_killer: Arc::new(Mutex::new(killer)),
            exit_status,
        })
    }

    /// Write bytes to the PTY's stdin.
    pub async fn write(&self, bytes: &[u8]) -> Result<()> {
        self.cmd_tx
            .send(Cmd::Write(bytes.to_vec()))
            .await
            .map_err(|_| anyhow::anyhow!("shell session writer is closed"))
    }

    /// Resize the PTY.
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.cmd_tx
            .send(Cmd::Resize { cols, rows })
            .await
            .map_err(|_| anyhow::anyhow!("shell session writer is closed"))
    }

    /// Receive the next chunk of PTY output.
    pub async fn recv_output(&mut self) -> Option<Vec<u8>> {
        self.output_rx.recv().await
    }

    /// Best-effort observation of the child's exit code.
    #[must_use]
    pub fn exit_code(&self) -> Option<Option<i32>> {
        self.exit_status.lock().ok().and_then(|g| *g)
    }

    /// Send a kill signal to the child.
    pub fn kill(&self) -> Result<()> {
        let mut killer = self
            .child_killer
            .lock()
            .map_err(|_| anyhow::anyhow!("child killer mutex poisoned"))?;
        killer.kill().context("kill child")
    }
}

impl std::fmt::Debug for ShellSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSession")
            .field("exit_code", &self.exit_code())
            .finish_non_exhaustive()
    }
}

impl Drop for ShellSession {
    fn drop(&mut self) {
        if let Ok(mut killer) = self.child_killer.lock() {
            let _ = killer.kill();
        }
    }
}

fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    output_tx: mpsc::Sender<Vec<u8>>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("nexus-pty-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; DEFAULT_READ_CHUNK_BYTES];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        debug!("PTY reader hit EOF");
                        break;
                    }
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        if output_tx.blocking_send(chunk).is_err() {
                            debug!("PTY output channel closed");
                            break;
                        }
                    }
                    Err(err) => {
                        warn!(error = %err, "PTY read error");
                        break;
                    }
                }
            }
        })
        .expect("spawn reader")
}

fn spawn_writer_thread(
    mut writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    mut cmd_rx: mpsc::Receiver<Cmd>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("nexus-pty-writer".into())
        .spawn(move || {
            while let Some(cmd) = cmd_rx.blocking_recv() {
                match cmd {
                    Cmd::Write(bytes) => {
                        if let Err(err) = writer.write_all(&bytes) {
                            warn!(error = %err, "PTY write failed");
                            break;
                        }
                        if let Err(err) = writer.flush() {
                            warn!(error = %err, "PTY flush failed");
                            break;
                        }
                    }
                    Cmd::Resize { cols, rows } => {
                        let size = PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        };
                        if let Err(err) = master.resize(size) {
                            warn!(error = %err, cols, rows, "PTY resize failed");
                        }
                    }
                }
            }
            debug!("PTY command channel closed");
        })
        .expect("spawn writer")
}

fn spawn_waiter_thread(
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    exit_status: Arc<Mutex<Option<Option<i32>>>>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("nexus-pty-waiter".into())
        .spawn(move || match child.wait() {
            Ok(status) => {
                let code = if status.success() {
                    Some(0)
                } else {
                    let raw = status.exit_code();
                    if raw == 0 {
                        Some(0)
                    } else {
                        i32::try_from(raw).ok()
                    }
                };
                if let Ok(mut g) = exit_status.lock() {
                    *g = Some(code);
                }
            }
            Err(err) => {
                warn!(error = %err, "child wait failed");
                if let Ok(mut g) = exit_status.lock() {
                    *g = Some(None);
                }
            }
        })
        .expect("spawn waiter")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::shell::select::ShellSelect;

    #[tokio::test]
    async fn open_and_drop_round_trip() {
        let cmd = ShellSelect::for_host();
        let session = ShellSession::open(cmd, 80, 24).expect("open");
        drop(session);
    }

    #[tokio::test]
    async fn echo_marker_round_trip() {
        let cmd = ShellSelect::for_host();
        let mut session = ShellSession::open(cmd, 80, 24).expect("open shell");
        let probe: &[u8] = if cfg!(target_os = "windows") {
            b"Write-Host probe-marker\r\n"
        } else {
            b"echo probe-marker\n"
        };
        session.write(probe).await.expect("write to PTY");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let mut buf = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                panic!(
                    "timeout waiting for marker; collected: {:?}",
                    String::from_utf8_lossy(&buf)
                );
            }
            match timeout(remaining, session.recv_output()).await {
                Ok(Some(chunk)) => {
                    buf.extend_from_slice(&chunk);
                    if String::from_utf8_lossy(&buf).contains("probe-marker") {
                        return;
                    }
                }
                Ok(None) | Err(_) => panic!(
                    "stream closed / timeout; collected: {:?}",
                    String::from_utf8_lossy(&buf)
                ),
            }
        }
    }
}
