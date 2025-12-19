# 🐾 Baby Step 2.2: Agent Detection Mode

> Add detection mode to agents for EDR-style operation.

## 📋 Objective

Transform `nexus-agent` to support a detection mode where it collects telemetry and runs detection logic instead of executing offensive tasks.

## ✅ Prerequisites

- [ ] Baby Step 2.1 complete (gRPC protocol)
- [ ] nexus-detection crate available (Phase 1)
- [ ] Understand agent architecture

## 🔧 Implementation Steps

### Step 1: Add Detection Mode Configuration

<!-- TODO: Add config changes -->

```toml
[agent]
mode = "detection"  # or "legacy" for backward compat

[detection]
enabled = true
telemetry_interval = 30
```

### Step 2: Create Detection Agent Module

<!-- TODO: Add module structure -->

```rust
// nexus-agent/src/detection_mode.rs
pub struct DetectionAgent {
    config: DetectionConfig,
    detector: nexus_detection::Detector,
}

impl DetectionAgent {
    pub async fn run(&self) -> Result<()> {
        // Collection loop
        // Detection logic
        // Telemetry submission
    }
}
```

### Step 3: Integrate with nexus-detection

<!-- TODO: Add integration -->

### Step 4: Implement Telemetry Collection

<!-- TODO: Add telemetry collection -->

### Step 5: Add Mode Switching

<!-- TODO: Add mode switching logic -->

## ✅ Verification Checklist

- [ ] Agent starts in detection mode
- [ ] Telemetry collected and submitted
- [ ] Detection events generated
- [ ] Mode switching works
- [ ] Legacy mode still functional
- [ ] Unit tests pass

## 📤 Expected Output

- `detection_mode.rs` module
- Agent supports detection configuration
- Telemetry flows to server

## ➡️ Next Step

[03-behavioral-analysis.md](03-behavioral-analysis.md)

---
**Estimated Time**: 2 weeks
**Complexity**: High
**Assigned To**: Detection Engine Agent
