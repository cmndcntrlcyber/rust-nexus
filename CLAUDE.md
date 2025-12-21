# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust-Nexus is an enterprise network management framework built in Rust. It's a Cargo workspace with 7 crates that together form a C2 (Command and Control) system for authorized security testing, red team operations, and security research.

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
| `nexus-common` | Shared library: crypto (AES-256-GCM), message types, error handling |
| `nexus-infra` | Infrastructure: Cloudflare DNS, Let's Encrypt ACME, gRPC server/client, BOF loader |
| `nexus-agent` | C2 agent: execution, evasion, persistence, fiber techniques |
| `nexus-webui` | Web dashboard: warp HTTP server, WebSocket real-time updates |
| `nexus-recon` | Reconnaissance: browser fingerprinting, system profiling |
| `nexus-hybrid-exec` | Multi-protocol execution: SSH, WMI, PowerShell, HTTP API |
| `nexus-web-comms` | Fallback comms: HTTP/WebSocket fallback, domain fronting, traffic obfuscation |

### Dependency Flow

```
nexus-common (base library)
    ↓
nexus-infra (infrastructure + gRPC)
    ↓
nexus-agent, nexus-webui, nexus-recon, nexus-hybrid-exec, nexus-web-comms
```

### gRPC Service Definition

The gRPC service is defined in `nexus-infra/proto/nexus.proto`. The `nexus-infra/build.rs` compiles this using `tonic-build`, generating Rust code in the `proto` module. Key service: `NexusC2` with methods for agent registration, heartbeat, task streaming, file transfer, and execution.

### Agent Build Process

`nexus-agent/build.rs` has special Windows handling:
- Compiles keylogger BOF (`bofs/keylogger/nexus_keylogger.c`) using MSVC `cl.exe`
- On non-Windows, creates an empty placeholder
- BOF binary is embedded via `include_bytes!()`

## Configuration

Configuration uses TOML format. Key files:
- `nexus.toml.example` - Full template with all options
- `config/agent-windows.toml` - Windows agent config
- `config/agent-linux.toml` - Linux agent config

Main sections: `[cloudflare]`, `[letsencrypt]`, `[grpc_server]`, `[webui]`, `[reconnaissance]`, `[hybrid_exec]`

## Key Patterns

### Feature Flags

Most crates use Cargo features for optional functionality:
- `nexus-agent`: `bof-loading`, `wmi-execution`, `anti-debug`, `anti-vm`, `process-injection`
- `nexus-hybrid-exec`: `ssh`, `wmi`, `api`, `powershell`
- `nexus-web-comms`: `http-fallback`, `websocket-fallback`, `domain-fronting`

### Platform-Specific Code

Windows-specific code uses `#[cfg(target_os = "windows")]` with `windows-sys` crate. Linux uses `#[cfg(target_os = "linux")]` with `libc`. Check `nexus-agent/src/execution.rs` (largest module at 27KB) for examples.

### Async Runtime

All async code uses Tokio with full features. gRPC uses Tonic. HTTP uses reqwest (client) and warp (server).

### Release Profile

Binaries are optimized for size and performance:
- `opt-level = 3`, `lto = true`, `codegen-units = 1`
- `panic = "abort"`, `strip = true`

## Important Files

- `nexus-infra/proto/nexus.proto` - gRPC service definitions (source of truth for protocol)
- `nexus-infra/src/config.rs` - All configuration structures
- `nexus-agent/src/execution.rs` - Main task execution logic
- `nexus-agent/src/agent.rs` - Core agent loop and lifecycle
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
