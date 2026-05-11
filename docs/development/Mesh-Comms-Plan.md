# Replicating GhostWire Mesh Methodology in rust-nexus Communication Layers

Phased plan for adding peer-to-peer mesh communication to rust-nexus by re-implementing the relevant GhostWire patterns from scratch on top of `libp2p`. This is a replication plan, not an embedding plan.

## Why replicate, not embed

GhostWire (`Phantomojo/GhostWire-secure-mesh-communication`) is licensed **AGPL-3.0-or-later** with a paid commercial alternative. Linking it into rust-nexus would force rust-nexus and any downstream operator deployment under AGPL — including the source-availability obligations that trigger when the software is reachable over a network. That is not acceptable for offensive tooling shipped to clients.

The underlying primitives GhostWire uses are themselves permissively licensed and are what we replicate directly:

| Capability | GhostWire's implementation | Our equivalent | License |
|---|---|---|---|
| Peer discovery | libp2p Kademlia | libp2p Kademlia | MIT/Apache-2.0 |
| Pub/sub fanout | libp2p Gossipsub | libp2p Gossipsub | MIT/Apache-2.0 |
| Identity | Ed25519 + X25519 | Ed25519 + X25519 (already partially in `nexus-common/src/crypto.rs`) | – |
| Session crypto | XChaCha20-Poly1305 | AES-256-GCM (existing) → optionally add XChaCha20-Poly1305 | – |
| Onion routing | `sphinx-rs` (sub-crate inside GhostWire) | `sphinx-rs` if we want it — `ghostwire-libs/sphinx-rs` is **MIT + Apache-2.0**, not AGPL, so can be reused independently | MIT/Apache-2.0 |
| Store-and-forward DTN | Custom GhostWire code | Reimplement with `sled` or `redb` | – |

**Scope cuts** (things in GhostWire we deliberately do NOT replicate): React/Tauri UI, Axum HTTP admin API, GNN routing, LightGBM anomaly detection, ONNX runtime, LoRa/Bluetooth transports, Web-of-Trust UX, duress PIN. These are operator-facing or research features that bloat the agent binary and add attack surface.

## Current rust-nexus comms baseline (as of `d24e8de`)

- Hub-and-spoke. All agents talk to one C2 server over gRPC/mTLS (`nexus-infra/src/grpc_server.rs`).
- Fallback transports in `nexus-web-comms/src/lib.rs`: HTTP polling, WebSocket, domain fronting, traffic obfuscation.
- Crypto: AES-256-GCM in `nexus-common/src/crypto.rs`, no application-layer Ed25519 identity — agents are identified by server-assigned `agent_id`.
- Message envelope: `nexus-common/src/messages.rs` `Message` (legacy JSON) and protobuf gRPC types.
- No peer-to-peer, relay, gossip, or mesh code anywhere in the workspace.

## Target end state

A new `nexus-mesh` workspace crate plus a small `nexus-mesh-bridge` binary that together provide:

1. Agents join a libp2p mesh directly. The C2 server does **not** join the mesh.
2. Peer discovery via Kademlia, bootstrapping from dedicated bridge containers (not from the C2).
3. Existing `nexus-common::Message` payloads carried over Gossipsub topics, serialized with `prost` so mesh and gRPC share message types.
4. End-to-end encryption (sender → recipient peer key) on top of libp2p's transport security.
5. Store-and-forward queueing for offline peers, replayed on reconnect.
6. `nexus-web-comms` gets a new `MeshTransport` option, selectable in config and slotted into the existing fallback chain.
7. Optional peer-to-peer relay so a deeply-firewalled agent can reach the C2 by hopping through other agents and a bridge.

Each phase below is independently committable, leaves the tree green, and either flag-gates new behavior or runs as a standalone example until Phase 7.

## Architecture: bridge model (locked decision, Q1)

The C2 stays gRPC-native and never speaks libp2p. A small fleet of stateless bridge containers sit between the C2 and the mesh:

