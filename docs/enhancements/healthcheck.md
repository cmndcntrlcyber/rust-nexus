# Codebase Health Check

> **Snapshot warning:** This healthcheck was generated on 2026-06-15 and
> reflects the workspace state at that time. Some findings (build status,
> test counts, empty-file reports) may be stale. Re-run the checks below
> to get current results.

**Date:** 2026-06-15
**Scope:** Full workspace audit — build health, code quality, testing, documentation, security, observability, CI/CD
**Rust edition:** 2021 | **Workspace members:** 14 crates + integration-tests

---

## Executive Summary

The workspace is architecturally sound — clean crate boundaries, consistent async patterns (Tokio throughout), strong cryptographic foundations (Ed25519, AES-256-GCM, BLAKE3), and comprehensive deployment automation. However, **the workspace does not compile** due to a tonic import error in `nexus-agent`, and several hygiene issues need attention before the next release.

| Area | Grade | Blocker? |
|------|-------|----------|
| Build health | **F** | Yes — `nexus-agent` fails to compile |
| Dependency management | **B** | Intentional dual-version strategy, one invalid dep |
| Code quality | **B+** | Good patterns; dead-code suppressions and mixed logging |
| Test coverage | **C+** | Strong in core crates, weak at periphery |
| Documentation | **B** | Excellent deployment docs; zero public API rustdoc |
| Security hygiene | **C** | Hardcoded Cloudflare API token in tracked file |
| Observability | **B-** | Split between `log` and `tracing` crates |
| CI/CD | **A-** | 6 workflows; no coverage tracking |

---

## 1. Build Health

### 1.1 Compile status

```
cargo check --workspace --exclude nexus-console
```

**Result: FAIL** — 1 error in `nexus-agent`

| Crate | Status | Issue |
|-------|--------|-------|
| nexus-common | OK | |
| nexus-infra | OK | |
| nexus-a2a | OK | |
| nexus-mesh | OK | |
| nexus-web-comms | OK | |
| nexus-webui | OK | |
| nexus-recon | OK | |
| nexus-hybrid-exec | OK | |
| **nexus-agent** | **FAIL** | `a2a_client.rs:45` — unresolved `tonic::` |
| nexus-console | Excluded (needs libwebkit2gtk) | |

**Root cause:** `nexus-agent/Cargo.toml` renames tonic 0.14 as `tonic_14`, but `nexus-agent/src/a2a_client.rs:45` references bare `tonic::transport::ClientTlsConfig`. The compiler cannot resolve `tonic` since it's only available as `tonic_14`.

### 1.2 Build script issue (latent)

`nexus-agent/build.rs` uses `Command::new("cl.exe")` on lines 33 and 46 without importing `std::process::Command`. This doesn't fail today because the function is `#[cfg(target_os = "windows")]`-gated and Linux builds skip it. It **will** fail on any Windows build attempt.

### 1.3 Dependency version duplication

Intentional dual-version strategy for forward compatibility with A2A upstream:

| Dependency | Versions | Reason |
|------------|----------|--------|
| tonic | 0.10.2, 0.14.6 | nexus-infra legacy (0.10) vs nexus-a2a upstream (0.14) |
| prost | 0.12.6, 0.14.3 | Follows tonic split |
| rustls | 0.21.12, 0.23.14 | tonic 0.14 pulls 0.23 transitively |

Binary size impact: ~20-30% larger than single-version builds. Documented in `nexus-a2a/Cargo.toml` comments.

### 1.4 Invalid dependency

`nexus-infra/Cargo.toml` declares `hickory-dns = { workspace = true }`, but `hickory-dns` 0.24 has no library target. Cargo emits a warning and ignores it. The code actually uses `hickory-resolver` (correctly declared separately).

### 1.5 Unmaintained dependencies

| Dependency | Version | Last updated | Risk |
|------------|---------|-------------|------|
| acme-lib | 0.8 | 2021 | HIGH — no security patches in 4+ years |
| pwsh | 0.1.0 | Pre-1.0 stub | LOW — disabled by default in nexus-hybrid-exec |
| rquickjs | 0.4 | Outdated (0.5+ available) | LOW — optional feature in nexus-recon |

---

## 2. Code Quality

### 2.1 Error handling

Each major crate defines its own `thiserror`-derived error enum:

- `nexus-common` — `NexusError` (13 variants)
- `nexus-infra` — `InfraError` (10 variants)
- `nexus-a2a` — Module-specific errors: `CardError`, `TokenError`, `TlsError`, `CapabilityError`, `S3SinkError`
- Other crates — `thiserror` enums per module

**No unified error hierarchy** exists across crates. `From` conversions are defined ad-hoc (e.g., `NexusError` implements `From<serde_json::Error>` and `From<std::io::Error>`). Cross-crate error propagation often requires explicit mapping.

