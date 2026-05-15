# rust-nexus — Project Roadmap

Project-wide versioning roadmap. Sibling tracks (agent-swarm-mesh, ATT&CK technique modules, infrastructure work) sequence here. For the agent-swarm-mesh track specifically, see [`enhancements/agent-swarm-mesh/ROADMAP.md`](enhancements/agent-swarm-mesh/ROADMAP.md).

---

## Versioning policy

rust-nexus uses **two distinct version axes**:

| Axis | What it numbers | Where it lives |
|---|---|---|
| **Doc-track version** (`v2.1`, `v2.1.1`, `v2.1.2`, …) | An *enhancement initiative* — a coherent slice of capability spanning one or more crates | Doc filenames under `docs/enhancements/<track>/v<X.Y.Z>-*.md` |
| **Crate version** (`package.version` in each `Cargo.toml`) | Per-crate semver | Each workspace member's `Cargo.toml` |

The two axes are decoupled by design:

- A single doc-track version (e.g., `v2.1`) typically touches **multiple crates** and bumps each independently per semver. `nexus-mesh` may go `0.1 → 0.2 → 1.0` over the course of `v2.1`'s 10 phases while `nexus-common` only adds a minor (`0.1 → 0.2` for the `NodeIdentity` addition in Phase 1).
- A **project-wide git tag** (`nexus-2.1.0`, `nexus-2.1.2`, …) marks the commit at which a doc-track version is considered shipped. This is the operator-facing version number.

All workspace crates currently sit at `0.1.0`; this is the v2.0 baseline. The first crate-version bump should happen when a doc-track version begins landing user-visible work (v2.1 Phase 7 is the earliest such moment — see the swarm-mesh implementation plan).

### Action item: nexus-mesh Cargo.toml version-string reconciliation

`nexus-mesh/Cargo.toml` currently has:

```toml
description = "libp2p-based peer-to-peer mesh layer for rust-nexus (v2.9.6)"
```

The `v2.9.6` string has no anchor anywhere else in the repo and conflicts with the `v2.1.x` doc-track filenames. **Fix**: change the description string to reference `v2.1` (matching the doc track) when the Phase 0 wiring lands. Tracked in [`enhancements/agent-swarm-mesh/STATUS.md`](enhancements/agent-swarm-mesh/STATUS.md) as a Phase 0 gap.

---

## Track register

| Track | Doc-track version | Status | Scope | Detail |
|---|---|---|---|---|
| **Baseline** | `v2.0` | ✓ shipped | The 7 original crates (`nexus-common`, `nexus-infra`, `nexus-agent`, `nexus-webui`, `nexus-recon`, `nexus-hybrid-exec`, `nexus-web-comms`) + initial ATT&CK technique crates (T1059, T1547, T1021.006) | Reflected in `Cargo.toml` workspace `members` |
| **Agent swarm mesh** | `v2.1`, `v2.1.1`, `v2.1.2` | ▶ in progress | Peer-to-peer mesh transport, GML telemetry noise-barometer, MatrixA2A protocol interop | [Track roadmap](enhancements/agent-swarm-mesh/ROADMAP.md) · [Status](enhancements/agent-swarm-mesh/STATUS.md) |
| **ATT&CK technique modules** | `v2.0.x` (additive) | ✓ ongoing | New technique-specific crates added as needed (next likely candidates: T1055 process injection, T1003 credential dumping) | Each new technique is an additive minor bump |
| **Web-comms hardening** | `v2.2` (placeholder) | ⏸ unscheduled | Tightening `nexus-web-comms` fallback ordering, domain-fronting robustness, traffic-obfuscation profiles | No design doc yet |
| **Future tracks** | `v2.3+` | ⏸ unscheduled | Reserved | — |

---

## Cross-track dependencies

- **v2.1.1 GML → v2.1 mesh.** The GML noise-barometer ingests temporal graph snapshots derived from mesh telemetry (Gossipsub events, peer connectivity, message volume). v2.1 Phase 4 (Gossipsub) must be live before v2.1.1's R2 milestone can use real data.
- **v2.1.1 GML throttling → v2.1.2 MatrixA2A.** The control loop's per-agent attribution surface (edge-attention weights → per-agent rate adjustment) lands cleanest if there's already a structured agent-skill matrix to address. MatrixA2A's `CapabilityMatrix` and per-agent `AgentCard` give the GML controller named handles to throttle.
- **v2.1.2 MatrixA2A ↔ v2.1 mesh.** Both define transports. The decision is **not** "which one wins" — they solve different problems:
  - libp2p mesh (v2.1) = C2 reachback when gRPC is blocked.
  - MatrixA2A gRPC (v2.1.2) = structured agent↔agent dialogue over the Linux-Foundation A2A protocol.
  - Default fallback ordering once both ship: gRPC (Tonic, primary) → mesh (libp2p, when firewalled) → HTTP/WebSocket. MatrixA2A messages may optionally tunnel over mesh (v2.1 Phase 7 onwards).

---

## Recommended release ordering

```
v2.0 (shipped)
  └─► v2.1            mesh transport       (Phases 0–7 user-visible; 8–10 hardening)
        ├─► v2.1.2    MatrixA2A interop    (depends on mesh transport story for Phase 7 fallback)
        └─► v2.1.1    GML noise-barometer  (depends on mesh telemetry + A2A control surface)
                       Research first (R1–R5), then a future implementation plan
```

Rationale for putting v2.1.2 ahead of v2.1.1:

1. v2.1.2 unblocks Python-side red/blue team adapters (rtpi, btpi, d3tect-nexus) — high external value, mostly orthogonal to v2.1 internals.
2. v2.1.1 needs **both** v2.1 (telemetry source) and v2.1.2 (per-agent control surface) to be useful end-to-end. Doing it third avoids re-architecting the control loop.
3. v2.1.1 is currently research-phase only; sequencing it last gives v2.1 and v2.1.2 time to produce real telemetry against which the GML model can be trained.

---

## Open project-wide decisions

| Decision | Owner | Blocks | Notes |
|---|---|---|---|
| Crate-version bump policy (per-crate semver vs lockstep) | — | First v2.1 release | Recommendation: per-crate semver; `nexus-common` is the only one likely to need coordinated bumps because its types cross crate boundaries |
| Project-wide git-tag scheme (`nexus-2.1.0` vs `v2.1.0` vs `release/2.1.0`) | — | First v2.1 release | Default: `nexus-<doc-track-version>` to avoid collision with crate tags |
| Where ATT&CK technique crates live in the version timeline (additive v2.0.x bumps vs new track) | — | Next T-crate addition | Current pattern is additive; keep until 5+ T-crates exist, then consider track |

---

## Maintenance

This file is the index for all version-tracked work. When opening a new enhancement track:

1. Create `docs/enhancements/<track-name>/` with at least a `ROADMAP.md` and `STATUS.md`.
2. Add a row to the track register above.
3. Add cross-track dependency notes if relevant.

Last updated: 2026-05-15.