```
              [operator console]
                     |
                     | gRPC over mTLS (existing)
                     v
            +-------------------+
            |   nexus-server    |   (existing, unchanged on the wire)
            +-------------------+
                     |
                     | new internal gRPC: MeshBridgeService
                     |   (BridgeEnqueue + BridgeDequeue stream)
                     v
   +------------------------------------------+
   |  nexus-mesh-bridge x N (Docker)          |   low-value disposable hosts
   +------------------------------------------+
       |              |              |
       v              v              v
   libp2p peer    libp2p peer    libp2p peer
       \              |              /
        \             |             /
         +-------- mesh ------------+
              ^         ^         ^
              |         |         |
            agent     agent     agent
```

Properties this gives us:

- **Bridge compromise ≠ C2 compromise.** A bridge holds no operation state, no recipient long-term keys, and only its own libp2p identity. Burning one means losing one mesh ingress point.
- **C2 stays narrow.** No new attack surface added to the most valuable host. The C2 doesn't open any new ports; it still receives over its existing gRPC interface.
- **Agents see "the server inbox topic," not the server.** Bridges subscribe to `nexus/server/inbox` and forward sealed envelopes byte-for-byte to the C2. Bridges cannot decrypt — the C2 holds the recipient X25519 key.
- **Geographic / provider diversity.** Bridges should be deployed across distinct cloud accounts and ASNs so a defender cannot block the mesh by blocking one IP block.
- **Horizontal capacity.** Add more bridges → more mesh-side throughput and more redundant entry points. Bridge count is set per engagement (typically 3).

---

## Phase 0 — Decisions, scaffold, and design lock-in

**Goal:** agree on scope and create the empty crate. No behavior change.

Steps:
1. Capture the no-AGPL rule and mesh threat model in `docs/development/Mesh-Threat-Model.md`. Specifically: which peers are trusted, what an attacker on the mesh can see, how compromise of one agent affects others.
2. Add `nexus-mesh/` to the workspace `[workspace] members` in `Cargo.toml`. Empty crate with `lib.rs` that compiles.
3. Add `mesh = []` feature flag to `nexus-agent/Cargo.toml` and `nexus-infra/Cargo.toml`. Off by default.
4. Add a `[mesh]` section stub to `config/agent-linux.toml` and `config/examples/nexus-config.toml` (commented out).

Verification: `cargo check --workspace` and `cargo check --workspace --all-features` both clean. No runtime behavior change.

Size: ~50 LoC, half a day.

---

## Phase 1 — Application-layer Ed25519 identity in `nexus-common`

**Goal:** every node has a stable cryptographic identity independent of TLS certs and `agent_id`.

Files: `nexus-common/src/crypto.rs`, `nexus-common/src/lib.rs`, new `nexus-common/src/identity.rs`.

Steps:
1. Add `ed25519-dalek` and `x25519-dalek` to `nexus-common/Cargo.toml`.
2. Implement `NodeIdentity { ed25519_signing: SigningKey, x25519_secret: StaticSecret }` with `generate()`, `from_seed(&[u8; 32])`, `peer_id() -> [u8; 32]` (BLAKE3 of ed25519 public key), `sign(&[u8])`, `verify(...)`.
3. Persist identity to disk (path from config), load on startup, generate-and-save on first run. Handle Windows/Linux paths.
4. **Keep `agent_id` and `peer_id` as separate fields (locked, Q2).** `agent_id` remains server-assigned and used by the existing gRPC paths unchanged. `peer_id` is independently derived from the ed25519 pubkey and used only by mesh code. The server-side mapping table joining the two lives in `nexus-infra` and is populated when an agent first appears on either channel — a Phase 7 concern, not a Phase 1 concern.
5. Unit tests: round-trip persist/load, sign/verify, peer-id determinism.

Verification: `cargo test -p nexus-common identity::`. Existing agent + server still build and run unchanged because nobody calls `NodeIdentity` yet.

Size: ~250 LoC, ~1 day.

---

## Phase 2 — Minimal libp2p node in `nexus-mesh`

**Goal:** spin up a libp2p Swarm with TCP + Noise + Yamux, no protocols beyond Identify and Ping.

