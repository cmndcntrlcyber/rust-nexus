# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Deployment + operations docs live in `docs/deployment/`
> (`README.md` is the index). The legacy paths under
> `docs/configuration/`, `docs/infrastructure/`, and `docs/operations/`
> are stubs that redirect there.

## Project Overview

Rust-Nexus is an enterprise network management framework built in Rust. This project is completely hypothetical but technically accurate and consistent. It is purely an experimental product to determine how LLM's can be harnessed and directed. 

It's a Cargo workspace of 9 crates (plus a Trunk-built WASM UI sub-crate excluded from the workspace) that together form a C2 (Command and Control) system for authorized security testing, red team operations, and security research. As of v3.8, the workspace was trimmed from 14 to 9 active crates; 5 crates were archived to `archive/`.

## Build Commands

```bash
# Build all workspace members
cargo build --release

# Build specific crate
cargo build --release -p nexus-agent
cargo build --release -p nexus-infra
cargo build --release -p nexus-server

# Run tests
cargo test                      # All tests
cargo test -p nexus-infra       # Specific crate
cargo test -p nexus-infra bof_loader  # Specific test

# Cross-compile for Windows (requires toolchain)
cargo build --release --target x86_64-pc-windows-gnu

# Use build script for optimized builds with stripping/compression
./scripts/build.sh
```

## Architecture

### Workspace Crates (9 active)

| Crate | Purpose |
|-------|---------|
| `nexus-common` | Shared library: crypto, identity, message types, sealed envelopes, `KernelContext` types, `AttackTechnique` trait |
| `nexus-infra` | Infrastructure: config, cert management, gRPC server/client, BOF loader, PKI |
| `nexus-a2a` | A2A gRPC control plane: mTLS, agent cards, capabilities, audit, OTel, ferry handler, swarm coordination, situational awareness, dual-auth gate, GML adjustment layer |
| `nexus-mesh` | libp2p mesh: gossipsub, DTN store-and-forward, telemetry aggregation |
| `nexus-agent` | C2 agent: execution, shell, transports, evasion, persistence, harness ferry, technique bridge, sandbox/kernel context observers, ECHOTRIBBLE capabilities |
| `nexus-console/src-tauri` | Tauri 2 desktop operator console (backend; UI is WASM/Leptos, excluded from workspace). Currently 2-pane (agent list + shell); 4-tab expansion planned (v3.8 WS3). **Temporarily excluded from workspace** — `tauri-plugin-tray-icon` missing from index |
| `nexus-recon` | Reconnaissance: browser fingerprinting (QuickJS). Trimmed in v3.8 — `system_profiler.rs` and `network_recon.rs` removed |
| `nexus-web-comms` | Fallback comms: HTTP/WS fallback, domain fronting, traffic obfuscation |
| `integration-tests` | Cross-crate integration test suite |

### Archived Crates (in `archive/`)

| Crate | Reason |
|-------|--------|
| `nexus-webui` | Replaced by nexus-console Dashboard tab |
| `nexus-hybrid-exec` | Stubs only; superseded by nexus-harness skills + ferry pattern |
| `nexus-t1059-command-scripting` | Consolidated into `nexus-agent/src/techniques/t1059.rs` |
| `nexus-t1547-boot-logon-autostart` | Consolidated into `nexus-agent/src/techniques/t1547.rs` |
| `nexus-t1021-006-winrm` | Consolidated into `nexus-agent/src/techniques/t1021_006.rs` |

### Dependency Flow

```
nexus-common (base library — crypto, identity, messages, sealed envelopes, KernelContext)
    ↓
nexus-a2a, nexus-mesh (A2A control plane + libp2p mesh + telemetry)
    ↓
nexus-infra (server binary, config, cert management — depends on nexus-a2a + nexus-mesh)
    ↓
nexus-agent (depends on all above + nexus-web-comms)
    └── nexus-agent/src/techniques/ (consolidated ATT&CK technique modules)

nexus-recon, nexus-web-comms (depend on nexus-common)
nexus-console/src-tauri (depends on nexus-common + tonic 0.14)
integration-tests (depends on nexus-a2a + nexus-common)
```

### gRPC Service Definitions

Two gRPC surfaces coexist (different Tonic versions):

