use nexus_common::{
    AttackTechnique, ExecutionContext, NexusError, Platform, Result, Tactic,
    TechniqueParams, TechniqueResult,
};
use std::process::Command;

/// T1059 - Command and Scripting Interpreter (native shell)
pub struct ShellInterpreter;

#[async_trait::async_trait]
impl AttackTechnique for ShellInterpreter {
    fn technique_id(&self) -> &str {
        "T1059"
    }

    fn name(&self) -> &str {
        "Command and Scripting Interpreter"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Execution]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::All]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["shell_execution".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["shell".to_string()]
    }

    async fn execute(&self, _ctx: &ExecutionContext, params: TechniqueParams) -> Result<TechniqueResult> {
        let command = params.parameters.get("command")
            .or_else(|| if !params.command.is_empty() { Some(&params.command) } else { None })
            .ok_or_else(|| NexusError::TaskExecutionError("Missing command parameter".to_string()))?;

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd")
            .args(&["/C", command])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Command execution failed: {}", e)))?;

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh")
            .args(&["-c", command])
            .output()
            .map_err(|e| NexusError::TaskExecutionError(format!("Command execution failed: {}", e)))?;

        if output.status.success() {
            Ok(TechniqueResult::ok(String::from_utf8_lossy(&output.stdout).to_string()))
        } else {
            Ok(TechniqueResult::ok(format!(
                "Error ({}): {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }
}

/// T1059.001 - PowerShell
pub struct PowerShellInterpreter;

#[async_trait::async_trait]
impl AttackTechnique for PowerShellInterpreter {
    fn technique_id(&self) -> &str {
        "T1059.001"
    }

    fn name(&self) -> &str {
        "PowerShell"
    }

    fn tactics(&self) -> &[Tactic] {
        &[Tactic::Execution]
    }

    fn platforms(&self) -> &[Platform] {
        &[Platform::Windows]
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["powershell_execution".to_string()]
    }

    fn task_types(&self) -> Vec<String> {
        vec!["powershell".to_string()]
    }

    async fn execute(&self, _ctx: &ExecutionContext, _params: TechniqueParams) -> Result<TechniqueResult> {
        #[cfg(target_os = "windows")]
        {
            let command = _params.parameters.get("command")
                .or_else(|| if !_params.command.is_empty() { Some(&_params.command) } else { None })
                .ok_or_else(|| NexusError::TaskExecutionError("Missing command parameter".to_string()))?;

            let output = Command::new("powershell")
                .args(&["-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", command])
                .output()
                .map_err(|e| NexusError::TaskExecutionError(format!("PowerShell execution failed: {}", e)))?;

            if output.status.success() {
                Ok(TechniqueResult::ok(String::from_utf8_lossy(&output.stdout).to_string()))
            } else {
                Ok(TechniqueResult::ok(format!(
                    "PowerShell Error ({}): {}",
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                )))
            }
        }

        #[cfg(not(target_os = "windows"))]
        Err(NexusError::TaskExecutionError("PowerShell not available on this platform".to_string()))
    }
}

/// Register all T1059 sub-techniques
pub fn register() -> Vec<Box<dyn AttackTechnique>> {
    vec![
        Box::new(ShellInterpreter),
        #[cfg(target_os = "windows")]
        Box::new(PowerShellInterpreter),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_technique_metadata() {
        let shell = ShellInterpreter;
        assert_eq!(shell.technique_id(), "T1059");
        assert_eq!(shell.tactics(), &[Tactic::Execution]);
        assert_eq!(shell.task_types(), vec!["shell".to_string()]);
    }

    #[test]
    fn test_powershell_technique_metadata() {
        let ps = PowerShellInterpreter;
        assert_eq!(ps.technique_id(), "T1059.001");
        assert_eq!(ps.platforms(), &[Platform::Windows]);
        assert_eq!(ps.task_types(), vec!["powershell".to_string()]);
    }

    #[test]
    fn test_register() {
        let techniques = register();
        // At minimum, shell interpreter is always registered
        assert!(!techniques.is_empty());
        assert!(techniques.iter().any(|t| t.technique_id() == "T1059"));
    }

    #[tokio::test]
    async fn test_shell_execution() {
        let shell = ShellInterpreter;
        let crypto = std::sync::Arc::new(nexus_common::Crypto::new(nexus_common::Crypto::generate_key()));
        let ctx = ExecutionContext {
            crypto,
            agent_id: "test-agent".to_string(),
            platform: Platform::Linux,
        };
        let mut params = TechniqueParams {
            command: String::new(),
            parameters: std::collections::HashMap::new(),
            timeout: None,
        };
        params.parameters.insert("command".to_string(), "echo hello".to_string());

        let result = shell.execute(&ctx, params).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }
}