Files: `nexus-mesh/Cargo.toml`, `nexus-mesh/src/lib.rs`, `nexus-mesh/src/node.rs`, `nexus-mesh/examples/two_node_ping.rs`.

Steps:
1. Add `libp2p = { version = "0.53", features = ["tcp","noise","yamux","tokio","macros","identify","ping"] }` and `tokio` to the crate.
2. `MeshNode::new(identity: NodeIdentity, listen: Multiaddr) -> Self`. Convert `NodeIdentity`'s ed25519 key into a `libp2p::identity::Keypair` so the libp2p PeerId is derived from the same key.
3. `async fn run(self) -> impl Stream<Item = MeshEvent>` event loop.
4. Example: two `MeshNode`s ping each other on localhost; print round-trip times.

Verification: `cargo run --example two_node_ping -p nexus-mesh` shows successful Identify exchange and ping replies in both directions. No integration with the agent yet.

Size: ~300 LoC, 1–2 days.

---

## Phase 3 — Kademlia DHT discovery

**Goal:** a third node bootstrapping from a single seed can find the second.

Files: `nexus-mesh/src/discovery.rs`, update `node.rs`.

Steps:
1. Add `kad` to the libp2p feature list. Define a `NetworkBehaviour` combining Identify + Ping + Kademlia.
2. Configure Kademlia in server mode with a custom DHT protocol name (so we don't talk to the public IPFS DHT). Default value `/nexus-mesh/kad/1.0.0` for now; **per-engagement derived names is Q4 (still open)** — wire this as a `String` config field so we can flip behavior without re-architecting.
3. `MeshNode::add_bootstrap(addr: Multiaddr, peer: PeerId)` and `MeshNode::bootstrap()` to seed the routing table. In production the bootstrap list will be the bridge containers (see architecture section); for the example any libp2p node works.
4. Example `three_node_discover.rs`: start node A, node B bootstraps off A, node C bootstraps off A, then C dials B by PeerId-only and succeeds.

Verification: example prints "C connected to B" without C ever being given B's address directly.

Size: ~250 LoC, ~1 day.

---

## Phase 4 — Gossipsub with `nexus-common::Message` payloads

**Goal:** publish-subscribe across the mesh, carrying existing message types.

Files: `nexus-mesh/src/pubsub.rs`, `nexus-mesh/src/topics.rs`, new `nexus-infra/proto/nexus_mesh.proto`.

Steps:
1. Add `gossipsub` to libp2p features. Add to the `NetworkBehaviour`.
2. Define topic naming convention in `topics.rs`:
   - `nexus/op/{op_id}` — broadcast within an operation
   - `nexus/agent/{peer_id_hex}/inbox` — direct to agent
   - `nexus/server/inbox` — to C2 (subscribed by bridges, forwarded over gRPC)
   - `nexus/heartbeat` — global presence
3. **Wire format is `prost`-generated protobuf (locked, Q3).** Define `MeshMessage`, `MeshHeartbeat`, `MeshTaskAssignment`, `MeshTaskResult` in a new `nexus_mesh.proto` colocated with the existing `nexus.proto`. Where the semantics are identical to gRPC types, reuse the existing `nexus.proto` messages by `import`. Mesh and gRPC share types; no duplicate definitions.
4. `MeshNode::publish(topic, msg: impl prost::Message)` — encode with `prost::Message::encode_to_vec`.
5. `MeshNode` exposes a `tokio::sync::broadcast::Receiver<MeshIncoming>` for subscribers, where `MeshIncoming` carries the raw bytes plus the topic and source PeerId. Decoding to a concrete protobuf type is the caller's responsibility (so we don't bake type knowledge into `nexus-mesh`).
6. Example `pubsub_demo.rs`: 3-node mesh, one node publishes a `MeshHeartbeat`, other two decode and print it.

Verification: example output. Plus a unit test that round-trips each protobuf message type and checks that an unknown-tag field is preserved (forward compatibility).

Size: ~350 LoC, ~2 days. (Slightly larger than bincode because the .proto file and build.rs wiring grow.)

