//! Asset Inventory Module
//!
//! Collects comprehensive asset information for compliance and governance purposes.
//! Includes hardware, software, configuration, and security tool inventory.

use gov_common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

/// Represents a software package installed on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwarePackage {
    pub name: String,
    pub version: String,
    pub vendor: Option<String>,
    pub install_date: Option<String>,
    pub install_location: Option<String>,
}

/// Represents a security tool detected on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTool {
    pub name: String,
    pub tool_type: SecurityToolType,
    pub is_running: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityToolType {
    Antivirus,
    EDR,
    Firewall,
    DLP,
    SIEM,
    VulnerabilityScanner,
    IdentityManagement,
    Other,
}

/// Represents a running service on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub status: ServiceStatus,
    pub start_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Paused,
    Unknown,
}

/// Comprehensive asset inventory for compliance purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInventory {
    // Basic system identification
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub username: String,
    pub process_id: u32,
    pub process_name: String,
    pub primary_ip: String,

    // Extended inventory fields
    pub installed_software: Vec<SoftwarePackage>,
    pub security_tools: Vec<SecurityTool>,
    pub running_services: Vec<ServiceInfo>,
    pub configuration_baseline: HashMap<String, String>,
    pub compliance_tags: Vec<String>,
    pub last_scan: chrono::DateTime<chrono::Utc>,

    // Hardware information
    pub cpu_count: usize,
    pub total_memory_mb: u64,
    pub disk_info: Vec<DiskInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub total_space_gb: u64,
    pub free_space_gb: u64,
    pub filesystem_type: String,
}

impl AssetInventory {
    /// Collect comprehensive asset inventory
    pub async fn collect() -> Result<Self> {
        let hostname = Self::get_hostname();
        let os_name = Self::get_os_name();
        let os_version = Self::get_os_version();
        let architecture = Self::get_architecture();
        let username = Self::get_username();
        let process_id = std::process::id();
        let process_name = Self::get_process_name();
        let primary_ip = Self::get_primary_ip().await;

        let security_tools = Self::detect_security_tools().await;
        let running_services = Self::enumerate_services().await;
        let cpu_count = num_cpus::get();
        let total_memory_mb = Self::get_total_memory();

        Ok(Self {
            hostname,
            os_name,
            os_version,
            architecture,
            username,
            process_id,
            process_name,
            primary_ip,
            installed_software: Vec::new(), // TODO: Implement software inventory
            security_tools,
            running_services,
            configuration_baseline: HashMap::new(),
            compliance_tags: Vec::new(),
            last_scan: chrono::Utc::now(),
            cpu_count,
            total_memory_mb,
            disk_info: Vec::new(), // TODO: Implement disk enumeration
        })
    }

