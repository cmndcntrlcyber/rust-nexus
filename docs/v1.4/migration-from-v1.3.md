# v1.3 → v1.4 migration

v1.4 is **wire-additive, config-additive, and API-additive** —
existing v1.3 operators and agents keep working unchanged. New
defenses and integrations are opt-in.

## Wire additions

| New surface | Where | Notes |
|---|---|---|
| `IssueOperatorToken` RPC | `nexus_a2a::pb` | Server signs tokens with its NodeIdentity; clients cache + refresh at 50% lifetime |
| `StreamAuditRecords` RPC | `nexus_a2a::pb` | Server streams records to subscribers when `with_broadcast_audit(...)` is wired |
| `x-nexus-operator-token` gRPC metadata header | `nexus_a2a::tokens::TOKEN_METADATA_KEY` | Hex-encoded 97-byte token; server verifies before dispatch |
| `pb_upstream` proto module | `nexus_a2a::pb_upstream` | Vendored upstream A2A proto for compile-time wire-compatibility verification |

All v1.3 wire format (`shell-open`, `agent-register`, `Message`,
`Part`, audit records, capability JSON) remains unchanged.

## Config additions

| New section / file | Doc |
|---|---|
| `nexus-a2a/vendor/a2a-upstream/a2a.v1.proto` | Vendored upstream A2A proto stub. Operators replace via `scripts/vendor-a2a-proto.sh` |
| `nexus-a2a/VENDORED-VERSION` | Records the pinned upstream sha + sha256 fingerprint |
| `deploy/helm/nexus-server/values.yaml` | Helm-side config (mirrors kustomize) |
| `[auth] max_token_lifetime_seconds` in `nexus.toml` | Cap on token lifetime requested by `IssueOperatorToken` |
| `[otel] enabled / endpoint / service_name` in `nexus.toml` | OTel trace export (when `otel` Cargo feature is on) |

## API additions

| Builder / method | Module | Phase |
|---|---|---|
| `nexus_a2a::cards::sign / verify` (unchanged from v1.2 — used by tokens) | `nexus-a2a/src/cards.rs` | (v1.2) |
| `nexus_a2a::tokens::OperatorToken::issue / decode_verified` | `nexus-a2a/src/tokens.rs` | 1.4.7 |
| `nexus_a2a::tokens::TOKEN_METADATA_KEY` constant | `nexus-a2a/src/tokens.rs` | 1.4.7 |
| `nexus_a2a::audit::BroadcastSink::new / subscribe` | `nexus-a2a/src/audit.rs` | 1.4.3 |
| `nexus_a2a::audit::MultiSink::new` | `nexus-a2a/src/audit.rs` | 1.3.7 (reused) |
| `nexus_a2a::audit::AuditFilter::matches` | `nexus-a2a/src/audit.rs` | 1.4.3 |
| `nexus_a2a::otel::init_tracing_with_otel` (gated on `otel` feature) | `nexus-a2a/src/otel.rs` | 1.4.5 |
| `nexus_a2a::A2aServer::with_broadcast_audit` | `nexus-a2a/src/server.rs` | 1.4.3 |
| `nexus_a2a::A2aServer::with_server_identity` | `nexus-a2a/src/server.rs` | 1.4.3 / 1.4.7 |
| `nexus_a2a::A2aServer::with_max_token_lifetime` | `nexus-a2a/src/server.rs` | 1.4.7 |
| `nexus_a2a::pb_upstream::*` (vendored upstream client + messages) | `nexus-a2a/src/lib.rs` | 1.4.2 |
| `nexus_mesh::dtn::DtnQueue::{open, enqueue, drain_for, depth_for}` | `nexus-mesh/src/dtn.rs` | 1.4.10 |
| `nexus_mesh::dtn::publish_helpers::{publish_then_dtn, drain_on_reconnect}` | `nexus-mesh/src/dtn.rs` | 1.4.10 finish |
| `nexus_infra::mesh_listener::pump_mesh_decoded` | `nexus-infra/src/mesh_listener.rs` | 1.4.3 finish |
| `nexus_agent::audit::AgentAudit::open` + lifecycle helpers | `nexus-agent/src/audit.rs` | 1.4.9 |
| `nexus_a2a::audit_s3::S3SinkOptions` (config scaffold; real impl deferred) | `nexus-a2a/src/audit_s3.rs` | 1.4.9 partial |
| `ShellOpenParams.operator_token: Option<Vec<u8>>` | `nexus-a2a/src/handler.rs` | 1.4.7 |
| `CapabilityCheck` internals → `MatrixRouter` (public API preserved) | `nexus-a2a/src/capabilities.rs` | 1.4.6 |

