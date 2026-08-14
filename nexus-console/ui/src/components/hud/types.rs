//! HUD-specific types for notifications, approvals, and engagement status.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Notification {
    pub id: String,
    pub severity: NotificationSeverity,
    pub title: String,
    pub body: String,
    pub source: String,
    pub timestamp_unix: u64,
    pub read: bool,
    pub requires_action: bool,
    pub action_ref: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApprovalRequest {
    pub approval_id: String,
    pub skill_name: String,
    pub target: String,
    pub risk_level: String,
    pub requesting_agent: String,
    pub description: String,
    pub techniques: Vec<String>,
    pub created_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Clone, Debug)]
pub struct EngagementInfo {
    pub name: String,
    pub current_phase: u32,
    pub findings_count: u32,
    pub active_agents: u32,
}
