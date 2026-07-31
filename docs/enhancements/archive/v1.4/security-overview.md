# v1.4 security overview

> See [`../../../deployment/README.md`](../../../deployment/README.md) for
> deployment guidance, [`../v1.3/security-overview.md`](../v1.3/security-overview.md)
> for the v1.3 defense matrix, and
> [`../../agent-swarm-mesh/v1.2/security-overview.md`](../../agent-swarm-mesh/v1.2/security-overview.md) for
> the original v1.2 baseline.

v1.4 extends the v1.3 matrix with **eight** new defenses and closes
two operational gaps that v1.3 left as foundational scaffolding.

## v1.4 additions

| Defense | Module | Status |
|---|---|---|
| Pure-Rust upstream A2A wire interop | `nexus_a2a::pb_upstream` + `tests/upstream_compat.rs` | Shipped |
| Ed25519-signed operator tokens (decoupled from cert lifetime) | `nexus_a2a::tokens::OperatorToken` | Shipped (issue + verify + gRPC metadata extraction) |
| `IssueOperatorToken` A2A RPC | `nexus_a2a::pb::IssueOperatorTokenRequest/Reply` | Shipped |
| `StreamAuditRecords` A2A RPC (broadcast audit fan-out to subscribers) | `nexus_a2a::pb::StreamAuditRecordsRequest/AuditRecordEvent` + `BroadcastSink` | Shipped |
| `AuditFilter` (actor / action / since_unix) for the streaming RPC | `nexus_a2a::audit::AuditFilter` | Shipped |
| `nalgebra`-backed MatrixRouter for capability composition | `nexus_a2a::capabilities::CapabilityCheck` (internals) | Shipped |
| Per-agent audit log | `nexus_agent::audit::AgentAudit` | Shipped |
| DTN store-and-forward queue | `nexus_mesh::dtn::DtnQueue` + `publish_helpers::*` | Shipped |
| OTel OTLP/gRPC trace export | `nexus_a2a::otel::init_tracing_with_otel` (behind `otel` feature) | Shipped |
| Mesh listener payload pump | `nexus_infra::mesh_listener::pump_mesh_decoded` | Shipped |
| Multi-arch Docker (`linux/amd64` + `linux/arm64`) | `.github/workflows/docker.yml` | Shipped |
| Helm chart | `deploy/helm/nexus-server/` | Shipped |

## v1.4 deferrals (queued for v1.5)

| Item | Why deferred |
|---|---|
| ACME workflow re-port (Phase 1.4.1) | acme-lib 0.8 API port; deferred again because the staging endpoint needs network access not available in the dev sandbox |
| Tauri audit log viewer UI | Leptos component work; backend RPC ready, UI iteration deferred |
| Real `S3Sink` impl | `aws-sdk-s3` integration; configuration scaffold shipped, real upload buffer + retry deferred |
| DTN integration into `MeshNode` publish path | libp2p Swarm timeout coupling; `publish_helpers` exposed as caller-driven API so operators can wire DTN today without waiting for the Swarm integration |

## Threat model updates from v1.3

### Operator identity decoupled from mTLS certs

v1.3 keyed per-operator scoping on TLS client cert CN. v1.4
operators can additionally present a **signed token** (D-V1.4-D) on
each RPC via the `x-nexus-operator-token` gRPC metadata header.
This decouples:

- **Cert rotation cadence** (typically months/years) from **operator
  identity rotation cadence** (now 24h by default, configurable via
  `[auth] max_token_lifetime_seconds`).
- **Multiple operators sharing a host** can each present distinct
  tokens issued by `IssueOperatorToken` without re-keying the host's
  TLS material.

Wire format (97 bytes total):

```
[ ver(1) ][ operator_id(16) ][ issued(8) ][ expires(8) ][ ed25519_sig(64) ]
```

