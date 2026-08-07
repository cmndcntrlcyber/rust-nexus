//! Tauri commands invoked by the WASM frontend.

use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::{A2aClient, DualAuthGate};
use serde::Serialize;
use tauri::{AppHandle, State};
use tracing::{info, warn};

use crate::session::open_session;
use crate::state::{Connection, ConnectionSummary, ConsoleState, SessionHandle};

/// Result of `connect_c2`.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectResponse {
    /// Same fields as `ConnectionSummary`.
    #[serde(flatten)]
    pub summary: ConnectionSummary,
    /// Whether dual-auth (gRPC + mesh) was achieved.
    pub dual_authenticated: bool,
    /// Mesh peer ID if mesh connected.
    pub mesh_peer_id: Option<String>,
}

/// Startup configuration read from environment variables.
#[derive(Debug, Serialize)]
pub struct StartupConfig {
    /// Value of `NEXUS_SERVER_ADDR` if set.
    pub server_addr: Option<String>,
}

/// Return environment-supplied startup config to the frontend.
#[tauri::command]
pub fn get_startup_config() -> StartupConfig {
    StartupConfig {
        server_addr: std::env::var("NEXUS_SERVER_ADDR").ok(),
    }
}

/// Connect to the C2's A2A service. After gRPC connect, also attempts
/// to establish a mesh connection via `MeshNode::spawn()` and validates
/// both auth paths via `DualAuthGate` before returning (WS1 Phase 1d).
#[tauri::command]
pub async fn connect_c2(
    state: State<'_, ConsoleState>,
    addr: String,
    insecure_network: bool,
) -> Result<ConnectResponse, String> {
    info!(c2 = %addr, insecure_network, "console: connecting");

    // ── Phase 1: gRPC connection ──────────────────────────────────
    let tls = nexus_a2a::tls::load_client_config_from_env().ok();
    let addr2 = addr.clone();
    let (client, card) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        async move {
            let mut c = A2aClient::connect_with_optional_tls(&addr2, insecure_network, tls)
                .await
                .map_err(|e| format!("connect: {e:#}"))?;
            let card = c
                .get_agent_card()
                .await
                .map_err(|e| format!("get_agent_card: {e}"))?;
            Ok::<_, String>((c, card))
        },
    )
    .await
    .map_err(|_| {
        "timed out after 15 s — is the nexus-server A2A service running at that address?"
            .to_string()
    })?
    .map_err(|e| e)?;

    let grpc_authenticated = true;
    let operator_id = Some("console-operator".to_string());

    // ── Phase 2: Mesh connection (WS1 Phase 1d) ───────────────────
    let (mesh_authenticated, mesh_peer_id) = match try_mesh_connect(&state, insecure_network).await
    {
        Ok(peer_id) => {
            info!(mesh_peer = %peer_id, "console: mesh connected");
            (true, Some(peer_id))
        }
        Err(e) => {
            warn!(error = %e, "console: mesh connection failed — continuing with gRPC only");
            (false, None)
        }
    };

    // ── Phase 3: Dual-auth validation ─────────────────────────────
    let require_dual = !insecure_network;
    let gate = DualAuthGate::new(require_dual);
    let auth_result = DualAuthGate::build_result(
        grpc_authenticated,
        operator_id,
        mesh_authenticated,
        mesh_peer_id.clone(),
    );

    if let Err(e) = gate.validate(&auth_result) {
        state.clear_connection().await;
        return Err(format!("dual-auth validation failed: {e}"));
    }

    let dual_authenticated = auth_result.is_fully_authenticated();

    let conn = Connection {
        addr: addr.clone(),
        insecure_network,
        client,
        server_card: card.clone(),
    };
    state.set_connection(conn).await;

    Ok(ConnectResponse {
        summary: ConnectionSummary {
            addr,
            insecure_network,
            server_name: card.name,
            server_version: card.version,
        },
        dual_authenticated,
        mesh_peer_id,
    })
}

/// Attempt to spawn a mesh node and connect to the server's mesh
/// address. Returns the local mesh peer ID on success.
async fn try_mesh_connect(
    state: &State<'_, ConsoleState>,
    insecure_network: bool,
) -> Result<String, String> {
    use nexus_common::NodeIdentity;
    use nexus_mesh::MeshNode;

    let identity = NodeIdentity::generate();

    let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .map_err(|e| format!("parse listen addr: {e}"))?;

    let handle = MeshNode::spawn(&identity, listen_addr)
        .map_err(|e| format!("mesh spawn: {e}"))?;

    // Subscribe to heartbeat topic for liveness detection.
    let heartbeat_topic = nexus_mesh::topics::heartbeat();
    handle
        .subscribe(&heartbeat_topic)
        .await
        .map_err(|e| format!("mesh subscribe heartbeat: {e}"))?;

    // Optionally dial the server's mesh address if provided via env.
    if let Ok(mesh_addr) = std::env::var("NEXUS_MESH_ADDR") {
        if let Ok(addr) = mesh_addr.parse::<libp2p::Multiaddr>() {
            handle
                .dial(addr)
                .await
                .map_err(|e| format!("mesh dial: {e}"))?;
        }
    }

    let peer_id = handle.local_peer_id().to_string();
    state.set_mesh(handle).await;
    Ok(peer_id)
}

