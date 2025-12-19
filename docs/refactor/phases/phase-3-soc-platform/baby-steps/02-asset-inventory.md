# 🐾 Baby Step 3.2: Asset Inventory

> Build the asset inventory and monitoring dashboard.

## 📋 Objective

Create the asset inventory system that tracks all endpoints with EDR agents, their health status, and key metrics.

## ✅ Prerequisites

- [ ] Baby Step 3.1 complete (dashboard scaffold)
- [ ] Agents reporting telemetry (Phase 2)
- [ ] Understand asset tracking requirements

## 🔧 Implementation Steps

### Step 1: Define Asset Data Model

<!-- TODO: Add data model -->

```rust
pub struct Asset {
    pub id: Uuid,
    pub hostname: String,
    pub ip_address: String,
    pub os: OperatingSystem,
    pub agent_version: String,
    pub last_seen: DateTime<Utc>,
    pub health_status: HealthStatus,
    pub tags: Vec<String>,
}

pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Offline,
}
```

### Step 2: Create Asset Registry

<!-- TODO: Add registry implementation -->

### Step 3: Build Asset List View

<!-- TODO: Add list view component -->

### Step 4: Create Asset Detail View

<!-- TODO: Add detail view component -->

### Step 5: Add Health Monitoring

<!-- TODO: Add health monitoring -->

### Step 6: Implement Search/Filter

<!-- TODO: Add search functionality -->

## ✅ Verification Checklist

- [ ] Assets discovered and tracked
- [ ] Health status calculated correctly
- [ ] List view displays all assets
- [ ] Detail view shows full info
- [ ] Search/filter works
- [ ] Real-time updates work
- [ ] Offline detection works

## 📤 Expected Output

- Asset inventory page
- Asset detail view
- Health monitoring dashboard
- Search and filtering

## ➡️ Next Step

[03-response-actions.md](03-response-actions.md)

---
**Estimated Time**: 1-2 weeks
**Complexity**: Medium
**Assigned To**: SOC Platform Agent