---

## Phase 5 — End-to-end encryption layer above Gossipsub

**Goal:** a `Message` published to peer X's inbox topic is opaque to every other peer on the mesh, even though the topic is public and Gossipsub broadcasts it.

Files: new `nexus-mesh/src/sealed.rs`, update `pubsub.rs`.

Steps:
1. Define `SealedEnvelope` as a protobuf message in `nexus_mesh.proto` with fields `sender_pubkey: bytes`, `ephemeral_x25519_pub: bytes`, `nonce: bytes`, `ciphertext: bytes`, `signature: bytes`. Generated via `prost` like the rest of the mesh wire types (Q3 decision).
2. Sender flow: ephemeral X25519 keypair → ECDH with recipient's static X25519 → HKDF → AES-256-GCM key → encrypt the prost-encoded inner `MeshMessage` → sign the whole envelope (concatenated fields) with sender's ed25519.
3. Recipient flow: verify signature → ECDH → decrypt. Reject on any failure.
4. Recipient public-key directory: simple in-memory `HashMap<PeerId, X25519PublicKey>` populated from Identify (extend identify with a custom protocol field) or from a signed announce on `nexus/keys`.
5. Replay protection: per-sender sliding-window nonce cache, drop on duplicate.

Verification: 3-node example where node A sends sealed message to C, B in the middle logs the gossipsub frame and confirms it cannot decrypt. Add fuzz test for malformed envelopes.

Size: ~400 LoC, ~2–3 days.

---

## Phase 6 — Store-and-forward queue for offline peers

**Goal:** a sender can publish to a recipient that's currently offline; recipient receives it on reconnect.

Files: `nexus-mesh/src/dtn.rs`, optional new dep `redb` or `sled`.

Steps:
1. Pick `redb` (single-file, pure Rust, MIT/Apache, smaller than sled).
2. Outbound queue: when a `SealedEnvelope` is published and gossipsub reports zero subscribed peers for the recipient inbox topic, persist it locally with a TTL.
3. On peer-connected events for a peer that owns a queued message's recipient topic, drain.
4. Cap the per-peer queue at a configurable size and reject newest-first when full (prevents amplification attacks against operator disk).
5. Receiver-side dedupe by sealed envelope hash — DTN replay through multiple relays must not deliver twice.

Verification: integration test — start A and C, A sends to C, A and C disconnect, A reconnects, C reconnects, C sees the message exactly once.

Size: ~350 LoC, ~2 days.

---

## Phase 7 — Agent transport + `nexus-mesh-bridge` component

**Goal:** the agent comms layer can pick "mesh" as a transport, and a new bridge binary connects the mesh to the C2 over gRPC. First user-visible phase.

This phase has two halves that ship together: the agent side and the bridge side.

Files (agent side): `nexus-web-comms/src/mesh_transport.rs`, update `nexus-web-comms/src/lib.rs`, agent main loop.

Files (bridge side): new workspace member `nexus-mesh-bridge/` (binary crate), new `nexus-infra/proto/mesh_bridge.proto` extending the existing proto package with a `MeshBridgeService`, generated code wired into `nexus-server`.

Steps — agent side:
1. Define a `Transport` trait inside `nexus-web-comms` if one doesn't already exist (`WebCommsClient` currently has methods rather than a trait — extract one).
2. `MeshTransport` wraps a `MeshNode` handle and implements the trait by publishing to `nexus/server/inbox` and subscribing to its own `nexus/agent/{me}/inbox`.
3. Config: `[mesh] enabled=false, bootstrap=[…multiaddrs of bridge containers], identity_path="…", dht_protocol="…"`. Off by default.
4. Fallback ordering when enabled: gRPC → mesh → HTTP → WebSocket. Justification: gRPC is cheapest when reachable; mesh routes through bridges and other agents, which is exactly when gRPC has been blocked.

