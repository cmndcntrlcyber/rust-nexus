# 🗺️ Component Mapping

> Mapping C2 components to SOC equivalents.

## 📋 Overview

This document maps each C2 component to its transformed SOC equivalent.

## 🔄 Crate Transformations

| Original | Original Purpose | New Purpose | New Name |
|----------|------------------|-------------|----------|
| nexus-agent | Offensive agent | EDR agent | nexus-edr-agent |
| nexus-infra | C2 infrastructure | Detection infrastructure | nexus-infra (modified) |
| - | - | Detection capabilities | nexus-detection (new) |

## 📁 File-Level Mapping

### nexus-agent

| Original File | Original Purpose | New Purpose | New Location |
|---------------|------------------|-------------|--------------|
| `agent.rs` | C2 agent core | EDR agent core | `edr_agent.rs` |
| `execution.rs` | Execute payloads | Execute responses | `response.rs` |
| `evasion.rs` | Avoid detection | Detect evasion | `nexus-detection/evasion_detection.rs` |
| `communication.rs` | C2 comms | SOC telemetry | `telemetry.rs` |

### nexus-infra

| Original File | Original Purpose | New Purpose | Changes |
|---------------|------------------|-------------|---------|
| `grpc_server.rs` | C2 tasking | SOC telemetry | Modified protocol |
| `domain_manager.rs` | Domain rotation | LitterBox DNS | Extended |
| `cert_manager.rs` | C2 certs | Service certs | Minimal changes |
| `bof_loader.rs` | BOF execution | Threat hunting | Repurposed |

## 🔧 Code-Level Examples

### Example 1: Evasion → Detection

**Before** (`nexus-agent/src/evasion.rs`):
```rust
pub fn detect_vm() -> bool {
    // Check if running in VM to evade analysis
}
```

**After** (`nexus-detection/src/evasion_detection.rs`):
```rust
pub fn detect_vm_evasion_attempt(process: &ProcessInfo) -> DetectionResult {
    // Detect if target process is checking for VMs
}
```

### Example 2: Execution → Response

<!-- TODO: Add more examples -->

## 📝 Migration Notes

<!-- TODO: Add migration notes per component -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Architecture Lead
