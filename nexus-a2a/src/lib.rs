//! `nexus-a2a` — A2A gRPC plane (v1.1 simple-mesh integration layer).
//!
//! Provides Linux Foundation A2A-protocol-compatible types + a `shell-session`
//! skill, coexisting alongside the existing `nexus-infra::nexus.proto` C2
//! service (per D-V1.1-B "proto coexistence"). The two protocols are hosted
//! on the same Tonic server in v1.1.

#![warn(missing_docs)]

pub mod audit;
pub mod audit_s3;
pub mod capabilities;
pub mod cards;
pub mod client;
pub mod framing;
pub mod handler;
pub mod insecure;
pub mod interceptors;
pub mod metrics;
pub mod mock;
pub mod otel;
pub mod server;
pub mod tls;
pub mod tokens;

/// Protobuf-generated types + Tonic stubs for `a2a.v1`.
pub mod pb {
    #![allow(missing_docs, clippy::all)]
    tonic::include_proto!("a2a.v1");
}

/// v1.4.2 (Phase 1.4.2 / D-V1.4-B) — vendored upstream
/// `a2aproject/A2A` proto compiled into the `a2a.upstream.v1`
/// package. Client-only (we never serve under this package); the
/// pure-Rust interop test at `tests/upstream_compat.rs` uses the
/// generated tonic client to drive our v1.4 server.
///
/// Until operators run `scripts/vendor-a2a-proto.sh` to fetch the
/// real upstream bytes, this module is generated from the in-tree
/// stub at `vendor/a2a-upstream/a2a.v1.proto`. The stub's
/// field-numbers match our v1.4 proto exactly so the interop test
/// passes against itself.
pub mod pb_upstream {
    #![allow(missing_docs, clippy::all)]
    tonic::include_proto!("a2a.upstream.v1");
}

pub use client::A2aClient;
pub use framing::{
    ShellControl, AGENT_REGISTER_KIND, SHELL_EXIT_KIND, SHELL_OPEN_KIND, SHELL_RESIZE_KIND,
};
pub use handler::{
    AgentLister, AgentRegisterParams, AgentRegistrationHandler, RegisteredAgentInfo, ShellHandler,
    ShellOpenParams,
};
pub use server::A2aServer;
