//! WS1 Phase 1d security test: single-auth (gRPC only, mesh only) must
//! fail; both-auth must succeed.
//!
//! Validates the `DualAuthGate` enforces both authentication paths when
//! `require_dual` is true, and relaxes to gRPC-only when false (dev mode).

use nexus_a2a::{DualAuthGate, DualAuthResult};

#[test]
fn dual_auth_both_paths_succeeds() {
    let gate = DualAuthGate::new(true);
    let result = DualAuthGate::build_result(
        true,
        Some("operator-1".into()),
        true,
        Some("12D3KooW...".into()),
    );
    assert!(result.is_fully_authenticated());
    assert!(gate.validate(&result).is_ok());
}

#[test]
fn single_auth_grpc_only_fails_when_dual_required() {
    let gate = DualAuthGate::new(true);
    let result = DualAuthGate::build_result(
        true,
        Some("operator-1".into()),
        false,
        None,
    );
    assert!(!result.is_fully_authenticated());
    let err = gate.validate(&result);
    assert!(err.is_err());
    let status = err.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert!(status.message().contains("mesh"));
}

#[test]
fn single_auth_mesh_only_fails() {
    let gate = DualAuthGate::new(true);
    let result = DualAuthGate::build_result(
        false,
        None,
        true,
        Some("12D3KooW...".into()),
    );
    assert!(!result.is_fully_authenticated());
    let err = gate.validate(&result);
    assert!(err.is_err());
    let status = err.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert!(status.message().contains("gRPC"));
}

#[test]
fn no_auth_fails() {
    let gate = DualAuthGate::new(true);
    let result = DualAuthGate::build_result(false, None, false, None);
    assert!(!result.is_fully_authenticated());
    assert!(gate.validate(&result).is_err());
}

#[test]
fn dev_mode_allows_grpc_only() {
    let gate = DualAuthGate::new(false);
    let result = DualAuthGate::build_result(
        true,
        Some("dev-operator".into()),
        false,
        None,
    );
    assert!(!result.is_fully_authenticated());
    assert!(gate.validate(&result).is_ok());
}

#[test]
fn dev_mode_still_requires_grpc() {
    let gate = DualAuthGate::new(false);
    let result = DualAuthGate::build_result(false, None, true, Some("peer-id".into()));
    assert!(gate.validate(&result).is_err());
}

#[test]
fn default_gate_requires_dual() {
    let gate = DualAuthGate::default();
    let grpc_only = DualAuthGate::build_result(true, Some("op".into()), false, None);
    assert!(gate.validate(&grpc_only).is_err());
}