Steps — bridge side:
5. Define `MeshBridgeService` in `mesh_bridge.proto`: streaming `BridgeIngress(stream SealedEnvelopeBytes) -> stream Ack` (bridge → server) and `BridgeEgress(SubscribeRequest) -> stream SealedEnvelopeBytes` (server → bridge). Sealed envelopes pass through opaquely; the bridge never decrypts.
6. `nexus-mesh-bridge` binary: starts a `MeshNode`, subscribes to `nexus/server/inbox`, opens a `BridgeIngress` stream to the C2 over mTLS, and forwards each gossipsub message it receives. In the other direction, holds open a `BridgeEgress` stream and republishes whatever the C2 hands it onto the mesh under the topic the C2 specifies.
7. `nexus-server` side: registers `MeshBridgeService`, dispatches incoming sealed envelopes into the same code path the existing gRPC `Heartbeat`/`TaskResult` handlers use, decrypting only at the very end with the server's X25519 key. Maintains the `agent_id ↔ peer_id` mapping table from Phase 1 — populated whenever the same agent is seen on both channels.
8. Bridge auth to C2: bridges hold a per-bridge mTLS client cert issued by the same CA as agent certs but with a `bridge` SAN; C2 enforces that only `bridge`-flagged certs may call `MeshBridgeService`.
9. Bridge discovery: bridges' multiaddrs are baked into agent configs (or fetched once at first contact via the existing gRPC channel). When bridges rotate, agents pick up new addresses on next heartbeat.

Verification: end-to-end test — three docker bridges + one C2 + two agents on the mesh, one agent with gRPC blocked at the firewall. The blocked agent's heartbeat must reach C2 via mesh → bridge → gRPC, and a task assignment must flow back the same way. Kill one bridge mid-flow, confirm next message routes through a different bridge.

Size: ~900 LoC across the agent transport (~400) and the bridge component (~500). ~5 days.

---

## Phase 8 — Multi-hop relay and routing policy

**Goal:** make the mesh actually useful when the firewall is restrictive — a deeply-sandboxed agent reaches the C2 via two or more hops.

Steps:
1. Gossipsub already gives us multi-hop fanout for free; the work in this phase is **measurement and policy**, not new transport code.
2. Add per-message hop counter and TTL to `SealedEnvelope`, decrement on rebroadcast, drop at 0. Prevents loops if a peer misbehaves.
3. Add an explicit "relay-only" peer mode — a node that accepts and rebroadcasts sealed envelopes for topics it isn't subscribed to. Used for staging boxes that operators control but don't want to issue tasks from.
4. Metrics counters: messages delivered, messages dropped (TTL/queue-full/dedupe), median hops to server. Surface in the existing logging.
5. Optional: deny-list of peer IDs we refuse to relay for, in case an agent is suspected burned.

Verification: 5-node line topology (A — B — C — D — server), confirm A's heartbeat reaches the server with hop=3, and tasks flow back the same way.

Size: ~250 LoC, ~1 day.

---

## Phase 9 — Optional: Sphinx mix-style sender anonymity

**Goal:** an attacker who owns one mesh peer cannot tell which agent originated a given message. Only adopt if the threat model in Phase 0 demands it — adds latency and complexity.

Files: new `nexus-mesh/src/sphinx.rs`.

Steps:
1. Vendor `ghostwire-libs/sphinx-rs` (which is **MIT + Apache-2.0**, separately from the AGPL parent project — verify the per-file license headers before vendoring) OR pull a maintained alternative such as `sphinx-packet`.
2. Build a fixed-length onion packet wrapping the existing `SealedEnvelope`, with N intermediate hops chosen from the Kademlia routing table.
3. Cover traffic: a low-rate scheduled "decoy" message to a random topic per node, sized to the same Sphinx packet length, so an observer cannot distinguish real sends from cover.
4. Document the latency cost in operator-facing config — this is opt-in per operation, not always-on.

Verification: traffic-analysis test — a controlled observer sees N nodes' egress and cannot correlate which one originated a message above the rate of cover traffic. (This is informal; a real anonymity audit is out of scope.)

Size: ~600 LoC, ~3–4 days. Skip unless a specific engagement requires it.

---

## Phase 10 — Operational polish and rollout

**Goal:** make the mesh usable by operators without reading source.

