//! Frontend mirrors of the Tauri backend's serialized types.

use serde::{Deserialize, Serialize};

/// Mirror of backend `ConnectionSummary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Address.
    pub addr: String,
    /// Loopback gate state.
    pub insecure_network: bool,
    /// Server name.
    pub server_name: String,
    /// Server version.
    pub server_version: String,
}

/// Mirror of backend `ConnectResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectResponse {
    /// Address.
    pub addr: String,
    /// Loopback gate state.
    pub insecure_network: bool,
    /// Server name.
    pub server_name: String,
    /// Server version.
    pub server_version: String,
}

impl From<ConnectResponse> for ConnectionInfo {
    fn from(r: ConnectResponse) -> Self {
        Self {
            addr: r.addr,
            insecure_network: r.insecure_network,
            server_name: r.server_name,
            server_version: r.server_version,
        }
    }
}

/// Mirror of backend `AgentInfo`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentInfo {
    /// Hex peer id.
    pub peer_id: String,
    /// OS label.
    pub os: String,
    /// Version.
    pub version: String,
    /// Tag.
    pub tag: String,
    /// Last seen.
    pub last_seen_unix: u64,
}

/// `shell-output` event payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellOutputPayload {
    /// Session.
    pub session_id: u64,
    /// Bytes.
    pub bytes: Vec<u8>,
}

/// `shell-exit` event payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellExitPayload {
    /// Session.
    pub session_id: u64,
    /// Code.
    pub code: Option<i32>,
}

/// `shell-error` event payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellErrorPayload {
    /// Session.
    pub session_id: u64,
    /// Error message.
    pub message: String,
}
