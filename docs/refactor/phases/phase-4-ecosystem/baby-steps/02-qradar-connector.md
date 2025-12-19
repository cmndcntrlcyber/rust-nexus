# 🐾 Baby Step 4.2: QRadar Connector

> Integrate with IBM QRadar SIEM.

## 📋 Objective

Build a QRadar connector that forwards detection events using syslog/LEEF format for QRadar ingestion.

## ✅ Prerequisites

- [ ] Baby Step 4.1 complete (Splunk connector)
- [ ] QRadar instance available for testing
- [ ] Understand QRadar log source requirements

## 🔧 Implementation Steps

### Step 1: Define Connector Configuration

<!-- TODO: Add config structure -->

```toml
[siem.qradar]
enabled = true
syslog_host = "qradar.example.com"
syslog_port = 514
protocol = "tcp"  # or "udp"
format = "leef"   # or "cef"
log_source_identifier = "d3tect-nexus"
```

### Step 2: Create Connector Structure

<!-- TODO: Add connector implementation -->

```rust
pub struct QRadarConnector {
    config: QRadarConfig,
    socket: TcpStream,  // or UdpSocket
}

impl QRadarConnector {
    pub async fn send_event(&self, event: &DetectionEvent) -> Result<()> {
        let leef = self.format_leef(event);
        self.send_syslog(&leef).await
    }
}
```

### Step 3: Implement LEEF/CEF Formatting

<!-- TODO: Add event formatting -->

### Step 4: Add Connection Management

<!-- TODO: Add connection handling -->

### Step 5: Implement Health Check

<!-- TODO: Add health monitoring -->

## ✅ Verification Checklist

- [ ] Connector sends syslog messages
- [ ] LEEF/CEF format correct
- [ ] Events appear in QRadar
- [ ] Log source type recognized
- [ ] Connection recovery works
- [ ] Health check functional

## 📤 Expected Output

- Working QRadar connector
- Events visible in QRadar console
- Log source properly configured

## ➡️ Next Step

[03-sentinel-connector.md](03-sentinel-connector.md)

---
**Estimated Time**: 1 week
**Complexity**: Medium
**Assigned To**: Integration Agent
