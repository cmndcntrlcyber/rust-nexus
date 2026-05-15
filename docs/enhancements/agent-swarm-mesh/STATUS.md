# Agent Swarm Mesh — Implementation Status Tracker

Live status of every phase in the `v2.1.x` track. Update fields below in the **same commit** as the work that changes them — do not defer to a follow-up commit.

For sequencing context, see [`ROADMAP.md`](ROADMAP.md). For the implementation plans this file tracks, see the per-version `*-plan.md` files.

Last updated: 2026-05-15.

---

## Legend

| Symbol | Meaning |
|---|---|
| ◯ | todo — not started |
| ▶ | in progress — work has begun |
| ✓ | done — exit criteria met, evidence recorded |
| ⊘ | blocked — see notes |
| ⏸ | deferred — intentionally pushed; see notes |

---

## Per-version summary

| Version | Status | Phases done | Current blocker | Next action |
|---|---|---|---|---|
| [v2.1](v2.1-implementation-plan.md) — mesh transport | ▶ | 0 of 11 (Phase 0 partial) | None — Phase 0 wiring outstanding | Finish Phase 0: add `nexus-mesh` to workspace members, fix Cargo.toml description, add feature flags + config stub |
| [v2.1.1](v2.1.1-research-plan.md) — GML noise barometer (research) | ◯ | 0 of 5 research milestones | Hard-blocked on v2.1 Phase 4 (Gossipsub) for real telemetry; soft-blocked on v2.1.2 for control surface | Begin R1 (architecture survey) — can proceed on synthetic data without v2.1 |
| [v2.1.2](v2.1.2-implementation-plan.md) — MatrixA2A interop | ◯ | 0 of 4 phases | D-2.1.2-A (hosting model) decision outstanding | Resolve D-2.1.2-A in [`ROADMAP.md`](ROADMAP.md), then start Phase 1 |

---

## v2.1 — mesh transport (Phase 0–10)

| Phase | Status | Owner | Evidence | Notes / gaps |
|---|---|---|---|---|
| **0 — Scaffold + decisions** | ▶ | — | Commit `eb532e9` (added nexus-mesh plan); `nexus-mesh/src/lib.rs` placeholder exists | **Gaps:** (1) `nexus-mesh` is **not** in `Cargo.toml` workspace `members`; (2) `nexus-mesh/Cargo.toml` description still says `v2.9.6` instead of `v2.1`; (3) no `mesh = []` feature flag in `nexus-agent/Cargo.toml` or `nexus-infra/Cargo.toml`; (4) no `[mesh]` config stub in `config/agent-linux.toml` or `nexus.toml.example`; (5) `docs/development/Mesh-Threat-Model.md` not written |
| **1 — Ed25519 identity in nexus-common** | ◯ | — | — | Files: `nexus-common/src/identity.rs` (new), `nexus-common/src/crypto.rs`, `nexus-common/src/lib.rs` |
| **2 — Minimal libp2p node** | ◯ | — | — | Files: `nexus-mesh/src/node.rs`, `nexus-mesh/examples/two_node_ping.rs` |
| **3 — Kademlia DHT discovery** | ◯ | — | — | Files: `nexus-mesh/src/discovery.rs`, `nexus-mesh/examples/three_node_discover.rs` |
| **4 — Gossipsub** | ◯ | — | — | Files: `nexus-mesh/src/pubsub.rs`, `nexus-mesh/src/topics.rs`, `nexus-mesh/examples/pubsub_demo.rs`. **Unblocks v2.1.1 R2.** |
| **5 — End-to-end encryption (SealedEnvelope)** | ◯ | — | — | Files: `nexus-mesh/src/sealed.rs` |
| **6 — Store-and-forward DTN** | ◯ | — | — | Files: `nexus-mesh/src/dtn.rs`; new dep `redb` |
| **7 — Wire into nexus-web-comms** | ◯ | — | — | Files: `nexus-web-comms/src/mesh_transport.rs`. **Milestone M1 lands here — first user-visible value.** |
| **8 — Multi-hop relay + routing policy** | ◯ | — | — | TTL/hop counter on `SealedEnvelope`; relay-only peer mode |
| **9 — Sphinx mix-style anonymity (OPTIONAL)** | ⏸ | — | — | Deferred unless a specific engagement requires it. License re-verification of `sphinx-rs` required before vendoring |
| **10 — Operational polish + rollout** | ◯ | — | — | CLI flags, health check, kill switch, docs |

---

## v2.1.1 — GML noise barometer (research)

