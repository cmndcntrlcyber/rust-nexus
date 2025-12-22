//! Persistence Audit Module
//!
//! Audits system for unauthorized persistence mechanisms.
//! This module transforms persistence installation logic into READ-ONLY audit logic.
//! Instead of writing registry keys or service files, it reads and reports findings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk level for persistence findings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// A persistence mechanism finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceFinding {
    pub location: String,
    pub mechanism: PersistenceMechanism,
    pub value: String,
    pub executable_path: Option<String>,
    pub risk_level: RiskLevel,
    pub description: String,
    pub is_known_good: bool,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistenceMechanism {
    RegistryRunKey,
    RegistryRunOnceKey,
    RegistryServicesKey,
    ScheduledTask,
    SystemdService,
    SystemdTimer,
    CronJob,
    StartupFolder,
    LoginHook,
    LaunchAgent,
    LaunchDaemon,
    Other(String),
}

/// Known good/whitelisted persistence entries
#[derive(Debug, Clone)]
pub struct PersistenceWhitelist {
    entries: HashMap<String, String>, // location -> expected value pattern
}

impl PersistenceWhitelist {
    pub fn new() -> Self {
        let mut entries = HashMap::new();

        // Common Windows known-good entries
        entries.insert(
            "SecurityHealth".to_string(),
            "Windows Security".to_string(),
        );
        entries.insert("iTunesHelper".to_string(), "Apple".to_string());
        entries.insert("OneDrive".to_string(), "Microsoft".to_string());

        Self { entries }
    }

    pub fn is_known_good(&self, entry_name: &str, value: &str) -> bool {
        if let Some(expected) = self.entries.get(entry_name) {
            return value.to_lowercase().contains(&expected.to_lowercase());
        }
        false
    }
}

impl Default for PersistenceWhitelist {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistence auditor for compliance checking
pub struct PersistenceAuditor {
    whitelist: PersistenceWhitelist,
}

impl PersistenceAuditor {
    pub fn new() -> Self {
        Self {
            whitelist: PersistenceWhitelist::new(),
        }
    }

    /// Audit all persistence mechanisms
    pub async fn audit_all(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        #[cfg(target_os = "windows")]
        {
            findings.extend(self.audit_registry_run_keys().await);
            findings.extend(self.audit_scheduled_tasks().await);
            findings.extend(self.audit_startup_folder().await);
            findings.extend(self.audit_services().await);
        }

        #[cfg(target_os = "linux")]
        {
            findings.extend(self.audit_systemd_units().await);
            findings.extend(self.audit_cron_jobs().await);
            findings.extend(self.audit_init_scripts().await);
        }

        findings
    }

