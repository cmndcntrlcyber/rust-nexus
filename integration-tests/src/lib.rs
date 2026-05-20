//! Cross-crate integration tests for the v1.1 simple-mesh integration.
//!
//! Tests live in `tests/`:
//!
//! - `a2a_loopback.rs` — operator → A2A → EchoShellHandler bytes round-trip
//!   (proves the operator-facing A2A wire path inside the integrated
//!   workspace). Does **not** exercise the overlay's nexus-infra (which has
//!   pre-existing compile issues; full server-side integration is gated on
//!   v1.2 overlay maintenance per `docs/v1.1/integration-overview.md`).
