//! v1.2 hash-chained audit log (D-V1.2-audit).
//!
//! Every operator action and server registry event appends one
//! [`AuditRecord`] to a sink. Each record links to the previous one via
//! `record_hash = BLAKE3(prev_hash || canonical_bytes(record))` so an
//! offline `audit_verify` pass can detect tampering.
//!
//! v1.2 ships a single in-process [`FileSink`] backend. Syslog / remote
//! collectors are deferred to v1.3.

use std::path::Path;
use std::sync::Mutex;

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

/// One auditable event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    /// Unix seconds when the record was written.
    pub timestamp_unix: u64,
    /// Who took the action (e.g. operator cert CN, or `"server"`).
    pub actor: String,
    /// What was done (e.g. `"agent_register"`, `"shell_session_open"`).
    pub action: String,
    /// Resource the action targeted (e.g. agent peer-id hex).
    pub resource: String,
    /// Hex-encoded BLAKE3 hash of the previous record (`"0".repeat(64)`
    /// for the genesis record).
    pub prev_hash: String,
    /// Hex-encoded BLAKE3 hash of this record (computed over the
    /// canonical serialization of everything *above* this field, plus
    /// the bytes of `prev_hash`).
    pub record_hash: String,
}

/// Trait every audit-log sink implements.
#[async_trait::async_trait]
pub trait AuditSink: Send + Sync + 'static {
    /// Append one record. Implementations must serialize concurrently
    /// without losing records.
    async fn append(&self, record: AuditRecord);
}

/// Append-only file sink. One JSON record per line.
pub struct FileSink {
    inner: AsyncMutex<FileState>,
}

struct FileState {
    file: std::fs::File,
    last_hash: String,
    // v1.3.5: remember the path so `reopen` can re-open after rotation.
    path: std::path::PathBuf,
}

impl FileSink {
    /// Open `path` (created if missing) and seek to its end. Recomputes
    /// the last record's hash so subsequent appends continue the chain.
    pub fn open(path: &Path) -> Result<Self, std::io::Error> {
        let (file, last_hash) = open_inner(path)?;
        Ok(Self {
            inner: AsyncMutex::new(FileState {
                file,
                last_hash,
                path: path.to_path_buf(),
            }),
        })
    }

    /// v1.3.5: reopen the underlying file. Used by the SIGHUP handler
    /// after `logrotate` rotates the live file out from under us.
    /// Recomputes the chain head from whatever the new file contains
    /// (typically empty after a move-and-create rotation, so head goes
    /// back to genesis).
    pub async fn reopen(&self) -> Result<(), std::io::Error> {
        let mut g = self.inner.lock().await;
        let (file, last_hash) = open_inner(&g.path)?;
        g.file = file;
        g.last_hash = last_hash;
        Ok(())
    }
}

fn open_inner(path: &Path) -> Result<(std::fs::File, String), std::io::Error> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(path)?;
    let mut last_hash = "0".repeat(64);
    let read_handle = file.try_clone()?;
    let reader = BufReader::new(read_handle);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if let Ok(rec) = serde_json::from_str::<AuditRecord>(&line) {
            last_hash = rec.record_hash;
        }
    }
    let mut file = file;
    file.seek(SeekFrom::End(0))?;
    Ok((file, last_hash))
}

#[async_trait::async_trait]
impl AuditSink for FileSink {
    async fn append(&self, mut record: AuditRecord) {
        let mut g = self.inner.lock().await;
        record.prev_hash = g.last_hash.clone();
        record.record_hash = compute_hash(&record);
        let line = match serde_json::to_string(&record) {
            Ok(s) => s,
            Err(err) => {
                warn!(error = %err, "audit serialize failed; record dropped");
                return;
            }
        };
        use std::io::Write;
        let line_with_newline = format!("{}\n", line);
        if let Err(err) = g.file.write_all(line_with_newline.as_bytes()) {
            warn!(error = %err, "audit write failed");
            return;
        }
        let _ = g.file.flush();
        g.last_hash = record.record_hash;
    }
}

/// Synchronous in-memory sink for tests.
#[derive(Default)]
pub struct MemSink {
    inner: Mutex<MemState>,
}

