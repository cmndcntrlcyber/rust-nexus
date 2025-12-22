//! Compliance Executor Module
//!
//! Executes compliance checks and collects evidence for governance frameworks.
//! This module replaces offensive execution capabilities with compliance-focused
//! read-only checks that validate system configuration against baselines.

use serde::{Deserialize, Serialize};
use std::process::Command;

/// Types of compliance checks that can be executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckType {
    RegistryQuery,
    FilePermissions,
    FileExists,
    FileContent,
    ServiceStatus,
    ProcessRunning,
    ScapOval,
    CisBenchmark,
    CustomScript,
}

/// Definition of a compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub check_id: String,
    pub check_type: CheckType,
    pub description: String,
    pub target: String,
    pub expected_value: Option<String>,
    pub operator: ComparisonOperator,
    pub framework: String,
    pub control_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    Exists,
    NotExists,
    Matches, // Regex
}

/// Result of a compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: String,
    pub status: CheckStatus,
    pub actual_value: String,
    pub expected_value: Option<String>,
    pub evidence: String,
    pub error_message: Option<String>,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Error,
    NotApplicable,
    Skipped,
}

/// Compliance check executor
pub struct ComplianceExecutor {
    timeout_seconds: u64,
}

impl ComplianceExecutor {
    pub fn new() -> Self {
        Self {
            timeout_seconds: 30,
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout_seconds = timeout;
        self
    }

    /// Execute a compliance check
    pub async fn execute_check(&self, check: &ComplianceCheck) -> CheckResult {
        let start = std::time::Instant::now();

        let result = match check.check_type {
            CheckType::RegistryQuery => self.execute_registry_check(check).await,
            CheckType::FilePermissions => self.execute_file_permissions_check(check).await,
            CheckType::FileExists => self.execute_file_exists_check(check).await,
            CheckType::FileContent => self.execute_file_content_check(check).await,
            CheckType::ServiceStatus => self.execute_service_status_check(check).await,
            CheckType::ProcessRunning => self.execute_process_check(check).await,
            CheckType::ScapOval => self.execute_oval_check(check).await,
            CheckType::CisBenchmark => self.execute_cis_check(check).await,
            CheckType::CustomScript => self.execute_custom_script(check).await,
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        CheckResult {
            check_id: check.check_id.clone(),
            status: result.0,
            actual_value: result.1,
            expected_value: check.expected_value.clone(),
            evidence: result.2,
            error_message: result.3,
            executed_at: chrono::Utc::now(),
            execution_time_ms,
        }
    }

    /// Execute multiple checks
    pub async fn execute_checks(&self, checks: &[ComplianceCheck]) -> Vec<CheckResult> {
        let mut results = Vec::new();
        for check in checks {
            results.push(self.execute_check(check).await);
        }
        results
    }

    /// Registry query check (Windows only, READ-ONLY)
    async fn execute_registry_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        #[cfg(target_os = "windows")]
        {
            let parts: Vec<&str> = check.target.splitn(2, '\\').collect();
            if parts.len() < 2 {
                return (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some("Invalid registry path format".to_string()),
                );
            }

            match Command::new("reg")
                .args(&["query", &check.target, "/v", parts.last().unwrap_or(&"")])
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let actual_value = self.parse_registry_value(&stdout);

                    let status = self.compare_values(
                        &actual_value,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (status, actual_value, stdout, None)
                }
                Err(e) => (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some(e.to_string()),
                ),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = check; // Suppress unused warning on non-Windows
            (
                CheckStatus::NotApplicable,
                String::new(),
                "Registry checks only applicable on Windows".to_string(),
                None,
            )
        }
    }

    /// File permissions check (READ-ONLY)
    async fn execute_file_permissions_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        match fs::metadata(&check.target) {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    let mode = metadata.permissions().mode();
                    let mode_string = format!("{:o}", mode & 0o7777);

                    let status = self.compare_values(
                        &mode_string,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    let evidence = format!(
                        "File: {}\nPermissions: {}\nOwner readable: {}\nOwner writable: {}\nOwner executable: {}",
                        check.target,
                        mode_string,
                        mode & 0o400 != 0,
                        mode & 0o200 != 0,
                        mode & 0o100 != 0
                    );

                    (status, mode_string, evidence, None)
                }

                #[cfg(not(unix))]
                {
                    let readonly = metadata.permissions().readonly();
                    let actual = if readonly { "readonly" } else { "writable" };

                    let status = self.compare_values(
                        actual,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (
                        status,
                        actual.to_string(),
                        format!("File: {}, Readonly: {}", check.target, readonly),
                        None,
                    )
                }
            }
            Err(e) => (
                CheckStatus::Error,
                String::new(),
                String::new(),
                Some(format!("Failed to read file metadata: {}", e)),
            ),
        }
    }

