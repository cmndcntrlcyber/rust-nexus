use crate::{current_timestamp, generate_uuid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub ip_address: String,
    pub username: String,
    pub process_id: u32,
    pub process_name: String,
    pub architecture: String,
    pub capabilities: Vec<String>,
    pub last_seen: u64,
    pub first_seen: u64,
    pub tasks: Vec<String>,               // Task IDs
    pub results: HashMap<String, String>, // Task ID -> Result
    pub status: AgentStatus,
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Online,
    Offline,
    Stale,
    Compromised,
    Unknown,
}

/// Configuration for creating a new Agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub ip_address: String,
    pub username: String,
    pub process_id: u32,
    pub process_name: String,
    pub architecture: String,
    pub capabilities: Vec<String>,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_uuid(),
            hostname: config.hostname,
            os_type: config.os_type,
            os_version: config.os_version,
            ip_address: config.ip_address,
            username: config.username,
            process_id: config.process_id,
            process_name: config.process_name,
            architecture: config.architecture,
            capabilities: config.capabilities,
            last_seen: now,
            first_seen: now,
            tasks: Vec::new(),
            results: HashMap::new(),
            status: AgentStatus::Online,
            metadata: HashMap::new(),
        }
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = current_timestamp();
        self.update_status();
    }

    pub fn update_status(&mut self) {
        let now = current_timestamp();
        let time_since_last_seen = now - self.last_seen;

        self.status = if time_since_last_seen < 60 {
            AgentStatus::Online
        } else if time_since_last_seen < 300 {
            AgentStatus::Stale
        } else {
            AgentStatus::Offline
        };
    }

    pub fn add_task(&mut self, task_id: String) {
        if !self.tasks.contains(&task_id) {
            self.tasks.push(task_id);
        }
    }

    pub fn remove_task(&mut self, task_id: &str) -> bool {
        if let Some(pos) = self.tasks.iter().position(|x| x == task_id) {
            self.tasks.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_next_task(&mut self) -> Option<String> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }

    pub fn add_result(&mut self, task_id: String, result: String) {
        self.results.insert(task_id, result);
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    pub fn add_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    pub fn is_windows(&self) -> bool {
        self.os_type.to_lowercase().contains("windows")
    }

    pub fn is_linux(&self) -> bool {
        self.os_type.to_lowercase().contains("linux")
    }

    pub fn supports_fiber_execution(&self) -> bool {
        self.is_windows() && self.has_capability("fiber_execution")
    }

    pub fn supports_process_injection(&self) -> bool {
        self.has_capability("process_injection")
    }

    pub fn supports_shellcode_execution(&self) -> bool {
        self.has_capability("shellcode_execution")
    }

    pub fn get_display_info(&self) -> String {
        format!(
            "{} ({}) - {} @ {} - {} tasks pending",
            self.hostname,
            &self.id[..8],
            self.username,
            self.ip_address,
            self.tasks.len()
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentCapabilities {
    pub os_type: String,
    pub architecture: String,
    pub shell_execution: bool,
    pub file_operations: bool,
    pub network_operations: bool,
    pub process_injection: bool,
    pub shellcode_execution: bool,
    pub fiber_execution: bool,
    pub privilege_escalation: bool,
    pub persistence: bool,
    pub anti_analysis: bool,
    pub custom_capabilities: Vec<String>,
}

impl AgentCapabilities {
    pub fn new_windows_basic() -> Self {
        Self {
            os_type: "Windows".to_string(),
            architecture: "x64".to_string(),
            shell_execution: true,
            file_operations: true,
            network_operations: true,
            process_injection: false,
            shellcode_execution: false,
            fiber_execution: false,
            privilege_escalation: false,
            persistence: false,
            anti_analysis: false,
            custom_capabilities: Vec::new(),
        }
    }

    pub fn new_windows_advanced() -> Self {
        Self {
            os_type: "Windows".to_string(),
            architecture: "x64".to_string(),
            shell_execution: true,
            file_operations: true,
            network_operations: true,
            process_injection: true,
            shellcode_execution: true,
            fiber_execution: true,
            privilege_escalation: true,
            persistence: true,
            anti_analysis: true,
            custom_capabilities: vec![
                "fiber_hollowing".to_string(),
                "early_bird_injection".to_string(),
                "apc_injection".to_string(),
                "dll_injection".to_string(),
            ],
        }
    }

    pub fn new_linux_basic() -> Self {
        Self {
            os_type: "Linux".to_string(),
            architecture: "x64".to_string(),
            shell_execution: true,
            file_operations: true,
            network_operations: true,
            process_injection: false,
            shellcode_execution: false,
            fiber_execution: false,
            privilege_escalation: false,
            persistence: false,
            anti_analysis: false,
            custom_capabilities: Vec::new(),
        }
    }

    pub fn to_string_list(&self) -> Vec<String> {
        let mut capabilities = Vec::new();

        if self.shell_execution {
            capabilities.push("shell_execution".to_string());
        }
        if self.file_operations {
            capabilities.push("file_operations".to_string());
        }
        if self.network_operations {
            capabilities.push("network_operations".to_string());
        }
        if self.process_injection {
            capabilities.push("process_injection".to_string());
        }
        if self.shellcode_execution {
            capabilities.push("shellcode_execution".to_string());
        }
        if self.fiber_execution {
            capabilities.push("fiber_execution".to_string());
        }
        if self.privilege_escalation {
            capabilities.push("privilege_escalation".to_string());
        }
        if self.persistence {
            capabilities.push("persistence".to_string());
        }
        if self.anti_analysis {
            capabilities.push("anti_analysis".to_string());
        }

        capabilities.extend(self.custom_capabilities.clone());
        capabilities
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentSession {
    pub agent_id: String,
    pub session_id: String,
    pub start_time: u64,
    pub last_activity: u64,
    pub command_count: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active: bool,
}

impl AgentSession {
    pub fn new(agent_id: String) -> Self {
        let now = current_timestamp();
        Self {
            agent_id,
            session_id: generate_uuid(),
            start_time: now,
            last_activity: now,
            command_count: 0,
            bytes_sent: 0,
            bytes_received: 0,
            active: true,
        }
    }

    pub fn update_activity(&mut self, bytes_sent: u64, bytes_received: u64) {
        self.last_activity = current_timestamp();
        self.command_count += 1;
        self.bytes_sent += bytes_sent;
        self.bytes_received += bytes_received;
    }

    pub fn close(&mut self) {
        self.active = false;
    }

    pub fn duration_seconds(&self) -> u64 {
        current_timestamp() - self.start_time
    }

    pub fn is_stale(&self, threshold_seconds: u64) -> bool {
        (current_timestamp() - self.last_activity) > threshold_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig {
            hostname: "test-host".to_string(),
            os_type: "Windows".to_string(),
            os_version: "10.0".to_string(),
            ip_address: "192.168.1.100".to_string(),
            username: "user".to_string(),
            process_id: 1234,
            process_name: "test.exe".to_string(),
            architecture: "x64".to_string(),
            capabilities: vec!["shell_execution".to_string()],
        };
        let agent = Agent::new(config);

        assert_eq!(agent.hostname, "test-host");
        assert_eq!(agent.status, AgentStatus::Online);
        assert!(!agent.id.is_empty());
    }

    #[test]
    fn test_agent_capabilities() {
        let config = AgentConfig {
            hostname: "test-host".to_string(),
            os_type: "Windows".to_string(),
            os_version: "10.0".to_string(),
            ip_address: "192.168.1.100".to_string(),
            username: "user".to_string(),
            process_id: 1234,
            process_name: "test.exe".to_string(),
            architecture: "x64".to_string(),
            capabilities: vec!["fiber_execution".to_string()],
        };
        let mut agent = Agent::new(config);

        assert!(agent.supports_fiber_execution());
        assert!(!agent.supports_process_injection());

        agent.add_capability("process_injection".to_string());
        assert!(agent.supports_process_injection());
    }

    #[test]
    fn test_task_management() {
        let config = AgentConfig {
            hostname: "test-host".to_string(),
            os_type: "Windows".to_string(),
            os_version: "10.0".to_string(),
            ip_address: "192.168.1.100".to_string(),
            username: "user".to_string(),
            process_id: 1234,
            process_name: "test.exe".to_string(),
            architecture: "x64".to_string(),
            capabilities: vec![],
        };
        let mut agent = Agent::new(config);

        let task_id = "task-123".to_string();
        agent.add_task(task_id.clone());
        assert_eq!(agent.tasks.len(), 1);

        let next_task = agent.get_next_task();
        assert_eq!(next_task, Some(task_id));
        assert_eq!(agent.tasks.len(), 0);
    }
}