The Ed25519 signature covers the 33-byte prefix and is produced by
the server's `NodeIdentity`. Verifiers know the server's Ed25519
public key (the same one that signs `AgentCard`s — see
`nexus_a2a::cards`).

**What it doesn't defend against**: a compromised C2 host can
manufacture tokens for any operator identity. This is the same trust
boundary as v1.3 cert CN scoping — the server is the root of trust
either way.

### Audit log streaming to remote subscribers

v1.4's `StreamAuditRecords` RPC lets operator consoles tail the
audit log in real time. The broadcast tap (`BroadcastSink`) is
**non-authoritative** — the chain head still lives in the inner sink
(typically `FileSink`). Lagging subscribers see a `DataLoss` status
when the broadcast buffer overflows, which is documented behavior:
audit subscribers must accept the trade-off between live tailing and
guaranteed delivery (the file is always authoritative).

**Filter trust**: `AuditFilter` is applied on the server side, so
subscribers can't bypass filtering by sending an empty filter and
discarding records locally — they receive only records matching the
filter, full stop.

### MatrixRouter compositional gating

v1.4 replaces v1.3's `HashMap<peer_id, HashSet<skill>>` lookup with
three dense matrices (`agent_skills`, `operator_agents`,
`operator_skills`) plus wildcard hash-set overlays. The verification
path is now a triple-AND of scalar boolean lookups — sub-microsecond
per check, and naturally extends to v1.5 compositional matrix
operations (adjacency × capability × trust).

Same public API as v1.3 (`verify`, `verify_with_operator`); same
JSON config format. The refactor is internal; pre-v1.3 capability
files keep working.

### DTN as availability guarantee, not delivery confirmation

`DtnQueue` persists outbound envelopes to
`<root>/<peer_id_hex>/<unix_seconds>-<seq>.bin` when the recipient
is deemed offline. Queue depth + age are bounded (default 1000
entries, 7 days); oldest-first eviction.

**Threat model**:

- Queued envelopes remain **sealed** (`SealedEnvelope` ciphertext).
  A compromised sender disk can prevent delivery but cannot decrypt
  the queued payload.
- DTN doesn't certify delivery — it only retries. Senders that need
  confirmed delivery must implement application-level acks.
- The queue's persistence layer is the local filesystem; operators
  who don't trust their disk can mount a tmpfs at the queue root
  (lose the queue on reboot).

## Observability surface (v1.3 + v1.4)

v1.4 reuses v1.3's Prometheus exposition. Operators MAY add per-v1.4
counters in their custom server wrapper:

- `nexus_a2a_audit_stream_subscribers` — current `StreamAuditRecords`
  subscriber count
- `nexus_a2a_operator_tokens_issued_total{operator}` — issuance rate
- `nexus_mesh_dtn_queue_depth{recipient}` — per-recipient queue depth

These aren't auto-registered in v1.4 (`Metrics::global()` is
extensible by operators). v1.5 will fold them into the canonical
registry.

## Verification

```bash
cargo test --workspace --exclude nexus-console
# target ≥ 221 tests pass
cargo test -p nexus-a2a --test upstream_compat
# 4 tests: pure-Rust interop against the vendored upstream stub
cargo test -p nexus-a2a --test loopback v1_4_audit_streaming
# end-to-end: audit broadcast + token issuance + token-in-metadata
cargo test -p integration-tests --test v1_4_regression
# 5 tests covering MatrixRouter, tokens, DTN, MultiSink, per-agent audit
./scripts/demo.sh
# [demo] PASS
```

## Related docs

- [`migration-from-v1.3.md`](migration-from-v1.3.md) — wire / config
  changes between v1.3 and v1.4
- [`../deployment/production.md`](../deployment/production.md) —
  hardening checklist (now includes operator token guidance)
- [`../deployment/operations.md`](../deployment/operations.md) —
  cert rotation, identity rotation, audit retention, capability
  updates, **operator token rotation** (new section in v1.4)