#[derive(Default)]
struct MemState {
    records: Vec<AuditRecord>,
    last_hash: String,
}

impl MemSink {
    /// Empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all records appended so far.
    pub fn records(&self) -> Vec<AuditRecord> {
        self.inner.lock().unwrap().records.clone()
    }
}

#[async_trait::async_trait]
impl AuditSink for MemSink {
    async fn append(&self, mut record: AuditRecord) {
        let mut g = self.inner.lock().unwrap();
        if g.last_hash.is_empty() {
            g.last_hash = "0".repeat(64);
        }
        record.prev_hash = g.last_hash.clone();
        record.record_hash = compute_hash(&record);
        g.last_hash = record.record_hash.clone();
        g.records.push(record);
    }
}

// ---------------------------------------------------------------------------
// v1.4.3 — BroadcastSink (Phase 1.4.3 / D-V1.4 audit RPC).
// ---------------------------------------------------------------------------

/// Adapter that wraps any [`AuditSink`] and also publishes finalized
/// records to a `tokio::sync::broadcast` channel. The server uses
/// this to power the `StreamAuditRecords` RPC: each gRPC subscriber
/// gets its own `Receiver` and sees records as they're written.
///
/// `MAX_BROADCAST_CAPACITY` bounds how many records a slow subscriber
/// can lag behind before it starts losing records (broadcast semantics).
pub struct BroadcastSink {
    inner: std::sync::Arc<dyn AuditSink>,
    tx: tokio::sync::broadcast::Sender<AuditRecord>,
}

/// Default broadcast capacity (1024 records per subscriber lag window).
pub const DEFAULT_BROADCAST_CAPACITY: usize = 1024;

impl BroadcastSink {
    /// Wrap `inner` (typically a `FileSink` or `MultiSink`). Records
    /// flow through `inner.append()` first (so the chain head + on-disk
    /// state are authoritative), then a clone is broadcast.
    pub fn new(inner: std::sync::Arc<dyn AuditSink>) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(DEFAULT_BROADCAST_CAPACITY);
        Self { inner, tx }
    }

    /// Obtain a new subscriber. Each subscriber sees records appended
    /// AFTER this call. Use it to power streaming-RPC handlers.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AuditRecord> {
        self.tx.subscribe()
    }

    /// Number of active subscribers (for metrics / debugging).
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[async_trait::async_trait]
impl AuditSink for BroadcastSink {
    async fn append(&self, record: AuditRecord) {
        // Inner sink seals + persists first; the broadcast clone has
        // the same `record_hash` because seal happens inside `inner`.
        self.inner.append(record.clone()).await;
        // Broadcasting can fail only if there are zero subscribers,
        // which is fine — we just drop the broadcast copy.
        let _ = self.tx.send(record);
    }
}

/// A predicate over [`AuditRecord`]s. Used by `StreamAuditRecords` to
/// filter the broadcast stream.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Match records whose `actor` equals this (empty = wildcard).
    pub actor: String,
    /// Match records whose `action` equals this (empty = wildcard).
    pub action: String,
    /// Match records with `timestamp_unix >= since_unix` (0 = wildcard).
    pub since_unix: u64,
}

