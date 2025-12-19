# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Context

D3tect-Nexus is undergoing transformation from a C2/offensive framework into a **detection and response platform**. See `d3tect-nexus-transformation.md` for the full plan. The transformation follows a "Baby Steps" approach across four phases.

### Transformation Overview

**Current State**: Offensive C2 framework with gRPC comms, infrastructure automation, agent execution

**Target State**: SOC platform with threat detection, automated response, and LitterBox malware analysis integration

**Key Changes Planned**:
1. New `nexus-detection/` crate integrating reverse-shell-detector capabilities
2. LitterBox sandbox automated deployment via existing infrastructure
3. Agent transformation from offensive to EDR-style detection
4. gRPC channels repurposed for SOC telemetry
5. SIEM integration (Splunk, QRadar, Sentinel)

## Build Commands

```bash
# Build all workspace members
cargo build --release

# Build specific crate
cargo build --release -p nexus-agent
cargo build --release -p nexus-infra
cargo build --release -p nexus-common

# Run tests
cargo test
cargo test -p nexus-infra

# Cross-compile for Windows (requires target installed)
cargo build --release --target x86_64-pc-windows-gnu

# Full build script with cross-compilation
./scripts/build.sh
```

## Current Architecture (Pre-Transformation)

Rust workspace with 7 crates:

### Core Crates

- **nexus-common**: Shared types, AES-256-GCM encryption, message serialization
- **nexus-infra**: Infrastructure automation (Cloudflare DNS, Let's Encrypt ACME, cert management, gRPC server/client, BOF/COFF loader). Proto at `nexus-infra/proto/nexus.proto`
- **nexus-agent**: Cross-platform agent with gRPC, execution capabilities, evasion

### Supporting Crates

- **nexus-webui**: Warp web server + WebSocket for operator interface
- **nexus-web-comms**: HTTP/WebSocket fallback, domain fronting
- **nexus-hybrid-exec**: Remote execution (SSH, WMI, PowerShell)
- **nexus-recon**: Web fingerprinting, HTML parsing, network recon

### Planned New Crate

- **nexus-detection/**: Detection module integrating reverse-shell-detector (network monitoring, process tracking, 30+ signature patterns, behavioral analysis)

## Transformation Architecture

### Phase 1: Integration
- `nexus-detection/` crate with reverse-shell-detector core
- LitterBox automated deployment via `nexus-infra` DNS/cert automation
- Event correlation combining detection + infrastructure telemetry

### Phase 2: Transformation
- gRPC channels → SOC communication
- Domain fronting → legitimate security monitoring
- Agents → EDR-style detection agents
- Anti-analysis → threat detection capabilities

### Phase 3: SOC Platform
- C2 server → SOC management platform
- Agent management → asset monitoring dashboard
- Task distribution → automated response orchestration

### Phase 4: Ecosystem Integration
- SIEM connectors (Splunk, QRadar, Sentinel)
- Threat intelligence feeds
- Compliance reporting

## Key Configuration

TOML format. Example at `nexus.toml.example`.

**Current sections**:
- `[cloudflare]`: API token, zone_id, domain
- `[letsencrypt]`: ACME, cert storage
- `[grpc_server]`: Bind, port, mTLS
- `[domains]`: Primary/backup, rotation
- `[security]`: Encryption, obfuscation flags

**Planned sections** (from transformation):
```toml
[litterbox]
enabled = true
auto_deploy = true
instances_per_region = 2

[litterbox.analysis]
static_analysis_enabled = true
dynamic_analysis_enabled = true
high_priority_threshold = 0.8

[litterbox.integration]
reverse_shell_detector = true
auto_submit_detections = true
min_confidence_threshold = 0.7
```

## gRPC Protocol

Proto at `nexus-infra/proto/nexus.proto`. Services:
- `RegisterAgent` / `Heartbeat`: Agent lifecycle
- `GetTasks` (streaming) / `SubmitTaskResult`: Task distribution
- `ExecuteShellcode` / `ExecuteBOF`: Advanced execution (to be repurposed for threat hunting)
- `UploadFile` / `DownloadFile`: File transfer

## Feature Flags

**nexus-agent**: `bof-loading`, `wmi-execution`, `domain-fronting`, `anti-debug`, `anti-vm`, `process-injection`

**nexus-web-comms**: `http-fallback`, `websocket-fallback`, `domain-fronting`, `traffic-obfuscation`

## Key Dependencies

From workspace Cargo.toml:
- `tonic`/`prost`: gRPC and protobuf
- `rustls`/`tokio-rustls`: TLS
- `reqwest`: HTTP client (Cloudflare API, LitterBox API)
- `hickory-dns`/`hickory-resolver`: DNS
- `goblin`/`pelite`: PE/COFF parsing
- `windows-sys`: Windows API (conditional)

## Project Structure

```
d3tect-nexus-transformation.md  # Transformation plan (read this first)
nexus-infra/proto/nexus.proto   # gRPC service definitions
config/                          # Agent configuration templates
scripts/build.sh                # Cross-platform build automation
docs/                           # Development and infrastructure docs
examples/                       # Deployment examples
```

## Development Guidelines

When implementing transformation work:
1. Follow "Baby Steps" incremental approach from transformation plan
2. Reuse existing infrastructure (DNS, certs, gRPC) rather than rebuilding
3. New detection code goes in `nexus-detection/` crate
4. LitterBox integration uses existing `DomainManager` and `CertManager`
5. Maintain cross-platform (Windows/Linux) support
