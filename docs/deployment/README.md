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
  [`../v1.2/security-overview.md`](../v1.2/security-overview.md) for
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
+--------------------+        operator-A2A         +--------------------+
|  nexus-console     | <─── mTLS + signed card ──► |  nexus-infra       |
|  Tauri + Leptos    |   :50052 (Tonic 0.14)       |  C2 server         |
|  + xterm.js        |                             |  (NodeIdentity,    |
+--------------------+                             |   AgentChannels,   |
                                                   |   capability       |
+--------------------+      v1.2 agent-mode        |   matrix, audit    |
|  nexus-agent       | <─ mTLS + AgentRegister ─►  |   sink, rate       |
|  (PTY shell,       |    :50052 first frame       |   limiter)         |
|   OS-aware shell   |    + per-session task_id    |                    |
|   select)          |                             +────┬───────────────+
+--------------------+                                  │
                                                        │ (Tonic 0.10 lane
+--------------------+     legacy task-pull             │  unchanged)
|  overlay agents    | <────────────────────────────────┘
|  (Cloudflare DNS,  |   :50051
|   BOF, fiber...)   |
+--------------------+
```

Two gRPC services run on the same `nexus-infra` server process but on
separate ports:

- `:50052` — A2A plane (Tonic 0.14, v1.2 mTLS, signed AgentCards,
  capability matrix, audit log, agent-mode bidi).
- `:50051` — Legacy NexusC2 plane (Tonic 0.10). Overlay agents from the
  v1.0 era register here and pull tasks. This lane is untouched by v1.2.

## Where to find more

- **Wire reference**: [`../v1.0/shell-session-protocol.md`](../v1.0/shell-session-protocol.md)
  — the `shell-session` skill framing (also updated for v1.2's
  `agent-register` control frame).
- **API additions in v1.2**: [`../v1.2/migration-from-v1.1.md`](../v1.2/migration-from-v1.1.md).
- **Defense matrix + threat model**: [`../v1.2/security-overview.md`](../v1.2/security-overview.md).
- **Tauri codesigning (CI)**: [`../v1.2/codesigning.md`](../v1.2/codesigning.md).
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
