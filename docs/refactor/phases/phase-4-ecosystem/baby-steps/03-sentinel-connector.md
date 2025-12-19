# 🐾 Baby Step 4.3: Microsoft Sentinel Connector

> Integrate with Microsoft Sentinel via Log Analytics.

## 📋 Objective

Build a Microsoft Sentinel connector that forwards detection events via the Azure Log Analytics Data Collector API.

## ✅ Prerequisites

- [ ] Baby Step 4.2 complete (QRadar connector)
- [ ] Azure subscription with Sentinel
- [ ] Log Analytics workspace configured

## 🔧 Implementation Steps

### Step 1: Define Connector Configuration

<!-- TODO: Add config structure -->

```toml
[siem.sentinel]
enabled = true
workspace_id_env = "SENTINEL_WORKSPACE_ID"
shared_key_env = "SENTINEL_SHARED_KEY"
log_type = "D3tectNexus"
azure_resource_id = "/subscriptions/.../resourceGroups/.../..."
```

### Step 2: Create Connector Structure

<!-- TODO: Add connector implementation -->

```rust
pub struct SentinelConnector {
    config: SentinelConfig,
    client: reqwest::Client,
}

impl SentinelConnector {
    pub async fn send_event(&self, event: &DetectionEvent) -> Result<()> {
        let json = self.format_log_analytics(event);
        self.send_to_log_analytics(&json).await
    }

    fn build_signature(&self, date: &str, content_length: usize) -> String {
        // Build HMAC-SHA256 signature for Azure
    }
}
```

### Step 3: Implement Azure Authentication

<!-- TODO: Add Azure auth -->

### Step 4: Add Event Formatting

<!-- TODO: Add Log Analytics formatting -->

### Step 5: Implement Health Check

<!-- TODO: Add health monitoring -->

## ✅ Verification Checklist

- [ ] Connector authenticates with Azure
- [ ] Events sent to Log Analytics
- [ ] Custom log type created
- [ ] Events queryable in Sentinel
- [ ] Batching works correctly
- [ ] Health check functional

## 📤 Expected Output

- Working Sentinel connector
- Events in Log Analytics
- Queryable in Sentinel workbooks

## ➡️ Next Step

[04-threat-feed-ingestion.md](04-threat-feed-ingestion.md)

---
**Estimated Time**: 1 week
**Complexity**: Medium
**Assigned To**: Integration Agent
