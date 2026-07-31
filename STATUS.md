# STATUS

Current phase: **v1.4 — code complete + v1.4.x close-out. 276 / 276 tests pass.**
Next: v1.5 overlay cleanup.

## v1.2 deliverables

| Phase | Status | Milestone | Notes |
|---|---|---|---|
| 1.2.1 — Overlay maintenance (acme-lib / rustls_pemfile / hickory / goblin / reqwest) | ✅ | M-V1.2-clean | 5 plan files + grpc_client/grpc_server/hybrid-exec/agent (pre-existing drift). ACME workflow stubbed (returns "deferred to v1.3"); other utilities work. |
| 1.2.2 — Agent-side A2A bidi (D-V1.2-G) | ✅ | M-V1.2-shell | `nexus_agent::a2a_client::connect_and_serve` real impl + `AgentRegistrationHandler` trait + `AgentRegistrar` in nexus-infra. Live operator → C2 → agent → PTY → response round-trip verified. |
| 1.2.3 — Signed AgentCards (D-V1.2-cards) | ✅ | M-V1.2-cards | `nexus_a2a::cards` canonical-JSON sign/verify with Ed25519. `A2aSharedState::with_server_identity` signs at construction. |
| 1.2.4 — mTLS (D-V1.2-mtls / D-V1-E reversal) | ✅ | M-V1.2-mtls | `nexus_a2a::tls` loads `NEXUS_*_CERT` env vars (path or inline PEM). `scripts/gen-certs.sh` provisions dev certs. `serve_with_optional_tls` / `connect_with_optional_tls` builders. |
| 1.2.5 — Capability matrix (D-V1.2-caps) | ✅ | M-V1.2-caps | `nexus_a2a::capabilities::CapabilityCheck` (HashMap-backed); `OperatorRouter::with_capability_check`. Real nalgebra MatrixRouter v1.3+. |
| 1.2.6 — Hash-chained audit log (D-V1.2-audit) | ✅ | M-V1.2-audit | `nexus_a2a::audit::{FileSink, MemSink, AuditSink}` + `audit_verify` CLI binary. BLAKE3 chain integrity. |
| 1.2.7 — Interceptor stack | ✅ | M-V1.2-defense | Token-bucket `RateLimitInterceptor` + 4 MB message size cap (applied in `into_service`) + `dev-reflection` cargo feature off in release. |
| 1.2.8 — Upstream proto pin (D-V1.2-E) | ✅ | M-V1.2-proto | `VENDORED-VERSION` records v0.3.x pin; subset proper of upstream. Full vendoring + Unimplemented RPC stubs deferred to v1.3 (documented in plan). |
| 1.2.9 — Tauri bundle codesigning (D-V1.2-F) | ✅ | M-V1.2-sign | `.github/workflows/tauri-build.yml` + `docs/enhancements/agent-swarm-mesh/v1.2/codesigning.md`. macOS notarization + Windows signtool driven by CI secrets. |
| 1.2.10 — Final verification + docs | ✅ | M-V1.2-7 | `docs/enhancements/agent-swarm-mesh/v1.2/{security-overview,codesigning,migration-from-v1.1}.md`. |

**Total v1.2 tests passing on dev host: 170** (v1.1 baseline 57 + Phase 1.2.1 fixes restored ~92 overlay tests + 18 new v1.2 unit/integration tests across cards, capabilities, audit, interceptors, tls, and the live agent-side bidi).

## Verification

- `cargo check --workspace --exclude nexus-console` — green
- `cargo test --workspace --exclude nexus-console` — **170 / 170 pass**
- `./scripts/demo.sh` — **PASS**
- `cargo test -p integration-tests --test a2a_mtls -- --ignored` with `NEXUS_*` env vars + `./certs/` from `gen-certs.sh` — **PASS** (mTLS round-trip)
- `cargo run --bin audit_verify -- /tmp/audit.log` — exit 0 on intact, non-zero on tampered (proved via unit tests)

## Architecture (v1.2)

