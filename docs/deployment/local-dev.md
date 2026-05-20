# Local development quickstart

Take a fresh box from `git clone` to a working three-process demo
(C2 server + agent + Tauri console) with mTLS in ~15 minutes.

> Tested on Linux (Ubuntu 24.04 + Fedora 40). macOS and Windows work
> for the workspace build but the Tauri console section assumes Linux
> for the example commands.

---

## 1. Prerequisites

| Component | Version | Why |
|---|---|---|
| Rust toolchain | stable (1.85+) | Workspace pin in `rust-toolchain.toml` |
| `openssl` | 1.1.1+ | Used by `scripts/gen-certs.sh` to provision dev certs |
| `protoc` (system) | Optional | The workspace vendors `protoc-bin-vendored`; no manual install needed |
| `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf` | latest | Tauri webview prerequisites (Linux only) |
| Trunk | latest | Builds the Leptos WASM UI: `cargo install trunk` |
| `wasm32-unknown-unknown` target | — | `rustup target add wasm32-unknown-unknown` |

On Debian / Ubuntu:

```bash
sudo apt update
sudo apt install -y \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    libssl-dev openssl build-essential pkg-config
cargo install trunk
rustup target add wasm32-unknown-unknown
```

## 2. Clone + workspace layout

```bash
git clone https://github.com/yourorg/rust-nexus.git
cd rust-nexus
```

The workspace has 14 crates. The ones you'll touch in this walkthrough:

```
nexus-common/         # NodeIdentity, sealed envelope, OsKind
nexus-a2a/            # A2A gRPC (cards, tls, capabilities, audit, interceptors)
nexus-infra/          # C2 server (A2A on :50052, NexusC2 on :50051)
nexus-agent/          # Cross-platform agent (PTY shell, transports/grpc.rs)
nexus-console/        # Operator desktop app
  ├── src-tauri/      # Tauri v2 wrapper (depends on Rust)
  └── ui/             # Leptos WASM frontend (excluded from workspace; built by trunk)
integration-tests/    # Cross-crate tests
scripts/              # gen-certs.sh, demo.sh, build.sh
```

## 3. Sanity check the workspace

```bash
cargo check --workspace --exclude nexus-console
```

Expected: clean build, ~20 warnings (mostly unused imports inherited
from the overlay crates).

Run the full test suite:

```bash
cargo test --workspace --exclude nexus-console
```

Expected: **170 / 170 pass**. If anything fails, stop and triage — the
deployment story assumes a green baseline.

## 4. Provision dev mTLS certs

```bash
./scripts/gen-certs.sh
```

This writes a self-signed Ed25519 CA + server cert (CN=`localhost`) +
client cert (CN=`operator-dev`) into `./certs/`:

```
certs/
├── ca.crt.pem          # CA root certificate
├── ca.key.pem          # CA private key (0o600; keep offline in prod)
├── server.crt.pem      # Server cert (SAN=localhost,127.0.0.1)
├── server.key.pem      # Server private key
├── client.crt.pem      # Operator client cert
└── client.key.pem      # Operator client private key
```

> These certs are **dev-only**. They're self-signed by a CA that lives
> on the same box. For production, see
> [`production.md`](production.md) § CA strategy.

Export the canonical env vars (reserved names — never rename):

```bash
export NEXUS_CA_CERT="$(pwd)/certs/ca.crt.pem"
export NEXUS_SERVER_CERT="$(pwd)/certs/server.crt.pem"
export NEXUS_SERVER_KEY="$(pwd)/certs/server.key.pem"
export NEXUS_CLIENT_CERT="$(pwd)/certs/client.crt.pem"
export NEXUS_CLIENT_KEY="$(pwd)/certs/client.key.pem"
```

## 5. Run the C2 server

```bash
cargo run --release --bin nexus-server
```

Expected log lines (look for these to confirm v1.2 features are on):

```
A2A server starting addr=127.0.0.1:50052 insecure_network=false mtls=true
A2A AgentCard signed with server identity
A2A agent registered peer_id=<32-byte hex> os=linux version=0.2.0
```

Without a `NodeIdentity` configured, the AgentCard ships unsigned and
you'll see `... ships unsigned (no server identity configured)`. That's
fine for local dev — see [`production.md`](production.md) § Server
provisioning for the signed-card setup.

