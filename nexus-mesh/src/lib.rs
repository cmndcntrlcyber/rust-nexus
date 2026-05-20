//! `nexus-mesh` — libp2p-based peer mesh transport (v1.1 simple-mesh layer).
//!
//! Modules:
//!
//! - [`node::MeshNode`] — libp2p Swarm wrapper (TCP + Noise + Yamux +
//!   Identify + Gossipsub + Ping). Spawn one node, get a [`node::MeshHandle`]
//!   with async `publish` / `subscribe` / `dial` / `next_event`.
//! - [`topics`] — stable gossipsub topic-name helpers.
//!
//! The sealed-envelope crypto layer that rides over gossipsub lives in
//! `nexus_common::sealed` (since both this crate and `nexus-a2a` use it).

#![warn(missing_docs)]

pub mod discovery;
pub mod dtn;
pub mod node;
pub mod topics;

pub use node::{MeshEvent, MeshHandle, MeshNode};

/// Re-export `SealedEnvelope` from `nexus-common` so consumers don't need
/// two crate imports for the mesh-publication flow.
pub use nexus_common::SealedEnvelope;