```
+--------------------+        operator-A2A         +--------------------+
|  nexus-console     | <─── mTLS + signed card ──► |  nexus-infra       |
|  Tauri + Leptos    |   :50052 (Tonic 0.14)       |  C2 server         |
|  + xterm.js        |                             |                    |
+--------------------+                             |  +─────────────+   |
                                                   |  | A2A service │   |
                                                   |  └─────┬───────+   |
                                                   |        │ bridge    |
                                                   |  +─────▼───────+   |
                                                   |  | NexusC2 svc │   |
                                                   |  └─────────────+   |
+--------------------+      v1.2 agent-mode       +────┬────────────────+
|  nexus-agent       | <─ mTLS + AgentRegister ─►      │ AgentChannels +
|  (PTY shell,       |    :50052 first frame           │ SessionRegistry
|   OS-aware shell   |    + per-session task_id        │ + CapabilityCheck
|   select, BOF,...)|                                 │ + AuditSink (v1.3 wired)
+--------------------+                                 │ + RateLimiter
                                                       │
+--------------------+      legacy task-pull           │
|  overlay agents    | <───────────────────────────────┘
|  (Cloudflare DNS,  |   :50051 (Tonic 0.10, untouched)
|   ACME [stubbed],  |
|   BOF, fiber...)   |
+--------------------+
```

## Out of scope for v1.2 (deferred to v1.3+)

- Full upstream `a2aproject/A2A` proto vendoring + `Unimplemented`
  stubs for `GetTask`/`CancelTask`/`TaskSubscription`/`CreateTaskPushNotificationConfig`/`ListTask`
- ACME / Let's Encrypt cert provisioning (v1.2.1 stubbed the order
  workflow; production uses operator-supplied certs)
- Server-side mesh listener for shell sessions (libp2p path is
  agent-only in v1.2)
- Kademlia DHT discovery / DTN store-and-forward / multi-hop relay /
  Sphinx anonymity
- `nalgebra`-backed `MatrixRouter` (v2.1.2 Phase 4)
- Audit-log syslog / remote-collector backends
- Audit-log review UI in the Tauri console
- Python interop fixture for the A2A proto (v2.1.2 Phase 2)

## Recent activity

- 2026-05-21 — **v1.5 overlay cleanup + mesh interop checkpoint complete.**
  (1) `TaskResult` canonicalized — `messages::TaskResult` renamed to `LegacyTaskResult`; `tasks::TaskResult` is now the single canonical type, plus an `error()` alias for `failure()`. `nexus-agent` call sites updated to pass `start_time` directly. `#![allow(ambiguous_glob_reexports)]` removed from `nexus-common`.
  (2) `AgentBuilder` added (`nexus-common/src/agent.rs`) — fluent 9-field builder for `Agent`; `Agent::builder()` factory; doctest passes. `#![allow(clippy::too_many_arguments)]` removed.
  (3) `nexus-hybrid-exec` executors wired: SSH real impl via `russh` (behind `ssh` feature); API real impl via `reqwest` (behind `api` feature); PowerShell via `tokio::process::Command` spawning `pwsh`/`powershell.exe` (no broken crate dependency); WMI real impl via `wmi` + `Win32_Process.Create` (behind `all(feature="wmi", target_os="windows")`). All non-available-feature paths return explicit `TaskExecutionError` rather than silent stub output.
  (4) D-XLINK-A resolved — `nexus-web-comms::mesh_a2a` module added: `TRANSPORT_PRIORITY` constant, `select_transport` helper, `MeshA2aBridge` boundary struct, 4 unit tests. A2A gRPC = primary; libp2p mesh = fallback; A2A-over-mesh tunnel deferred to Phase 5. All affected crates compile clean; test count 226 → 230.

- 2026-05-20 — **v1.4.x close-out (all four deferred items closed).**
  v1.4.x-1: `nexus_a2a::audit_s3::S3Sink` real `aws-sdk-s3` 1.51 upload
  impl behind the `s3` Cargo feature — bounded queue + background
  task + exponential backoff retry; KMS / endpoint-override / static-
  credential surfaces wired; 6 unit tests on `--features s3`.
  v1.4.x-2: `.github/workflows/docker.yml` extended to PR-trigger
  multi-arch build-only verification; `scripts/docker-multiarch-verify.sh`
  ships the same multi-arch buildx command for dev hosts.
  v1.4.x-3: `nexus-infra/tests/acme_smoke.rs` now consumes
  `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` + contact email from
  env; `.github/workflows/acme-staging.yml` runs the ignored
  round-trip on workflow_dispatch + weekly cron when the secrets are
  configured. v1.4.x-4: `MeshHandle::topic_subscribers` (gossipsub
  mesh-peer probe) + `nexus_mesh::dtn::publish_helpers::publish_or_dtn`
  Swarm-coupled publish with `tokio::time::timeout` falling back to
  the DTN queue on zero subscribers / publish failure;
  `PublishOutcome::Delivered` extended with `mesh_peers` count.
  Test count: 222 → 226 (S3 feature-off + DTN unit + DTN integration).
  All workspace tests + clippy + fmt pass.