/// Drop the active connection.
#[tauri::command]
pub async fn disconnect_c2(state: State<'_, ConsoleState>) -> Result<(), String> {
    state.clear_connection().await;
    Ok(())
}

/// Connection metadata snapshot.
#[tauri::command]
pub async fn connection_summary(
    state: State<'_, ConsoleState>,
) -> Result<Option<ConnectionSummary>, String> {
    Ok(state.connection_summary().await)
}

/// Tauri view of one registered agent.
#[derive(Debug, Clone, Serialize)]
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

/// List currently-registered agents.
#[tauri::command]
pub async fn list_agents(state: State<'_, ConsoleState>) -> Result<Vec<AgentInfo>, String> {
    let mut client = state.client().await.map_err(|e| e.to_string())?;
    let pb_agents = client
        .list_registered_agents()
        .await
        .map_err(|e| format!("list_registered_agents: {e}"))?;
    Ok(pb_agents
        .into_iter()
        .map(|a| AgentInfo {
            peer_id: hex_lower(&a.peer_id),
            os: a.os,
            version: a.version,
            tag: a.tag,
            last_seen_unix: a.last_seen_unix,
        })
        .collect())
}

/// Open a shell session.
#[tauri::command]
pub async fn open_shell_session(
    app: AppHandle,
    state: State<'_, ConsoleState>,
    target_peer_id: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<u64, String> {
    let client = state.client().await.map_err(|e| e.to_string())?;
    let session_id = state.allocate_session_id();
    let (tx, task) = open_session(app, client, session_id, target_peer_id.clone(), cols, rows)
        .await
        .map_err(|e| format!("open_session: {e:#}"))?;
    state
        .insert_session(
            session_id,
            SessionHandle {
                tx,
                task,
                target_agent_id: target_peer_id,
            },
        )
        .await;
    Ok(session_id)
}

/// Push raw bytes to a shell session.
#[tauri::command]
pub async fn send_shell_bytes(
    state: State<'_, ConsoleState>,
    session_id: u64,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let Some(tx) = state.session_tx(session_id).await else {
        return Err(format!("session {session_id} not active"));
    };
    let msg = bytes_request(&format!("console-{session_id}"), bytes);
    tx.send(msg)
        .await
        .map_err(|_| format!("session {session_id} closed"))
}

/// Resize a shell session.
#[tauri::command]
pub async fn resize_shell(
    state: State<'_, ConsoleState>,
    session_id: u64,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let Some(tx) = state.session_tx(session_id).await else {
        return Err(format!("session {session_id} not active"));
    };
    let frame = ShellControl::ShellResize { cols, rows };
    let msg = control_request(&format!("console-{session_id}"), &frame)
        .map_err(|e| format!("encode resize: {e}"))?;
    tx.send(msg)
        .await
        .map_err(|_| format!("session {session_id} closed"))
}

/// Close a shell session.
#[tauri::command]
pub async fn close_shell_session(
    state: State<'_, ConsoleState>,
    session_id: u64,
) -> Result<(), String> {
    if let Some(handle) = state.remove_session(session_id).await {
        let SessionHandle { tx, task, .. } = handle;
        drop(tx);
        if tokio::time::timeout(std::time::Duration::from_millis(200), task)
            .await
            .is_err()
        {
            warn!(session_id, "session driver did not exit within 200ms");
        }
    }
    Ok(())
}

/// Switch the active tab in the console UI.
///
/// Validates tab_id is in range [0, 3] for the 4-tab architecture
/// (WS3 Phase 3a). Backend hook point for future session routing.
#[tauri::command]
pub async fn switch_tab(tab_id: u64) -> Result<(), String> {
    if tab_id > 3 {
        return Err(format!("invalid tab_id {tab_id}: expected 0..3"));
    }
    Ok(())
}

/// Query whether the console is dual-authenticated (gRPC + mesh).
#[tauri::command]
pub async fn is_dual_authenticated(state: State<'_, ConsoleState>) -> Result<bool, String> {
    Ok(state.is_dual_connected().await)
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

// ---------------------------------------------------------------------
// v1.4.4 — Tauri audit log viewer (Phase 1.4.4).
// ---------------------------------------------------------------------

/// One audit-log record as it travels to the Leptos UI.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct AuditRecord {
    pub timestamp_unix: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub prev_hash: String,
    pub record_hash: String,
}

impl From<nexus_a2a::pb::AuditRecordEvent> for AuditRecord {
    fn from(e: nexus_a2a::pb::AuditRecordEvent) -> Self {
        Self {
            timestamp_unix: e.timestamp_unix,
            actor: e.actor,
            action: e.action,
            resource: e.resource,
            prev_hash: e.prev_hash,
            record_hash: e.record_hash,
        }
    }
}

/// Optional filter applied to the stream.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct AuditFilter {
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub since_unix: u64,
}

