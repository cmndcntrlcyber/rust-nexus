# 🔄 Before/After Code Examples

> Concrete code transformation examples.

## 📋 Overview

This document shows actual code transformations from C2 to SOC.

## Example 1: gRPC Task Handler

### Before (C2 Tasking)

```rust
// nexus-infra/src/grpc_server.rs
async fn get_tasks(&self, request: Request<TaskRequest>) -> Result<Response<TaskStream>> {
    let agent_id = request.into_inner().agent_id;
    let tasks = self.task_queue.get_pending_tasks(&agent_id).await?;
    // Return offensive tasks (shell commands, file ops, etc.)
    Ok(Response::new(TaskStream::new(tasks)))
}
```

### After (SOC Telemetry)

```rust
// nexus-infra/src/detection_server.rs
async fn get_detection_tasks(&self, request: Request<DetectionRequest>) -> Result<Response<DetectionStream>> {
    let agent_id = request.into_inner().agent_id;
    let tasks = self.detection_queue.get_pending_scans(&agent_id).await?;
    // Return detection tasks (scan requests, IOC checks, etc.)
    Ok(Response::new(DetectionStream::new(tasks)))
}
```

### Migration Notes
<!-- TODO: Add migration notes -->

---

## Example 2: Agent Execution

### Before (Payload Execution)

```rust
// nexus-agent/src/execution.rs
pub async fn execute_shellcode(&self, shellcode: &[u8]) -> ExecutionResult {
    // Execute arbitrary shellcode on target
}
```

### After (Response Execution)

```rust
// nexus-edr-agent/src/response.rs
pub async fn execute_response(&self, response: &ResponseAction) -> ResponseResult {
    match response {
        ResponseAction::IsolateHost => self.isolate_network().await,
        ResponseAction::KillProcess(pid) => self.terminate_process(pid).await,
        ResponseAction::QuarantineFile(path) => self.quarantine(path).await,
    }
}
```

---

## Example 3: Evasion Detection

<!-- TODO: Add more examples -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: All Agents (collaborative)
