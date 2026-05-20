//! v1.4 DTN store-and-forward (Phase 1.4.10 / D-V1.4-H).
//!
//! Per-recipient on-disk queue at `<root>/<peer_id_hex>/`. Each queued
//! envelope is one file: `<unix_seconds>-<seq>.bin`, containing the
//! `SealedEnvelope` in bincode form. The 13-character filename prefix
//! is the timestamp so directory listings come out chronologically.
//!
//! Bounded by:
//! - Per-recipient queue depth (oldest evicted on overflow).
//! - Per-envelope max age (default 7 days); the drain step skips
//!   stale entries.
//!
//! When a target peer reconnects (caller's responsibility to invoke
//! [`DtnQueue::drain_for`]), the queue replays its envelopes in
//! arrival order.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, warn};

/// Default DTN root directory.
pub const DEFAULT_ROOT: &str = "/var/lib/nexus-mesh/dtn";

/// Default per-recipient queue depth cap.
pub const DEFAULT_MAX_DEPTH: usize = 1_000;

/// Default queued-envelope max age (7 days).
pub const DEFAULT_MAX_AGE_SECONDS: u64 = 7 * 86_400;

/// Configuration for the DTN queue.
#[derive(Debug, Clone)]
pub struct DtnOptions {
    /// Root directory for the queue. Per-recipient sub-dirs live inside.
    pub root: PathBuf,
    /// Maximum queued envelopes per recipient.
    pub max_depth: usize,
    /// Maximum queued-envelope age in seconds.
    pub max_age_seconds: u64,
}

impl Default for DtnOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from(DEFAULT_ROOT),
            max_depth: DEFAULT_MAX_DEPTH,
            max_age_seconds: DEFAULT_MAX_AGE_SECONDS,
        }
    }
}

/// Persistent per-recipient queue.
pub struct DtnQueue {
    opts: DtnOptions,
    seq: AtomicU64,
}

/// One drained envelope ready for re-delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainedEnvelope {
    /// Unix seconds the envelope was queued at.
    pub queued_unix: u64,
    /// Bincode bytes of the original `SealedEnvelope`.
    pub bytes: Vec<u8>,
}

/// Errors emitted by DTN operations.
#[derive(Debug, thiserror::Error)]
pub enum DtnError {
    /// Filesystem I/O failure.
    #[error("dtn io: {0}")]
    Io(#[from] std::io::Error),
    /// Path / filename was malformed (corrupt queue dir).
    #[error("dtn malformed entry: {0}")]
    Malformed(String),
}

impl DtnQueue {
    /// Create a queue rooted at `opts.root`. Creates the directory
    /// tree if it doesn't exist yet.
    pub fn open(opts: DtnOptions) -> Result<Self, DtnError> {
        fs::create_dir_all(&opts.root)?;
        Ok(Self {
            opts,
            seq: AtomicU64::new(0),
        })
    }

    /// Queue `bytes` (a bincode-encoded `SealedEnvelope`) for delivery
    /// to `recipient_peer_id_hex` the next time the peer reconnects.
    /// Evicts the oldest entry if the queue would exceed `max_depth`.
    pub fn enqueue(&self, recipient_peer_id_hex: &str, bytes: &[u8]) -> Result<(), DtnError> {
        let dir = self.recipient_dir(recipient_peer_id_hex);
        fs::create_dir_all(&dir)?;

        // Evict the oldest entries until we're under cap.
        let mut entries = Self::list_entries(&dir)?;
        while entries.len() >= self.opts.max_depth {
            let oldest = entries.pop_front().expect("non-empty");
            let _ = fs::remove_file(dir.join(&oldest));
            debug!(recipient = %recipient_peer_id_hex, file = %oldest, "dtn: evicted oldest");
        }

        let now = unix_secs();
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let filename = format!("{:013}-{:08x}.bin", now, seq);
        let path = dir.join(&filename);
        fs::write(&path, bytes)?;
        debug!(recipient = %recipient_peer_id_hex, file = %filename, bytes = bytes.len(), "dtn: enqueued");
        Ok(())
    }

