//! Reactive state for all HUD components.

use super::types::{ApprovalRequest, EngagementInfo, Notification};

#[derive(Clone, Debug, Default)]
pub struct HudState {
    pub notifications: Vec<Notification>,
    pub active_toasts: Vec<Notification>,
    pub pending_approvals: Vec<ApprovalRequest>,
    pub engagement: Option<EngagementInfo>,
}

impl HudState {
    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }

    pub fn add_toast(&mut self, notification: Notification) {
        self.active_toasts.push(notification);
        if self.active_toasts.len() > 5 {
            self.active_toasts.remove(0);
        }
    }

    pub fn dismiss_toast(&mut self, id: &str) {
        self.active_toasts.retain(|t| t.id != id);
    }

    pub fn mark_read(&mut self, id: &str) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
            n.read = true;
        }
    }
}