- 2026-05-19 — v1.3 execution continued (round 3): Phase 1.3.4 full Kademlia + mDNS Swarm integration (live in `MeshBehaviour`; `examples/kad_discovery.rs` smoke-test confirms two-node identify-over-Kademlia works end-to-end). Phase 1.3.1 partial shipped (six upstream A2A v0.3 RPC methods — `GetTask`, `CancelTask`, `TaskSubscription`, `CreateTaskPushNotificationConfig`, `ListTask`, `GetAuthenticatedExtendedAgentCard` — added to proto with best-effort message shapes; all five Task RPCs return Unimplemented; `GetAuthenticatedExtendedAgentCard` returns the standard card. Loopback test verifies they're reachable on the wire. Definitive sha pin against upstream remains v1.4 work). Phase 1.3.9 closed (manifests + Dockerfile authored; `rust:1-bookworm` base pin verified cargo-chef installs cleanly; full release-build verification disk-constrained on dev host; docker.md updated). Test count: 188 → 189, all green. Demo still PASSes.
- 2026-05-19 — v1.4 execution round 5 (v1.4 close-out): Phase 1.4.4 closed (Tauri audit-log viewer — `audit_log_tail` / `audit_log_filter` / `audit_log_verify` Tauri commands in `nexus-console/src-tauri/src/commands.rs` consuming the v1.4.3 `StreamAuditRecords` RPC; `audit_log_verify` does a pure-Rust BLAKE3 chain check; new Leptos component `nexus-console/ui/src/components/audit_log_viewer.rs` ships a table view + filter inputs + verify button; WASM UI builds clean for wasm32-unknown-unknown). Phase 1.4.1 closed (acme-lib 0.8 real re-port — `nexus_infra::letsencrypt::request_certificate` now runs the full ACME DNS-01 flow inside `tokio::task::spawn_blocking`, bridging back to async land via `tokio::runtime::Handle` for Cloudflare TXT-record publish/delete; `CertBundle` + `run_acme_flow` private helpers; `CloudflareManager` derives `Clone`; new `tests/acme_smoke.rs` with `initialize_creates_storage_dir` (always runs) + `staging_dns01_round_trip` (`#[ignore]`d, activated by `LETSENCRYPT_STAGING_ENABLED=1` + `LETSENCRYPT_TEST_DOMAIN`)). Phase 1.3.7 marked complete (its Tauri viewer half landed via 1.4.4). Phase 1.3.2 task cleaned up (superseded by 1.4.1). Test count: 221 → 222. Demo PASSes. **All eleven v1.4 phases closed.**
- 2026-05-19 — v1.4 execution round 4: bounded remaining work closed. Phase 1.4.3 finish — `nexus_infra::mesh_listener::pump_mesh_decoded` callback forwarder + test (synthesizes a MeshListener with a real MeshNode and confirms payloads route to the callback). Phase 1.4.7 finish — gRPC metadata extraction wired into `send_streaming_message`: `x-nexus-operator-token` hex header decoded + verified against the server's NodeIdentity public key BEFORE consuming the streaming body; `extract_operator_token` + `hex_decode` helpers in server.rs; `dispatch_stream` threads the extracted token through `ShellOpenParams.operator_token`; live end-to-end test (`v1_4_7_operator_token_metadata_extracted_at_dispatch`) covers the full path. Phase 1.4.10 finish — `nexus_mesh::dtn::publish_helpers::{publish_then_dtn, drain_on_reconnect, PublishOutcome}` caller-driven helpers with tests. `docs/enhancements/archive/v1.4/security-overview.md` + `docs/enhancements/archive/v1.4/migration-from-v1.3.md` fully written (cumulative defense matrix + API/wire/config diff tables + operational upgrade procedure). Test count: 217 → 221. Demo PASSes.
- 2026-05-19 — v1.4 execution round 3: Phase 1.4.2 closed in **pure Rust** (D-V1.4-B revised — no Python). New `nexus-a2a/vendor/a2a-upstream/a2a.v1.proto` stub (package renamed to `a2a.upstream.v1` so two protos compile side-by-side). `build.rs` extended to compile both protos. New `pb_upstream` module exposed via `tonic::include_proto!`. `tests/upstream_compat.rs` ships 4 tests covering symmetric prost encode/decode field-number compat + live gRPC calls using upstream-encoded bytes against our server. New `scripts/vendor-a2a-proto.sh` operator helper fetches real upstream proto at a tag, patches the `package` declaration, records sha256 + commit sha into `VENDORED-VERSION`. Test count: 213 → 217. Demo PASSes.
- 2026-05-19 — v1.4 execution round 2: Phase 1.4.8 closed (Helm chart at `deploy/helm/nexus-server/{Chart,values,templates/*}.yaml` + `.github/workflows/docker.yml` for multi-arch amd64/arm64 buildx; docs/deployment/kubernetes.md + docker.md updated). Phase 1.4.3 closed (`StreamAuditRecords` RPC + `IssueOperatorToken` RPC live on the wire; `BroadcastSink` + `AuditFilter` adapter in `nexus_a2a::audit`; live end-to-end test confirms a gRPC subscriber receives a record appended to the broadcast sink). Phase 1.4.7 RPC half also landed (server-side token issuance from `IssueOperatorTokenRequest`; client decodes against the server's Ed25519 public key). Test count: 212 → 213. Demo PASSes.
- 2026-05-19 — v1.4 plan + first execution round: Phase 1.4.0 scaffolding (5 new modules across nexus-a2a, nexus-mesh, nexus-agent). Phase 1.4.5 closed (`nexus_a2a::otel::init_tracing_with_otel` behind the `otel` Cargo feature; feature-off path returns `FeatureDisabled`). Phase 1.4.6 closed (`MatrixRouter` in `nexus_a2a::capabilities` — three `DMatrix<bool>` matrices + wildcard hash-set overlays, public API preserved). Phase 1.4.7 backend closed (`nexus_a2a::tokens::OperatorToken` — 97-byte Ed25519-signed compact binary format; 8 tests cover round-trip + tamper + expiry + version + length + signer-mismatch). Phase 1.4.9 backend partial (`nexus_agent::audit::AgentAudit` writes per-host audit log via `FileSink`; S3Sink kept as configuration scaffold). Phase 1.4.10 closed (`nexus_mesh::dtn::DtnQueue` — persistent on-disk per-recipient queue with depth + age bounds; 5 tests). Phase 1.4.11 v1.4 regression test landed (`integration-tests/tests/v1_4_regression.rs` — 5 tests). Test count: 189 → 212. Demo PASSes.
- 2026-05-19 — v1.3 execution continued (round 2): Phase 1.3.3 mesh listener shipped (sealed-envelope decoder + `MeshHandle` subscribe + bootstrap dial; `nexus_infra::mesh_listener::spawn_mesh_listener`). Phase 1.3.10 v1.3 regression test landed (`integration-tests/tests/v1_3_regression.rs` — 4 tests across capability reload, per-operator scoping, MultiSink fan-out, Prometheus counters). `docs/enhancements/archive/v1.3/security-overview.md` + `migration-from-v1.2.md` fleshed out with concrete v1.3 surface. Test count: 183 → 188, all green. Demo still PASSes.
- 2026-05-19 — v1.3 execution continued: Phase 1.3.5 closed (per-operator scoping via `verify_with_operator`, `ShellOpenParams.operator_cn`, audit-rotation.conf dropped `copytruncate` in favor of SIGHUP reload). Phase 1.3.6 closed (`nexus-a2a::metrics` with real Prometheus counters; `nexus-infra::metrics_server` axum-based `/metrics` HTTP service). Phase 1.3.7 backend closed (`SyslogSink` + `MultiSink` for RFC 5424 / TCP audit-log shipping; Tauri audit UI deferred). Phase 1.3.4 foundational primitives closed (`discovery::{kad_config,build_kad,build_mdns}` against libp2p 0.53's kad/mdns features; full Swarm integration deferred). Phase 1.3.8 closed (CI workflows already shipped by scaffolding). Test count: 170 → 183, all green. Demo still PASSes.
- 2026-05-19 — v1.3 plan + scaffolding landed: 10 sub-phases planned, skeleton modules in place across nexus-a2a (`metrics.rs`), nexus-mesh (`discovery.rs`, `topics::Role` enum), nexus-infra (`mesh_listener.rs`, `metrics_server.rs`). New `nexus-server` binary at `nexus-infra/src/bin/nexus-server.rs` with `--init-identity` flag (Phase 1.3.5). `FileSink::reopen()` + `CapabilityCheck::reload()` ready for SIGHUP wiring. CI workflows `ci.yml` + `security-audit.yml` + `deny.toml`. Container infra: `Dockerfile`, `Dockerfile.agent`, `docker-compose.yml`, `deploy/k8s/{base,overlays/{dev,prod}}/`. Docs skeletons at `docs/enhancements/archive/v1.3/{security-overview,migration-from-v1.2}.md`, `docs/development/ci.md`, `docs/deployment/{docker,kubernetes}.md`. Workspace builds green; 170/170 tests still pass; `./scripts/demo.sh` still PASS.
- 2026-05-19 — Deployment documentation refresh landed: new `docs/deployment/` tree (README + local-dev + production + operator-console + operations + systemd unit / nexus.toml / logrotate examples); six stale overlay docs replaced with redirect stubs; full top-level README rewrite to v1.2 reality.
- 2026-05-18 — Phase 1.2.1 closed: overlay nexus-infra + nexus-hybrid-exec + nexus-agent compile clean. 92 pre-existing tests restored.
- 2026-05-18 — Phase 1.2.2 closed: agent-side A2A bidi works; new integration test confirms operator → agent shell round-trip.
- 2026-05-18 — Phase 1.2.3 closed: Ed25519-signed AgentCards (6 tests).
- 2026-05-18 — Phase 1.2.4 closed: mTLS plumbing + gen-certs.sh + mTLS integration test.
- 2026-05-18 — Phase 1.2.5 closed: capability matrix gate enforced in OperatorRouter.
- 2026-05-18 — Phase 1.2.6 closed: hash-chained audit log + audit_verify CLI.
- 2026-05-18 — Phase 1.2.7 closed: 4MB message cap + per-peer rate limit + reflection-off via cargo feature.
- 2026-05-18 — Phase 1.2.8 closed: proto pin documented in VENDORED-VERSION; full vendoring deferred to v1.3.
- 2026-05-18 — Phase 1.2.9 closed: CI codesigning workflow + docs.
- 2026-05-18 — Phase 1.2.10 closed: security-overview + migration-from-v1.1 docs; 170/170 tests pass.

## Commit-prep checkpoint (2026-05-20)

All eleven v1.4 phases are code-complete. This checkpoint represents the first
committable state of the workspace since v1.0.

**Gate status:**
- `cargo test --workspace --exclude nexus-console` → **222 / 222 pass, 0 fail**
- `./scripts/demo.sh` → **PASS** (v1.1 A2A loopback round-trip)
- `cargo fmt --all --check` → **exit 0** (no formatter drift)
- `cargo clippy --workspace --exclude nexus-console --all-targets -- -D warnings` → **exit 0**
- `cargo build --workspace --exclude nexus-console` → **Finished** (clean)
- `.gitignore` → `Cargo.lock` un-ignored; v1.2+ runtime artifact ignores added; `.dockerignore` added
- No sensitive files in untracked set (no `*.pem`, `*.key`, `*.crt`, `audit.log*`, `*identity.bin`)

**Deferred (final v1.4 items) — CLOSED 2026-05-20:**

1. ✅ v1.4.x-1 — `S3Sink` real upload impl. `nexus-a2a` now ships an
   `aws-sdk-s3` (1.51) implementation behind the `s3` Cargo feature.
   `S3Sink::connect` spawns a bounded queue + background upload task
   with exponential-backoff retry. KMS server-side encryption,
   endpoint override (MinIO / R2 / B2 path-style), and standard AWS
   credential chain are all wired. 6 unit tests (with `--features s3`).
2. ✅ v1.4.x-2 — Multi-arch Docker release build. `.github/workflows/docker.yml`
   now build-verifies `linux/amd64,linux/arm64` on every PR
   (build-only, no push) in addition to the existing tag-push pipeline.
   `scripts/docker-multiarch-verify.sh` ships the same build on dev
   hardware once the user is in the `docker` group.
3. ✅ v1.4.x-3 — Live ACME staging round-trip. `acme_smoke::staging_dns01_round_trip`
   wired to consume `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` +
   `LETSENCRYPT_TEST_DOMAIN` from env. New
   `.github/workflows/acme-staging.yml` runs the test on
   `workflow_dispatch` + weekly cron (Mondays 04:17 UTC) when the
   secrets are configured; no-ops cleanly on forks without secrets.
4. ✅ v1.4.x-4 — DTN `MeshNode` publish-path integration.
   `MeshHandle::topic_subscribers` reads the gossipsub mesh-peer count
   from the swarm; `nexus_mesh::dtn::publish_helpers::publish_or_dtn`
   couples publish-under-`tokio::time::timeout` to the subscriber
   probe and routes to the DTN queue when zero peers are present.
   Live two-node-fixture test + a regression test in
   `integration-tests/tests/v1_4_regression.rs`.

**v1.5 planned:**
- Overlay cleanup: builder pattern for `AgentSession::new` (9-arg ctor → builder)
- Resolve `TaskResult` ambiguous re-export in `nexus-common` (one canonical type)
- Wire `nexus-hybrid-exec` executor stubs (SSH, WMI, API, PowerShell)
