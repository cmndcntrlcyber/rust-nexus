# v1.1 — simple-mesh integration overview

> Looking for deployment instructions? See [`../deployment/README.md`](../deployment/README.md).
> This doc covers v1.1 design / architecture only; v1.2 production
> rollout lives under `docs/deployment/`.

## What v1.1 ships

v1.1 introduces an **additive** layer alongside the existing rust-nexus C2
framework:

- A new **A2A protocol** plane (Linux Foundation A2A v0.3 subset) hosted on
  a separate gRPC port from the existing `NexusC2` service.
- A **libp2p mesh** transport with `MeshNode` + `SealedEnvelope` E2E
  encryption.
- A **Tauri + Leptos operator console** with xterm.js terminal.
- A `Transport` trait abstraction in `nexus-web-comms`.
- A **registry bridge** so the operator console can see agents that
  registered via the overlay's existing `RegisterAgent` flow.

Per **D-V1.1-A** all existing functionality is preserved unchanged: the
overlay's Cloudflare DNS automation, Let's Encrypt ACME lifecycle, gRPC
`NexusC2` service, BOF/COFF loader, fiber execution, anti-debug/anti-VM
evasion, and ATT&CK technique crates all continue to work as before.

## Architecture

```
            +-------------------------------------------------+
            |   nexus-infra (C2 server, same process)         |
            |                                                 |
operator    |   +----------------------+  +----------------+  |    overlay agents
  console  ─┼─► |  A2A service          |  | NexusC2 svc   | ◄┼─── (existing C2 flow)
  (Tauri)   |   |  port = 50052         |  | port = 50051  |  |
            |   |  - SendStreamingMsg  │  | - RegisterAgent│  |
            |   |  - GetAgentCard      │  | - Heartbeat   │  |
            |   |  - ListRegisterAgnts │  | - GetTasks    │  |
            |   +─────────┬────────────+  | - SubmitResult│  |
            |             │ visibility    +────────┬──────+  |
            |             │ bridge                 │         |
            |             ▼                        ▼         |
            |   +────────────────────────────────────────+   |
            |   |  Shared agent registry                  |   |
            |   |  Arc<RwLock<HashMap<UUID, AgentSession>>> |
            |   |  (overlay-owned; v1.1 reads via RegistryLister)
            |   +─────────────────────────────────────────+   |
            +─────────────────────────────────────────────────+
```

## Decisions (D-V1.1-A through J)

| ID | Decision | Choice |
|---|---|---|
| D-V1.1-A | Integration strategy | Additive — no removal of existing code |
| D-V1.1-B | Proto coexistence | nexus.proto + a2a.proto on separate ports (Tonic 0.10 vs 0.14) |
| D-V1.1-C | Crypto layering | Existing AES-256-GCM `Crypto` + new `NodeIdentity` (Ed25519+X25519) + `SealedEnvelope` coexist |
| D-V1.1-D | Agent runtime mode | Existing path + new optional A2A path |
| D-V1.1-E | nexus-mesh | Empty scaffold replaced with v1.0 libp2p impl |
| D-V1.1-F | Workspace members | + nexus-a2a, nexus-console/src-tauri, integration-tests |
| D-V1.1-G | v1.0 docs | docs/v1.0/ restored verbatim |
| D-V1.1-H | Identity format | 72-byte NXS_ID01 blob unchanged |
| D-V1.1-I | A2A target_agent_id | BLAKE3(UUID.bytes) from overlay's Agent.id |
| D-V1.1-J | Original v1.1 security | Deferred to v1.2 |

## Workspace map (post-v1.1)

| Crate | Origin | v1.1 changes |
|---|---|---|
| `nexus-common` | Overlay | + `identity`, `os`, `sealed` modules |
| `nexus-infra` | Overlay | + `a2a_lister`, `a2a_router`, `sessions`, `serve` modules |
| `nexus-agent` | Overlay | + `shell/`, `a2a_client`, `transports/`, `smoke` lib modules |
| `nexus-mesh` | Empty scaffold | Replaced with v1.0 libp2p impl |
| `nexus-web-comms` | Overlay | + `transport.rs` (Transport trait) |
| `nexus-webui` | Overlay | Unchanged |
| `nexus-recon` | Overlay | Unchanged |
| `nexus-hybrid-exec` | Overlay | Unchanged |
| `nexus-t1059-command-scripting` | Overlay | Unchanged |
| `nexus-t1547-boot-logon-autostart` | Overlay | Unchanged |
| `nexus-t1021-006-winrm` | Overlay | Unchanged |
| **`nexus-a2a`** | **New** | A2A gRPC server/client (Tonic 0.14) |
| **`nexus-console/src-tauri`** | **New** | Tauri v2 operator console |
| **`nexus-console/ui`** | **New** | Leptos WASM frontend (excluded from workspace; targets wasm32) |
| **`integration-tests`** | **New** | v1.1 cross-crate tests |

