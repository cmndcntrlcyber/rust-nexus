//! Shared console state held in `tauri::State`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use nexus_a2a::pb as a2a_pb;
use nexus_a2a::A2aClient;
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

/// Per-session handle.
pub struct SessionHandle {
    /// Outbound channel into the A2A stream.
    pub tx: mpsc::Sender<a2a_pb::Message>,
    /// Driver task.
    pub task: JoinHandle<()>,
    /// Hex peer-id of the target agent.
    pub target_agent_id: Option<String>,
}

/// Active connection.
pub struct Connection {
    /// Original address.
    pub addr: String,
    /// Loopback gate state.
    pub insecure_network: bool,
    /// Live A2A client.
    pub client: A2aClient,
    /// Cached server card.
    pub server_card: a2a_pb::AgentCard,
}

/// Console state shared across Tauri commands.
#[derive(Default)]
pub struct ConsoleState {
    inner: Arc<Mutex<Inner>>,
    next_session_id: AtomicU64,
}

#[derive(Default)]
struct Inner {
    connection: Option<Connection>,
    sessions: HashMap<u64, SessionHandle>,
}

impl ConsoleState {
    /// New empty state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            next_session_id: AtomicU64::new(1),
        }
    }

    /// Allocate the next session id.
    pub fn allocate_session_id(&self) -> u64 {
        self.next_session_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Set the active connection.
    pub async fn set_connection(&self, conn: Connection) {
        self.inner.lock().await.connection = Some(conn);
    }

    /// Drop the active connection.
    pub async fn clear_connection(&self) -> Option<Connection> {
        self.inner.lock().await.connection.take()
    }

    /// Clone the active A2A client.
    pub async fn client(&self) -> Result<A2aClient> {
        let guard = self.inner.lock().await;
        guard
            .connection
            .as_ref()
            .map(|c| c.client.clone())
            .ok_or_else(|| anyhow!("not connected to C2"))
    }

    /// Connection metadata snapshot.
    pub async fn connection_summary(&self) -> Option<ConnectionSummary> {
        self.inner
            .lock()
            .await
            .connection
            .as_ref()
            .map(|c| ConnectionSummary {
                addr: c.addr.clone(),
                insecure_network: c.insecure_network,
                server_name: c.server_card.name.clone(),
                server_version: c.server_card.version.clone(),
            })
    }

    /// Insert a session.
    pub async fn insert_session(&self, session_id: u64, handle: SessionHandle) {
        self.inner.lock().await.sessions.insert(session_id, handle);
    }

    /// Remove a session.
    pub async fn remove_session(&self, session_id: u64) -> Option<SessionHandle> {
        self.inner.lock().await.sessions.remove(&session_id)
    }

    /// Clone of the outbound channel for `session_id`.
    pub async fn session_tx(&self, session_id: u64) -> Option<mpsc::Sender<a2a_pb::Message>> {
        self.inner
            .lock()
            .await
            .sessions
            .get(&session_id)
            .map(|h| h.tx.clone())
    }

    /// Count of active sessions.
    pub async fn active_session_count(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_state_construction() {
        let state = ConsoleState::new();
        let _ = state;
    }

    #[test]
    fn test_console_state_default() {
        let state = ConsoleState::default();
        let _ = state;
    }

    #[test]
    fn test_allocate_session_id_increments() {
        let state = ConsoleState::new();
        let id1 = state.allocate_session_id();
        let id2 = state.allocate_session_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_no_connection_returns_error() {
        let state = ConsoleState::new();
        let result = state.client().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_summary_none_when_disconnected() {
        let state = ConsoleState::new();
        assert!(state.connection_summary().await.is_none());
    }

    #[tokio::test]
    async fn test_active_session_count_starts_at_zero() {
        let state = ConsoleState::new();
        assert_eq!(state.active_session_count().await, 0);
    }
}

/// Tauri-command-friendly summary.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionSummary {
    /// Address the operator typed.
    pub addr: String,
    /// Whether the loopback gate was bypassed.
    pub insecure_network: bool,
    /// Cached server name.
    pub server_name: String,
    /// Cached server version.
    pub server_version: String,
}