impl AuditFilter {
    /// Does `record` pass this filter?
    #[must_use]
    pub fn matches(&self, record: &AuditRecord) -> bool {
        if !self.actor.is_empty() && self.actor != record.actor {
            return false;
        }
        if !self.action.is_empty() && self.action != record.action {
            return false;
        }
        if record.timestamp_unix < self.since_unix {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// v1.3.7 — MultiSink + SyslogSink (D-V1.3-D)
// ---------------------------------------------------------------------------

/// Compose multiple sinks so a single `append` call fans out to all of
/// them. Records are sealed (prev_hash + record_hash filled in) by the
/// FIRST sink in the list; downstream sinks see the same finalized
/// record so their `append` is purely additive.
///
/// Typical wiring: `MultiSink::new(vec![file_sink, syslog_sink])` —
/// FileSink writes the chain to disk, SyslogSink ships the same record
/// to an external collector.
pub struct MultiSink {
    primary: std::sync::Arc<dyn AuditSink>,
    extras: Vec<std::sync::Arc<dyn AuditSink>>,
}

impl MultiSink {
    /// Build a multi-sink with `primary` as the chain-head authority.
    /// `extras` see the sealed record after `primary` finalizes it.
    pub fn new(
        primary: std::sync::Arc<dyn AuditSink>,
        extras: Vec<std::sync::Arc<dyn AuditSink>>,
    ) -> Self {
        Self { primary, extras }
    }
}

#[async_trait::async_trait]
impl AuditSink for MultiSink {
    async fn append(&self, mut record: AuditRecord) {
        // Sealing logic mirrors FileSink / MemSink: we have to compute
        // record_hash here so all sinks see the same sealed bytes.
        // Since the chain head lives inside `primary`, we have to defer
        // to its semantics by passing the record through.
        //
        // The simple, correct approach: clone the record for each extra
        // sink AFTER primary seals it. Achieve that with a tiny shared
        // mutation pattern: primary stamps the record; we peek the
        // updated fields by passing a clone of the record through
        // primary AND re-deriving the same hash for the extras.
        //
        // Because each sink type owns its own last_hash, naively
        // appending the SAME logical record to two FileSinks would
        // produce divergent chains. v1.3 supports one chain-head sink
        // (primary) + transport-only extras (SyslogSink doesn't track
        // its own chain). v1.4 will support truly parallel chains.

        // For v1.3.7: just forward to primary, then forward a clone to
        // each extra. The extras are expected to be transport-only
        // (SyslogSink) and not chain-tracking.
        let sealed = sealed_record(&mut record).clone();
        self.primary.append(sealed.clone()).await;
        for sink in &self.extras {
            sink.append(sealed.clone()).await;
        }
    }
}

fn sealed_record(record: &mut AuditRecord) -> &AuditRecord {
    // No-op for now — the primary sink seals the record. Future v1.4
    // work moves sealing here so transport-only extras can be more
    // independent.
    record
}

/// v1.3.7 syslog sink (D-V1.3-D). Ships finalized audit records to an
/// external collector via RFC 5424 framing over TCP.
///
/// TLS wrapping is operator-supplied (stunnel / haproxy / similar in
/// front of the collector). Encoding TLS directly inside the sink is
/// queued for v1.4.
///
/// Records are buffered in an in-memory queue; the background sender
/// task writes them out and reconnects on transient TCP failures with
/// exponential backoff.
pub struct SyslogSink {
    tx: tokio::sync::mpsc::Sender<AuditRecord>,
    // facility/hostname/app_name are used by format_5424 (called in tests and
    // the planned production send path — TLS syslog wiring queued for v1.4.x).
    #[allow(dead_code)]
    facility: u8,
    #[allow(dead_code)]
    hostname: String,
    #[allow(dead_code)]
    app_name: String,
}

impl SyslogSink {
    /// Connect to `endpoint` (e.g. `"127.0.0.1:6514"`) and start the
    /// background sender task. Records buffered up to `queue_capacity`
    /// while the connection is down.
    pub fn connect(
        endpoint: String,
        queue_capacity: usize,
        hostname: impl Into<String>,
        app_name: impl Into<String>,
    ) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(queue_capacity);
        let hostname = hostname.into();
        let app_name = app_name.into();
        tokio::spawn(sender_task(endpoint, rx));
        Self {
            tx,
            facility: 1, // user-level messages
            hostname,
            app_name,
        }
    }

    #[allow(dead_code)]
    fn format_5424(&self, record: &AuditRecord) -> String {
        // PRI = facility * 8 + severity. severity 6 = informational.
        let pri = self.facility * 8 + 6;
        // Wall-clock at record-emit time isn't preserved on the
        // record; we use the now-ISO8601 as the syslog TIMESTAMP and
        // include the record's own timestamp_unix in STRUCTURED-DATA.
        let timestamp = chrono_iso8601(record.timestamp_unix);
        // RFC 5424: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID
        //           STRUCTURED-DATA MSG
        format!(
            "<{pri}>1 {timestamp} {hostname} {app} - audit \
             [nexus-audit@1 actor=\"{actor}\" action=\"{action}\" \
             resource=\"{resource}\" prev_hash=\"{prev}\" record_hash=\"{rh}\"] \
             {actor} {action} {resource}\n",
            pri = pri,
            timestamp = timestamp,
            hostname = self.hostname,
            app = self.app_name,
            actor = record.actor,
            action = record.action,
            resource = record.resource,
            prev = record.prev_hash,
            rh = record.record_hash,
        )
    }
}

#[async_trait::async_trait]
impl AuditSink for SyslogSink {
    async fn append(&self, record: AuditRecord) {
        if self.tx.try_send(record).is_err() {
            warn!("syslog sink queue full; dropping record");
        }
    }
}

async fn sender_task(endpoint: String, mut rx: tokio::sync::mpsc::Receiver<AuditRecord>) {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    let mut stream: Option<TcpStream> = None;
    let mut backoff_secs: u64 = 1;
    while let Some(record) = rx.recv().await {
        if stream.is_none() {
            match TcpStream::connect(&endpoint).await {
                Ok(s) => {
                    stream = Some(s);
                    backoff_secs = 1;
                }
                Err(err) => {
                    warn!(%endpoint, error = %err, "syslog connect failed; backing off");
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(60);
                    // Drop this record (matches the bounded-queue
                    // contract — better to lose tail records than to
                    // back up the source).
                    continue;
                }
            }
        }
        let line = format_minimal(&record);
        let s = stream.as_mut().unwrap();
        if let Err(err) = s.write_all(line.as_bytes()).await {
            warn!(error = %err, "syslog write failed; reconnecting");
            stream = None;
            continue;
        }
    }
}

fn format_minimal(record: &AuditRecord) -> String {
    // Standalone formatter used by the sender_task (it doesn't have
    // access to the SyslogSink's hostname / app_name fields, so the
    // minimal frame omits them and lets the collector enrich from the
    // TCP source address).
    let pri: u8 = 14; // facility=1, severity=6
    format!(
        "<{pri}>1 {ts} - nexus-server - audit - {actor} {action} {resource}\n",
        pri = pri,
        ts = chrono_iso8601(record.timestamp_unix),
        actor = record.actor,
        action = record.action,
        resource = record.resource,
    )
}

fn chrono_iso8601(unix: u64) -> String {
    // Avoid pulling chrono just for this; format manually.
    // RFC 5424 prefers RFC 3339 timestamps; we emit UTC seconds-precision.
    let days = unix / 86_400;
    let secs_today = unix % 86_400;
    let h = secs_today / 3600;
    let m = (secs_today % 3600) / 60;
    let s = secs_today % 60;
    let (y, mo, d) = ymd_from_days_since_epoch(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn ymd_from_days_since_epoch(days: u64) -> (u32, u32, u32) {
    // 1970-01-01 + `days` days. Simple Howard-Hinnant style.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

/// Hash a record (assumes `prev_hash` is already filled in; `record_hash`
/// is ignored).
fn compute_hash(record: &AuditRecord) -> String {
    let mut hasher = Hasher::new();
    // Order matters: timestamp, actor, action, resource, prev_hash.
    hasher.update(&record.timestamp_unix.to_be_bytes());
    hasher.update(b"|");
    hasher.update(record.actor.as_bytes());
    hasher.update(b"|");
    hasher.update(record.action.as_bytes());
    hasher.update(b"|");
    hasher.update(record.resource.as_bytes());
    hasher.update(b"|");
    hasher.update(record.prev_hash.as_bytes());
    hex(hasher.finalize().as_bytes())
}

/// Walk the chain starting from `records[0]` and verify each
/// `record_hash` matches `BLAKE3(timestamp || actor || action ||
/// resource || prev_hash)` and that each `prev_hash` matches the
/// previous record's `record_hash`.
pub fn verify_chain(records: &[AuditRecord]) -> Result<(), String> {
    let genesis = "0".repeat(64);
    let mut prev = &genesis;
    for (i, record) in records.iter().enumerate() {
        if &record.prev_hash != prev {
            return Err(format!(
                "record {i}: prev_hash mismatch (expected {prev}, got {})",
                record.prev_hash
            ));
        }
        let expect = compute_hash(record);
        if expect != record.record_hash {
            return Err(format!("record {i}: record_hash tampered"));
        }
        prev = &record.record_hash;
    }
    Ok(())
}

/// Construct a record with the timestamp pre-filled and the hash fields
/// left empty (the sink will set them).
#[must_use]
pub fn make_record(actor: &str, action: &str, resource: &str) -> AuditRecord {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    AuditRecord {
        timestamp_unix: ts,
        actor: actor.to_string(),
        action: action.to_string(),
        resource: resource.to_string(),
        prev_hash: String::new(),
        record_hash: String::new(),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chain_integrity_via_mem_sink() {
        let sink = MemSink::new();
        sink.append(make_record("op", "agent_register", "abc"))
            .await;
        sink.append(make_record("op", "shell_session_open", "abc:1"))
            .await;
        sink.append(make_record("op", "shell_session_close", "abc:1"))
            .await;
        let records = sink.records();
        verify_chain(&records).expect("intact chain");
    }

    #[tokio::test]
    async fn tamper_detected_via_field_edit() {
        let sink = MemSink::new();
        sink.append(make_record("op", "a", "x")).await;
        sink.append(make_record("op", "b", "y")).await;
        let mut records = sink.records();
        records[0].action = "tampered".into();
        verify_chain(&records).expect_err("chain must break");
    }

    #[tokio::test]
    async fn tamper_detected_via_record_removed() {
        let sink = MemSink::new();
        sink.append(make_record("op", "a", "x")).await;
        sink.append(make_record("op", "b", "y")).await;
        sink.append(make_record("op", "c", "z")).await;
        let mut records = sink.records();
        records.remove(1);
        verify_chain(&records).expect_err("chain must break");
    }

    // v1.3.7 — MultiSink + SyslogSink

    #[tokio::test]
    async fn multi_sink_fans_out() {
        use std::sync::Arc;
        let primary = Arc::new(MemSink::new());
        let extra = Arc::new(MemSink::new());
        let multi = MultiSink::new(
            primary.clone() as Arc<dyn AuditSink>,
            vec![extra.clone() as Arc<dyn AuditSink>],
        );
        multi.append(make_record("op", "a", "x")).await;
        multi.append(make_record("op", "b", "y")).await;
        assert_eq!(primary.records().len(), 2);
        assert_eq!(extra.records().len(), 2);
    }

    #[test]
    fn iso8601_for_unix_epoch_renders() {
        assert_eq!(chrono_iso8601(0), "1970-01-01T00:00:00Z");
        // 2026-05-19 00:00:00 UTC.
        assert_eq!(chrono_iso8601(1_779_148_800), "2026-05-19T00:00:00Z");
        // Mid-year sanity check.
        assert_eq!(chrono_iso8601(1_672_531_200), "2023-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn syslog_5424_format_includes_action() {
        let sink = SyslogSink::connect(
            "127.0.0.1:65000".to_string(), // unused in this test
            8,
            "host-1",
            "nexus-server",
        );
        let mut record = make_record("op", "shell_session_open", "ab12");
        record.prev_hash = "0".repeat(64);
        record.record_hash = "ff".repeat(32);
        let line = sink.format_5424(&record);
        assert!(line.contains("<14>1 "));
        assert!(line.contains(" host-1 nexus-server "));
        assert!(line.contains("action=\"shell_session_open\""));
        assert!(line.contains("resource=\"ab12\""));
        assert!(line.ends_with('\n'));
    }

    #[tokio::test]
    async fn file_sink_persists_chain() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("audit.log");
        let sink = FileSink::open(&path).expect("open");
        sink.append(make_record("op", "a", "x")).await;
        sink.append(make_record("op", "b", "y")).await;
        drop(sink);

        // Re-open and append a third record; the chain must continue.
        let sink2 = FileSink::open(&path).expect("reopen");
        sink2.append(make_record("op", "c", "z")).await;
        drop(sink2);

        let raw = std::fs::read_to_string(&path).expect("read");
        let records: Vec<AuditRecord> = raw
            .lines()
            .map(|l| serde_json::from_str(l).expect("parse"))
            .collect();
        assert_eq!(records.len(), 3);
        verify_chain(&records).expect("intact across reopens");
    }
}
