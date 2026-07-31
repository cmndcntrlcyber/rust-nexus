# Deployment

This tree is the canonical entry point for running rust-nexus —
locally for development or as a production service.

## Who this is for

- **Operators / red-teamers** running rust-nexus against authorized
  engagements. See [`production.md`](production.md) and
  [`operations.md`](operations.md).
- **Developers** onboarding to the codebase or evaluating the project.
  See [`local-dev.md`](local-dev.md).
- **Security reviewers** auditing the deployment surface. See
  [`../enhancements/agent-swarm-mesh/v1.2/security-overview.md`](../enhancements/agent-swarm-mesh/v1.2/security-overview.md) for
  the defense matrix.

## Deployment modes

| Mode | Doc | Audience |
|---|---|---|
| Local single-box dev demo | [`local-dev.md`](local-dev.md) | Developers, evaluators. Clone → demo PASS in ~15 minutes. |
| Production rollout | [`production.md`](production.md) | Operators provisioning a real C2 server + agents on dedicated infrastructure. |
| Operator console (Tauri bundle) | [`operator-console.md`](operator-console.md) | Anyone distributing or installing the desktop client. |
| Day-2 operations | [`operations.md`](operations.md) | On-call operators — cert/identity rotation, audit log retention, incident response. |

## Architecture at a glance

```
+--------------------+        mTLS :50052          +--------------------+
|  nexus-console     | <─── signed card ─────────► |  nexus-infra       |
|  Tauri + Leptos    |   (NEXUS_CA_CERT +           |  C2 server         |
|  + xterm.js        |    NEXUS_CLIENT_CERT/KEY)    |  (NodeIdentity,    |
+--------------------+                             |   AgentChannels,   |
                                                   |   capability       |
+--------------------+      v1.2 agent-mode        |   matrix, audit    |
|  nexus-agent       | <─ mTLS + AgentRegister ─►  |   sink, rate       |
|  (PTY shell,       |    :50052                   |   limiter)         |
|   OS-aware shell   |                             +────┬───────────────+
|   select)          |                                  │
+--------------------+                                  │ (Tonic 0.10 lane
                                                        │  unchanged)
+--------------------+     legacy task-pull             │
|  overlay agents    | <────────────────────────────────┘
|  (v1.0 era)        |   :50051
+--------------------+
```

All clients connect **directly** to the C2 server on port **50052** over
mTLS using a custom CA. Two gRPC services run on the same `nexus-infra`
server process:

- `:50052` — A2A plane (Tonic 0.14, v1.2 mTLS, signed AgentCards,
  capability matrix, audit log, agent-mode bidi).
- `:50051` — Legacy NexusC2 plane (Tonic 0.10). Overlay agents from the
  v1.0 era register here.

## Where to find more

- **Wire reference**: [`../enhancements/agent-swarm-mesh/v1.0/shell-session-protocol.md`](../enhancements/agent-swarm-mesh/v1.0/shell-session-protocol.md)
  — the `shell-session` skill framing (also updated for v1.2's
  `agent-register` control frame).
- **API additions in v1.2**: [`../enhancements/agent-swarm-mesh/v1.2/migration-from-v1.1.md`](../enhancements/agent-swarm-mesh/v1.2/migration-from-v1.1.md).
- **Defense matrix + threat model**: [`../enhancements/agent-swarm-mesh/v1.2/security-overview.md`](../enhancements/agent-swarm-mesh/v1.2/security-overview.md).
- **Tauri codesigning (CI)**: [`../enhancements/agent-swarm-mesh/v1.2/codesigning.md`](../enhancements/agent-swarm-mesh/v1.2/codesigning.md).
- **All-in-one server deployment**: [`../../scripts/deploy-server-prod.sh`](../../scripts/deploy-server-prod.sh)
  — single-script alternative to the multi-step `transfer-prep.sh` + `remote-host-prep.sh` workflow.
- **Current status + version**: [`../../STATUS.md`](../../STATUS.md),
  [`../../ROADMAP.md`](../../ROADMAP.md).

## Conventions used in these docs

- All paths are relative to the workspace root unless otherwise marked.
- Code references use ``backticks`` and link to the file via path —
  e.g. ``nexus-a2a/src/tls.rs``.
- Decisions reference IDs from `ROADMAP.md` — e.g. **D-V1.2-mtls**,
  **D-DEPLOY-B**.
- Environment variable names are reserved and load-bearing:
  `NEXUS_CA_CERT`, `NEXUS_SERVER_CERT`, `NEXUS_SERVER_KEY`,
  `NEXUS_CLIENT_CERT`, `NEXUS_CLIENT_KEY`. Never rename.
