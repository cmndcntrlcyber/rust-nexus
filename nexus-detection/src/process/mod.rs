//! Process monitoring module
//!
//! Monitors process creation, termination, and suspicious process
//! behaviors like injection, hollowing, and privilege escalation.

use crate::types::{DetectionEvent, DetectionSource, ProcessContext, Severity};

/// Process activity monitor
pub struct ProcessMonitor {
    /// Suspicious process names
    suspicious_processes: Vec<String>,
    /// Known LOLBins (Living Off the Land Binaries)
    lolbins: Vec<String>,
    /// Enable/disable flag
    enabled: bool,
}

/// Process creation event
#[derive(Debug, Clone)]
pub struct ProcessEvent {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub parent_pid: u32,
    /// Process name
    pub name: String,
    /// Executable path
    pub path: String,
    /// Command line
    pub command_line: String,
    /// User running the process
    pub user: String,
    /// Event type
    pub event_type: ProcessEventType,
}

/// Type of process event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessEventType {
    /// Process was created
    Created,
    /// Process terminated
    Terminated,
    /// Process accessed another process memory
    MemoryAccess,
    /// Process injected into another
    Injection,
    /// Process loaded module
    ModuleLoad,
}

impl ProcessMonitor {
    /// Create a new process monitor
    pub fn new() -> Self {
        Self {
            suspicious_processes: vec![
                "mimikatz".to_string(),
                "procdump".to_string(),
                "psexec".to_string(),
                "wmic".to_string(),
            ],
            lolbins: vec![
                "mshta.exe".to_string(),
                "certutil.exe".to_string(),
                "bitsadmin.exe".to_string(),
                "regsvr32.exe".to_string(),
                "rundll32.exe".to_string(),
                "msiexec.exe".to_string(),
                "cscript.exe".to_string(),
                "wscript.exe".to_string(),
            ],
            enabled: true,
        }
    }

    /// Check if process name is suspicious
    pub fn is_suspicious_name(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.suspicious_processes
            .iter()
            .any(|p| name_lower.contains(&p.to_lowercase()))
    }

    /// Check if process is a LOLBin
    pub fn is_lolbin(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.lolbins
            .iter()
            .any(|l| name_lower.ends_with(&l.to_lowercase()))
    }

    /// Analyze a process event
    pub fn analyze(&self, event: &ProcessEvent) -> Vec<DetectionEvent> {
        if !self.enabled {
            return Vec::new();
        }

        let mut detections = Vec::new();

        // Check for suspicious process name
        if self.is_suspicious_name(&event.name) {
            let mut detection = DetectionEvent::new(
                DetectionSource::Process,
                Severity::High,
                "PROC-001",
                format!("Suspicious process detected: {}", event.name),
            )
            .with_mitre("T1003");

            detection.context.process = Some(ProcessContext {
                pid: event.pid,
                name: event.name.clone(),
                path: Some(event.path.clone()),
                command_line: Some(event.command_line.clone()),
                parent_pid: Some(event.parent_pid),
                user: Some(event.user.clone()),
            });

            detections.push(detection);
        }

        // Check for LOLBin usage
        if self.is_lolbin(&event.name) {
            let detection = DetectionEvent::new(
                DetectionSource::Process,
                Severity::Medium,
                "PROC-002",
                format!("LOLBin execution detected: {}", event.name),
            )
            .with_mitre("T1218");

            detections.push(detection);
        }

        // Check for process injection
        if event.event_type == ProcessEventType::Injection {
            let detection = DetectionEvent::new(
                DetectionSource::Process,
                Severity::Critical,
                "PROC-003",
                format!("Process injection detected from PID {}", event.pid),
            )
            .with_mitre("T1055");

            detections.push(detection);
        }

        detections
    }

    /// Check if monitor is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the monitor
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the monitor
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Add a suspicious process name
    pub fn add_suspicious_process(&mut self, name: impl Into<String>) {
        self.suspicious_processes.push(name.into());
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = ProcessMonitor::new();
        assert!(monitor.is_enabled());
    }

    #[test]
    fn test_suspicious_name() {
        let monitor = ProcessMonitor::new();
        assert!(monitor.is_suspicious_name("mimikatz.exe"));
        assert!(monitor.is_suspicious_name("MIMIKATZ"));
        assert!(!monitor.is_suspicious_name("notepad.exe"));
    }

    #[test]
    fn test_lolbin_detection() {
        let monitor = ProcessMonitor::new();
        assert!(monitor.is_lolbin("mshta.exe"));
        assert!(monitor.is_lolbin("C:\\Windows\\System32\\certutil.exe"));
        assert!(!monitor.is_lolbin("notepad.exe"));
    }

    #[test]
    fn test_process_event_analysis() {
        let monitor = ProcessMonitor::new();
        let event = ProcessEvent {
            pid: 1234,
            parent_pid: 5678,
            name: "mimikatz.exe".to_string(),
            path: "C:\\temp\\mimikatz.exe".to_string(),
            command_line: "mimikatz.exe sekurlsa::logonpasswords".to_string(),
            user: "SYSTEM".to_string(),
            event_type: ProcessEventType::Created,
        };

        let detections = monitor.analyze(&event);
        assert!(!detections.is_empty());
        assert_eq!(detections[0].severity, Severity::High);
    }
}