    /// Audit Windows Registry Run keys (READ-ONLY)
    #[cfg(target_os = "windows")]
    pub async fn audit_registry_run_keys(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        let run_keys = [
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        ];

        for key_path in &run_keys {
            if let Ok(output) = std::process::Command::new("reg")
                .args(&["query", key_path])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);

                for line in output_str.lines() {
                    if line.contains("REG_SZ") || line.contains("REG_EXPAND_SZ") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let entry_name = parts[0];
                            let value = parts[2..].join(" ");

                            let is_known_good = self.whitelist.is_known_good(entry_name, &value);
                            let mechanism = if key_path.contains("RunOnce") {
                                PersistenceMechanism::RegistryRunOnceKey
                            } else {
                                PersistenceMechanism::RegistryRunKey
                            };

                            let risk_level = if is_known_good {
                                RiskLevel::Low
                            } else if value.contains("temp") || value.contains("appdata\\local\\temp") {
                                RiskLevel::High
                            } else {
                                RiskLevel::Medium
                            };

                            findings.push(PersistenceFinding {
                                location: format!("{}\\{}", key_path, entry_name),
                                mechanism,
                                value: value.clone(),
                                executable_path: Some(value.clone()),
                                risk_level,
                                description: format!(
                                    "Auto-start entry '{}' executes: {}",
                                    entry_name, value
                                ),
                                is_known_good,
                                recommendation: if is_known_good {
                                    "Known good entry, no action required".to_string()
                                } else {
                                    "Verify this auto-start entry is authorized".to_string()
                                },
                            });
                        }
                    }
                }
            }
        }

        findings
    }

    /// Audit Windows Scheduled Tasks (READ-ONLY)
    #[cfg(target_os = "windows")]
    pub async fn audit_scheduled_tasks(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        if let Ok(output) = std::process::Command::new("schtasks")
            .args(&["/query", "/fo", "csv", "/v"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);

            for line in output_str.lines().skip(1) {
                // Skip header
                let fields: Vec<&str> = line.split(',').collect();
                if fields.len() > 8 {
                    let task_name = fields[0].trim_matches('"');
                    let task_path = fields.get(8).map(|s| s.trim_matches('"')).unwrap_or("");

                    if !task_name.is_empty() && !task_name.starts_with("TaskName") {
                        findings.push(PersistenceFinding {
                            location: task_name.to_string(),
                            mechanism: PersistenceMechanism::ScheduledTask,
                            value: task_path.to_string(),
                            executable_path: Some(task_path.to_string()),
                            risk_level: RiskLevel::Medium,
                            description: format!("Scheduled task: {}", task_name),
                            is_known_good: task_name.starts_with("\\Microsoft\\"),
                            recommendation: "Review scheduled task for authorization".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }

    /// Audit Windows Startup Folder (READ-ONLY)
    #[cfg(target_os = "windows")]
    pub async fn audit_startup_folder(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        let startup_paths = [
            std::env::var("APPDATA")
                .map(|p| format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", p))
                .ok(),
            Some(
                "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup".to_string(),
            ),
        ];

        for path_opt in startup_paths.iter().flatten() {
            if let Ok(entries) = std::fs::read_dir(path_opt) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_path = entry.path().to_string_lossy().to_string();

                    findings.push(PersistenceFinding {
                        location: path_opt.clone(),
                        mechanism: PersistenceMechanism::StartupFolder,
                        value: file_name.clone(),
                        executable_path: Some(file_path),
                        risk_level: RiskLevel::Medium,
                        description: format!("Startup folder item: {}", file_name),
                        is_known_good: false,
                        recommendation: "Verify startup folder item is authorized".to_string(),
                    });
                }
            }
        }

        findings
    }

    /// Audit Windows Services (READ-ONLY)
    #[cfg(target_os = "windows")]
    pub async fn audit_services(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        if let Ok(output) = std::process::Command::new("sc")
            .args(&["query", "state=", "all"])
            .output()
        {
            // Basic service enumeration - in production would use WMI
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut current_service = String::new();

            for line in output_str.lines() {
                if line.trim().starts_with("SERVICE_NAME:") {
                    current_service = line
                        .split(':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                } else if line.contains("STATE") && !current_service.is_empty() {
                    findings.push(PersistenceFinding {
                        location: format!("Service: {}", current_service),
                        mechanism: PersistenceMechanism::RegistryServicesKey,
                        value: current_service.clone(),
                        executable_path: None,
                        risk_level: RiskLevel::Low,
                        description: format!("Windows service: {}", current_service),
                        is_known_good: true, // Would need deeper analysis
                        recommendation: "Review non-Microsoft services".to_string(),
                    });
                    current_service.clear();
                }
            }
        }

        findings
    }

    /// Audit systemd units (READ-ONLY)
    #[cfg(target_os = "linux")]
    pub async fn audit_systemd_units(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        let systemd_paths = [
            "/etc/systemd/system",
            "/usr/lib/systemd/system",
            "/lib/systemd/system",
        ];

        for path in &systemd_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_path = entry.path();

                    if file_name.ends_with(".service") || file_name.ends_with(".timer") {
                        let mechanism = if file_name.ends_with(".timer") {
                            PersistenceMechanism::SystemdTimer
                        } else {
                            PersistenceMechanism::SystemdService
                        };

                        let content =
                            std::fs::read_to_string(&file_path).unwrap_or_default();
                        let exec_start = content
                            .lines()
                            .find(|l| l.starts_with("ExecStart="))
                            .map(|l| l.trim_start_matches("ExecStart=").to_string());

                        findings.push(PersistenceFinding {
                            location: file_path.to_string_lossy().to_string(),
                            mechanism,
                            value: file_name.clone(),
                            executable_path: exec_start,
                            risk_level: RiskLevel::Low,
                            description: format!("Systemd unit: {}", file_name),
                            is_known_good: path.contains("/usr/lib") || path.contains("/lib"),
                            recommendation: "Review custom systemd units".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }

    /// Audit cron jobs (READ-ONLY)
    #[cfg(target_os = "linux")]
    pub async fn audit_cron_jobs(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        let cron_paths = [
            "/etc/crontab",
            "/etc/cron.d",
            "/etc/cron.daily",
            "/etc/cron.hourly",
            "/etc/cron.weekly",
            "/etc/cron.monthly",
        ];

        for path in &cron_paths {
            let path_obj = std::path::Path::new(path);

            if path_obj.is_file() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for line in content.lines() {
                        if !line.starts_with('#') && !line.trim().is_empty() {
                            findings.push(PersistenceFinding {
                                location: path.to_string(),
                                mechanism: PersistenceMechanism::CronJob,
                                value: line.to_string(),
                                executable_path: None,
                                risk_level: RiskLevel::Medium,
                                description: format!("Cron entry in {}", path),
                                is_known_good: false,
                                recommendation: "Review cron job for authorization".to_string(),
                            });
                        }
                    }
                }
            } else if path_obj.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        findings.push(PersistenceFinding {
                            location: entry.path().to_string_lossy().to_string(),
                            mechanism: PersistenceMechanism::CronJob,
                            value: file_name.clone(),
                            executable_path: Some(entry.path().to_string_lossy().to_string()),
                            risk_level: RiskLevel::Low,
                            description: format!("Cron script: {}", file_name),
                            is_known_good: false,
                            recommendation: "Review cron script for authorization".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }

    /// Audit init scripts (READ-ONLY)
    #[cfg(target_os = "linux")]
    pub async fn audit_init_scripts(&self) -> Vec<PersistenceFinding> {
        let mut findings = Vec::new();

        let init_paths = ["/etc/init.d", "/etc/rc.local"];

        for path in &init_paths {
            let path_obj = std::path::Path::new(path);

            if path_obj.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        findings.push(PersistenceFinding {
                            location: entry.path().to_string_lossy().to_string(),
                            mechanism: PersistenceMechanism::Other("init.d".to_string()),
                            value: file_name.clone(),
                            executable_path: Some(entry.path().to_string_lossy().to_string()),
                            risk_level: RiskLevel::Low,
                            description: format!("Init script: {}", file_name),
                            is_known_good: true,
                            recommendation: "Review init.d scripts".to_string(),
                        });
                    }
                }
            } else if path_obj.is_file() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if !content.trim().is_empty() && content.lines().count() > 2 {
                        findings.push(PersistenceFinding {
                            location: path.to_string(),
                            mechanism: PersistenceMechanism::Other("rc.local".to_string()),
                            value: "Custom rc.local content".to_string(),
                            executable_path: None,
                            risk_level: RiskLevel::Medium,
                            description: "rc.local has custom content".to_string(),
                            is_known_good: false,
                            recommendation: "Review rc.local for unauthorized entries".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }

    /// Get summary statistics for findings
    pub fn summarize_findings(findings: &[PersistenceFinding]) -> PersistenceAuditSummary {
        PersistenceAuditSummary {
            total_findings: findings.len(),
            critical_count: findings.iter().filter(|f| f.risk_level == RiskLevel::Critical).count(),
            high_count: findings.iter().filter(|f| f.risk_level == RiskLevel::High).count(),
            medium_count: findings.iter().filter(|f| f.risk_level == RiskLevel::Medium).count(),
            low_count: findings.iter().filter(|f| f.risk_level == RiskLevel::Low).count(),
            known_good_count: findings.iter().filter(|f| f.is_known_good).count(),
            unknown_count: findings.iter().filter(|f| !f.is_known_good).count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceAuditSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub known_good_count: usize,
    pub unknown_count: usize,
}

impl Default for PersistenceAuditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
    }

    #[test]
    fn test_whitelist() {
        let whitelist = PersistenceWhitelist::new();
        assert!(whitelist.is_known_good("SecurityHealth", "Windows Security Health"));
        assert!(!whitelist.is_known_good("SuspiciousEntry", "malware.exe"));
    }

    #[tokio::test]
    async fn test_persistence_auditor() {
        let auditor = PersistenceAuditor::new();
        let findings = auditor.audit_all().await;
        // Should return some findings on any system
        // (at least systemd units on Linux or services on Windows)
        println!("Found {} persistence entries", findings.len());
    }
}
