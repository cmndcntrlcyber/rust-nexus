# v1.1 → v1.2 migration

v1.2 is **wire-additive** — existing v1.1 operators and agents keep
working unchanged. The new defenses are opt-in.

## Proto changes

`AgentCard` gains two new fields:

```proto
message AgentCard {
    string name = 1;
    string description = 2;
    string version = 3;
    repeated AgentSkill skills = 4;
    bytes signature = 5;           // v1.2: D-V1.2-cards
    bytes signer_peer_id = 6;      // v1.2: D-V1.2-cards
}
```

v1.1 clients ignore unknown fields, so v1.2 servers signing their card
remain readable. v1.2 clients calling `get_agent_card()` (the existing,
unverified accessor) get back the same struct; only the new
`get_agent_card_verified()` API rejects tampered / unsigned cards.

## Framing additions

`ShellControl` gains an `AgentRegister` variant. v1.1 servers that
receive this frame on a `SendStreamingMessage` stream now reject with
`InvalidArgument` (unchanged); v1.2 servers route it to the new
`AgentRegistrationHandler` instead.

```jsonc
// First frame on an agent-mode stream (v1.2):
{"kind":"agent-register","peer_id_hex":"ab12…","os":"linux","version":"0.2.0","tag":"prod-host-1"}
```

## Config / env vars

New, all optional:

| Variable | Purpose |
|---|---|
| `NEXUS_CA_CERT` | mTLS CA bundle (path or inline PEM) |
| `NEXUS_SERVER_CERT` | Server cert |
| `NEXUS_SERVER_KEY` | Server key |
| `NEXUS_CLIENT_CERT` | Client cert |
| `NEXUS_CLIENT_KEY` | Client key |

v1.1 deployments without these vars continue to run plaintext over the
loopback gate.

## API additions

| Builder | Source | Effect |
|---|---|---|
| `A2aServer::with_agent_registration(handler)` | `nexus-a2a/src/server.rs` | Accept agent-mode bidi streams |
| `A2aServer::serve_with_optional_tls(addr, insecure, tls, shutdown)` | `nexus-a2a/src/server.rs` | mTLS at bind time |
| `A2aClient::connect_with_optional_tls(addr, insecure, tls)` | `nexus-a2a/src/client.rs` | mTLS at dial time |
| `A2aClient::get_agent_card_verified()` | `nexus-a2a/src/client.rs` | Reject tampered cards |
| `OperatorRouter::with_capability_check(check)` | `nexus-infra/src/a2a_router.rs` | Per-agent skill allowlist |
| `A2aSharedState::with_server_identity(identity)` | `nexus-infra/src/serve.rs` | Sign AgentCard at server build |

## Removed

Nothing. All v1.1 APIs and wire formats remain.

## Breaking compile-time changes (minor)

- `nexus_a2a::pb::AgentCard` struct literals must now include
  `signature: Vec::new(), signer_peer_id: Vec::new(),` (or be built
  via the prost-generated `Default` impl).

## Deferred to v1.3

- Full upstream A2A proto vendoring with `Unimplemented` stubs for
  `GetTask`, `CancelTask`, `TaskSubscription`, etc. (see
  `nexus-a2a/VENDORED-VERSION`).
- Server-side mesh listener for shell sessions (v1.2 mesh is agent-
  only; operators must use the gRPC mTLS path for interactive shells).
- ACME / Let's Encrypt cert provisioning (stubbed to defer in v1.2.1;
  use `scripts/gen-certs.sh` or an external CA in v1.2).
- `nalgebra`-backed MatrixRouter for capability composition.
- Hash-chained audit log syslog / remote-collector backends.
