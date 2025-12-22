use gov_common::*;
use gov_common::messages::TaskResult;
use crate::communication::NetworkClient;
use crate::execution::TaskExecutor;
use crate::asset::AssetInventory;
use std::time::Duration;
use tokio::time::timeout;

pub struct NexusAgent {
    id: Option<String>,
    server_addr: String,
    crypto: Crypto,
    network_client: NetworkClient,
    task_executor: TaskExecutor,
    asset_inventory: AssetInventory,
    capabilities: Vec<String>,
    last_heartbeat: u64,
    registered: bool,
}

impl NexusAgent {
    pub async fn new(server_addr: String, encryption_key: [u8; 32]) -> Result<Self> {
        let crypto = Crypto::new(encryption_key);
        let network_client = NetworkClient::new(server_addr.clone());
        let task_executor = TaskExecutor::new();
        let asset_inventory = AssetInventory::collect().await?;

        // Determine agent capabilities based on OS and available features
        let mut capabilities = vec![
            "compliance_scanning".to_string(),
            "asset_inventory".to_string(),
            "security_validation".to_string(),
            "persistence_audit".to_string(),
        ];

        #[cfg(target_os = "windows")]
        {
            capabilities.push("registry_audit".to_string());
            capabilities.push("service_audit".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            capabilities.push("systemd_audit".to_string());
            capabilities.push("cron_audit".to_string());
        }

        Ok(Self {
            id: None,
            server_addr,
            crypto,
            network_client,
            task_executor,
            asset_inventory,
            capabilities,
            last_heartbeat: 0,
            registered: false,
        })
    }

    pub async fn run_cycle(&mut self) -> Result<()> {
        // Register if not already registered
        if !self.registered {
            self.register().await?;
        }

        // Send heartbeat and check for tasks
        let tasks = self.heartbeat().await?;

        // Execute any received tasks
        for task_data in tasks {
            if let Err(e) = self.execute_task(task_data).await {
                #[cfg(debug_assertions)]
                eprintln!("Task execution error: {}", e);
            }
        }

        Ok(())
    }

    async fn register(&mut self) -> Result<()> {
        let registration_data = RegistrationData {
            hostname: self.asset_inventory.hostname.clone(),
            os_type: self.asset_inventory.os_name.clone(),
            os_version: self.asset_inventory.os_version.clone(),
            ip_address: self.asset_inventory.primary_ip.clone(),
            username: self.asset_inventory.username.clone(),
            process_id: self.asset_inventory.process_id,
            process_name: self.asset_inventory.process_name.clone(),
            architecture: self.asset_inventory.architecture.clone(),
            capabilities: self.capabilities.clone(),
        };

        let message = Message::registration(serde_json::to_string(&registration_data)?);
        let encrypted_message = self.crypto.encrypt(&serde_json::to_string(&message)?)?;

        // Send registration with timeout
        let response = timeout(
            Duration::from_secs(30),
            self.network_client.send_message(&encrypted_message)
        ).await
            .map_err(|_| NexusError::NetworkError("Registration timeout".to_string()))??;

        let decrypted_response = self.crypto.decrypt(&response)?;
        let response_message: Message = serde_json::from_str(&decrypted_response)?;

        if response_message.msg_type == MessageType::Registration {
            self.id = Some(response_message.content);
            self.registered = true;
            self.last_heartbeat = current_timestamp();
            
            #[cfg(debug_assertions)]
            println!("Successfully registered with agent ID: {}", self.id.as_ref().unwrap());
        }

        Ok(())
    }

    async fn heartbeat(&mut self) -> Result<Vec<TaskData>> {
        let agent_id = self.id.as_ref()
            .ok_or_else(|| NexusError::AgentError("Agent not registered".to_string()))?;

        let message = Message::heartbeat(agent_id.clone());
        let encrypted_message = self.crypto.encrypt(&serde_json::to_string(&message)?)?;

        // Send heartbeat with timeout
        let response = timeout(
            Duration::from_secs(15),
            self.network_client.send_message(&encrypted_message)
        ).await
            .map_err(|_| NexusError::NetworkError("Heartbeat timeout".to_string()))??;

        let decrypted_response = self.crypto.decrypt(&response)?;
        let response_message: Message = serde_json::from_str(&decrypted_response)?;

        self.last_heartbeat = current_timestamp();

        // Check if we received task assignments
        let mut tasks = Vec::new();
        match response_message.msg_type {
            MessageType::TaskAssignment => {
                if let Ok(task_data) = serde_json::from_str::<TaskData>(&response_message.content) {
                    tasks.push(task_data);
                }
            }
            _ => {}
        }

        Ok(tasks)
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
            self.task_executor.execute_task(task_data)
        ).await;

        let task_result = match result {
            Ok(Ok(output)) => {
                TaskResult::success(task_id.clone(), output, (current_timestamp() - start_time) * 1000)
            }
            Ok(Err(e)) => {
                TaskResult::error(task_id.clone(), e.to_string(), (current_timestamp() - start_time) * 1000)
            }
            Err(_) => {
                TaskResult::timeout(task_id.clone(), (current_timestamp() - start_time) * 1000)
            }
        };

        // Send task result back to server
        self.send_task_result(task_result).await?;

        Ok(())
    }

    async fn send_task_result(&mut self, task_result: TaskResult) -> Result<()> {
        let agent_id = self.id.as_ref()
            .ok_or_else(|| NexusError::AgentError("Agent not registered".to_string()))?;

        let message = Message::task_result(
            serde_json::to_string(&task_result)?,
            agent_id.clone()
        );
        let encrypted_message = self.crypto.encrypt(&serde_json::to_string(&message)?)?;

        // Send with timeout
        let _response = timeout(
            Duration::from_secs(15),
            self.network_client.send_message(&encrypted_message)
        ).await
            .map_err(|_| NexusError::NetworkError("Task result send timeout".to_string()))??;

        Ok(())
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
            assert!(capabilities.contains(&"registry_operations".to_string()));
            assert!(capabilities.contains(&"service_control".to_string()));
        }
    }
}