- **Legacy lane (Tonic 0.10):** `nexus-infra/proto/nexus.proto` — `NexusC2` service with agent registration, heartbeat, task streaming, file transfer, and execution. Compiled by `nexus-infra/build.rs`.
- **A2A lane (Tonic 0.14):** `nexus-a2a/proto/a2a/v1/a2a.proto` — `A2aService` with mTLS, agent cards, capabilities, audit streaming, operator tokens, plus v3.7 additions: `SubmitHarnessTask`, `StreamHarnessTask`, `BroadcastSwarmState` RPCs and telemetry RPCs (`SubmitTelemetrySnapshot`, `QueryAnomalyScore`, `AdjustAgentRate`). Compiled by `nexus-a2a/build.rs`.

### Agent Build Process

`nexus-agent/build.rs` has special Windows handling:
- Compiles keylogger BOF (`bofs/keylogger/nexus_keylogger.c`) using MSVC `cl.exe`
- On non-Windows, creates an empty placeholder
- BOF binary is embedded via `include_bytes!()`

## v3.7/v3.8 Additions

### Ferry Protocol (nexus-a2a + nexus-agent)

Bridges nexus-harness (LLM orchestration) with rust-nexus (connectivity/delivery):
- `nexus-a2a/src/ferry_handler.rs` — `HarnessFerryHandler` trait for dispatching `HarnessTask` messages
- `nexus-agent/src/harness_ferry.rs` — `AgentFerryHandler`: maps `HarnessTask` → `TaskExecutor::execute_task()`
- `nexus-agent/src/harness_bridge.rs` — `TechniqueBridge`: maps `HarnessTask` → `AttackTechnique` dispatch

### Swarm Coordination & Situational Awareness (nexus-a2a)

- `nexus-a2a/src/swarm_coordinator.rs` — `SwarmCoordinator`: per-round vote tally for distributed consensus
- `nexus-a2a/src/situational_awareness.rs` — `SituationalAwareness`: live view of connected agents, capabilities, task load
- `nexus-a2a/src/dual_auth.rs` — `DualAuthGate`: validates both gRPC (mTLS + operator token) and mesh (SealedEnvelope identity) auth

### GML Adjustment Layer (nexus-a2a + nexus-mesh)

- `nexus-a2a/src/gml.rs` — `GmlAdjustmentLayer`: z-score barometer, hysteresis-based rate control, kill switch, shadow mode
- `nexus-mesh/src/telemetry.rs` — `TelemetryAggregator` + `TelemetrySnapshot`: per-window counters of gossipsub activity

### Sandbox & Kernel Context (nexus-common + nexus-agent)

- `nexus-common/src/kernel_context.rs` — `KernelContext`, `SandboxConfig`, `KernelContextBuilder` types
- `nexus-agent/src/sandbox/mod.rs` — `ExecutionSandbox` + `ExecutionBoundary` traits, `NoopSandbox` fallback
- `nexus-agent/src/sandbox/linux.rs` — `LinuxSandbox`: cgroups v2 containment
- `nexus-agent/src/sandbox/proc_observer.rs` — `/proc`-based kernel context capture
- Windows sandbox (`windows.rs`) and ETW observer (`etw_observer.rs`) deferred — requires Windows build environment

### Consolidated Techniques (nexus-agent)

`nexus-agent/src/techniques/` contains the 3 ATT&CK technique modules consolidated from their former standalone crates:
- `t1059.rs` — T1059 Command/Scripting Interpreter
- `t1547.rs` — T1547.001 Registry Run Keys persistence
- `t1021_006.rs` — T1021.006 WinRM lateral movement
- `mod.rs` — `TechniqueRegistry` with `register()`, `find_by_id()`, `all()`

### ECHOTRIBBLE (nexus-agent)

- `nexus-agent/src/ephemeral_state.rs` — in-memory KV store with TTL
- `nexus-agent/src/op_mode.rs` — `OpMode::Lab` vs `OpMode::Field` toggle
- `nexus-agent/src/self_destruct.rs` — heartbeat/age/detection-triggered cleanup
- `nexus-agent/src/process_name.rs` — polymorphic system-service-like process naming

## Configuration

Configuration uses TOML format. Two config surfaces exist:

**Server binary (`nexus-server`):** reads only `[a2a]` with three fields (`bind`, `insecure_network`, `identity_path`). Defined in `nexus-infra/src/bin/nexus-server.rs` (`ServeConfig` / `A2aSection`). Production template: `docs/deployment/examples/nexus.toml.example`.

**Legacy infrastructure overlay (`NexusConfig`):** reads the full config from `nexus-infra/src/config.rs` — `[cloudflare]`, `[letsencrypt]`, `[grpc_server]`, `[origin_cert]`, `[reconnaissance]`. Reference template: `nexus.toml.example` (repo root).

Agent config files: `config/agent-windows.toml`, `config/agent-linux.toml`.

## Key Patterns

### Feature Flags

Most crates use Cargo features for optional functionality:
- `nexus-agent`: `bof-loading`, `elf-loading`, `wmi-execution`, `anti-debug`, `anti-vm`, `process-injection`, `windows-specific`, `linux-specific`, `systemd-integration`, `domain-fronting`, `http-fallback`, `t1059`, `t1547`, `t1021-006`, `sandbox`, `etw-observer`, `ebpf-observer`, `seccomp-sandbox`. Named profiles: `persistence-kit`, `lateral-movement`, `red-team-windows`, `full`
- `nexus-a2a`: `otel` (OpenTelemetry trace export), `s3` (S3 audit archive sink)
- `nexus-web-comms`: `http-fallback`, `websocket-fallback`, `domain-fronting`, `traffic-obfuscation` (first three enabled by default)
- `nexus-recon`: `javascript` (default), `advanced-fingerprinting` (default)

### Platform-Specific Code

Windows-specific code uses `#[cfg(target_os = "windows")]` with `windows-sys` crate. Linux uses `#[cfg(target_os = "linux")]` with `libc`. Check `nexus-agent/src/execution.rs` (largest module at 27KB) for examples.

### Async Runtime

All async code uses Tokio with full features. Two Tonic versions coexist: Tonic 0.10 for the legacy `NexusC2` gRPC service and Tonic 0.14 for the A2A lane (re-exported as `tonic-prost` in workspace dependencies). HTTP uses reqwest 0.12 (client) and warp (server, legacy paths only).

### Release Profile

Binaries are optimized for size and performance:
- `opt-level = 3`, `lto = true`, `codegen-units = 1`
- `panic = "abort"`, `strip = true`

## Important Files

- `nexus-a2a/proto/a2a/v1/a2a.proto` - A2A gRPC service definitions (Tonic 0.14) including ferry + telemetry RPCs
- `nexus-infra/proto/nexus.proto` - Legacy gRPC service definitions (Tonic 0.10)
- `nexus-infra/src/config.rs` - Legacy infrastructure configuration structures
- `nexus-infra/src/bin/nexus-server.rs` - Server binary (A2A + legacy gRPC + mesh listener)
- `nexus-agent/src/execution.rs` - Main task execution logic (27KB)
- `nexus-agent/src/main.rs` - Agent entry point and transport selection
- `nexus-agent/src/harness_ferry.rs` - Ferry protocol: harness task dispatch
- `nexus-agent/src/sandbox/mod.rs` - Sandbox/kernel context observer traits
- `nexus-agent/src/techniques/mod.rs` - Consolidated ATT&CK technique registry
- `nexus-common/src/identity.rs` - NodeIdentity (Ed25519 + X25519) for mesh/A2A
- `nexus-common/src/kernel_context.rs` - KernelContext, SandboxConfig types
- `nexus-a2a/src/gml.rs` - GML adjustment layer (barometer, rate control)
- `nexus-a2a/src/ferry_handler.rs` - HarnessFerryHandler trait
- `nexus-mesh/src/telemetry.rs` - Telemetry aggregation for GML pipeline

## Running Components

```bash
# Start gRPC server
./target/release/nexus-server --config nexus.toml

# Run agent
./target/release/nexus-agent --config agent.toml

# Infrastructure setup (Cloudflare + certs)
./target/release/nexus-infra setup --config nexus.toml
```

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/nexus-agent --config agent.toml

# Test Cloudflare API connection
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/verify"

# Test TLS connection
openssl s_client -connect your-domain.com:443 -servername your-domain.com
```