/// Tail the last `count` audit records via `StreamAuditRecords`.
#[tauri::command]
pub async fn audit_log_tail(
    state: State<'_, ConsoleState>,
    count: usize,
) -> Result<Vec<AuditRecord>, String> {
    audit_log_filter(state, AuditFilter::default(), count).await
}

/// Subscribe + filter + return up to `count` records, then close the
/// stream.
#[tauri::command]
pub async fn audit_log_filter(
    state: State<'_, ConsoleState>,
    filter: AuditFilter,
    count: usize,
) -> Result<Vec<AuditRecord>, String> {
    use futures::StreamExt as _;
    use std::time::Duration;

    let mut client = state
        .client()
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = client
        .stream_audit_records(nexus_a2a::pb::StreamAuditRecordsRequest {
            actor_filter: filter.actor,
            action_filter: filter.action,
            since_unix: filter.since_unix,
        })
        .await
        .map_err(|e| format!("stream_audit_records: {e}"))?;

    let mut out = Vec::with_capacity(count);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while out.len() < count {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(event))) => out.push(event.into()),
            Ok(Some(Err(status))) => {
                warn!(error = %status, "audit stream error");
                break;
            }
            Ok(None) | Err(_) => break,
        }
    }
    Ok(out)
}

/// Pure-Rust chain-integrity check.
#[tauri::command]
pub fn audit_log_verify(records: Vec<AuditRecord>) -> Result<Option<usize>, String> {
    use blake3::Hasher;
    let genesis = "0".repeat(64);
    let mut prev = genesis.as_str().to_string();
    for (i, record) in records.iter().enumerate() {
        if record.prev_hash != prev {
            return Ok(Some(i));
        }
        let mut hasher = Hasher::new();
        hasher.update(&record.timestamp_unix.to_be_bytes());
        hasher.update(b"|");
        hasher.update(record.actor.as_bytes());
        hasher.update(b"|");
        hasher.update(record.action.as_bytes());
        hasher.update(b"|");
        hasher.update(record.resource.as_bytes());
        hasher.update(b"|");
        hasher.update(record.prev_hash.as_bytes());
        let expected: String = hasher
            .finalize()
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        if expected != record.record_hash {
            return Ok(Some(i));
        }
        prev = record.record_hash.clone();
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_lower_empty() {
        assert_eq!(hex_lower(&[]), "");
    }

    #[test]
    fn test_hex_lower_known_values() {
        assert_eq!(hex_lower(&[0x00]), "00");
        assert_eq!(hex_lower(&[0xff]), "ff");
        assert_eq!(hex_lower(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn test_audit_log_verify_empty_chain() {
        let result = audit_log_verify(vec![]).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_audit_log_verify_valid_chain() {
        use blake3::Hasher;

        let genesis = "0".repeat(64);

        let make_hash = |ts: u64, actor: &str, action: &str, resource: &str, prev: &str| -> String {
            let mut h = Hasher::new();
            h.update(&ts.to_be_bytes());
            h.update(b"|");
            h.update(actor.as_bytes());
            h.update(b"|");
            h.update(action.as_bytes());
            h.update(b"|");
            h.update(resource.as_bytes());
            h.update(b"|");
            h.update(prev.as_bytes());
            h.finalize().as_bytes().iter().map(|b| format!("{:02x}", b)).collect()
        };

        let hash0 = make_hash(1000, "operator", "login", "console", &genesis);
        let hash1 = make_hash(1001, "operator", "shell_open", "agent-001", &hash0);

        let records = vec![
            AuditRecord {
                timestamp_unix: 1000,
                actor: "operator".into(),
                action: "login".into(),
                resource: "console".into(),
                prev_hash: genesis,
                record_hash: hash0.clone(),
            },
            AuditRecord {
                timestamp_unix: 1001,
                actor: "operator".into(),
                action: "shell_open".into(),
                resource: "agent-001".into(),
                prev_hash: hash0,
                record_hash: hash1,
            },
        ];

        let result = audit_log_verify(records).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_audit_log_verify_detects_tampered_record() {
        let genesis = "0".repeat(64);
        let records = vec![AuditRecord {
            timestamp_unix: 1000,
            actor: "operator".into(),
            action: "login".into(),
            resource: "console".into(),
            prev_hash: genesis,
            record_hash: "bad_hash".into(),
        }];

        let result = audit_log_verify(records).unwrap();
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_audit_filter_default() {
        let filter = AuditFilter::default();
        assert!(filter.actor.is_empty());
        assert!(filter.action.is_empty());
        assert_eq!(filter.since_unix, 0);
    }

    #[test]
    fn test_switch_tab_valid_ids() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        for id in 0..=3 {
            assert!(rt.block_on(switch_tab(id)).is_ok());
        }
    }

    #[test]
    fn test_switch_tab_invalid_id_rejected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert!(rt.block_on(switch_tab(4)).is_err());
        assert!(rt.block_on(switch_tab(99)).is_err());
    }
}
