//! `nexus-agent` — library half of the rust-nexus agent (v1.1 simple-mesh
//! integration additions).
//!
//! The existing `nexus-agent` binary lives in `main.rs` with its own
//! internal modules (`agent`, `communication`, `execution`, `evasion`,
//! `persistence`, `registry`, `system`, `fiber_execution`). Per D-V1.1-A
//! "additive integration" we don't touch those.
//!
//! This library exposes the **new** v1.1 modules:
//!
//! - [`shell`] — PTY-backed `ShellSession` via `portable-pty`.
//! - [`a2a_client`] — dials the C2's new A2A service and runs the
//!   agent-side bidi stream for interactive shells.
//! - [`transports`] — `Transport` trait variants (gRPC / mesh / legacy).
//! - [`smoke`] — `--shell-smoke-test` helper.

#![warn(missing_docs)]

pub mod a2a_client;
pub mod audit;
pub mod shell;
pub mod smoke;
pub mod transports;