    /// Detect installed security tools
    async fn detect_security_tools() -> Vec<SecurityTool> {
        let mut tools = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Common Windows EDR/AV processes
            let edr_processes = [
                ("CrowdStrike", "CSFalconService", SecurityToolType::EDR),
                ("Carbon Black", "CbDefense", SecurityToolType::EDR),
                ("SentinelOne", "SentinelAgent", SecurityToolType::EDR),
                ("Microsoft Defender", "MsMpEng", SecurityToolType::Antivirus),
                ("Symantec", "ccSvcHst", SecurityToolType::Antivirus),
                ("McAfee", "mcshield", SecurityToolType::Antivirus),
                ("Trend Micro", "coreServiceShell", SecurityToolType::Antivirus),
                ("Sophos", "SophosClean", SecurityToolType::EDR),
                ("Cylance", "CylanceSvc", SecurityToolType::EDR),
            ];

            if let Ok(output) = std::process::Command::new("tasklist")
                .args(&["/fo", "csv"])
                .output()
            {
                let task_list = String::from_utf8_lossy(&output.stdout).to_lowercase();
                for (name, process, tool_type) in &edr_processes {
                    let is_running = task_list.contains(&process.to_lowercase());
                    tools.push(SecurityTool {
                        name: name.to_string(),
                        tool_type: tool_type.clone(),
                        is_running,
                        version: None,
                    });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Common Linux security tools
            let security_processes = [
                ("CrowdStrike Falcon", "falcon-sensor", SecurityToolType::EDR),
                ("Carbon Black", "cbagentd", SecurityToolType::EDR),
                ("ClamAV", "clamd", SecurityToolType::Antivirus),
                ("OSSEC", "ossec-agentd", SecurityToolType::EDR),
                ("Wazuh", "wazuh-agentd", SecurityToolType::EDR),
                ("Auditd", "auditd", SecurityToolType::Other),
                ("SELinux", "selinuxenabled", SecurityToolType::Other),
            ];

            if let Ok(output) = std::process::Command::new("ps").args(&["aux"]).output() {
                let process_list = String::from_utf8_lossy(&output.stdout).to_lowercase();
                for (name, process, tool_type) in &security_processes {
                    let is_running = process_list.contains(process);
                    tools.push(SecurityTool {
                        name: name.to_string(),
                        tool_type: tool_type.clone(),
                        is_running,
                        version: None,
                    });
                }
            }
        }

        tools
    }

    /// Enumerate running services
    async fn enumerate_services() -> Vec<ServiceInfo> {
        let mut services = Vec::new();

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("sc")
                .args(&["query", "state=", "all"])
                .output()
            {
                // Parse service output (simplified)
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.trim().starts_with("SERVICE_NAME:") {
                        if let Some(name) = line.split(':').nth(1) {
                            services.push(ServiceInfo {
                                name: name.trim().to_string(),
                                display_name: None,
                                status: ServiceStatus::Unknown,
                                start_type: "unknown".to_string(),
                            });
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("systemctl")
                .args(&["list-units", "--type=service", "--no-pager", "--no-legend"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let status = match parts[2] {
                            "running" => ServiceStatus::Running,
                            "exited" | "dead" => ServiceStatus::Stopped,
                            _ => ServiceStatus::Unknown,
                        };
                        services.push(ServiceInfo {
                            name: parts[0].trim_end_matches(".service").to_string(),
                            display_name: None,
                            status,
                            start_type: "unknown".to_string(),
                        });
                    }
                }
            }
        }

        services
    }

    fn get_hostname() -> String {
        hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    fn get_os_name() -> String {
        #[cfg(target_os = "windows")]
        return "Windows".to_string();

        #[cfg(target_os = "linux")]
        return "Linux".to_string();

        #[cfg(target_os = "macos")]
        return "macOS".to_string();

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return "Unknown".to_string();
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "windows")]
        {
            "Windows 10+".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/etc/os-release")
                .ok()
                .and_then(|content| {
                    content
                        .lines()
                        .find(|line| line.starts_with("PRETTY_NAME="))
                        .map(|line| line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
                })
                .unwrap_or_else(|| "Linux Unknown".to_string())
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        "Unknown".to_string()
    }

    fn get_architecture() -> String {
        #[cfg(target_arch = "x86_64")]
        return "x86_64".to_string();

        #[cfg(target_arch = "x86")]
        return "x86".to_string();

        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
        return "unknown".to_string();
    }

    fn get_username() -> String {
        env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn get_process_name() -> String {
        env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    async fn get_primary_ip() -> String {
        match local_ip_address::local_ip() {
            Ok(ip) => ip.to_string(),
            Err(_) => "127.0.0.1".to_string(),
        }
    }

    fn get_total_memory() -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                if let Some(line) = meminfo.lines().find(|line| line.starts_with("MemTotal:")) {
                    if let Some(mem_kb) = line.split_whitespace().nth(1) {
                        if let Ok(mem_kb) = mem_kb.parse::<u64>() {
                            return mem_kb / 1024; // Convert to MB
                        }
                    }
                }
            }
            0
        }

        #[cfg(not(target_os = "linux"))]
        0
    }

    /// Check if a specific security control is present and running
    pub fn has_security_control(&self, control_type: SecurityToolType) -> bool {
        self.security_tools
            .iter()
            .any(|tool| tool.tool_type == control_type && tool.is_running)
    }

    /// Get all running security tools
    pub fn get_active_security_tools(&self) -> Vec<&SecurityTool> {
        self.security_tools.iter().filter(|t| t.is_running).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_asset_inventory_collection() {
        let inventory = AssetInventory::collect().await.unwrap();
        assert!(!inventory.hostname.is_empty());
        assert!(!inventory.os_name.is_empty());
        assert!(inventory.cpu_count > 0);
    }

    #[test]
    fn test_security_tool_type() {
        let tool = SecurityTool {
            name: "Test EDR".to_string(),
            tool_type: SecurityToolType::EDR,
            is_running: true,
            version: Some("1.0".to_string()),
        };
        assert_eq!(tool.tool_type, SecurityToolType::EDR);
        assert!(tool.is_running);
    }
}
