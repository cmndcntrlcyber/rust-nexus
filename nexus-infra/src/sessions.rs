//! `SessionRegistry` — `session_id → operator_tx` for routing inbound shell
//! output from an agent's A2A back-channel to the right operator stream.
//!
//! Used by the v1.1 [`crate::a2a_router::OperatorRouter`]. Session ids are
//! globally unique within a server process (atomic counter, not reused).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use nexus_a2a::pb as a2a_pb;
use tokio::sync::{mpsc, Mutex};
use tracing::debug;

/// Per-session record.
#[derive(Clone)]
pub struct SessionRecord {
    /// 32-byte peer id of the target agent (BLAKE3(uuid)).
    pub agent_peer_id: [u8; 32],
    /// Channel back to the operator's outbound A2A stream.
    pub operator_tx: mpsc::Sender<Result<a2a_pb::StreamResponse, tonic_14::Status>>,
}

/// Concurrent-access wrapper around `HashMap<u64, SessionRecord>`.
#[derive(Default, Clone)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<u64, SessionRecord>>>,
    next: Arc<AtomicU64>,
}

impl SessionRegistry {
    /// New empty registry. Ids start at 1; 0 is reserved.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::default(),
            next: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Allocate the next globally-unique session id.
    pub fn next_session_id(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// Insert a record.
    pub async fn insert(&self, session_id: u64, record: SessionRecord) {
        debug!(session_id, "session registered");
        self.inner.lock().await.insert(session_id, record);
    }

    /// Remove and return the record for `session_id`.
    pub async fn remove(&self, session_id: u64) -> Option<SessionRecord> {
        let removed = self.inner.lock().await.remove(&session_id);
        if removed.is_some() {
            debug!(session_id, "session unregistered");
        }
        removed
    }

    /// Get a clone of the record.
    pub async fn get(&self, session_id: u64) -> Option<SessionRecord> {
        self.inner.lock().await.get(&session_id).cloned()
    }

    /// Count of registered sessions.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// True when no sessions are registered.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}
