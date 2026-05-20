//! v1.3 regression test (Phase 1.3.10).
//!
//! Exercises the v1.3 surface area added on top of v1.2:
//!
//! 1. Capability SIGHUP-style reload (`CapabilityCheck::reload`).
//! 2. Per-operator scoping (`verify_with_operator`).
//! 3. Audit-log MultiSink fan-out.
//! 4. Prometheus counter increments.
//!
//! Surfaces that require external runtime (live mesh round-trip via
//! Kademlia, ACME staging, syslog TLS) are exercised by `#[ignore]`d
//! tests gated on env vars.

use std::sync::Arc;

use nexus_a2a::audit::{make_record, AuditSink, MemSink, MultiSink};
use nexus_a2a::capabilities::{CapabilityCheck, CapabilityError};
use nexus_a2a::metrics::{gather_text, Metrics};

#[test]
fn capability_reload_swaps_policy_in_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("capabilities.json");
    std::fs::write(&path, r#"{"agents":{"ab12":{"skills":["shell-session"]}}}"#).expect("write v1");

    let mut check = CapabilityCheck::from_json_file(&path).expect("load v1");
    check.verify("ab12", "shell-session").expect("v1 allows");

    // Operator edits the file: deny-all.
    std::fs::write(&path, r#"{"agents":{}}"#).expect("write v2");
    check.reload(&path).expect("reload v2");
    check
        .verify("ab12", "shell-session")
        .expect_err("v2 denies");
}

#[test]
fn per_operator_scoping_denies_unauthorized_operator() {
    let check = CapabilityCheck::from_json_str(
        r#"{
            "agents":    {"ab12": {"skills": ["shell-session"]}},
            "operators": {"operator-alice": {"agents": ["ab12"], "skills": ["shell-session"]}}
        }"#,
    )
    .expect("parse");

    // Alice is allowed.
    check
        .verify_with_operator("operator-alice", "ab12", "shell-session")
        .expect("alice allowed");

    // Bob isn't in the operators section → wildcard absent → deny.
    let err = check
        .verify_with_operator("operator-bob", "ab12", "shell-session")
        .expect_err("bob denied");
    matches!(err, CapabilityError::OperatorDenied { .. });
}

#[tokio::test]
async fn multi_sink_fans_out_audit_records() {
    let primary = Arc::new(MemSink::new());
    let extra = Arc::new(MemSink::new());
    let multi = MultiSink::new(
        primary.clone() as Arc<dyn AuditSink>,
        vec![extra.clone() as Arc<dyn AuditSink>],
    );
    multi
        .append(make_record("operator-alice", "shell_session_open", "ab12"))
        .await;
    multi
        .append(make_record(
            "operator-alice",
            "shell_session_close",
            "ab12:1",
        ))
        .await;
    assert_eq!(primary.records().len(), 2);
    assert_eq!(extra.records().len(), 2);
}

#[test]
fn prometheus_counters_increment() {
    let m = Metrics::global();
    let before = gather_text().expect("gather before");

    m.requests_total
        .with_label_values(&["GetAgentCard"])
        .inc_by(3);
    m.capability_denied_total
        .with_label_values(&["operator-alice", "ab12", "shell-session"])
        .inc();
    m.active_agent_sessions.set(5);

    let after = gather_text().expect("gather after");

    // Each metric we wrote shows up in the after-snapshot. The before
    // snapshot may already contain entries from earlier tests in the
    // same process, so we only assert the after-state is well-formed.
    assert!(after.contains("nexus_a2a_requests_total"));
    assert!(after.contains("rpc=\"GetAgentCard\""));
    assert!(after.contains("nexus_a2a_capability_denied_total"));
    assert!(after.contains("nexus_a2a_active_agent_sessions 5"));
    // Sanity: the before snapshot didn't contain a record for the
    // exact agent label we just wrote.
    assert!(
        !before.contains("agent=\"ab12\""),
        "unexpected pre-state contained our test labels"
    );
}