## CI additions

| Workflow | Trigger | Purpose |
|---|---|---|
| `.github/workflows/docker.yml` | Tag push (`v*`) | Multi-arch (amd64 + arm64) Docker builds for both `nexus-server` and `nexus-agent`; pushes to configured registry with SBOM + provenance |

## Removed

Nothing. All v1.3 APIs + wire formats remain.

## Operational upgrade procedure

1. Build v1.4 binaries (`cargo build --release -p nexus-infra --bin nexus-server`).
2. Stop the v1.3 server: `systemctl stop nexus-server`.
3. Install the new binary in place.
4. **Optional v1.4 enablements**:
   - To enable operator tokens, configure `[auth]
     max_token_lifetime_seconds = 86400` (or your preferred cap) and
     update operator clients to call `IssueOperatorToken` after the
     mTLS handshake.
   - To stream audit records to operator consoles, wire a
     `BroadcastSink` around your existing `FileSink` in the server
     builder.
   - To enable OTel, rebuild with `--features otel` and set
     `[otel] enabled = true` in `nexus.toml`.
5. Start v1.4: `systemctl start nexus-server`.
6. Confirm metrics are scrapeable: `curl http://127.0.0.1:9100/metrics`.
7. Confirm the v1.4 regression tests pass against your staging stack
   before rolling to production.

Agents do not need to be restarted. v1.3 agents continue to work
against v1.4 servers; the new RPCs are server-additions, not
agent-side requirements.

## Helm migration from kustomize (optional)

Operators using kustomize can stay on kustomize. To switch to Helm:

```bash
# Delete the kustomize-managed resources.
kubectl -n nexus delete -k deploy/k8s/overlays/prod/

# Install via Helm with the same TLS material.
helm install nexus-server ./deploy/helm/nexus-server \
    --namespace nexus --create-namespace \
    --set image.repository=ghcr.io/yourorg/nexus-server \
    --set image.tag=v1.4.0 \
    --set-file tls.caCert=./certs/ca.crt.pem \
    --set-file tls.serverCert=./certs/server.crt.pem \
    --set-file tls.serverKey=./certs/server.key.pem
```

The Helm chart's StatefulSet shape mirrors the kustomize base 1:1 —
the persistent volume + identity blob + audit log all migrate
transparently if the PVC name matches.

## Bumping the A2A upstream proto pin

```bash
./scripts/vendor-a2a-proto.sh v0.3.1
cargo test -p nexus-a2a --test upstream_compat
```

If the test passes the bump is drift-free; commit
`nexus-a2a/vendor/a2a-upstream/a2a.v1.proto`,
`nexus-a2a/VENDORED-VERSION`, and any test updates.

If the test fails:

- **`tonic-prost-build` codegen errors** → upstream proto changed
  the syntax/messages enough that build-time decoding mismatches.
  Reconcile by either adjusting our proto's field numbers (only for
  rust-nexus additions; the v1.4 additions are field 5/6 on
  `AgentCard` and the new RPCs at the end of the service block) or
  by adding a `serde`-style alias in the test glue.
- **Symmetric round-trip decode errors** → a shared message changed
  shape in a way prost can't reconcile. Update our proto to match
  upstream's new shape; ensure our v1.2 + v1.4 additions still fit
  via additive field numbers.