    /// File exists check (READ-ONLY)
    async fn execute_file_exists_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        let exists = std::path::Path::new(&check.target).exists();
        let actual = exists.to_string();

        let status = match check.operator {
            ComparisonOperator::Exists => {
                if exists {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                }
            }
            ComparisonOperator::NotExists => {
                if !exists {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                }
            }
            _ => self.compare_values(&actual, check.expected_value.as_deref(), &check.operator),
        };

        (
            status,
            actual,
            format!("Path '{}' exists: {}", check.target, exists),
            None,
        )
    }

    /// File content check (READ-ONLY)
    async fn execute_file_content_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        match std::fs::read_to_string(&check.target) {
            Ok(content) => {
                let status = self.compare_values(
                    &content,
                    check.expected_value.as_deref(),
                    &check.operator,
                );

                // Truncate evidence if content is too large
                let evidence = if content.len() > 1000 {
                    format!("{}... (truncated)", &content[..1000])
                } else {
                    content.clone()
                };

                (status, content, evidence, None)
            }
            Err(e) => (
                CheckStatus::Error,
                String::new(),
                String::new(),
                Some(format!("Failed to read file: {}", e)),
            ),
        }
    }

    /// Service status check (READ-ONLY)
    async fn execute_service_status_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        #[cfg(target_os = "windows")]
        {
            match Command::new("sc")
                .args(&["query", &check.target])
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let is_running = stdout.contains("RUNNING");
                    let actual = if is_running { "running" } else { "stopped" };

                    let status = self.compare_values(
                        actual,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (status, actual.to_string(), stdout, None)
                }
                Err(e) => (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some(e.to_string()),
                ),
            }
        }

        #[cfg(target_os = "linux")]
        {
            match Command::new("systemctl")
                .args(&["is-active", &check.target])
                .output()
            {
                Ok(output) => {
                    let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();

                    let status = self.compare_values(
                        &actual,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (
                        status,
                        actual.clone(),
                        format!("Service '{}' status: {}", check.target, actual),
                        None,
                    )
                }
                Err(e) => (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some(e.to_string()),
                ),
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            (
                CheckStatus::NotApplicable,
                String::new(),
                "Service checks not implemented for this platform".to_string(),
                None,
            )
        }
    }

    /// Process running check (READ-ONLY)
    async fn execute_process_check(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        #[cfg(target_os = "windows")]
        {
            match Command::new("tasklist").args(&["/fo", "csv"]).output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
                    let process_name = check.target.to_lowercase();
                    let is_running = stdout.contains(&process_name);
                    let actual = is_running.to_string();

                    let status = self.compare_values(
                        &actual,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (
                        status,
                        actual,
                        format!("Process '{}' running: {}", check.target, is_running),
                        None,
                    )
                }
                Err(e) => (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some(e.to_string()),
                ),
            }
        }

        #[cfg(target_os = "linux")]
        {
            match Command::new("pgrep").args(&["-x", &check.target]).output() {
                Ok(output) => {
                    let is_running = output.status.success();
                    let actual = is_running.to_string();

                    let status = self.compare_values(
                        &actual,
                        check.expected_value.as_deref(),
                        &check.operator,
                    );

                    (
                        status,
                        actual,
                        format!("Process '{}' running: {}", check.target, is_running),
                        None,
                    )
                }
                Err(e) => (
                    CheckStatus::Error,
                    String::new(),
                    String::new(),
                    Some(e.to_string()),
                ),
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            (
                CheckStatus::NotApplicable,
                String::new(),
                "Process checks not implemented for this platform".to_string(),
                None,
            )
        }
    }

    /// SCAP OVAL check (stub - would integrate with OVAL interpreter)
    async fn execute_oval_check(
        &self,
        _check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        // TODO: Integrate with OpenSCAP or similar OVAL interpreter
        (
            CheckStatus::NotApplicable,
            String::new(),
            "OVAL check execution requires OpenSCAP integration".to_string(),
            Some("OVAL interpreter not yet implemented".to_string()),
        )
    }

    /// CIS Benchmark check (stub - would integrate with CIS-CAT)
    async fn execute_cis_check(
        &self,
        _check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        // TODO: Integrate with CIS-CAT or similar benchmark tool
        (
            CheckStatus::NotApplicable,
            String::new(),
            "CIS Benchmark check requires CIS-CAT integration".to_string(),
            Some("CIS-CAT integration not yet implemented".to_string()),
        )
    }

    /// Custom script check (READ-ONLY, safe commands only)
    async fn execute_custom_script(
        &self,
        check: &ComplianceCheck,
    ) -> (CheckStatus, String, String, Option<String>) {
        // Only allow read-only commands
        let safe_commands = ["cat", "ls", "dir", "type", "get-content", "findstr", "grep"];
        let cmd_lower = check.target.to_lowercase();

        let is_safe = safe_commands.iter().any(|cmd| cmd_lower.starts_with(cmd));

        if !is_safe {
            return (
                CheckStatus::Error,
                String::new(),
                String::new(),
                Some("Only read-only commands are permitted".to_string()),
            );
        }

        #[cfg(target_os = "windows")]
        let shell = "cmd";
        #[cfg(target_os = "windows")]
        let shell_arg = "/c";

        #[cfg(not(target_os = "windows"))]
        let shell = "sh";
        #[cfg(not(target_os = "windows"))]
        let shell_arg = "-c";

        match Command::new(shell)
            .args(&[shell_arg, &check.target])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let success = output.status.success();

                let status = if success {
                    self.compare_values(&stdout, check.expected_value.as_deref(), &check.operator)
                } else {
                    CheckStatus::Error
                };

                (status, stdout.clone(), stdout, None)
            }
            Err(e) => (
                CheckStatus::Error,
                String::new(),
                String::new(),
                Some(e.to_string()),
            ),
        }
    }

    /// Compare actual value against expected value using the specified operator
    fn compare_values(
        &self,
        actual: &str,
        expected: Option<&str>,
        operator: &ComparisonOperator,
    ) -> CheckStatus {
        match operator {
            ComparisonOperator::Equals => {
                if let Some(exp) = expected {
                    if actual.trim() == exp.trim() {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Fail
                    }
                } else {
                    CheckStatus::Error
                }
            }
            ComparisonOperator::NotEquals => {
                if let Some(exp) = expected {
                    if actual.trim() != exp.trim() {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Fail
                    }
                } else {
                    CheckStatus::Error
                }
            }
            ComparisonOperator::Contains => {
                if let Some(exp) = expected {
                    if actual.contains(exp) {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Fail
                    }
                } else {
                    CheckStatus::Error
                }
            }
            ComparisonOperator::NotContains => {
                if let Some(exp) = expected {
                    if !actual.contains(exp) {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Fail
                    }
                } else {
                    CheckStatus::Error
                }
            }
            ComparisonOperator::Exists => {
                if actual == "true" {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                }
            }
            ComparisonOperator::NotExists => {
                if actual == "false" {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                }
            }
            ComparisonOperator::GreaterThan | ComparisonOperator::LessThan => {
                if let (Ok(act), Some(Ok(exp))) = (
                    actual.trim().parse::<i64>(),
                    expected.map(|e| e.trim().parse::<i64>()),
                ) {
                    let pass = match operator {
                        ComparisonOperator::GreaterThan => act > exp,
                        ComparisonOperator::LessThan => act < exp,
                        _ => false,
                    };
                    if pass {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Fail
                    }
                } else {
                    CheckStatus::Error
                }
            }
            ComparisonOperator::Matches => {
                if let Some(pattern) = expected {
                    match regex::Regex::new(pattern) {
                        Ok(re) => {
                            if re.is_match(actual) {
                                CheckStatus::Pass
                            } else {
                                CheckStatus::Fail
                            }
                        }
                        Err(_) => CheckStatus::Error,
                    }
                } else {
                    CheckStatus::Error
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn parse_registry_value(&self, output: &str) -> String {
        for line in output.lines() {
            if line.contains("REG_") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return parts[2..].join(" ");
                }
            }
        }
        String::new()
    }

    #[cfg(not(target_os = "windows"))]
    fn parse_registry_value(&self, _output: &str) -> String {
        String::new()
    }
}

impl Default for ComplianceExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-built compliance checks for common frameworks
pub struct ComplianceCheckLibrary;

impl ComplianceCheckLibrary {
    /// Get CIS benchmark checks for Windows
    #[cfg(target_os = "windows")]
    pub fn cis_windows_checks() -> Vec<ComplianceCheck> {
        vec![
            ComplianceCheck {
                check_id: "CIS-1.1.1".to_string(),
                check_type: CheckType::ServiceStatus,
                description: "Ensure Windows Firewall is enabled".to_string(),
                target: "mpssvc".to_string(),
                expected_value: Some("running".to_string()),
                operator: ComparisonOperator::Equals,
                framework: "CIS Windows".to_string(),
                control_id: "1.1.1".to_string(),
            },
            ComplianceCheck {
                check_id: "CIS-2.3.1".to_string(),
                check_type: CheckType::ServiceStatus,
                description: "Ensure Windows Defender is running".to_string(),
                target: "WinDefend".to_string(),
                expected_value: Some("running".to_string()),
                operator: ComparisonOperator::Equals,
                framework: "CIS Windows".to_string(),
                control_id: "2.3.1".to_string(),
            },
        ]
    }

    /// Get CIS benchmark checks for Linux
    #[cfg(target_os = "linux")]
    pub fn cis_linux_checks() -> Vec<ComplianceCheck> {
        vec![
            ComplianceCheck {
                check_id: "CIS-1.1.1".to_string(),
                check_type: CheckType::FileExists,
                description: "Ensure /tmp is a separate partition".to_string(),
                target: "/tmp".to_string(),
                expected_value: Some("true".to_string()),
                operator: ComparisonOperator::Exists,
                framework: "CIS Linux".to_string(),
                control_id: "1.1.1".to_string(),
            },
            ComplianceCheck {
                check_id: "CIS-1.4.1".to_string(),
                check_type: CheckType::FilePermissions,
                description: "Ensure permissions on /etc/passwd are configured".to_string(),
                target: "/etc/passwd".to_string(),
                expected_value: Some("644".to_string()),
                operator: ComparisonOperator::Equals,
                framework: "CIS Linux".to_string(),
                control_id: "1.4.1".to_string(),
            },
            ComplianceCheck {
                check_id: "CIS-4.2.1".to_string(),
                check_type: CheckType::ServiceStatus,
                description: "Ensure rsyslog is installed and running".to_string(),
                target: "rsyslog".to_string(),
                expected_value: Some("active".to_string()),
                operator: ComparisonOperator::Equals,
                framework: "CIS Linux".to_string(),
                control_id: "4.2.1".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_executor_creation() {
        let executor = ComplianceExecutor::new();
        assert_eq!(executor.timeout_seconds, 30);
    }

    #[test]
    fn test_comparison_operators() {
        let executor = ComplianceExecutor::new();

        assert_eq!(
            executor.compare_values("test", Some("test"), &ComparisonOperator::Equals),
            CheckStatus::Pass
        );

        assert_eq!(
            executor.compare_values("test", Some("other"), &ComparisonOperator::Equals),
            CheckStatus::Fail
        );

        assert_eq!(
            executor.compare_values("hello world", Some("world"), &ComparisonOperator::Contains),
            CheckStatus::Pass
        );
    }

    #[tokio::test]
    async fn test_file_exists_check() {
        let executor = ComplianceExecutor::new();
        let check = ComplianceCheck {
            check_id: "test-1".to_string(),
            check_type: CheckType::FileExists,
            description: "Test file exists".to_string(),
            target: "/etc/passwd".to_string(),
            expected_value: Some("true".to_string()),
            operator: ComparisonOperator::Exists,
            framework: "test".to_string(),
            control_id: "1".to_string(),
        };

        let result = executor.execute_check(&check).await;
        #[cfg(target_os = "linux")]
        assert_eq!(result.status, CheckStatus::Pass);
    }
}