## Pre-existing overlay build issues (not v1.1 regressions)

The overlay shipped with **~35 pre-existing compilation errors** in
modules that v1.1 is required by plan to **not modify** (see plan §"Files
explicitly NOT touched"):

- `nexus-infra/src/letsencrypt.rs` — acme-lib 0.8 API drift
  (`AccountCredentials`, `OrderStatus`, `Persist` trait)
- `nexus-infra/src/cert_manager.rs` — `rustls_pemfile::private_key`,
  `pem::parse` API drift
- `nexus-infra/src/domain_manager.rs` — hickory-dns async signature
- `nexus-infra/src/bof_loader.rs` — goblin `SymbolTable::len` removed
- `nexus-infra/src/cloudflare.rs` — async/tokio timeout shape mismatch

These need **overlay maintenance** (a v1.1.1 follow-up). The v1.1 modules
are written so they compile cleanly once the overlay is fixed:

- `nexus-common`: ✅ builds (36 tests pass)
- `nexus-mesh`: ✅ builds (2 tests + 2 examples pass)
- `nexus-a2a`: ✅ builds (9 + 2 tests pass)
- `nexus-web-comms`: ✅ builds (8 tests pass)
- `nexus-console/ui`: ✅ builds for wasm32
- `integration-tests`: ✅ builds (2 tests pass)
- `nexus-console/src-tauri`: requires libwebkit2gtk-4.1-dev (per v1.0
  Phase 0 convention)
- `nexus-infra`, `nexus-agent`: blocked by overlay pre-existing errors

## Demo

```bash
./scripts/demo.sh
# → "[demo] PASS — v1.1 A2A loopback round-trip verified"
```

The v1.1 demo gate is the in-process `cargo test -p integration-tests`
loopback (operator → A2A → EchoShellHandler → bytes back). The full
three-process demo against the overlay's actual `nexus-server` binary is
gated on the overlay pre-existing errors being fixed (v1.1.1+).

## What's deferred to v1.2

Per plan D-V1.1-J:

- **Agent-side A2A bidi** — agents dial the C2's A2A service to receive
  operator-initiated interactive shells. v1.1 has `nexus_agent::a2a_client`
  scaffolding (`probe_c2` works; `connect_and_serve` returns
  `Err("v1.2 deferred")`).
- **mTLS** — env-var names already reserved (`NEXUS_CA_CERT`,
  `NEXUS_CLIENT_CERT`, `NEXUS_CLIENT_KEY`).
- **Signed AgentCards + capability matrix + hash-chained audit log** —
  v2.1.2 Phase 3 work moved from v1.1 to v1.2 to make room for the
  integration.
- **A2A proto upstream reconciliation** — D-2.1.2-B still open.
- **Tauri bundle codesigning** (macOS + Windows).
- **Kademlia / DTN / multi-hop relay / Sphinx** mesh hardening.
- **MatrixRouter** + matrix algebra.
- **Server-side mesh listener** — operator console can't currently use
  the mesh transport.

## How to test the integrated workspace today

```bash
# v1.1 modules: all green.
cargo test -p nexus-common      # 36 tests
cargo test -p nexus-mesh        # 2 tests + manual examples
cargo test -p nexus-a2a         # 11 tests
cargo test -p nexus-web-comms   # 8 tests
cargo test -p integration-tests # 2 tests

# Mesh examples (interactive verification).
cargo run -p nexus-mesh --example two_node_ping
cargo run -p nexus-mesh --example sealed_pubsub

# Tauri UI wasm32 build.
cd nexus-console/ui && cargo build --target wasm32-unknown-unknown

# v1.1 demo gate.
./scripts/demo.sh
```

The overlay's `nexus-server` and `nexus-agent` binaries require the
pre-existing overlay errors to be addressed first (v1.1.1 follow-up).