This version is **research-phase**. Each row below is a research milestone producing a section of the future `v2.1.1-gml-implementation-design.md` design doc.

| Milestone | Status | Owner | Evidence | Notes |
|---|---|---|---|---|
| **R1 — Architecture survey** | ◯ | — | — | Output: 1-page tradeoff matrix covering GCN, GraphSAGE, GAT, R-GCN, Graph Transformers, RGNN/DRE/AGNN. Can begin immediately |
| **R2 — Noise-barometer math** | ◯ | — | — | Hard-blocked on v2.1 Phase 4 (Gossipsub) for real telemetry; can prototype on synthetic data |
| **R3 — Control-loop design** | ◯ | — | — | Hysteresis thresholds, rate floors, edge-attention attribution. Can run in parallel with R1 |
| **R4 — Safeguard analysis** | ◯ | — | — | Label leakage, adversarial robustness, drift detection |
| **R5 — Roadmap synthesis** | ◯ | — | — | Output: complete `v2.1.1-gml-implementation-design.md` design doc; future `v2.1.1-implementation-plan.md` is then opened |

Post-research milestones (not started, not yet planned in detail):

| Milestone | Status | Notes |
|---|---|---|
| **M5 — Shadow-mode go-live** | ◯ | First implementation phase after research completes |
| **M6 — Throttling go-live** | ◯ | Production deployment with conservative rate floors |

---

## v2.1.2 — MatrixA2A interop

| Phase | Status | Owner | Evidence | Notes |
|---|---|---|---|---|
| **0 — Pre-flight decision** | ◯ | — | — | Resolve D-2.1.2-A (hosting model) and D-2.1.2-B (A2A version pin) in [`ROADMAP.md`](ROADMAP.md) before opening Phase 1 |
| **1 — Vendor protos + Tonic build** | ◯ | — | — | Files: `nexus-a2a/proto/a2a/v1/a2a.proto`, `nexus-a2a/proto/a2a_matrix/v1/matrix.proto`, `nexus-a2a/build.rs` |
| **2 — A2AService server + client (mTLS)** | ◯ | — | — | **Milestone M2 lands here.** Files: `nexus-a2a/src/server.rs`, `nexus-a2a/src/client.rs`, `nexus-a2a/src/mtls.rs` |
| **3 — Security hardening (signed AgentCards, audit log, capability matrix)** | ◯ | — | — | **Milestone M4 lands here.** Files: `nexus-a2a/src/security/*` |
| **4 — Matrix router (adjacency/capability/trust) + interop tests** | ◯ | — | — | `nalgebra` or `ndarray` for matrix ops; Python ↔ Rust round-trip test in CI |

---

## Open decisions snapshot

These should be resolved before the relevant phase begins (see [`ROADMAP.md`](ROADMAP.md) for full context):

| ID | Decision | Status | Blocks |
|---|---|---|---|
| D-2.1-A | libp2p version pin | open | v2.1 Phase 2 |
| D-2.1-B | Identity persistence path | open | v2.1 Phase 1 |
| D-2.1-C | sphinx-rs vendoring (if Phase 9 proceeds) | open | v2.1 Phase 9 |
| D-2.1.2-A | Hosting model for `a2a-matrix-rs` | open | v2.1.2 Phase 1 |
| D-2.1.2-B | A2A version pin | open | v2.1.2 Phase 1 |
| D-2.1.2-C | Proto-surface separation from `nexus.proto` | open | v2.1.2 Phase 2 |
| D-XLINK-A | Transport semantics (mesh vs A2A) | open (recommendation in roadmap) | v2.1 Phase 7 + v2.1.2 Phase 2 |
| D-2.1.1-A | GML model architecture | open | v2.1.1 R1 exit |
| D-2.1.1-B | SIEM training data source | open | v2.1.1 R2 |
| D-2.1.1-C | Shadow-mode telemetry endpoint | open | v2.1.1 M5 |

---

## Update cadence

When you complete a phase or sub-milestone:

1. Flip the status symbol in the relevant table.
2. Fill in the **Owner** column (your name / handle).
3. Add **Evidence** — commit hash, test output snippet, or example invocation that proves the exit criterion was met.
4. Update the **Phases done** count in the Per-version summary at the top.
5. Commit this file alongside the work — not in a follow-up.

When a decision in the Open decisions snapshot is resolved:

1. Strike through the row or change status to `resolved`.
2. Move the resolution body into a "Resolved decisions" appendix in [`ROADMAP.md`](ROADMAP.md) with the date and resolver.
