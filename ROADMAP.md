# ROADMAP

## v1.0 decisions (carried into v1.1)

| ID | Decision | Answer |
|---|---|---|
| D-V1-A | WASM frontend framework | Leptos |
| D-V1-B | Terminal renderer | xterm.js via Tauri webview |
| D-V1-C | PTY library | `portable-pty` |
| D-V1-D | Shell transport channel | A2A `SendStreamingMessage` |
| D-V1-E | mTLS in v1.0 | Deferred; loopback-only gate |
| D-V1-F | xterm.js delivery | Vendored npm via Trunk pre-build hook |
| D-V1-G | Workspace task runner | None — shell scripts |
| D-V1-H | Agent cross-compile | `cross` |
| D-V1-I | Structured logging | `tracing` |
| D-V1-J | Integration test harness | Workspace-root `integration-tests/` crate |

## v1.1 decisions

| ID | Decision | Answer |
|---|---|---|
| D-V1.1-A | Integration strategy | **Additive.** Don't replace overlay functionality |
| D-V1.1-B | Proto coexistence | nexus.proto on :50051 (Tonic 0.10), a2a.proto on :50052 (Tonic 0.14) |
| D-V1.1-C | Crypto layering | Overlay AES-GCM `Crypto` + new `NodeIdentity` + `SealedEnvelope` |
| D-V1.1-D | Agent runtime mode | Both modes coexist |
| D-V1.1-E | nexus-mesh | Empty scaffold replaced with v1.0 libp2p impl |
| D-V1.1-F | Workspace members | + nexus-a2a, nexus-console/src-tauri, integration-tests |
| D-V1.1-G | v1.0 docs | Restored |
| D-V1.1-H | Identity format | 72-byte NXS_ID01 blob unchanged |
| D-V1.1-I | A2A target_agent_id | BLAKE3(UUID) from overlay's Agent.id |
| D-V1.1-J | Original v1.1 security | Deferred to v1.2 |

## Milestones

### v1.0 (planning legacy from simple-mesh/, overlay-supplied)

- v1.0 shell-session protocol + Tauri console implementation supplied to
  the user via `simple-mesh/v1.0-scaffolding-plan.md`. Re-introduced in
  this workspace via v1.1's additive integration.

### v1.1 — simple-mesh integration

- [x] Phase 1.1.1 — nexus-common: identity / os / sealed (23 new tests)
- [x] Phase 1.1.2 — nexus-mesh: libp2p Swarm + topics + examples (2 tests)
- [x] Phase 1.1.3 — nexus-a2a crate (M-V1.1-2; 11 tests)
- [x] Phase 1.1.4 — nexus-infra A2A bridge (modules written; runtime exercise blocked on overlay errors)
- [x] Phase 1.1.5 — nexus-agent shell + transports + a2a_client (modules written; runtime blocked on overlay)
- [x] Phase 1.1.6 — nexus-web-comms Transport trait (4 tests)
- [x] Phase 1.1.7 — nexus-console Tauri + Leptos (UI builds on wasm32; backend gated on libwebkit2gtk-4.1-dev)
- [x] Phase 1.1.8 — integration-tests + scripts + docs (2 integration tests; `./scripts/demo.sh` PASS)

**v1.1 status: code complete. Demo PASS on dev host.**

### v1.1.1 — overlay maintenance (deferred)

Fix pre-existing overlay compilation errors so the full integrated
workspace builds end to end:

- `nexus-infra/src/letsencrypt.rs` — acme-lib 0.8 API drift
- `nexus-infra/src/cert_manager.rs` — rustls_pemfile / pem API drift
- `nexus-infra/src/domain_manager.rs` — hickory-dns async shape
- `nexus-infra/src/bof_loader.rs` — goblin SymbolTable API drift
- `nexus-infra/src/cloudflare.rs` — reqwest/tokio timeout shape

Once green, the three-process demo (overlay's nexus-server + agent +
operator console) can be added to `scripts/demo.sh` and CI.

### v1.2 — overlay maintenance + security hardening + functional completion

**Status: code complete (170 tests pass).** See `STATUS.md` for the per-phase rollup.

- [x] Phase 1.2.1 — Overlay maintenance (acme-lib / rustls_pemfile / pem / hickory-dns / goblin)
- [x] Phase 1.2.2 — Agent-side A2A bidi (`connect_and_serve` real impl)
- [x] Phase 1.2.3 — Ed25519-signed AgentCards
- [x] Phase 1.2.4 — mTLS (D-V1-E reversal)
- [x] Phase 1.2.5 — Capability matrix gate
- [x] Phase 1.2.6 — Hash-chained audit log + `audit_verify` CLI
- [x] Phase 1.2.7 — Interceptor stack (rate limit + 4MB size cap + reflection-off)
- [x] Phase 1.2.8 — A2A proto upstream pin documented (full vendoring → v1.3)
- [x] Phase 1.2.9 — Tauri bundle codesigning (CI infra)
- [x] Phase 1.2.10 — Final verification + docs
- [x] **Deployment documentation refresh (2026-05-19)** — new `docs/deployment/` tree + README rewrite + stale-doc redirects

