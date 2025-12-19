# 🐾 Baby Step 4.1: Splunk Connector

> Integrate with Splunk via HTTP Event Collector (HEC).

## 📋 Objective

Build a Splunk connector that forwards detection events, alerts, and telemetry to Splunk via HEC for centralized log management and correlation.

## ✅ Prerequisites

- [ ] Phase 3 complete
- [ ] Splunk instance available for testing
- [ ] Understand Splunk HEC API

## 🔧 Implementation Steps

### Step 1: Define Connector Configuration

<!-- TODO: Add config structure -->

```toml
[siem.splunk]
enabled = true
hec_endpoint = "https://splunk.example.com:8088/services/collector"
hec_token_env = "SPLUNK_HEC_TOKEN"
index = "d3tect_nexus"
source = "d3tect-nexus"
sourcetype = "detection:event"
batch_size = 100
flush_interval_seconds = 5
```

### Step 2: Create Connector Structure

<!-- TODO: Add connector implementation -->

```rust
pub struct SplunkConnector {
    config: SplunkConfig,
    client: reqwest::Client,
    buffer: Vec<Event>,
}

impl SplunkConnector {
    pub async fn send_event(&self, event: &DetectionEvent) -> Result<()> {
        // Format for HEC
        // Send to Splunk
    }
}
```

### Step 3: Implement Event Formatting

<!-- TODO: Add event formatting for Splunk -->

### Step 4: Add Batching and Retry

<!-- TODO: Add batching logic -->

### Step 5: Implement Health Check

<!-- TODO: Add health monitoring -->

### Step 6: Add Metrics

<!-- TODO: Add connector metrics -->

## ✅ Verification Checklist

- [ ] Connector authenticates with Splunk
- [ ] Events formatted correctly
- [ ] Events appear in Splunk index
- [ ] Batching works correctly
- [ ] Retry on failure works
- [ ] Health check functional
- [ ] Metrics collected

## 📤 Expected Output

- Working Splunk connector
- Events searchable in Splunk
- Monitoring dashboard

## ➡️ Next Step

[02-qradar-connector.md](02-qradar-connector.md)

---
**Estimated Time**: 1 week
**Complexity**: Medium
**Assigned To**: Integration Agent
