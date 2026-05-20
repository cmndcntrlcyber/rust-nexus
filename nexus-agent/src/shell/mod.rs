//! PTY-backed shell sessions (v1.1 simple-mesh layer).

pub mod pty;
pub mod select;

pub use pty::{ShellSession, DEFAULT_READ_CHUNK_BYTES};
pub use select::{ShellCommand, ShellSelect};