Steps:
1. Config surface: `[mesh]` block in both server and agent configs, fully documented in `docs/configuration/`.
2. CLI flags on the agent and server binaries: `--mesh-enable`, `--mesh-bootstrap=<multiaddr>`, `--mesh-listen=<multiaddr>`.
3. Health check: server-side endpoint reports number of mesh peers, last gossip received, queued DTN messages.
4. Kill switch: a signed control message on `nexus/control` that tells all nodes to leave the mesh and fall back to gRPC-only. Signed by an offline operator key, not the C2's TLS key — so a compromised C2 cannot issue it.
5. Update `docs/integration/` and `README.md` mesh section. Document the AGPL boundary explicitly so future contributors don't paste GhostWire code in.
6. Feature flag `mesh` flipped to default-on once two real engagements have used it without incident.

Verification: a fresh operator can follow the docs, bring up a 3-node mesh on three VMs, deliver a task, and tear it down. No source-code reading required.

Size: ~200 LoC + docs, ~2 days.

---

## Phase 11 — Optional: WebRTC transport for egress evasion and NAT traversal

**Goal:** an alternate libp2p transport that blends with Teams/Zoom/Meet egress and gets NAT traversal via STUN/TURN. Adopt when the engagement profile demands it; skip when TCP/443 is reliably allowed.

Files: `nexus-mesh/Cargo.toml` (add `libp2p-webrtc` behind a `webrtc` feature flag), `nexus-mesh/src/node.rs` (extend transport builder to compose WebRTC alongside TCP), engagement-bundle additions for STUN/TURN config, `nexus-mesh-bridge` Docker compose to optionally co-locate a coturn container.

Steps:
1. Add `libp2p-webrtc` to libp2p features behind a `webrtc` Cargo feature flag on `nexus-mesh`. Off by default to keep the binary lean for engagements that don't use it.
2. Extend `MeshNode::new` to optionally build a WebRTC listener alongside TCP, both upgraded by Noise. Either or both can be enabled per node.
3. Engagement bundle gains a `[mesh.webrtc]` block with STUN server list and optional TURN credentials. Operators run their own TURN server (a coturn container alongside the bridges is the natural deployment).
4. Bridges enable WebRTC by default once Phase 11 ships; agents enable it via config based on the egress profile of their target network.
5. End-to-end test: agent on a network where outbound TCP/443 is blocked but UDP/443 + STUN are reachable; confirm it joins the mesh via WebRTC and reaches a bridge.

Why deferred (not cut):
- libp2p's WebRTC transport is newer than Noise/TCP and has had API churn across recent libp2p-rust releases. Wait until Phase 7's TCP integration is stable before adding a less-mature transport.
- Operating WebRTC requires running STUN and (for symmetric NAT) TURN — more components, more attack surface, more cost.
- The bridge model already gives agents a routable target on TCP/443. WebRTC's NAT-traversal advantage is incremental for typical engagements, transformative only for aggressive symmetric NAT.

Adopt when: targets are behind aggressive symmetric NAT, or egress filtering blocks plain TCP/443 but allows WebRTC for collaboration tools (common in tightly-managed enterprises).

Size: ~400 LoC, ~2–3 days.

---

## Phase totals and ordering notes

| Phase | LoC | Days | Independently shippable |
|---|---|---|---|
| 0  | 50  | 0.5 | yes (no-op) |
| 1  | 250 | 1   | yes |
| 2  | 300 | 1.5 | yes (example only) |
| 3  | 250 | 1   | yes (example only) |
| 4  | 350 | 2   | yes (example only) |
| 5  | 400 | 2.5 | yes (example only) |
| 6  | 350 | 2   | yes (example only) |
| 7  | 900 | 5   | **first user-visible value** (agent + bridge) |
| 8  | 250 | 1   | yes |
| 9  | 600 | 3.5 | optional, gated |
| 10 | 250 | 2   | yes (adds bridge ops docs + Docker compose) |
| 11 | 400 | 2.5 | optional, gated |

