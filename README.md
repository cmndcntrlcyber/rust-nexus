# rust-nexus

A Rust workspace for an authorized-engagement C2 / agent framework.
Combines a libp2p-based mesh, an A2A gRPC control plane with mTLS +
signed agent cards + a capability matrix + a hash-chained audit log,
cross-platform agents with PTY-backed interactive shells, and a
Tauri 2 + Leptos desktop operator console.

> **Use only with explicit authorization.** rust-nexus is intended for
> authorized security testing, red-team engagements, and security
> research. See the [Security Notice](#security-notice) below.

---

## Status

**v1.4 — code complete.** 222 / 222 tests pass; `./scripts/demo.sh`
PASSes; workspace builds clean with `-D warnings` (all eleven v1.4
phases closed, commit-prep done 2026-05-20).

For the per-phase rollup and what's deferred to v1.4.x / v1.5, see
[`STATUS.md`](STATUS.md) and [`ROADMAP.md`](ROADMAP.md).

---

## Quickstart

Local single-box demo in ~15 minutes:

```bash
git clone <repo>
cd rust-nexus
cargo test --workspace --exclude nexus-console   # 170/170 pass
./scripts/gen-certs.sh                            # dev mTLS certs
./scripts/demo.sh                                 # headless PASS gate
```

For the full end-to-end walkthrough (server + agent + Tauri console
with interactive shell), see [`docs/deployment/local-dev.md`](docs/deployment/local-dev.md).

For production rollouts, see
[`docs/deployment/README.md`](docs/deployment/README.md).

---

## Workspace map

14 crates plus a Tauri UI sub-crate. v1.2 additions in **bold**.

| Crate | Role |
|---|---|
| `nexus-common` | NodeIdentity (Ed25519 + X25519), sealed envelopes, OS detection, shared error / message types |
| `nexus-a2a` | **v1.2 A2A gRPC plane:** server / client, signed AgentCards, mTLS env-var loader, capability matrix, hash-chained audit log, rate limit + 4 MiB message cap, agent-registration handler |
| `nexus-mesh` | libp2p mesh transport (TCP + Noise + Yamux + Gossipsub + Identify + Ping); replicates GhostWire primitives from MIT/Apache libp2p crates (no AGPL paths) |
| `nexus-infra` | C2 server runtime — OperatorRouter, AgentRegistrar, SessionRegistry, RegistryLister, mTLS plumbing. Hosts the A2A service (port 50052) alongside the legacy NexusC2 service (port 50051) |
| `nexus-agent` | Cross-platform agent — PTY shell (`portable-pty`), transports (gRPC / mesh / legacy), **`a2a_client::connect_and_serve` agent-side bidi** |
| `nexus-web-comms` | `Transport` trait abstraction + legacy HTTP / WebSocket transports |
| **`nexus-console/src-tauri`** | **v1.2 Tauri 2 operator console** (Rust backend) — connect dialog, agent list, shell session management |
| **`nexus-console/ui`** | **v1.2 Leptos + WASM frontend** with xterm.js terminal (excluded from the workspace; built by Trunk) |
| `nexus-webui` | Optional web UI (overlay-era) |
| `nexus-recon` | Reconnaissance helpers |
| `nexus-hybrid-exec` | Hybrid SSH / WMI / API / PowerShell executor (feature-gated, mostly stubbed in v1.2.1) |
| `nexus-t1059-command-scripting` | ATT&CK T1059 (Command and Scripting Interpreter) |
| `nexus-t1547-boot-logon-autostart` | ATT&CK T1547 (Boot or Logon Autostart Execution) |
| `nexus-t1021-006-winrm` | ATT&CK T1021.006 (Remote Services: Windows Remote Management) |
| **`integration-tests`** | **v1.1 + v1.2 cross-crate integration tests** (A2A loopback, mTLS round-trip, agent-side bidi PTY round-trip) |

---

## Architecture

```
+--------------------+        operator-A2A         +--------------------+
|  nexus-console     | <─── mTLS + signed card ──► |  nexus-infra       |
|  Tauri + Leptos    |   :50052 (Tonic 0.14)       |  C2 server         |
|  + xterm.js        |                             |  (NodeIdentity,    |
+--------------------+                             |   AgentChannels,   |
                                                   |   capability       |
+--------------------+      v1.2 agent-mode        |   matrix, audit    |
|  nexus-agent       | <─ mTLS + AgentRegister ─►  |   sink, rate       |
|  (PTY shell,       |    :50052 first frame       |   limiter)         |
|   OS-aware shell   |    + per-session task_id    |                    |
|   select)          |                             +────┬───────────────+
+--------------------+                                  │
                                                        │ (Tonic 0.10 lane
+--------------------+     legacy task-pull             │  unchanged)
|  overlay agents    | <────────────────────────────────┘
|  (Cloudflare DNS,  |   :50051
|   BOF, fiber...)   |
+--------------------+
```

Two gRPC services run on the same `nexus-infra` server process but on
separate ports: A2A on 50052 (v1.2 mTLS + signed cards + capability
gating + audit log + agent-mode bidi), and the legacy NexusC2 on 50051
(Tonic 0.10, untouched). Overlay agents from the v1.0 era keep
working via the legacy lane; v1.2 agents register on the A2A lane and
support interactive shells.

For the wire reference see
[`docs/v1.0/shell-session-protocol.md`](docs/v1.0/shell-session-protocol.md);
for the v1.2 API surface see
[`docs/v1.2/migration-from-v1.1.md`](docs/v1.2/migration-from-v1.1.md).

---

## v1.2 features

- **mTLS** via reserved env vars: `NEXUS_CA_CERT`, `NEXUS_SERVER_CERT`,
  `NEXUS_SERVER_KEY`, `NEXUS_CLIENT_CERT`, `NEXUS_CLIENT_KEY` —
  see [`nexus-a2a/src/tls.rs`](nexus-a2a/src/tls.rs).
- **Ed25519-signed AgentCards** with canonical-JSON encoding — see
  [`nexus-a2a/src/cards.rs`](nexus-a2a/src/cards.rs).
- **Per-agent capability matrix** (HashMap-backed JSON; nalgebra
  MatrixRouter queued for v1.3) — see
  [`nexus-a2a/src/capabilities.rs`](nexus-a2a/src/capabilities.rs)
  and [`config/capabilities.example.json`](config/capabilities.example.json).
- **Hash-chained audit log** (`BLAKE3(prev || record)`) with an
  `audit_verify` CLI — see
  [`nexus-a2a/src/audit.rs`](nexus-a2a/src/audit.rs) and
  [`nexus-a2a/src/bin/audit_verify.rs`](nexus-a2a/src/bin/audit_verify.rs).
- **Token-bucket rate limit** + **4 MiB message size cap** — see
  [`nexus-a2a/src/interceptors.rs`](nexus-a2a/src/interceptors.rs).
- **Agent-side A2A bidi** — operator → C2 → agent → PTY → response —
  see [`nexus-agent/src/a2a_client.rs`](nexus-agent/src/a2a_client.rs).
- **Tauri 2 + Leptos operator console** — see
  [`nexus-console/`](nexus-console/) and
  [`docs/deployment/operator-console.md`](docs/deployment/operator-console.md).
- **CI codesigning** for macOS + Windows Tauri bundles — see
  [`.github/workflows/tauri-build.yml`](.github/workflows/tauri-build.yml)
  and [`docs/v1.2/codesigning.md`](docs/v1.2/codesigning.md).

The complete defense matrix and threat model is at
[`docs/v1.2/security-overview.md`](docs/v1.2/security-overview.md).

---

## Documentation map

| What you want | Where to look |
|---|---|
| Local-dev quickstart | [`docs/deployment/local-dev.md`](docs/deployment/local-dev.md) |
| Production deployment | [`docs/deployment/production.md`](docs/deployment/production.md) |
| Operator console distribution | [`docs/deployment/operator-console.md`](docs/deployment/operator-console.md) |
| Day-2 operations / runbook | [`docs/deployment/operations.md`](docs/deployment/operations.md) |
| v1.2 security overview | [`docs/v1.2/security-overview.md`](docs/v1.2/security-overview.md) |
| v1.2 migration notes (from v1.1) | [`docs/v1.2/migration-from-v1.1.md`](docs/v1.2/migration-from-v1.1.md) |
| Tauri codesigning (CI) | [`docs/v1.2/codesigning.md`](docs/v1.2/codesigning.md) |
| Wire reference (shell-session protocol) | [`docs/v1.0/shell-session-protocol.md`](docs/v1.0/shell-session-protocol.md) |
| Project status + version | [`STATUS.md`](STATUS.md) |
| Roadmap / decisions log | [`ROADMAP.md`](ROADMAP.md) |
| Per-PR / development notes | [`CLAUDE.md`](CLAUDE.md), [`docs/development/`](docs/development/) |

---

## Development

```bash
# Workspace builds (excludes the Tauri shell — that needs libwebkit2gtk).
cargo check --workspace --exclude nexus-console
cargo test --workspace --exclude nexus-console
cargo clippy --workspace --exclude nexus-console --all-targets -- -D warnings
cargo fmt --all --check

# Tauri UI compiles for wasm32 via trunk:
cd nexus-console/ui && cargo build --target wasm32-unknown-unknown && cd ../..

# Headless PASS gate (integration-tests/A2A loopback):
./scripts/demo.sh
```

Development notes live under [`docs/development/`](docs/development/);
the working agreement for AI-assisted contributions is in
[`CLAUDE.md`](CLAUDE.md).

---

## Project conventions

- **Mixed Tonic versions are intentional.** The new A2A plane uses
  Tonic 0.14 (`tonic_14 = { package = "tonic", ... }`) while the
  legacy NexusC2 plane stays on Tonic 0.10. Both compile side-by-side.
- **`NEXUS_*_CERT` env var names are reserved.** Do not rename. They
  are referenced from operator docs, CI workflows, and operator
  consoles built in the field.
- **`shared NodeIdentity`** between the mesh (libp2p) and A2A (signed
  cards). One identity per node — do not introduce a second key.
- **No AGPL code paths.** rust-nexus replicates GhostWire-style mesh
  primitives directly from MIT/Apache libp2p crates rather than
  embedding the AGPL-licensed GhostWire crate.

For the full set of decisions (D-V1.0-A through D-V1.2-H, plus the
v2.1.2 series), see [`ROADMAP.md`](ROADMAP.md).

---

## Security Notice

rust-nexus is designed for **authorized security testing, red-team
engagements, and security research**. Users must:

- Ensure compliance with applicable laws and regulations.
- Obtain explicit written authorization before deployment against any
  system, network, or organization you do not own.
- Follow responsible disclosure for any vulnerabilities discovered
  using this framework.
- Respect system, network, and organizational boundaries.

**The authors are not responsible for misuse of this software.**

---

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE).
