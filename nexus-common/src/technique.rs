use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::{Crypto, Result};

/// MITRE ATT&CK Tactic categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tactic {
    Reconnaissance,
    ResourceDevelopment,
    InitialAccess,
    Execution,
    Persistence,
    PrivilegeEscalation,
    DefenseEvasion,
    CredentialAccess,
    Discovery,
    LateralMovement,
    Collection,
    CommandAndControl,
    Exfiltration,
    Impact,
}

/// Target platform for a technique
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
    All,
}

impl Platform {
    /// Returns true if this platform matches the current compilation target
    pub fn is_current(&self) -> bool {
        match self {
            Platform::All => true,
            #[cfg(target_os = "windows")]
            Platform::Windows => true,
            #[cfg(target_os = "linux")]
            Platform::Linux => true,
            #[cfg(target_os = "macos")]
            Platform::MacOS => true,
            _ => false,
        }
    }
}

/// Parameters passed to a technique for execution
#[derive(Debug, Clone)]
pub struct TechniqueParams {
    pub command: String,
    pub parameters: HashMap<String, String>,
    pub timeout: Option<u64>,
}

impl From<&crate::messages::TaskData> for TechniqueParams {
    fn from(task: &crate::messages::TaskData) -> Self {
        Self {
            command: task.command.clone(),
            parameters: task.parameters.clone(),
            timeout: task.timeout,
        }
    }
}

/// Result returned by a technique after execution
#[derive(Debug, Clone)]
pub struct TechniqueResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl TechniqueResult {
    pub fn ok(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// Shared context available to all techniques during execution
pub struct ExecutionContext {
    pub crypto: Arc<Crypto>,
    pub agent_id: String,
    pub platform: Platform,
}

/// The core trait that every ATT&CK technique crate must implement.
///
/// Each technique maps to a MITRE ATT&CK technique ID (e.g., T1059 Command and Scripting Interpreter).
/// Technique crates register implementations of this trait, which the agent's
/// TechniqueRegistry uses for dispatch.
#[async_trait::async_trait]
pub trait AttackTechnique: Send + Sync {
    /// MITRE ATT&CK technique ID (e.g., "T1059", "T1055.001")
    fn technique_id(&self) -> &str;

    /// Human-readable technique name
    fn name(&self) -> &str;

    /// ATT&CK tactics this technique falls under
    fn tactics(&self) -> &[Tactic];

    /// Platforms this technique supports
    fn platforms(&self) -> &[Platform];

    /// Capability strings advertised to the C2 server
    fn capabilities(&self) -> Vec<String>;

    /// Task type strings this technique handles (used for dispatch routing)
    fn task_types(&self) -> Vec<String>;

    /// Execute the technique with the given parameters
    async fn execute(&self, ctx: &ExecutionContext, params: TechniqueParams) -> Result<TechniqueResult>;

    /// Validate parameters before execution (default: no-op)
    fn validate(&self, _params: &TechniqueParams) -> Result<()> {
        Ok(())
    }
}
