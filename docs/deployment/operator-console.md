# Operator console

The operator console is a Tauri 2 + Leptos desktop application that
connects to the C2's A2A gRPC plane. The `deploy-operator-console.sh`
script handles building, cert wiring, and launching.

---

## Quick start

```bash
# From the repo root on the operator workstation
NEXUS_SERVER_ADDR=https://c2.example.com:50052 \
  ./scripts/deploy-operator-console.sh
```

The script:
1. Checks / installs build deps (WebKitGTK, protobuf-compiler, trunk,
   cargo-tauri, npm).
2. Builds the Tauri release bundle (~3 min on first build, fast on
   subsequent runs if nothing changed).
3. Auto-detects `certs/prod/` (if it exists) and exports cert env vars.
4. Launches the binary with `NEXUS_SERVER_ADDR` pre-set.

### Script flags

| Flag | Effect |
|---|---|
| *(none)* | Build + launch (release bundle) |
| `--dev` | `cargo tauri dev` — hot-reload, no bundling |
| `--no-build` | Skip build; launch previously-built binary |
| `--build-only` | Build only; do not launch |

### Env overrides

| Variable | Default | Notes |
|---|---|---|
| `NEXUS_SERVER_ADDR` | *(prompted)* | C2 URL, e.g. `https://c2.example.com:50052` |
| `CERT_DIR` | `./certs/prod` if present, else `./certs/nexus-agent` | Directory with `ca.crt.pem`, `client.crt.pem`, `client.key.pem` |

---

## Connect dialog

When the console opens, the **Connect** dialog appears:

| Field | Description |
|---|---|
| C2 A2A address | Pre-filled from `NEXUS_SERVER_ADDR` if set |
| Allow non-loopback address | Check when connecting to a remote C2 (non-localhost) |

Click **Connect**. The console:
1. Dials the C2 endpoint.
2. Calls `GetAgentCard` to verify the server identity.
3. Shows the agent list.

If the connection succeeds, the dialog transitions to the main console view:
the agent list on the left, terminal pane on the right, and status bar.

### Connection errors

| Error | Likely cause |
|---|---|
| `timed out after 15 s` | nexus-server not running or port 50052 is blocked by firewall |
| `connect: transport error` | gRPC not supported at that address/port — verify port 50052 is open |
| `TLS handshake failed` | Cert mismatch — `NEXUS_CA_CERT` doesn't trust the server's cert |
| `PermissionDenied` | Agent peer-id not in `capabilities.json` |

---

## Build output

The console builds to:

```
target/release/nexus-console               # binary (run directly)
target/release/bundle/deb/nexus-console_*.deb
target/release/bundle/rpm/nexus-console_*.rpm
target/release/bundle/appimage/nexus-console_*.AppImage
```

The deploy script runs the binary directly (`target/release/nexus-console`),
not the bundle packages.

---

## Manual build

If you prefer to build without the deploy script:

```bash
cd nexus-console/src-tauri
cargo tauri build
# Binary at ../../target/release/nexus-console
```

Launch it manually with env vars:

```bash
NEXUS_SERVER_ADDR=https://c2.example.com:50052 \
NEXUS_CA_CERT=./certs/prod/ca.crt.pem \
NEXUS_CLIENT_CERT=./certs/prod/operator.crt.pem \
NEXUS_CLIENT_KEY=./certs/prod/operator.key.pem \
  ./target/release/nexus-console
```

> `NEXUS_CA_CERT` must point to the CA that signed the server's cert.
> `NEXUS_CLIENT_CERT` / `NEXUS_CLIENT_KEY` are your operator mTLS identity.

---

## Using the console

### Agent list

The left panel lists all agents currently registered with the C2 server.
Each row shows:
- Tag (from `capabilities.json` label, if set)
- OS
- Version
- Peer-id (truncated hex)
- Last seen timestamp

Click a row to select it.

### Shell session

With an agent selected, the terminal pane activates. Type commands; the
shell runs on the agent host via the C2's PTY relay.

The shell is OS-aware:
- Linux agents: `/bin/bash` (or `/bin/sh` as fallback)
- Windows agents: `powershell.exe` (or `cmd.exe` as fallback)

### Status bar

Shows: connected C2 URL, server name/version, active session id, agent count.

---

## Headless operator (CI / scripted)

For CI smoke tests or scripted operator flows, use the headless example:

```bash
cargo run --release -p nexus-a2a --example headless_operator -- \
    --c2 https://c2.example.com:50052
```

This exercises `get_agent_card`, `list_registered_agents`, and
`send_streaming_message` without the GUI.

---

## Cert setup reference

The deploy script reads from `CERT_DIR` (defaults to `./certs/prod` if
present). The expected files are:

```
$CERT_DIR/ca.crt.pem          # CA cert — verifies the server's TLS cert
$CERT_DIR/client.crt.pem      # Operator client cert (→ operator.crt.pem in prod)
$CERT_DIR/client.key.pem      # Operator client key  (→ operator.key.pem in prod)
```

Generate with:
```bash
./scripts/gen-certs-prod.sh \
  --domain c2.example.com \
  --ip <public-ip> \
  --out ./certs/prod
# gen-certs-prod.sh creates client.crt.pem / client.key.pem as symlinks
# automatically — no manual renaming needed.
```