    /// Drain all currently-queued envelopes for `recipient_peer_id_hex`.
    /// Entries older than `max_age_seconds` are dropped silently
    /// during draining.
    pub fn drain_for(&self, recipient_peer_id_hex: &str) -> Result<Vec<DrainedEnvelope>, DtnError> {
        let dir = self.recipient_dir(recipient_peer_id_hex);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let entries = Self::list_entries(&dir)?;
        let now = unix_secs();
        let cutoff = now.saturating_sub(self.opts.max_age_seconds);
        let mut out = Vec::with_capacity(entries.len());
        for filename in entries {
            let path = dir.join(&filename);
            let queued_unix = match parse_timestamp_prefix(&filename) {
                Some(t) => t,
                None => {
                    warn!(file = %filename, "dtn: malformed filename, removing");
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };
            // Always remove (whether we deliver or expire).
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(err) => {
                    warn!(file = %filename, error = %err, "dtn: read failed");
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };
            let _ = fs::remove_file(&path);
            if queued_unix < cutoff {
                debug!(file = %filename, age = now - queued_unix, "dtn: expired during drain");
                continue;
            }
            out.push(DrainedEnvelope { queued_unix, bytes });
        }
        Ok(out)
    }

    /// Current queue depth for `recipient_peer_id_hex` (does not
    /// drain).
    pub fn depth_for(&self, recipient_peer_id_hex: &str) -> Result<usize, DtnError> {
        let dir = self.recipient_dir(recipient_peer_id_hex);
        if !dir.exists() {
            return Ok(0);
        }
        Ok(Self::list_entries(&dir)?.len())
    }

    fn recipient_dir(&self, recipient_peer_id_hex: &str) -> PathBuf {
        self.opts.root.join(recipient_peer_id_hex)
    }

    /// Return filenames in chronological order. Filenames begin with
    /// `<unix_seconds>-<seq>` so lexicographic sort = arrival order.
    fn list_entries(dir: &Path) -> Result<VecDeque<String>, DtnError> {
        let mut names: Vec<String> = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.ends_with(".bin") {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(VecDeque::from(names))
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_timestamp_prefix(filename: &str) -> Option<u64> {
    let dash = filename.find('-')?;
    filename[..dash].parse().ok()
}

/// v1.4.10 finish — convenience helpers pairing publish with DTN
/// queueing. Caller-driven: the operator decides what counts as
/// "offline" (typically: no subscriber in the topic's gossipsub mesh
/// peer set within a timeout), then calls these helpers.
///
/// The decoupling matters: libp2p gossipsub `publish` can succeed
/// with zero subscribers, so "publish returned Ok" ≠ "delivered".
/// DTN policy stays at the caller's layer.
pub mod publish_helpers {
    use super::*;

    /// Outcome of a [`publish_then_dtn`] call.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PublishOutcome {
        /// Caller indicated recipient is online; payload was not queued.
        Delivered,
        /// Recipient was deemed offline; payload was queued in the DTN.
        Queued,
    }

    /// Queue `bytes` in DTN unless `live_delivery_ok` is true.
    pub fn publish_then_dtn(
        queue: &DtnQueue,
        recipient_peer_id_hex: &str,
        bytes: &[u8],
        live_delivery_ok: bool,
    ) -> Result<PublishOutcome, DtnError> {
        if live_delivery_ok {
            return Ok(PublishOutcome::Delivered);
        }
        queue.enqueue(recipient_peer_id_hex, bytes)?;
        Ok(PublishOutcome::Queued)
    }

    /// Drain a recipient's queue (symmetry with `publish_then_dtn`).
    pub fn drain_on_reconnect(
        queue: &DtnQueue,
        recipient_peer_id_hex: &str,
    ) -> Result<Vec<DrainedEnvelope>, DtnError> {
        queue.drain_for(recipient_peer_id_hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir_opts() -> (tempfile::TempDir, DtnOptions) {
        let dir = tempfile::tempdir().expect("tempdir");
        let opts = DtnOptions {
            root: dir.path().to_path_buf(),
            max_depth: 5,
            max_age_seconds: 60,
        };
        (dir, opts)
    }

    #[test]
    fn enqueue_then_drain() {
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts).expect("open");
        queue.enqueue("aa", b"first").expect("enq");
        queue.enqueue("aa", b"second").expect("enq");
        assert_eq!(queue.depth_for("aa").unwrap(), 2);

        let drained = queue.drain_for("aa").expect("drain");
        assert_eq!(drained.len(), 2);
        assert_eq!(&drained[0].bytes, b"first");
        assert_eq!(&drained[1].bytes, b"second");
        assert_eq!(queue.depth_for("aa").unwrap(), 0);
    }

    #[test]
    fn depth_limit_evicts_oldest() {
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts.clone()).expect("open");
        for i in 0..(opts.max_depth + 3) {
            queue.enqueue("aa", &(i as u64).to_be_bytes()).expect("enq");
        }
        assert_eq!(queue.depth_for("aa").unwrap(), opts.max_depth);
        let drained = queue.drain_for("aa").expect("drain");
        // Earliest enqueues (0, 1, 2) should have been evicted.
        let first = u64::from_be_bytes(drained[0].bytes.as_slice().try_into().unwrap());
        assert!(first >= 3);
    }

    #[test]
    fn nonexistent_recipient_drains_empty() {
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts).expect("open");
        let drained = queue.drain_for("never-enqueued").expect("drain");
        assert!(drained.is_empty());
    }

    #[test]
    fn malformed_entry_is_removed() {
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts.clone()).expect("open");
        let dir = opts.root.join("aa");
        fs::create_dir_all(&dir).unwrap();
        // Write a file that doesn't match the `<ts>-<seq>.bin` shape.
        fs::write(dir.join("garbage.bin"), b"x").unwrap();
        let drained = queue.drain_for("aa").expect("drain");
        assert!(drained.is_empty(), "malformed entries skipped");
        assert!(!dir.join("garbage.bin").exists(), "malformed entry removed");
    }

    #[test]
    fn parse_timestamp_prefix_round_trip() {
        assert_eq!(
            parse_timestamp_prefix("0001779148800-deadbeef.bin"),
            Some(1_779_148_800)
        );
        assert_eq!(parse_timestamp_prefix("nope.bin"), None);
    }

    // v1.4.10 finish — publish_then_dtn / drain_on_reconnect.

    #[test]
    fn publish_then_dtn_queues_when_offline() {
        use publish_helpers::{publish_then_dtn, PublishOutcome};
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts).expect("open");

        let outcome = publish_then_dtn(&queue, "deadbeef", b"hello", false).expect("queue");
        assert_eq!(outcome, PublishOutcome::Queued);
        assert_eq!(queue.depth_for("deadbeef").unwrap(), 1);

        let outcome = publish_then_dtn(&queue, "deadbeef", b"live", true).expect("live");
        assert_eq!(outcome, PublishOutcome::Delivered);
        assert_eq!(queue.depth_for("deadbeef").unwrap(), 1);
    }

    #[test]
    fn drain_on_reconnect_returns_queued_envelopes() {
        use publish_helpers::{drain_on_reconnect, publish_then_dtn};
        let (_tmp, opts) = tempdir_opts();
        let queue = DtnQueue::open(opts).expect("open");
        publish_then_dtn(&queue, "deadbeef", b"first", false).expect("q1");
        publish_then_dtn(&queue, "deadbeef", b"second", false).expect("q2");

        let drained = drain_on_reconnect(&queue, "deadbeef").expect("drain");
        assert_eq!(drained.len(), 2);
        assert_eq!(&drained[0].bytes, b"first");
        assert_eq!(&drained[1].bytes, b"second");
        assert_eq!(queue.depth_for("deadbeef").unwrap(), 0);
    }
}