### 2.2 Unsafe code

4 files, 12 total `unsafe` blocks — all in platform-specific FFI paths:

| File | Count | Justification |
|------|-------|---------------|
| `nexus-agent/src/execution.rs` | 1 | C callback for keylogger BOF |
| `nexus-agent/src/fiber_execution.rs` | 3 | Shellcode execution, process hollowing, early bird injection |
| `nexus-infra/src/bof_loader.rs` | 7 | COFF/BOF loading and Win32 FFI |
| `nexus-a2a/build.rs` | 1 | Build script env access |

All unsafe usage is concentrated in Windows-specific execution paths and is contextually appropriate for a C2 framework.

### 2.3 Dead code suppressions

13 `#[allow(dead_code)]` annotations across 6 crates:

| File | Scope |
|------|-------|
| `nexus-a2a/src/audit.rs` | 4 struct fields |
| `nexus-a2a/src/audit_s3.rs` | Module-level |
| `nexus-a2a/src/tls.rs` | One struct |
| `nexus-a2a/src/otel.rs` | Module-level |
| `nexus-infra/src/letsencrypt.rs` | Module-level |
| `nexus-infra/src/pki.rs` | Module-level |
| `nexus-infra/src/bof_loader.rs` | One struct |
| `nexus-agent/src/audit.rs` | Module-level |
| `nexus-recon/src/lib.rs` | Module-level |
| `nexus-web-comms/src/lib.rs` | Module-level |

Module-level suppressions signal forward-declared code for upcoming features or incomplete integrations. These should be resolved or narrowed to specific items.

### 2.4 TODO markers

16 TODOs, 0 FIXMEs, 0 HACKs across production code:

| File | Count | Summary |
|------|-------|---------|
| `nexus-infra/src/grpc_server.rs` | 8 | Domain rotation, config provision, file/shellcode/BOF queueing — all placeholder stubs |
| `nexus-webui/src/handlers.rs` | 3 | gRPC client integration, domain manager integration |
| `nexus-infra/src/cert_manager.rs` | 1 | CA signature validation |
| `nexus-infra/src/domain_manager.rs` | 1 | Dictionary-based domain generation |
| `nexus-infra/src/letsencrypt.rs` | 1 | SAN domain parsing |
| `nexus-hybrid-exec/src/lib.rs` | 2 | gRPC and WebSocket execution |

The gRPC server TODOs represent the largest gap — 8 placeholder implementations that return empty/default responses. These are in the legacy `nexus.v1` service path (A2A handles the active path).

### 2.5 Unwrap/expect density

119 `unwrap()` calls and 269 `expect()` calls workspace-wide. Most are in test code (acceptable). Production files with notable density:

| File | unwrap + expect | Risk |
|------|----------------|------|
| `nexus-a2a/src/capabilities.rs` | 24 | Matrix algebra operations with known-good inputs |
| `nexus-mesh/src/dtn.rs` | 35 | DTN queue operations — some in async paths |
| `nexus-infra/src/pki.rs` | 21 | Cert generation with validated inputs |
| `nexus-common/src/sealed.rs` | 12 | Crypto operations with pre-validated keys |

---

## 3. Test Coverage

### 3.1 Per-crate breakdown

Test run: `cargo test` on compilable crates (nexus-agent excluded due to build failure)

| Crate | Files with `#[cfg(test)]` | Total `.rs` files | Coverage ratio | Tests passing |
|-------|---------------------------|-------------------|----------------|---------------|
| nexus-common | 7 | 9 | 78% | 36 |
| nexus-infra | 14 | 17 | 82% | 44 |
| nexus-a2a | 11 | 17 | 65% | 53 |
| nexus-mesh | 4 | 5 | 80% | 13 |
| nexus-web-comms | 3 | 7 | 43% | 1 |
| nexus-webui | 1 | 5 | 20% | — |
| nexus-recon | 1 | 5 | 20% | — |
| nexus-hybrid-exec | 1 | 5 | 20% | — |
| nexus-console | 0 | 5 | 0% | — |
| integration-tests | — | 5 test files | — | Blocked by nexus-agent |

**Total passing: 169 tests** (across compilable crates)

### 3.2 Coverage gaps

High-risk modules with no or minimal tests:

- `nexus-agent/src/execution.rs` (851 lines) — main task dispatch; has test module but blocked by build error
- `nexus-console/src-tauri/src/commands.rs` — Tauri IPC commands; zero tests
- `nexus-hybrid-exec/src/lib.rs` — SSH/WMI/API execution; 1 test file
- `nexus-recon/src/lib.rs` — Fingerprinting and recon; 1 test file
- `nexus-webui/src/handlers.rs` — HTTP handlers; minimal coverage

### 3.3 Integration tests

`integration-tests/` contains 5 test files covering A2A loopback, mTLS round-trip, interactive shell, mesh DTN, and regression scenarios. These are currently blocked by the nexus-agent build failure.

