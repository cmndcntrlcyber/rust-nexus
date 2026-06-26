# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Deployment + operations docs live in `docs/deployment/`
> (`README.md` is the index). The legacy paths under
> `docs/configuration/`, `docs/infrastructure/`, and `docs/operations/`
> are stubs that redirect there.

## Project Overview

Rust-Nexus is an enterprise network management framework built in Rust. This project is completely hypothetical but technically accurate and consistent. It is purely an experimental product to determine how LLM's can be harnessed and directed. 

It's a Cargo workspace of 14 crates (plus a Trunk-built WASM UI sub-crate excluded from the workspace) that together form a C2 (Command and Control) system for authorized security testing, red team operations, and security research.

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

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `nexus-common` | Shared library: crypto, identity, message types, sealed envelopes |
| `nexus-infra` | Infrastructure: config, cert management, gRPC server/client, BOF loader, PKI |
| `nexus-a2a` | A2A gRPC control plane: mTLS, agent cards, capabilities, audit, OTel |
| `nexus-mesh` | libp2p mesh: gossipsub, DTN store-and-forward |
| `nexus-agent` | C2 agent: execution, shell, transports, evasion, persistence |
| `nexus-console/src-tauri` | Tauri 2 desktop operator console (backend; UI is WASM/Leptos, excluded from workspace) |
| `nexus-webui` | Web dashboard: warp HTTP server, WebSocket real-time updates |
| `nexus-recon` | Reconnaissance: browser fingerprinting, system profiling |
| `nexus-hybrid-exec` | Multi-protocol execution: SSH, WMI, PowerShell, HTTP API |
| `nexus-web-comms` | Fallback comms: HTTP/WS fallback, domain fronting, traffic obfuscation |
| `nexus-t1059-command-scripting` | ATT&CK T1059 Command and Scripting Interpreter |
| `nexus-t1547-boot-logon-autostart` | ATT&CK T1547 Boot/Logon Autostart Execution |
| `nexus-t1021-006-winrm` | ATT&CK T1021.006 Windows Remote Management |
| `integration-tests` | Cross-crate integration test suite |

### Dependency Flow

```
nexus-common (base library — crypto, identity, messages, sealed envelopes)
    ↓
nexus-a2a, nexus-mesh (A2A control plane + libp2p mesh)
    ↓
nexus-infra (server binary, config, cert management — depends on nexus-a2a + nexus-mesh)
    ↓
nexus-agent (depends on all above + nexus-web-comms)
    ├── nexus-t1059-*  (optional, feature-gated)
    ├── nexus-t1547-*  (optional, feature-gated)
    └── nexus-t1021-006-*  (optional, feature-gated)

nexus-webui, nexus-recon, nexus-hybrid-exec, nexus-web-comms (depend on nexus-common)
nexus-console/src-tauri (depends on nexus-common + tonic 0.14)
integration-tests (depends on nexus-a2a + nexus-common)
```

### gRPC Service Definitions

Two gRPC surfaces coexist (different Tonic versions):

- **Legacy lane (Tonic 0.10):** `nexus-infra/proto/nexus.proto` — `NexusC2` service with agent registration, heartbeat, task streaming, file transfer, and execution. Compiled by `nexus-infra/build.rs`.
- **A2A lane (Tonic 0.14):** `nexus-a2a/proto/a2a/v1/a2a.proto` — `A2aService` with mTLS, agent cards, capabilities, audit streaming, operator tokens. Compiled by `nexus-a2a/build.rs`.

### Agent Build Process

`nexus-agent/build.rs` has special Windows handling:
- Compiles keylogger BOF (`bofs/keylogger/nexus_keylogger.c`) using MSVC `cl.exe`
- On non-Windows, creates an empty placeholder
- BOF binary is embedded via `include_bytes!()`

## Configuration

Configuration uses TOML format. Two config surfaces exist:

**Server binary (`nexus-server`):** reads only `[a2a]` with three fields (`bind`, `insecure_network`, `identity_path`). Defined in `nexus-infra/src/bin/nexus-server.rs` (`ServeConfig` / `A2aSection`). Production template: `docs/deployment/examples/nexus.toml.example`.

**Legacy infrastructure overlay (`NexusConfig`):** reads the full config from `nexus-infra/src/config.rs` — `[cloudflare]`, `[letsencrypt]`, `[grpc_server]`, `[origin_cert]`, `[webui]`, `[reconnaissance]`, `[hybrid_exec]`. Reference template: `nexus.toml.example` (repo root).

Agent config files: `config/agent-windows.toml`, `config/agent-linux.toml`.

## Key Patterns

### Feature Flags

Most crates use Cargo features for optional functionality:
- `nexus-agent`: `bof-loading`, `elf-loading`, `wmi-execution`, `anti-debug`, `anti-vm`, `process-injection`, `windows-specific`, `linux-specific`, `systemd-integration`, `domain-fronting`, `http-fallback`, `t1059`, `t1547`, `t1021-006`. Named profiles: `persistence-kit`, `lateral-movement`, `red-team-windows`, `full`
- `nexus-a2a`: `otel` (OpenTelemetry trace export), `s3` (S3 audit archive sink)
- `nexus-hybrid-exec`: `ssh`, `wmi`, `api`, `powershell`, `cross-platform`
- `nexus-web-comms`: `http-fallback`, `websocket-fallback`, `domain-fronting`, `traffic-obfuscation` (first three enabled by default)
- `nexus-webui`: `full` (default), `websockets`, `templates`, `static-files`
- `nexus-recon`: `javascript` (default), `advanced-fingerprinting` (default), `network-recon`

### Platform-Specific Code

Windows-specific code uses `#[cfg(target_os = "windows")]` with `windows-sys` crate. Linux uses `#[cfg(target_os = "linux")]` with `libc`. Check `nexus-agent/src/execution.rs` (largest module at 27KB) for examples.

### Async Runtime

All async code uses Tokio with full features. gRPC uses Tonic 0.10 (legacy `NexusC2` lane) and Tonic 0.14 (A2A lane, aliased as `tonic_14` in crates that need both). HTTP uses reqwest (client) and warp (server).

### Release Profile

Binaries are optimized for size and performance:
- `opt-level = 3`, `lto = true`, `codegen-units = 1`
- `panic = "abort"`, `strip = true`

## Important Files

- `nexus-infra/proto/nexus.proto` - Legacy gRPC service definitions (Tonic 0.10)
- `nexus-a2a/proto/a2a/v1/a2a.proto` - A2A gRPC service definitions (Tonic 0.14)
- `nexus-infra/src/config.rs` - Legacy infrastructure configuration structures
- `nexus-infra/src/bin/nexus-server.rs` - Server binary (A2A + legacy gRPC + mesh listener)
- `nexus-agent/src/execution.rs` - Main task execution logic
- `nexus-agent/src/main.rs` - Agent entry point and transport selection
- `nexus-common/src/identity.rs` - NodeIdentity (Ed25519 + X25519) for mesh/A2A
- `nexus-common/src/messages.rs` - Legacy TCP message protocol (being replaced by gRPC)

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
