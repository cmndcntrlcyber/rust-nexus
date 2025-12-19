//! Network monitoring module
//!
//! Monitors network traffic for suspicious patterns, C2 beaconing,
//! data exfiltration, and lateral movement indicators.

use crate::types::{DetectionEvent, DetectionSource, NetworkContext, Severity};

/// Network traffic monitor
pub struct NetworkMonitor {
    /// Known malicious IPs/domains
    blocklist: Vec<String>,
    /// Suspicious port list
    suspicious_ports: Vec<u16>,
    /// Enable/disable flag
    enabled: bool,
}

/// Network connection event
#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    /// Source IP
    pub src_ip: String,
    /// Source port
    pub src_port: u16,
    /// Destination IP
    pub dst_ip: String,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: Protocol,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Process ID that created connection
    pub pid: Option<u32>,
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Other,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new() -> Self {
        Self {
            blocklist: Vec::new(),
            suspicious_ports: vec![
                4444,  // Metasploit default
                5555,  // Common RAT port
                8080,  // Alternative HTTP
                1337,  // Leet port
                31337, // Elite port
                6667,  // IRC
                6697,  // IRC SSL
            ],
            enabled: true,
        }
    }

    /// Add IP or domain to blocklist
    pub fn add_to_blocklist(&mut self, indicator: impl Into<String>) {
        self.blocklist.push(indicator.into());
    }

    /// Check if destination is suspicious
    pub fn is_suspicious(&self, event: &ConnectionEvent) -> bool {
        // Check blocklist
        if self.blocklist.contains(&event.dst_ip) {
            return true;
        }

        // Check suspicious ports
        if self.suspicious_ports.contains(&event.dst_port) {
            return true;
        }

        false
    }

    /// Analyze a connection event
    pub fn analyze(&self, event: &ConnectionEvent) -> Vec<DetectionEvent> {
        if !self.enabled {
            return Vec::new();
        }

        let mut detections = Vec::new();

        // Check for suspicious connection
        if self.is_suspicious(event) {
            let mut detection = DetectionEvent::new(
                DetectionSource::Network,
                Severity::Medium,
                "NET-001",
                format!(
                    "Suspicious network connection to {}:{}",
                    event.dst_ip, event.dst_port
                ),
            );

            detection.context.network = Some(NetworkContext {
                src_ip: Some(event.src_ip.clone()),
                src_port: Some(event.src_port),
                dst_ip: Some(event.dst_ip.clone()),
                dst_port: Some(event.dst_port),
                protocol: Some(format!("{:?}", event.protocol)),
                bytes: Some(event.bytes_sent + event.bytes_received),
            });

            detections.push(detection);
        }

        // Check for blocklisted destination
        if self.blocklist.contains(&event.dst_ip) {
            let detection = DetectionEvent::new(
                DetectionSource::Network,
                Severity::High,
                "NET-002",
                format!("Connection to blocklisted IP: {}", event.dst_ip),
            )
            .with_mitre("T1071");

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
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = NetworkMonitor::new();
        assert!(monitor.is_enabled());
        assert!(!monitor.suspicious_ports.is_empty());
    }

    #[test]
    fn test_suspicious_port() {
        let monitor = NetworkMonitor::new();
        let event = ConnectionEvent {
            src_ip: "192.168.1.100".to_string(),
            src_port: 54321,
            dst_ip: "10.0.0.1".to_string(),
            dst_port: 4444,
            protocol: Protocol::Tcp,
            bytes_sent: 1024,
            bytes_received: 2048,
            pid: Some(1234),
        };

        assert!(monitor.is_suspicious(&event));
    }

    #[test]
    fn test_blocklist() {
        let mut monitor = NetworkMonitor::new();
        monitor.add_to_blocklist("evil.com");

        let event = ConnectionEvent {
            src_ip: "192.168.1.100".to_string(),
            src_port: 54321,
            dst_ip: "evil.com".to_string(),
            dst_port: 443,
            protocol: Protocol::Tcp,
            bytes_sent: 1024,
            bytes_received: 2048,
            pid: Some(1234),
        };

        let detections = monitor.analyze(&event);
        assert!(!detections.is_empty());
    }
}