---

## 4. Documentation Gaps

### 4.1 Empty documentation files

| File | Size |
|------|------|
| `docs/v1.3/migration-from-v1.2.md` | 0 bytes |
| `docs/v1.3/security-overview.md` | 0 bytes |

v1.3 is a shipped milestone with implemented features but no migration or security documentation.

### 4.2 Public API documentation

**Zero `///` doc-comments** found on public functions across the entire workspace. Module-level `//!` docs exist in ~104 of ~230 Rust files, but no function, struct, or trait has rustdoc.

### 4.3 Documentation strengths

- Deployment docs (`docs/deployment/`) are excellent — local dev quickstart, production rollout, operations runbook
- `README.md` is comprehensive with workspace map and quickstart links
- `STATUS.md` tracks all v1.4 milestones
- `examples/basic-deployment/` is thorough and runnable
- Version history comments in source files document API evolution (v1.1 through v1.5)

### 4.4 Missing documentation

- No disaster recovery / backup playbook for `/var/lib/nexus/` state
- No multi-region deployment guide
- No certificate pinning or OCSP guidance
- No mesh multi-node troubleshooting runbook
- `docs/integration/CATCH_TAURI_INTEGRATION.md` references a submodule that isn't visible in the workspace

---

## 5. Security Hygiene

### 5.1 Secrets in repository

**CRITICAL:** `nexus.toml:41` contains a hardcoded Cloudflare API token:
```
api_token = "cfut_REDACTED"
```
This file is tracked in git. The token and zone ID (`nexus.toml:42`) are visible to anyone with repository access.

### 5.2 Certificate files

- `certs/` directory contains dev/test self-signed certificates (CN=localhost) — acceptable for dev
- `certs/prod/` — the `.gitignore` covers `certs/` and these are not tracked by git (verified: `git ls-files -- certs/prod/` returns empty). However, the user has the file open in IDE, suggesting active local use
- `.gitignore` patterns for `**/*.pem`, `**/*.key`, `**/*.crt` are comprehensive

### 5.3 Gitignore assessment

The `.gitignore` is well-structured with explicit patterns for:
- Runtime artifacts (`**/*identity.bin`, `audit.log*`, `dtn-queue/`)
- Config with secrets (`nexus.toml`, `agent.toml`, `*-production.toml`)
- Build outputs, transfer staging, operator packages