Total to "user-visible mesh transport works" (through Phase 7): ~2850 LoC, ~15 working days.
Total including hardening (Phases 8 + 10): ~3350 LoC, ~18 days.
With Phase 9 sender anonymity: ~3950 LoC, ~22 days.
With Phase 11 WebRTC transport: ~3750 LoC, ~21 days (independent of Phase 9; either, both, or neither).

Ordering is sequential — each phase builds on the previous. Phases 9 and 11 are both optional, independent of each other, and either can land after Phase 10 in either order. Phases 0–6 build everything as standalone examples in `nexus-mesh/examples/`, so the agent and server keep working unchanged. Phase 7 is the integration commit; it should land behind the `mesh` feature flag and the `[mesh].enabled=false` config default until Phase 10's rollout decision.

## Decisions

### Locked

1. **Bootstrap topology — bridge model.** C2 never speaks libp2p. A small fleet of disposable Docker containers (`nexus-mesh-bridge`) sit between the C2 and the mesh, are the only mesh participants on the server side, and serve as bootstrap nodes for agents. Burning a bridge does not burn the C2. See "Architecture: bridge model" above.
2. **`peer_id` and `agent_id` are kept separate.** `agent_id` remains server-assigned and used by existing gRPC paths unchanged. `peer_id` is derived from the agent's ed25519 pubkey and used only by mesh code. The server maintains a join table populated when an agent is seen on either channel. No schema migration of `agent_id`.
3. **Wire format is `prost`-generated protobuf.** Mesh and gRPC share types via a single proto package. New messages live in `nexus_mesh.proto` and `mesh_bridge.proto` alongside the existing `nexus.proto`.

4. **DHT protocol-name OPSEC — auto-generated 128-bit `engagement_id` (locked, Q4).** No operator-supplied strings; no derivation from human-chosen secrets. At engagement init, `nexus-server init-engagement` generates `engagement_id = randombytes(16)`, hex-encodes it, and writes it into the engagement config bundle distributed to operators. The same identifier namespaces both the DHT protocol name and the Gossipsub topic prefix:

   ```
   dht_protocol = "/{engagement_id_hex}/kad/1.0.0"
   topic_prefix = "nexus/{engagement_id_hex}/..."
       e.g. nexus/{id}/server/inbox
            nexus/{id}/agent/{peer_id_hex}/inbox
            nexus/{id}/op/{op_id}
            nexus/{id}/heartbeat
   ```

   Properties:
   - 2^128 search space — brute-forcing the DHT protocol name to find live engagements is infeasible.
   - Operator never types the value, so weak strings like `redteam2026` are structurally impossible.
   - The string in the binary is a high-entropy random — useless as a YARA rule across engagements.
   - **Not a cryptographic secret.** If `engagement_id` leaks (captured agent, reversed binary), the attacker can join the DHT but cannot decrypt sealed envelopes or impersonate agents — authentication and encryption are independent. The entropy bar is "make scanning infeasible," not "protect a key."
   - Cheap rotation: new engagement = new bundle = new ID. No re-keying of long-term identities.
   - Topic-prefix namespacing prevents cross-engagement bleed even if two engagement bundles ever shared a DHT (shouldn't happen, but defense-in-depth).

   Implementation impact:
   - Phase 0: add `engagement_id` to the `[mesh]` config block as a 32-char hex string, no default.
   - Phase 3: read `engagement_id` from config, build the DHT protocol name dynamically from it, refuse to start if missing.
   - Phase 4: topic helpers in `nexus-mesh/src/topics.rs` take `engagement_id` and prepend it.
   - Phase 7: bridge config carries the same `engagement_id`; bridges only forward envelopes from topics under their configured prefix.
   - Phase 10: add `nexus-server init-engagement` subcommand that generates the bundle (engagement_id, server X25519 pubkey, bridge multiaddrs, agent CA cert) and writes it as a single tarball operators distribute out of band.

   Encoding: lowercase hex (32 chars) to match existing crypto idioms in `nexus-common`. Crockford-base32 (26 chars) is acceptable if shorter strings ever matter for logs or URLs.
