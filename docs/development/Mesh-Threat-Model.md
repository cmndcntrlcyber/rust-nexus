# nexus-mesh — Threat Model

**Initiative:** v2.9.6 Nexus Mesh Comms
**Phase:** A0 (deliverable required by mesh Phase 0)
**Status:** Living document — update as mesh phases land
**Last reviewed:** 2026-05-06

## Purpose

This document defines the threats the `nexus-mesh` libp2p layer is designed to resist, the threats it explicitly does *not* resist, and the operator-visible boundaries that constrain mesh deployments. Mesh code lands under the `mesh = []` Cargo feature and `[mesh].enabled = false` runtime flag — operators must read this document before flipping either.

## Trust model

A mesh peer is **one of**:

1. **Operator-controlled C2 / staging node** — fully trusted; operator owns the host and the operator key.
2. **Trusted agent** — runs on a target the operator has authorization to access; trusted to forward and to honor protocol semantics; **not** trusted with secrets beyond its own session keys.
3. **Compromised agent** (hostile) — assumed possible at any time. The mesh must contain blast radius from one bad peer.
4. **Outside observer** — sees Gossipsub frames on the wire, can run their own libp2p node and join the mesh if they obtain a bootstrap address.

There is no "external public" trust class. Mesh is a private overlay; bootstrap addresses are operator-distributed, not advertised.

## Threats in scope

### T1 — Eavesdropping on payload contents

**Attacker:** any peer on the mesh (compromised agent, outside observer who joined with a stolen bootstrap address).

**Mitigation:** every payload above the libp2p Noise transport is wrapped in a `SealedEnvelope` (Phase 5): X25519 ECDH ephemeral → HKDF → AES-256-GCM, signed with sender Ed25519. Only the named recipient peer can decrypt. Topic membership conveys no payload visibility.

**Residual risk:** observers learn topic membership, message timing, and message size. Phase 9 (Sphinx) — explicitly out of scope for this initiative — would be required to defeat traffic analysis.

### T2 — Message tampering

**Attacker:** any peer on the mesh, or any libp2p relay along the gossip path.

**Mitigation:** Ed25519 signature over the full sealed envelope. Verification happens before decryption; tampered envelopes are dropped silently and logged.

**Residual risk:** none above the cryptographic assumptions.

### T3 — Replay

**Attacker:** any peer that captured an envelope.

**Mitigation:** per-sender sliding-window nonce cache (Phase 5). Receivers drop duplicate `(sender_pubkey, nonce)` pairs. DTN dedupe (Phase 6) extends this across reconnects via envelope-hash storage with a TTL.

**Residual risk:** if the receiver's nonce window is too small, a sufficiently delayed replay slips through. Default window ≥ 1024 entries per sender; TTL ≥ envelope max-age + clock-skew tolerance.

### T4 — One compromised agent leaks others' secrets

**Attacker:** a single agent fully compromised (root on the host).

**Mitigation:** each peer holds only its own Ed25519/X25519 key pair (Phase 1). No shared mesh-wide secret. A compromised peer can read messages addressed to it and forge messages bearing its own identity, but cannot impersonate other peers, decrypt traffic between other peers, or read their DTN queues.

**Residual risk:** the compromised peer remains a valid mesh participant until its peer ID is added to the relay deny-list (Phase 8) or a `nexus/control` kill message excludes it. Both require operator action.

### T5 — Storage exhaustion via DTN amplification

**Attacker:** compromised peer (or a peer acting as an attacker) publishes a flood of sealed envelopes to high-fan-out topics, hoping relays queue them.

**Mitigation:** per-peer DTN queue cap (Phase 6) with newest-first rejection. Per-relay rate limiting (Phase 8). Total disk budget enforced by `redb` size cap configured in `[mesh]`.

**Residual risk:** within the per-peer cap, an attacker still consumes storage. Operators must monitor DTN queue depth via Phase 10's health endpoint.

### T6 — Routing loops / TTL exhaustion

**Attacker:** misbehaving or compromised relay rebroadcasts indefinitely.

**Mitigation:** per-message hop counter and TTL in `SealedEnvelope` (Phase 8). Each relay decrements; drop at 0.

### T7 — Bootstrap-list abuse

**Attacker:** outside observer obtains a leaked bootstrap multiaddr.

**Mitigation:** bootstrap addresses are operator-distributed (operator-side discipline; no in-protocol mitigation). Once an attacker is on the mesh, T1–T6 still apply — they can observe topic membership and message timing but not payload contents. The custom Kademlia protocol name `/nexus-mesh/kad/1.0.0` (Phase 3) at least prevents accidental entry from public-IPFS DHT scrapers.

**Residual risk:** an attacker with a valid bootstrap address can perform traffic analysis. Rotate bootstrap addresses on engagement boundaries.

### T8 — Kill-switch abuse / unauthorized fleet teardown

**Attacker:** compromised C2 server, or mesh peer with stolen kill-switch key.

**Mitigation:** the `nexus/control` kill message (Phase 10) is signed by an *offline* operator key, **not** the C2's TLS key. A compromised C2 cannot issue it. The kill-switch verification key is embedded in agent binaries at compile time.

**Residual risk:** loss of the offline key prevents emergency shutdown via this channel. Operators retain out-of-band agent termination via existing gRPC/HTTP paths.

## Threats explicitly out of scope

- **Sender anonymity / traffic analysis defeat.** Phase 9 (Sphinx) addresses this and is explicitly out of scope per the v2.9.6 plan and the parent rtpi initiative.
- **Defense against a global passive observer.** libp2p's transport security plus our application-layer sealing assumes a network observer is *not* also a mesh peer with knowledge of every topic's membership. A nation-state-level observer co-resident on every relay is out of scope.
- **Side-channel attacks against the agent host** (CPU timing, cache attacks, etc.). Standard `aes_gcm` and `ed25519_dalek` constant-time guarantees apply, but no extra mitigation.
- **Protection against the operator.** If the operator's own key is compromised, the entire mesh is compromised. Standard caveat.

## Operator obligations

Before flipping `[mesh].enabled = true` on any deployment, the operator MUST:

1. Read this document and confirm the threats-in-scope set covers the intended deployment context.
2. Distribute bootstrap multiaddrs out-of-band (signed Slack message, encrypted email, etc.) — never in plaintext channels.
3. Generate a fresh offline kill-switch key and embed its public component in the agent build (Phase 10 build-time config).
4. Confirm DTN storage cap is sized for the expected traffic volume × engagement duration.
5. Plan for bootstrap-address rotation on engagement boundaries.

## Compromise response

If a peer is suspected compromised:

1. Add its peer ID to the relay deny-list (Phase 8). All operator-controlled peers reload their deny-list and refuse to relay for it.
2. If broadcast rapid response is needed, issue a `nexus/control` kill message scoped to the suspected peer ID.
3. Investigate post-hoc using DTN queue history and per-peer message metrics from the Phase 10 health endpoint.

## Review cadence

This document is reviewed:

- At the close of each mesh implementation phase (revisit assumptions in light of code that has landed).
- Before each operator-visible default-on flip of `FF_NEXUS_MESH` (currently planned: after two field-tested engagements per Phase 10).
- On any reported security incident involving a mesh-enabled deployment.
