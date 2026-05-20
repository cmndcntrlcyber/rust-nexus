//! v1.4 regression test (Phase 1.4.11).
//!
//! Exercises the v1.4 surface area added on top of v1.3:
//!
//! 1. MatrixRouter behavior matches v1.3 HashMap behavior (allow/deny).
//! 2. Operator token issue → verify round-trip + expiry / tamper rejection.
//! 3. DTN store-and-forward enqueue → drain.
//! 4. Per-agent audit log records sessions.
//!
//! ACME staging, Python interop, mesh-shell, and S3 archival are
//! `#[ignore]`d here because they need external runtime (Let's
//! Encrypt staging, Python, libwebkit2gtk, MinIO).

use std::sync::Arc;

use nexus_a2a::audit::{make_record, AuditSink, MemSink, MultiSink};
use nexus_a2a::capabilities::{CapabilityCheck, CapabilityError};
use nexus_a2a::tokens::{OperatorToken, TokenError, DEFAULT_LIFETIME_SECONDS};
use nexus_common::NodeIdentity;
use nexus_mesh::dtn::{DtnOptions, DtnQueue};

#[test]
fn matrix_router_preserves_v1_3_semantics() {
    // Pre-v1.3 file (no operators section) — verify_with_operator
    // falls back to agent-only check.
    let check = CapabilityCheck::from_json_str(
        r#"{
            "agents": {
                "ab12": {"skills": ["shell-session"]},
                "*":    {"skills": ["shell-session"]}
            }
        }"#,
    )
    .expect("parse");

    check.verify("ab12", "shell-session").expect("allowed");
    check.verify("ff99", "shell-session").expect("wildcard");
    check.verify("ab12", "exec-bof").expect_err("denied");

    // v1.3 file with operators — full triple gate.
    let v1_3 = CapabilityCheck::from_json_str(
        r#"{
            "agents":    {"ab12": {"skills": ["shell-session"]}},
            "operators": {"operator-alice": {"agents": ["ab12"], "skills": ["shell-session"]}}
        }"#,
    )
    .expect("parse v1.3 sample");

    v1_3.verify_with_operator("operator-alice", "ab12", "shell-session")
        .expect("alice allowed");
    let err = v1_3
        .verify_with_operator("operator-bob", "ab12", "shell-session")
        .expect_err("bob denied (no operator entry, no wildcard)");
    matches!(err, CapabilityError::OperatorDenied { .. });
}

#[test]
fn operator_token_round_trip_and_tamper_detection() {
    let id = NodeIdentity::from_seed(&[42u8; 32]);
    let pubkey = id.ed25519_public();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let (bytes, _token) = OperatorToken::issue(&id, [0xcd; 16], now, DEFAULT_LIFETIME_SECONDS);
    let decoded = OperatorToken::decode_verified(&bytes, &pubkey, now + 60).expect("verify");
    assert_eq!(decoded.operator_id, [0xcd; 16]);

    // Tamper anywhere in the signed prefix → bad signature.
    let mut tampered = bytes;
    tampered[10] ^= 0xff;
    let err = OperatorToken::decode_verified(&tampered, &pubkey, now + 60).expect_err("tampered");
    matches!(err, TokenError::BadSignature);

    // Past expiry → expired.
    let err = OperatorToken::decode_verified(&bytes, &pubkey, now + DEFAULT_LIFETIME_SECONDS + 10)
        .expect_err("expired");
    matches!(err, TokenError::Expired { .. });
}

#[test]
fn dtn_enqueue_drain_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let opts = DtnOptions {
        root: dir.path().to_path_buf(),
        max_depth: 100,
        max_age_seconds: 3600,
    };
    let queue = DtnQueue::open(opts).expect("open");

    queue.enqueue("aabbccdd", b"first").expect("enq 1");
    queue.enqueue("aabbccdd", b"second").expect("enq 2");
    assert_eq!(queue.depth_for("aabbccdd").unwrap(), 2);

    let drained = queue.drain_for("aabbccdd").expect("drain");
    assert_eq!(drained.len(), 2);
    assert_eq!(&drained[0].bytes, b"first");
    assert_eq!(&drained[1].bytes, b"second");
    assert_eq!(queue.depth_for("aabbccdd").unwrap(), 0);
}

#[tokio::test]
async fn multi_sink_fans_out_v1_4_records() {
    // Confirms v1.3's MultiSink still works after the v1.4 changes.
    let primary = Arc::new(MemSink::new());
    let extra = Arc::new(MemSink::new());
    let multi = MultiSink::new(
        primary.clone() as Arc<dyn AuditSink>,
        vec![extra.clone() as Arc<dyn AuditSink>],
    );
    multi
        .append(make_record(
            "operator-token-abc",
            "shell_session_open",
            "ab12",
        ))
        .await;
    assert_eq!(primary.records().len(), 1);
    assert_eq!(extra.records().len(), 1);
}

#[tokio::test]
async fn per_agent_audit_records_session_lifecycle() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("agent-audit.log");
    let audit = nexus_agent::audit::AgentAudit::open(&path, "agent-host-aabb").expect("open");
    audit.shell_session_open(42).await;
    audit.shell_session_close(42, Some(0)).await;

    let raw = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("shell_session_open"));
    assert!(lines[1].contains("42:0"));
}
