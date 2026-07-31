# Dependency Strategy

## Dual-Version Dependencies

The workspace intentionally carries two versions of several crates to
support the transition from the legacy gRPC layer (nexus-infra, tonic 0.10)
to the A2A layer (nexus-a2a, tonic 0.14).

| Dependency | Versions | Reason |
|---|---|---|
| tonic | 0.10.2 + 0.14.6 | nexus-infra legacy gRPC (0.10) vs nexus-a2a upstream (0.14) |
| prost | 0.12.6 + 0.14.3 | follows the tonic split (each tonic pins its prost) |
| rustls | 0.21.12 + 0.23.x | tonic 0.10 → rustls 0.21; tonic 0.14 → rustls 0.23 (transitive) |

### Binary size impact

The dual versions add ~20-30% to the release binary compared to a
single-version build. This is acceptable during the transition period.

### Migration plan

When nexus-infra's legacy gRPC server (`grpc_server.rs`, `grpc_client.rs`)
is decommissioned in favour of the A2A plane, the workspace can:

1. Upgrade nexus-infra to tonic 0.14 (remove tonic 0.10 / prost 0.12).
2. Drop the `tonic_14` aliases — all crates use plain `tonic`.
3. Collapse to a single rustls version.

## Pre-1.0 and Unmaintained Dependencies

| Crate | Version | Status | Risk |
|---|---|---|---|
| `pwsh` | 0.1.0 | Pre-1.0 stub | LOW — disabled by default in nexus-hybrid-exec |
| `rquickjs` | 0.4 | Outdated (0.5+ available) | LOW — optional feature in nexus-recon |

Both are behind feature flags and are not compiled into default builds.
Upgrade when their APIs stabilise.

## Removed Dependencies (v1.5)

| Crate | Replaced by | Reason |
|---|---|---|
| `acme-lib` 0.8 | `instant-acme` 0.8 | Unmaintained since 2021 (HIGH risk); no security patches |
| `openssl` 0.10 (vendored) | *(removed)* | Only needed as transitive dep of acme-lib |
| `hickory-dns` 0.24 | *(removed)* | Binary-only crate, no lib target; code uses `hickory-resolver` |
| `log` 0.4 / `env_logger` 0.10 | `tracing` 0.1 | Unified logging on `tracing` (see `logging-standard.md`) |

## Supply Chain Auditing

`deny.toml` enforces:
- License allowlist (Apache-2.0, MIT, BSD, MPL-2.0; AGPL hard-denied)
- Advisory database checks via `cargo-deny`
- CI runs `security-audit.yml` on every push and weekly schedule
