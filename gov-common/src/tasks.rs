use serde::{Deserialize, Serialize};
use crate::{current_timestamp, generate_uuid, Result, NexusError};
use std::time::Duration;

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
    // Basic operations
    ShellCommand,
    PowerShellCommand,
    FileUpload,
    FileDownload,
    DirectoryListing,
    ProcessList,
    SystemInfo,
    NetworkInfo,
    // Windows registry/service operations (for compliance auditing)
    RegistryQuery,
    RegistrySet,
    ServiceControl,
    // Compliance checks
    ComplianceCheck,
    PolicyAudit,
    ConfigurationAudit,
    // Asset inventory
    AssetInventory,
    SoftwareInventory,
    // Evidence collection
    EvidenceCollection,
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
    LogFile,
    RegistryDump,
    FileContent,
    ConfigData,
    ComplianceReport,
    Evidence,
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

    pub fn log_file(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::LogFile, name, data)
    }

    pub fn file_content(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::FileContent, name, data)
    }

    pub fn compliance_report(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::ComplianceReport, name, data)
    }

    pub fn evidence(name: String, data: Vec<u8>) -> Self {
        Self::new(ArtifactType::Evidence, name, data)
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

    pub fn compliance_check(framework: String, control_id: String) -> Task {
        Task::new(TaskType::ComplianceCheck)
            .with_parameter("framework".to_string(), framework)
            .with_parameter("control_id".to_string(), control_id)
            .with_timeout(300)
    }

    pub fn policy_audit(policy_id: String) -> Task {
        Task::new(TaskType::PolicyAudit)
            .with_parameter("policy_id".to_string(), policy_id)
            .with_timeout(300)
    }

    pub fn asset_inventory() -> Task {
        Task::new(TaskType::AssetInventory)
            .with_timeout(600)
    }

    pub fn software_inventory() -> Task {
        Task::new(TaskType::SoftwareInventory)
            .with_timeout(600)
    }

    pub fn evidence_collection(evidence_type: String, target: String) -> Task {
        Task::new(TaskType::EvidenceCollection)
            .with_parameter("evidence_type".to_string(), evidence_type)
            .with_parameter("target".to_string(), target)
            .with_timeout(300)
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
    fn test_compliance_check_task() {
        let task = TaskBuilder::compliance_check("NIST_CSF".to_string(), "PR.AC-1".to_string());
        assert_eq!(task.task_type, TaskType::ComplianceCheck);
        assert_eq!(task.get_parameter("framework"), Some(&"NIST_CSF".to_string()));
        assert_eq!(task.get_parameter("control_id"), Some(&"PR.AC-1".to_string()));
    }
}
