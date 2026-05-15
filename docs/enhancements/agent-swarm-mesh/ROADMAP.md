# Agent Swarm Mesh — Track Roadmap

Strategic roadmap for the `v2.1.x` agent-swarm-mesh track. For project-wide context, see [`docs/ROADMAP.md`](../../ROADMAP.md). For current state, see [`STATUS.md`](STATUS.md).

---

## End-state vision

When `v2.1.x` is complete, rust-nexus will have:

1. **Peer-to-peer mesh reachback (v2.1).** Any deeply-sandboxed or firewall-blocked agent reaches the C2 by hopping through peer agents over a libp2p mesh. End-to-end encrypted, store-and-forward for offline peers, multi-hop relay with TTL.
2. **Standards-based agent-to-agent dialogue (v2.1.2).** rust-nexus speaks the Linux Foundation A2A protocol (v0.3+) over Tonic gRPC, allowing Python red/blue team tooling (rtpi, btpi, d3tect-nexus) to interoperate with rust-nexus agents using a shared `a2a.proto`.
3. **GML-driven noise barometer (v2.1.1).** A graph-ML model ingests mesh telemetry (Gossipsub volume, peer connectivity, command rates) and SIEM alert counts to produce an anomaly score; a hysteresis-based control loop throttles per-agent activity rates when alert pressure rises, preventing detection cascades.

All three pieces land behind feature flags and config gates; an operator who doesn't enable them sees no change from v2.0 behavior.

---

## Version-to-capability map

| Version | Capability | Source design doc | Depends on |
|---|---|---|---|
| `v2.1` | libp2p mesh transport in `nexus-mesh/` crate; wired into `nexus-web-comms` as an alternate transport | [`v2.1-nexus-mesh-comms.md`](v2.1-nexus-mesh-comms.md) | v2.0 baseline |
| `v2.1.1` | Graph-ML model + control loop for swarm noise barometer | [`v2.1.1-gml-agent-swarm-telemetry-analysis.md`](v2.1.1-gml-agent-swarm-telemetry-analysis.md) (research) → future `v2.1.1-gml-implementation-design.md` | v2.1 Phase 4 (Gossipsub telemetry source) + v2.1.2 (per-agent control surface) |
| `v2.1.2` | MatrixA2A interop (A2A v0.3+ gRPC binding hosted in a new `nexus-a2a/` workspace crate) | [`v2.1.2-matrix-a2a.md`](v2.1.2-matrix-a2a.md) | v2.0 baseline; soft dependency on v2.1 Phase 7 for mesh-tunneled A2A |

---

## Open architectural decisions

These must be resolved **before** each version begins implementation. Open status as of 2026-05-15:

### v2.1 — mesh transport

- **D-2.1-A: libp2p version pin.** The design doc specifies `libp2p = "0.53"`. At kick-off, verify this is still current and not blocked by a security advisory. If a major version has landed, decide between sticking with `0.53` for plan fidelity vs absorbing a churn-cost upgrade upfront.
- **D-2.1-B: identity persistence path.** `NodeIdentity` (Phase 1) needs an on-disk location. Recommendation: align with existing config-directory conventions in `nexus-infra/src/config.rs` (look for `dirs::config_dir()` use) rather than introducing a new path.
- **D-2.1-C: `sphinx-rs` vendoring (Phase 9 only).** The parent project is AGPL but `ghostwire-libs/sphinx-rs` is dual MIT/Apache. Required action if Phase 9 proceeds: per-file license header verification, captured as a checklist in the v2.1 implementation plan.

### v2.1.2 — MatrixA2A

- **D-2.1.2-A: hosting model for `a2a-matrix-rs`.** Two options:
  - *Host as workspace crate* (recommended): add `nexus-a2a/` to the rust-nexus workspace `members`, vendor `a2a.proto` + `matrix.proto` alongside the existing `nexus-infra/proto/nexus.proto`. Keeps the build local, no external dependency cycle.
  - *External crate*: `a2a-matrix-rs` lives in a separate repo, rust-nexus consumes via crates.io or git dep. Introduces release-coupling friction.