**Gap:** `nexus.toml` appears in `.gitignore` but is already tracked in git history (`.gitignore` only prevents future additions; it doesn't untrack already-committed files).

### 5.4 Supply chain

`deny.toml` is configured with advisory checks and license allowlist. AGPL is explicitly rejected. `cargo-deny` runs in CI via `security-audit.yml`.

---

## 6. Logging & Observability

### 6.1 Dual logging crate usage

The workspace uses both `log` (0.4) and `tracing` (0.1) without a unified strategy:

**`log` crate (12 modules):**
- `nexus-infra/src/grpc_server.rs`, `cert_manager.rs`, `letsencrypt.rs`, `cloudflare.rs`, `domain_manager.rs`, `bof_loader.rs`, `grpc_client.rs`
- `nexus-agent/src/execution.rs`
- `nexus-hybrid-exec/src/lib.rs`
- `nexus-web-comms/src/lib.rs`
- `nexus-recon/src/lib.rs`
- `nexus-webui/src/lib.rs`

**`tracing` crate (18 modules):**
- All of `nexus-a2a/src/` (audit, server, mock, tls, otel, metrics)
- `nexus-agent/src/` (svc, smoke, a2a_client, transports/grpc, transports/mesh, shell/pty)
- `nexus-infra/src/` (bin/nexus-server, serve, metrics_server, sessions, mesh_listener, a2a_router)
- `nexus-mesh/src/` (node, dtn)

**Pattern:** Newer code (v1.1+) uses `tracing`; older code (v1.0) uses `log`. No migration path is documented.

### 6.2 Observability features

- Prometheus metrics via `nexus-a2a/src/metrics.rs` (opt-in)
- OpenTelemetry export behind `otel` feature flag in nexus-a2a
- Audit chain (BLAKE3 hash-linked records) in nexus-a2a
- S3 audit archive sink behind `s3` feature flag
- `/healthz` endpoint via axum in nexus-infra

---

## 7. CI/CD & Automation

### 7.1 GitHub Actions workflows

| Workflow | Triggers | Jobs | Status |
|----------|----------|------|--------|
| `ci.yml` | push/PR/dispatch | check, test, clippy, fmt, mtls-integration, wasm-build | Active |
| `docker.yml` | tags + PR | Multi-arch Docker build (amd64/arm64) | Active |
| `tauri-build.yml` | tags | macOS notarization + Windows signing | Active |
| `security-audit.yml` | push/schedule | `cargo-deny` supply chain check | Active |
| `acme-staging.yml` | dispatch + weekly cron | ACME staging round-trip | Active |

### 7.2 CI gaps

- **No code coverage tracking** — tests run but no codecov/tarpaulin integration
- **No performance regression detection** — no benchmarks in CI
- **No nightly Rust testing** — only stable toolchain tested
- **CI will currently fail** on the nexus-agent compile error (unless `--exclude nexus-agent` is used)

### 7.3 Scripts quality

All scripts use `set -euo pipefail` and include argument validation. Key scripts:

| Script | Lines | Quality |
|--------|-------|---------|
| `gen-certs.sh` | 71 | Excellent — ED25519, proper SANs |
| `gen-certs-prod.sh` | 165 | Excellent — arg validation, IP regex, separate cert types |
| `transfer-prep.sh` | 392 | Excellent — generates remote setup script, UFW rules |
| `build-agent-bundles.sh` | ~100 | Excellent — per-host certs, zip output |
| `deploy-operator-console.sh` | ~200 | Excellent — dependency checking, cert mode enforcement |

**Missing scripts:**
- No cert expiry monitoring/alerting script
- No environment verification script (Rust version, toolchain match)

---

## 8. Architecture Notes

### 8.1 Strengths

- Clean workspace boundaries with explicit dependency flow: `nexus-common` -> `nexus-infra` -> downstream crates
- Feature-gated ATT&CK technique crates (`nexus-t1059`, `nexus-t1547`, `nexus-t1021-006`) with named profiles
- Consistent async runtime (Tokio exclusively, no mixed runtimes)
- Builder pattern for complex construction (agent config)
- Workspace-level dependency pinning prevents version drift

### 8.2 Deprecated / legacy code

- **nexus-webui** — legacy warp-based UI, replaced by Tauri console (`nexus-console`). Has `nexus-infra` dependency commented out due to compilation issues
- **nexus-infra/src/grpc_client.rs** — legacy v1.0 client, mostly superseded by A2A client
- **nexus-infra/src/grpc_server.rs** — legacy v1.0 server with 8 TODO stubs; coexists with A2A service

---

## 9. Recommendations

### P0 — Blockers (fix before next release)

| # | Issue | File | Fix |
|---|-------|------|-----|
| 1 | **Build failure** — bare `tonic::` reference | `nexus-agent/src/a2a_client.rs:45` | Change `tonic::` to `tonic_14::` (or add `use tonic_14 as tonic;`) |
| 2 | **Hardcoded API token** in tracked file | `nexus.toml:41` | Rotate the Cloudflare token; `git rm --cached nexus.toml`; ensure `.gitignore` prevents re-add |
| 3 | **Missing import** in build.rs (Windows) | `nexus-agent/build.rs` | Add `use std::process::Command;` inside the `#[cfg(target_os = "windows")]` function |

### P1 — Should fix (next sprint)

| # | Issue | Recommendation |
|---|-------|----------------|
| 4 | Invalid `hickory-dns` dependency | Remove from `nexus-infra/Cargo.toml`; `hickory-resolver` is already correctly declared |
| 5 | Empty v1.3 migration docs | Fill `docs/v1.3/migration-from-v1.2.md` and `security-overview.md` from STATUS.md history |
| 6 | Mixed `log`/`tracing` usage | Migrate remaining 12 `log`-using modules to `tracing`; remove `log` + `env_logger` from workspace |
| 7 | Module-level `#[allow(dead_code)]` | Narrow to specific items or remove; 6 crates suppress at module level |
| 8 | Test coverage for nexus-console | Add at minimum smoke tests for Tauri IPC commands |
| 9 | `acme-lib` 0.8 unmaintained | Evaluate replacement (e.g., `instant-acme`) or fork |

### P2 — Nice to have (backlog)

| # | Issue | Recommendation |
|---|-------|----------------|
| 10 | No public API rustdoc | Add `///` doc-comments to public types and functions, starting with `nexus-common` |
| 11 | No code coverage in CI | Add `cargo-tarpaulin` or `cargo-llvm-cov` to `ci.yml` |
| 12 | Dual tonic/rustls versions | Post-v1.4, consider upgrading `nexus-infra` to tonic 0.14 to unify |
| 13 | Legacy `nexus-webui` crate | Mark explicitly deprecated or remove if Tauri console is the sole UI path |
| 14 | 16 TODO stubs in grpc_server | Implement or remove placeholder RPCs that will never be filled |
| 15 | No disaster recovery docs | Document backup/restore for identity files, audit logs, and server state |
| 16 | No cert expiry monitoring script | Create `scripts/check-cert-expiry.sh` for operational alerting |
| 17 | Test coverage for nexus-recon, nexus-hybrid-exec | Both at 20% file coverage; add tests for core execution paths |