## 6. Run an agent

In another terminal, with the same env vars exported:

```bash
cargo run --release -p nexus-agent -- \
    --c2 https://localhost:50052 \
    --transport grpc
```

The agent dials the C2's A2A port, sends an `agent-register` first
frame, and stays connected. You'll see on the server side:

```
A2A agent registered peer_id=<hex> os=linux version=...
```

## 7. Run the Tauri operator console

In a third terminal:

```bash
# Build the Leptos WASM UI once
cd nexus-console/ui
trunk build --release
cd ..

# Run the Tauri shell in dev mode
cd src-tauri
cargo tauri dev
```

The console window opens. Click **Connect** and fill in:

- **C2 URL**: `https://localhost:50052`
- **CA cert**: `<repo>/certs/ca.crt.pem`
- **Client cert**: `<repo>/certs/client.crt.pem`
- **Client key**: `<repo>/certs/client.key.pem`

The connection dialog calls `GetAgentCard` (verifies the Ed25519
signature if the server is configured with a NodeIdentity), then
`ListRegisteredAgents`. Your agent from step 6 should appear in the
list.

## 8. Open an interactive shell

Click the agent's row → **Open shell**. An xterm.js terminal opens.
Type `whoami` (or `Get-Host` on Windows agents) and confirm the output
round-trips.

> Under the hood: the console sends a `shell-open` control frame
> targeting the agent's peer-id (hex). The C2's `OperatorRouter`
> consults the capability matrix, picks the agent's bidi back-channel
> from `AgentChannels`, and proxies the operator's PTY input + agent's
> PTY output via `Part::file` byte frames.

## 9. Headless PASS gate

If you don't have libwebkit2gtk available (e.g. CI container), skip
the console and run the integration test loop:

```bash
./scripts/demo.sh
# [demo] PASS — v1.1 A2A loopback round-trip verified
```

For the v1.2 mTLS round-trip:

```bash
cargo test -p integration-tests --test a2a_mtls -- --ignored
# test mtls_round_trip ... ok
```

For the v1.2 agent-side bidi round-trip (PTY echo):

```bash
cargo test -p integration-tests --test a2a_interactive_shell
# test agent_round_trip_via_a2a_bidi ... ok
```

---

## Troubleshooting

### `TLS handshake failed` / `UnknownIssuer`

The console is dialing the C2 but doesn't trust the CA. Confirm
`NEXUS_CA_CERT` points at the same `ca.crt.pem` that signed
`server.crt.pem`. The TLS verifier compares the chain — if the server
cert's CN doesn't include `localhost` in its SAN, regenerate certs.

### `error: bind 127.0.0.1:50052 — Address already in use`

Another nexus-server is running, or a previous run didn't shut down
cleanly. Find it:

```bash
ss -tlnp | grep 50052
kill <pid>
```

### Tauri build fails: `package libwebkit2gtk-4.1 was not found`

Install the dev package:

```bash
sudo apt install libwebkit2gtk-4.1-dev
```

On older Ubuntu (≤22.04), `libwebkit2gtk-4.0-dev` works too with a
Tauri config tweak — see Tauri's upstream docs.

### Agent connects but doesn't appear in operator's list

The `ListRegisteredAgents` lister bridges from the legacy NexusC2
registry by default. v1.2 A2A-mode agents show up via `AgentChannels`
once they send their `agent-register` frame. Check:

- Server log has `A2A agent registered peer_id=…` after the agent
  started. If not, the agent's first frame isn't reaching the C2 —
  inspect with `RUST_LOG=trace`.
- The operator's `ListRegisteredAgents` response is `RegisteredAgents
  { agents: [...] }`. If empty, the lister is bridging only legacy
  agents — see `nexus-infra/src/a2a_lister.rs`.

### `cargo tauri build` says "Permission denied" launching codesign

You're on macOS without a signing cert. That's normal for local dev.
Use `cargo tauri dev` instead, or build unsigned with the warning.

### `./scripts/demo.sh` PASSes but `cargo test -p integration-tests --test a2a_mtls` is skipped

The mTLS test is `#[ignore]` by default. Pass `-- --ignored` and the
five env vars (step 4) — without those env vars it bails out
correctly.
