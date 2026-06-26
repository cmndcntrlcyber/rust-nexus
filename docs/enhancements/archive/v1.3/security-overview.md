# v1.3 security overview

> See [`../../deployment/README.md`](../../deployment/README.md) for
> deployment guidance and the v1.2 security overview for the original
> baseline.

v1.3 extends the v1.2 defense matrix with **six** new defenses focused
on observability, supply chain, and mesh networking foundations.

## v1.3 defense matrix

| Defense | Module | Status |
|---|---|---|
| AES-256-GCM message encryption | `nexus_common::crypto` | Shipped (v1.0) |
| Ed25519 node identity + X25519 DH | `nexus_common::identity` | Shipped (v1.1) |
| Sealed-envelope authenticated encryption | `nexus_common::sealed` | Shipped (v1.1) |
| Replay window (64-packet sliding) | `nexus_common::sealed::ReplayWindow` | Shipped (v1.1) |
| mTLS (Ed25519 CA + per-entity certs) | `nexus-infra/src/pki.rs` | Shipped (v1.2) |
| BLAKE3 hash-chained audit log | `nexus_a2a::audit::AuditChain` | Shipped (v1.2) |
| Capability matrix (RBAC) | `nexus_a2a::capabilities::CapabilityCheck` | Shipped (v1.2) |
| Prometheus metrics (pull-based) | `nexus_a2a::metrics` + `nexus_infra::metrics_server` | **Shipped (v1.3)** |
| SyslogSink (RFC 5424 TCP) | `nexus_a2a::audit::SyslogSink` | **Shipped (v1.3)** |
| MultiSink (file + syslog fanout) | `nexus_a2a::audit::MultiSink` | **Shipped (v1.3)** |
| Per-operator scoping | `nexus_a2a::interceptors::verify_with_operator` | **Shipped (v1.3)** |
| Mesh listener with sealed-envelope decoder | `nexus_infra::mesh_listener` | **Shipped (v1.3)** |
| `cargo-deny` supply chain audit | `deny.toml` + `.github/workflows/security-audit.yml` | **Shipped (v1.3)** |

## Supply chain policy

`deny.toml` enforces:
- **License allowlist:** Apache-2.0, MIT, BSD-2/3-Clause, ISC, MPL-2.0,
  Unicode, Zlib, OpenSSL, CC0.
- **AGPL hard-deny:** implements D-2.1.2-A ("no AGPL code paths").
- **Advisory database check:** flags known CVEs in dependencies.
- **Yanked crate detection:** warns on yanked versions.

CI runs `cargo-deny check` on every push and on a weekly cron schedule.

## Audit integrity

The BLAKE3 hash-chained audit log (shipped in v1.2) now supports:
- **SyslogSink:** streams records to external SIEM via RFC 5424 TCP.
- **MultiSink:** combines file, syslog, and broadcast sinks for
  defense-in-depth audit shipping.
- **Offline verification:** `audit_verify` CLI validates the hash chain
  without network access.

## Metrics endpoint security

The Prometheus `/metrics` endpoint runs on a **separate plaintext HTTP
server** bound to `127.0.0.1:9100` by default.

- Never expose port 9100 to the WAN (unauthenticated).
- Firewall the port to localhost or the monitoring VLAN.
- mTLS for the metrics endpoint is planned for v1.5.

## v1.3 deferrals (queued for v1.4)

| Item | Why deferred |
|---|---|
| ACME workflow re-port | acme-lib 0.8 API port requires network staging endpoint |
| Operator tokens (decoupled from cert lifetime) | Requires v1.4 identity decoupling |
| Audit streaming RPC | BroadcastSink infrastructure shipped; RPC surface deferred |
| OTel trace export | Tracing subscriber wired; OTLP export deferred to v1.4 |
