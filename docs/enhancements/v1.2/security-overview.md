# v1.2 security overview

v1.2 ships seven independent defenses on the A2A plane plus Tauri
bundle codesigning. Each is opt-in via builder API; operators wire
them up in `nexus-infra::serve::run_a2a`. This doc maps each defense
to its module, configuration knob, and verification command.

For deployment guidance see [`../deployment/README.md`](../deployment/README.md).

| Defense | Module | Config knob | Verifies via |
|---|---|---|---|
| mTLS | `nexus_a2a::tls` | `NEXUS_*_CERT` env vars + `A2aClient::connect_with_optional_tls` | `cargo test -p integration-tests --test a2a_mtls -- --ignored` |
| Signed AgentCards | `nexus_a2a::cards` | `nexus_a2a::cards::sign(&mut card, &identity)` at server build | `cargo test -p nexus-a2a cards::` (6 tests) |
| Capability matrix | `nexus_a2a::capabilities` | `OperatorRouter::with_capability_check(CapabilityCheck::from_json_file(...))` | `cargo test -p nexus-a2a capabilities::` (4 tests) |
| Hash-chained audit log | `nexus_a2a::audit` | `FileSink::open(path)` + `A2aServer::with_audit_sink(...)` (v1.3 wiring; v1.2 ships the surface) | `cargo test -p nexus-a2a audit::` (4 tests) + `cargo run --bin audit_verify -- path` |
| Rate limit | `nexus_a2a::interceptors::RateLimitInterceptor` | `RateLimitInterceptor::new(rps).verify(&peer)` | `cargo test -p nexus-a2a interceptors::` (4 tests) |
| Message size cap | `nexus_a2a::interceptors::MAX_MESSAGE_SIZE` | applied by `into_service`, default 4 MiB | `cargo test -p nexus-a2a` against oversized payloads |
| Reflection-off in release | Cargo feature `dev-reflection` (off in release) | `cargo build --release` does not include reflection | `grpcurl describe` returns `Unimplemented` against release server |
| Tauri bundle codesigning | `.github/workflows/tauri-build.yml` | CI secrets (D-V1.2-F) | `spctl --assess` / `signtool verify` |

## Threat model (v1.2 scope)

- **Operator-to-C2 wire**: mTLS protects against passive eavesdropping
  and active MITM. Signed AgentCards prevent a malicious or compromised
  C2 from impersonating the genuine server.
- **Operator-to-agent dispatch**: the capability matrix prevents a
  rogue operator (with valid mTLS credentials) from invoking skills on
  agents they're not authorized to control.
- **Operator-to-server abuse**: the rate limit + message-size cap
  bound damage from a compromised operator account or buggy client.
  Reflection-off in release reduces info-leakage to network attackers.
- **Forensics**: the hash-chained audit log makes after-the-fact
  tampering detectable.

## What v1.2 explicitly does NOT defend against

- **Compromised server-side keys**: operators trust the C2's identity.
  If the C2's `NodeIdentity` is exfiltrated, a clone is indistinguishable
  from the genuine server. Mitigation: out-of-band attestation (TPM,
  SGX) is v1.3+ work.
- **Compromised agent-side keys**: same problem; an attacker with the
  agent's identity can register as that agent. Mitigation as above.
- **Compromised C2 host (process-level)**: an attacker with code
  execution on the C2 can append arbitrary audit records or rewrite
  the log file. Mitigation: ship audit records to an external sink
  (v1.3 work — `AuditSink` trait is already pluggable).
- **Side-channel + timing attacks**: out of scope.
- **Compromised CA**: the CA in `scripts/gen-certs.sh` is dev-only.
  Production deployments must provision their own CA with appropriate
  offline / HSM-backed key management — see
  [`../deployment/production.md#ca-strategy`](../deployment/production.md#ca-strategy).

## Verification matrix

Run all of the following from the workspace root. Green at every step
means v1.2 is intact.

```bash
# 1. Workspace builds clean.
cargo check --workspace --exclude nexus-console

# 2. All v1.2 unit / integration tests pass (≥170 tests).
cargo test --workspace --exclude nexus-console

# 3. mTLS round-trip works with operator-provided certs.
./scripts/gen-certs.sh
NEXUS_CA_CERT=$(pwd)/certs/ca.crt.pem \
NEXUS_SERVER_CERT=$(pwd)/certs/server.crt.pem \
NEXUS_SERVER_KEY=$(pwd)/certs/server.key.pem \
NEXUS_CLIENT_CERT=$(pwd)/certs/client.crt.pem \
NEXUS_CLIENT_KEY=$(pwd)/certs/client.key.pem \
cargo test -p integration-tests --test a2a_mtls -- --ignored

# 4. Headless demo gate still PASSes.
./scripts/demo.sh
```

## Related docs

- [`../deployment/production.md`](../deployment/production.md) —
  production hardening checklist + how to wire each defense.
- [`../deployment/operations.md`](../deployment/operations.md) —
  cert / identity rotation, audit log retention, incident response.
- [`codesigning.md`](codesigning.md) — Tauri bundle CI signing schema.
- [`migration-from-v1.1.md`](migration-from-v1.1.md) — wire / API
  changes introduced in v1.2.