## v1.2 decisions

| ID | Decision | Answer |
|---|---|---|
| D-V1.2-A | Audit-log backend | File-backed BLAKE3-chained; pluggable `AuditSink` trait |
| D-V1.2-B | Capability storage | HashMap JSON file (nalgebra MatrixRouter → v1.3) |
| D-V1.2-C | mTLS cert provisioning | Operator-supplied; `scripts/gen-certs.sh` for dev |
| D-V1.2-D | Reflection-off | Cargo feature `dev-reflection` (off in release) |
| D-V1.2-E | A2A upstream tag | v0.3.x family pin; full vendoring deferred to v1.3 |
| D-V1.2-F | Tauri codesign | `APPLE_*` / `WINDOWS_*` CI secrets |
| D-V1.2-G | Agent-side bidi | First-frame `agent-register` discriminator |
| D-V1.2-H | Backward-compat | All v1.1 frames keep parsing; new variants additive |

### v1.3 — mesh hardening + upstream proto + server-side mesh + ops integrations

**Status: code complete (all phases closed, test count 188 → 222).** Delivered:

- [x] Full upstream A2A v0.3 proto vendoring + `Unimplemented` stubs (6 RPCs)
- [x] ACME / Let's Encrypt DNS-01 real re-port (acme-lib 0.8, Phase 1.4.1)
- [x] Server-side mesh listener (`spawn_mesh_listener`, Phase 1.3.3)
- [x] `nalgebra` DMatrix MatrixRouter (Phase 1.4.6)
- [x] SyslogSink + MultiSink + BroadcastSink audit backends (Phase 1.3.7)
- [x] Capability matrix hot-reload + per-operator scoping (Phase 1.3.5)
- [x] CI workflows: `ci.yml`, `security-audit.yml`, `deny.toml`
- [x] Prometheus `/metrics` endpoint on `:9100` + OTel feature gate (Phase 1.3.6, 1.4.5)
- [x] Docker/Kubernetes manifests + Helm chart (Phases 1.3.9, 1.4.8)
- [x] `nexus-server --init-identity` flag + `FileSink::reopen` (Phase 1.3.5)
- [x] Kademlia DHT + mDNS peer discovery (Phase 1.3.4)
- [x] DTN store-and-forward queue (Phase 1.4.10)

### v1.4 — operator console, OperatorToken, ACME live, multi-arch Docker

**Status: code complete (222 / 222 tests pass, demo PASS, 2026-05-20 commit-prep done).**

- [x] Phase 1.4.0 — scaffolding (5 new modules)
- [x] Phase 1.4.1 — ACME DNS-01 live re-port (acme-lib 0.8)
- [x] Phase 1.4.2 — pure-Rust upstream A2A proto compat test (`upstream_compat.rs`)
- [x] Phase 1.4.3 — `StreamAuditRecords` + `IssueOperatorToken` RPCs live on wire
- [x] Phase 1.4.4 — Tauri audit-log viewer (`audit_log_tail/filter/verify` commands + Leptos component)
- [x] Phase 1.4.5 — OTel OTLP/gRPC trace export behind `otel` Cargo feature
- [x] Phase 1.4.6 — `MatrixRouter` (three `DMatrix<bool>` matrices + wildcard overlays)
- [x] Phase 1.4.7 — 97-byte Ed25519-signed `OperatorToken` + server-side metadata extraction
- [x] Phase 1.4.8 — Helm chart + multi-arch Docker buildx CI workflow
- [x] Phase 1.4.9 — `AgentAudit` per-host audit log; `S3Sink` configured as scaffold
- [x] Phase 1.4.10 — `DtnQueue` + `publish_helpers` DTN caller-driven helpers
- [x] Phase 1.4.11 — v1.4 regression test suite

**v1.4.x close-out (2026-05-20) — all four items closed:**

| ID | Item | Status |
|---|---|---|
| v1.4.x-1 | `S3Sink` real upload impl | ✅ `aws-sdk-s3` 1.51 behind `s3` Cargo feature; KMS / endpoint override / static creds; 6 unit tests |
| v1.4.x-2 | Multi-arch Docker release build verification | ✅ PR-trigger build-only added to `docker.yml`; `scripts/docker-multiarch-verify.sh` for dev hosts |
| v1.4.x-3 | Live ACME staging round-trip in CI | ✅ `acme_smoke` consumes CF token + zone id from env; `acme-staging.yml` workflow (dispatch + weekly cron) |
| v1.4.x-4 | DTN `MeshNode` publish-path integration | ✅ `MeshHandle::topic_subscribers` + `publish_or_dtn` with `tokio::time::timeout` Swarm coupling |

### v1.5 — overlay cleanup + mesh interop checkpoint

- Resolve `TaskResult` ambiguous re-export in `nexus-common` (one canonical type)
- Builder pattern for `AgentSession::new` (9-arg ctor → builder)
- Wire `nexus-hybrid-exec` executor stubs (SSH, WMI, API, PowerShell real impls)
- Mesh + A2A interop checkpoint (D-XLINK-A boundary work)
- Sphinx anonymity layer (v2.1 Phase 9)
