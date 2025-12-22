//! Security Validation Module
//!
//! Validates that required security controls are present and operational.
//! This module inverts the logic of evasion detection - instead of detecting
//! security tools to AVOID them, we detect security tools to VALIDATE they're working.

use serde::{Deserialize, Serialize};

/// Result of a security control assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAssessment {
    pub control_id: String,
    pub control_name: String,
    pub status: AssessmentStatus,
    pub findings: Vec<String>,
    pub evidence: Vec<String>,
    pub framework_mappings: Vec<FrameworkMapping>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssessmentStatus {
    Pass,
    Fail,
    NotApplicable,
    Error,
    ManualReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkMapping {
    pub framework: String,
    pub control_id: String,
    pub control_name: String,
}

/// Security control validator
pub struct SecurityValidator {
    /// Known EDR/AV processes to validate
    edr_processes: Vec<(&'static str, &'static str)>,
    /// Known Windows Firewall services
    firewall_services: Vec<&'static str>,
}

impl SecurityValidator {
    pub fn new() -> Self {
        Self {
            edr_processes: vec![
                ("CrowdStrike Falcon", "csfalconservice"),
                ("Carbon Black", "cbdefense"),
                ("SentinelOne", "sentinelagent"),
                ("Microsoft Defender", "msmpeng"),
                ("Symantec Endpoint", "ccsvchst"),
                ("McAfee", "mcshield"),
                ("Trend Micro", "coreserviceshell"),
                ("Sophos", "sophosclean"),
                ("Cylance", "cylancesvc"),
                ("ESET", "ekrn"),
                ("Kaspersky", "avp"),
                ("Bitdefender", "vsserv"),
            ],
            firewall_services: vec![
                "mpssvc",      // Windows Firewall
                "SharedAccess", // Internet Connection Sharing / Firewall
            ],
        }
    }

    /// Validate that endpoint protection (EDR/AV) is running
    ///
    /// INVERTED LOGIC: Returns PASS if security tool is detected
    /// (opposite of evasion which would return true to avoid)
    pub async fn validate_endpoint_protection(&self) -> ControlAssessment {
        let mut findings = Vec::new();
        let mut evidence = Vec::new();
        let mut edr_found = false;

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("tasklist")
                .args(&["/fo", "csv"])
                .output()
            {
                let task_list = String::from_utf8_lossy(&output.stdout).to_lowercase();

                for (name, process) in &self.edr_processes {
                    if task_list.contains(*process) {
                        edr_found = true;
                        findings.push(format!("{} is running", name));
                        evidence.push(format!("Process '{}' detected in task list", process));
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let linux_security = [
                ("CrowdStrike Falcon", "falcon-sensor"),
                ("Carbon Black", "cbagentd"),
                ("ClamAV", "clamd"),
                ("OSSEC", "ossec-agentd"),
                ("Wazuh", "wazuh-agentd"),
            ];

            if let Ok(output) = std::process::Command::new("ps").args(&["aux"]).output() {
                let process_list = String::from_utf8_lossy(&output.stdout).to_lowercase();

                for (name, process) in &linux_security {
                    if process_list.contains(process) {
                        edr_found = true;
                        findings.push(format!("{} is running", name));
                        evidence.push(format!("Process '{}' detected", process));
                    }
                }
            }
        }

        // INVERTED LOGIC: PASS if EDR is found, FAIL if not
        let status = if edr_found {
            AssessmentStatus::Pass
        } else {
            findings.push("No endpoint protection detected".to_string());
            AssessmentStatus::Fail
        };

        ControlAssessment {
            control_id: "SC-7".to_string(),
            control_name: "Endpoint Protection".to_string(),
            status,
            findings,
            evidence,
            framework_mappings: vec![
                FrameworkMapping {
                    framework: "NIST CSF".to_string(),
                    control_id: "PR.DS-1".to_string(),
                    control_name: "Data-at-rest is protected".to_string(),
                },
                FrameworkMapping {
                    framework: "ISO 27001".to_string(),
                    control_id: "A.8.6".to_string(),
                    control_name: "Management of technical vulnerabilities".to_string(),
                },
                FrameworkMapping {
                    framework: "CIS Controls".to_string(),
                    control_id: "10.1".to_string(),
                    control_name: "Deploy and Maintain Anti-Malware Software".to_string(),
                },
            ],
            checked_at: chrono::Utc::now(),
        }
    }

    /// Validate that host-based firewall is enabled
    pub async fn validate_firewall(&self) -> ControlAssessment {
        let mut findings = Vec::new();
        let mut evidence = Vec::new();
        let mut firewall_enabled = false;

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("netsh")
                .args(&["advfirewall", "show", "allprofiles", "state"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("ON") {
                    firewall_enabled = true;
                    findings.push("Windows Firewall is enabled".to_string());
                    evidence.push(output_str.to_string());
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check for iptables rules
            if let Ok(output) = std::process::Command::new("iptables")
                .args(&["-L", "-n"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.lines().count() > 3 {
                    firewall_enabled = true;
                    findings.push("iptables rules are configured".to_string());
                    evidence.push("iptables has active rules".to_string());
                }
            }

            // Check for ufw status
            if let Ok(output) = std::process::Command::new("ufw")
                .args(&["status"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("Status: active") {
                    firewall_enabled = true;
                    findings.push("UFW firewall is active".to_string());
                    evidence.push(output_str.to_string());
                }
            }
        }

        let status = if firewall_enabled {
            AssessmentStatus::Pass
        } else {
            findings.push("No active firewall detected".to_string());
            AssessmentStatus::Fail
        };

        ControlAssessment {
            control_id: "SC-7".to_string(),
            control_name: "Boundary Protection - Host Firewall".to_string(),
            status,
            findings,
            evidence,
            framework_mappings: vec![
                FrameworkMapping {
                    framework: "NIST 800-53".to_string(),
                    control_id: "SC-7".to_string(),
                    control_name: "Boundary Protection".to_string(),
                },
                FrameworkMapping {
                    framework: "CIS Controls".to_string(),
                    control_id: "13.1".to_string(),
                    control_name: "Maintain Firewall".to_string(),
                },
            ],
            checked_at: chrono::Utc::now(),
        }
    }

    /// Validate system logging is enabled
    pub async fn validate_logging(&self) -> ControlAssessment {
        let mut findings = Vec::new();
        let mut evidence = Vec::new();
        let mut logging_enabled = false;

        #[cfg(target_os = "windows")]
        {
            // Check Windows Event Log service
            if let Ok(output) = std::process::Command::new("sc")
                .args(&["query", "eventlog"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("RUNNING") {
                    logging_enabled = true;
                    findings.push("Windows Event Log service is running".to_string());
                    evidence.push("EventLog service state: RUNNING".to_string());
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check for rsyslog or systemd-journald
            if let Ok(output) = std::process::Command::new("systemctl")
                .args(&["is-active", "rsyslog"])
                .output()
            {
                if String::from_utf8_lossy(&output.stdout).trim() == "active" {
                    logging_enabled = true;
                    findings.push("rsyslog is active".to_string());
                }
            }

            if let Ok(output) = std::process::Command::new("systemctl")
                .args(&["is-active", "systemd-journald"])
                .output()
            {
                if String::from_utf8_lossy(&output.stdout).trim() == "active" {
                    logging_enabled = true;
                    findings.push("systemd-journald is active".to_string());
                }
            }

            // Check for auditd
            if let Ok(output) = std::process::Command::new("systemctl")
                .args(&["is-active", "auditd"])
                .output()
            {
                if String::from_utf8_lossy(&output.stdout).trim() == "active" {
                    findings.push("auditd is active".to_string());
                    evidence.push("Linux Audit Daemon is running".to_string());
                }
            }
        }

        let status = if logging_enabled {
            AssessmentStatus::Pass
        } else {
            findings.push("System logging may not be properly configured".to_string());
            AssessmentStatus::Fail
        };

        ControlAssessment {
            control_id: "AU-2".to_string(),
            control_name: "Audit Logging".to_string(),
            status,
            findings,
            evidence,
            framework_mappings: vec![
                FrameworkMapping {
                    framework: "NIST 800-53".to_string(),
                    control_id: "AU-2".to_string(),
                    control_name: "Auditable Events".to_string(),
                },
                FrameworkMapping {
                    framework: "CIS Controls".to_string(),
                    control_id: "8.2".to_string(),
                    control_name: "Collect Audit Logs".to_string(),
                },
            ],
            checked_at: chrono::Utc::now(),
        }
    }

    /// Validate disk encryption status
    pub async fn validate_disk_encryption(&self) -> ControlAssessment {
        let mut findings = Vec::new();
        let mut evidence = Vec::new();
        let mut encryption_enabled = false;

        #[cfg(target_os = "windows")]
        {
            // Check BitLocker status
            if let Ok(output) = std::process::Command::new("manage-bde")
                .args(&["-status"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("Protection On") || output_str.contains("Fully Encrypted") {
                    encryption_enabled = true;
                    findings.push("BitLocker encryption is enabled".to_string());
                    evidence.push("BitLocker status: Protection On".to_string());
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check for LUKS encrypted volumes
            if let Ok(output) = std::process::Command::new("lsblk")
                .args(&["-o", "NAME,TYPE"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("crypt") {
                    encryption_enabled = true;
                    findings.push("LUKS encryption detected".to_string());
                    evidence.push("Encrypted volume found via lsblk".to_string());
                }
            }
        }

        let status = if encryption_enabled {
            AssessmentStatus::Pass
        } else {
            findings.push("Disk encryption not detected".to_string());
            AssessmentStatus::Fail
        };

        ControlAssessment {
            control_id: "SC-28".to_string(),
            control_name: "Protection of Information at Rest".to_string(),
            status,
            findings,
            evidence,
            framework_mappings: vec![
                FrameworkMapping {
                    framework: "NIST 800-53".to_string(),
                    control_id: "SC-28".to_string(),
                    control_name: "Protection of Information at Rest".to_string(),
                },
                FrameworkMapping {
                    framework: "PCI DSS".to_string(),
                    control_id: "3.4".to_string(),
                    control_name: "Render PAN unreadable".to_string(),
                },
            ],
            checked_at: chrono::Utc::now(),
        }
    }

    /// Run all security validations
    pub async fn run_all_validations(&self) -> Vec<ControlAssessment> {
        vec![
            self.validate_endpoint_protection().await,
            self.validate_firewall().await,
            self.validate_logging().await,
            self.validate_disk_encryption().await,
        ]
    }

    /// Get overall security posture score
    pub fn calculate_security_score(assessments: &[ControlAssessment]) -> f64 {
        if assessments.is_empty() {
            return 0.0;
        }

        let passed = assessments
            .iter()
            .filter(|a| a.status == AssessmentStatus::Pass)
            .count();

        (passed as f64 / assessments.len() as f64) * 100.0
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_validator_creation() {
        let validator = SecurityValidator::new();
        assert!(!validator.edr_processes.is_empty());
    }

    #[test]
    fn test_control_assessment_status() {
        let assessment = ControlAssessment {
            control_id: "TEST-1".to_string(),
            control_name: "Test Control".to_string(),
            status: AssessmentStatus::Pass,
            findings: vec!["Test passed".to_string()],
            evidence: vec![],
            framework_mappings: vec![],
            checked_at: chrono::Utc::now(),
        };

        assert_eq!(assessment.status, AssessmentStatus::Pass);
    }

    #[test]
    fn test_security_score_calculation() {
        let assessments = vec![
            ControlAssessment {
                control_id: "1".to_string(),
                control_name: "Test 1".to_string(),
                status: AssessmentStatus::Pass,
                findings: vec![],
                evidence: vec![],
                framework_mappings: vec![],
                checked_at: chrono::Utc::now(),
            },
            ControlAssessment {
                control_id: "2".to_string(),
                control_name: "Test 2".to_string(),
                status: AssessmentStatus::Fail,
                findings: vec![],
                evidence: vec![],
                framework_mappings: vec![],
                checked_at: chrono::Utc::now(),
            },
        ];

        let score = SecurityValidator::calculate_security_score(&assessments);
        assert_eq!(score, 50.0);
    }
}
