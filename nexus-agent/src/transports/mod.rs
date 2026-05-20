//! v1.1 transport-trait implementations.
//!
//! Implements the `nexus_web_comms::Transport` trait for each transport
//! variant. v1.1 ships:
//!
//! - [`grpc::GrpcTransport`] — defers to [`crate::a2a_client::probe_c2`]
//!   for liveness; full interactive shell path lands at v1.2.
//! - [`mesh::MeshTransport`] — experimental libp2p mesh node.
//! - The overlay's existing TCP/HTTP path is exposed as `TransportKind::Legacy`
//!   (handled inside the existing `nexus-agent::communication.rs`; not
//!   wrapped by this trait yet).

pub mod grpc;
pub mod mesh;
