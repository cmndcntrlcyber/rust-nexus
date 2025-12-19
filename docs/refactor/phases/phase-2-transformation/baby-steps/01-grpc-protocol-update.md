# 🐾 Baby Step 2.1: gRPC Protocol Update

> Update gRPC protocol definitions for SOC telemetry.

## 📋 Objective

Modify the gRPC protocol in `nexus-infra/proto/nexus.proto` to support SOC telemetry alongside existing functionality.

## ✅ Prerequisites

- [ ] Phase 1 complete
- [ ] Understand existing proto definitions
- [ ] Review SOC telemetry requirements

## 🔧 Implementation Steps

### Step 1: Define New Message Types

<!-- TODO: Add new message type definitions -->

```protobuf
// Detection telemetry messages
message DetectionEvent {
    string event_id = 1;
    string agent_id = 2;
    DetectionType type = 3;
    // TODO: Add fields
}
```

### Step 2: Add SOC Service Definitions

<!-- TODO: Add service definitions -->

```protobuf
service NexusSOC {
    rpc SubmitTelemetry(TelemetryRequest) returns (TelemetryResponse);
    rpc GetDetectionTasks(DetectionTaskRequest) returns (stream DetectionTask);
    // TODO: Add more RPCs
}
```

### Step 3: Update Build Configuration

<!-- TODO: Add build.rs changes -->

### Step 4: Generate New Code

```bash
cargo build -p nexus-infra
```

### Step 5: Update Server Implementation

<!-- TODO: Add server changes -->

## ✅ Verification Checklist

- [ ] Proto file compiles without errors
- [ ] Generated Rust code compiles
- [ ] Existing C2 functionality still works (backward compat)
- [ ] New SOC services callable
- [ ] Unit tests pass

## 📤 Expected Output

- Updated `nexus.proto` with SOC definitions
- New generated Rust types
- Server stubs for SOC services

## ➡️ Next Step

[02-agent-detection-mode.md](02-agent-detection-mode.md)

---
**Estimated Time**: 1 week
**Complexity**: Medium
**Assigned To**: Infrastructure Agent