- **D-2.1.2-B: A2A version pin.** Source doc references A2A v0.3+ with v1.0 in dev. Pin to a specific upstream tag (e.g., `a2aproject/A2A` `v0.3.1`) and document the bump cadence.
- **D-2.1.2-C: relationship to existing `nexus-infra/proto/nexus.proto`.** Two separate protos in the same workspace are fine, but the service surface should not duplicate. Decision: `nexus.proto` stays for C2 internals; `a2a.proto` is the public interop surface. No methods overlap between them.

### v2.1.2 ↔ v2.1 — transport-plane overlap

- **D-XLINK-A: transport semantics.** Default recommendation:
  - **A2A gRPC** = primary control-plane for structured agent dialogue (skills, task lifecycle, streaming).
  - **libp2p mesh** = fallback transport when gRPC is blocked, agnostic to A2A.
  - A2A `Message` envelopes may be carried *over* mesh as one more bytes-level transport once v2.1 Phase 7 lands. The matrix plane (adjacency/capability/trust) stays in the A2A router regardless of transport.

### v2.1.1 — GML noise barometer

- **D-2.1.1-A: model architecture.** Source doc recommends starting with heterogeneous GraphSAGE on hourly snapshots, supervised on SIEM alert counts. Reconfirm at the end of the R1 (architecture survey) milestone — RGNN/AGNN may be more appropriate for the temporal dimension.
- **D-2.1.1-B: training data source.** SIEM type and access (Wazuh, Elastic, Splunk) must be named before R2 begins.
- **D-2.1.1-C: deployment mode.** Shadow → throttle progression is in scope. Question: shadow-mode metric reporting endpoint — `nexus-webui` dashboard, or external (Grafana/Prometheus)?

---

## Milestones

| ID | Name | Definition of done | Required versions |
|---|---|---|---|
| **M1** | Mesh transport working end-to-end | A v2.1 agent with gRPC blocked at the firewall connects to a peer agent that has gRPC; peer relays heartbeat to server; server task assignment flows back | v2.1 Phase 7 |
| **M2** | A2A interop with one Python agent | A Python A2A client and a rust-nexus A2A server share the same `.proto`, complete a streaming task | v2.1.2 Phase 2 |
| **M3** | Mesh hardening | Multi-hop relay verified across 5-node line topology; DTN store-and-forward survives reconnect | v2.1 Phase 8 |
| **M4** | A2A security hardening | mTLS, Ed25519-signed AgentCards, hash-chained audit log, capability matrix enforcement | v2.1.2 Phase 3 |
| **M5** | GML shadow-mode noise barometer | Heterogeneous GraphSAGE trained on real mesh telemetry + SIEM labels; predictions logged, no throttling yet | v2.1.1 implementation Phase 1 (post-research) |
| **M6** | GML throttling go-live | Control loop with conservative rate floors enabled in production for one engagement | v2.1.1 implementation Phase 2 |

---

## Risks for the track as a whole

- **Library churn.** libp2p, Tonic, and the A2A proto are all moving targets in 2025–2026. Pin versions explicitly in each implementation plan and revisit pins at each phase start.
- **Scope creep on the matrix plane.** v2.1.2's MatrixA2A Python library is large; the rust-nexus implementation must extract only the Rust-facing slice (`a2a-matrix-rs`) and resist pulling in the Python adapter ecosystem.
- **GML legitimacy.** A throttling control loop that silences agents based on a model output is a serious operational lever. Per the source doc: never deploy silent activity throttling without operator visibility — surface barometer state in `nexus-webui` from M5 onwards.
- **License hygiene.** GhostWire is AGPL; replicate patterns, never embed code. The `sphinx-rs` sub-crate is separately licensed (MIT/Apache) but the parent isn't — per-file verification required if vendored.
- **Operator UX.** All three versions land behind feature flags. Documenting the on/off path is part of v2.1 Phase 10 (rollout) — without it, capabilities exist but nobody enables them.

---

## Maintenance

- Update the **status of decisions** above in this file when they are made (move from "Open" to a "Resolved decisions" appendix).
- Update **milestone status** in [`STATUS.md`](STATUS.md), not here.
- When a new sub-version is opened (e.g., `v2.1.3`), add a row to the version-to-capability map and a milestone if it materially extends the end-state vision.
