# Operator console distribution

The operator console is a Tauri 2 + Leptos desktop application that
talks to the C2's A2A plane over mTLS. v1.2 ships codesigned bundles
for macOS and Windows via CI; Linux ships unsigned.

> Build / sign internals: [`../v1.2/codesigning.md`](../v1.2/codesigning.md).
> First-run UX assumes a deployment configured per
> [`production.md`](production.md).

---

## Build matrix

| Platform | Bundle format | Signing | Notarization |
|---|---|---|---|
| macOS (universal) | `.app` / `.dmg` | Developer ID Application (`APPLE_*` secrets) | Yes — Apple Notary service |
| Windows | `.msi` / `.exe` | Authenticode (`WINDOWS_*` secrets) | n/a |
| Linux | `.AppImage` / `.deb` / `.rpm` | Unsigned | n/a |

CI workflow:
[`.github/workflows/tauri-build.yml`](../../.github/workflows/tauri-build.yml).
Triggered on tag push (`v*`) or manual dispatch. Artifacts uploaded
per-OS.

## Where to download

| Source | Audience |
|---|---|
| GitHub Actions artifacts on each tagged release | Operators with repo access |
| Internal artifact registry (your team's distribution) | Production rollout |
| `nexus-console/src-tauri/target/release/bundle/` after local `cargo tauri build` | Developers, evaluators |

## First-run setup

### 1. Install the bundle

- **macOS**: open the `.dmg`, drag `nexus-console.app` to `/Applications`.
- **Windows**: run the `.msi`. Accept the UAC prompt.
- **Linux**: `chmod +x nexus-console.AppImage && ./nexus-console.AppImage`
  (or install the `.deb` via `apt`).

### 2. Verify the signature

Always verify on first install (and after any update):

#### macOS

```bash
spctl --assess --type execute --verbose /Applications/nexus-console.app
# Expected: "source=Notarized Developer ID"
```

If `spctl` rejects with `source=No matching credential` or similar,
the bundle is **not** the signed CI artifact — stop and re-download
from the canonical source.

#### Windows

```powershell
signtool verify /v /pa "C:\Program Files\nexus-console\nexus-console.exe"
# Expected: "Successfully verified" with the chain anchored at your CA.
```

#### Linux

Unsigned by design. Verify the SHA-256 of the AppImage against the
checksum published in the CI artifact:

```bash
sha256sum nexus-console.AppImage
# Compare against the SHA-256 in the GitHub Actions artifact metadata.
```

### 3. First launch + Connect dialog

Open the console. The **Connect** dialog asks for:

| Field | Example | Notes |
|---|---|---|
| C2 endpoint URL | `https://c2.internal:50052` | Must be `https://` for mTLS |
| CA cert path | `/etc/nexus-console/ca.crt.pem` | The CA that signed the server cert |
| Client cert path | `/etc/nexus-console/operator.crt.pem` | Your operator client cert |
| Client key path | `/etc/nexus-console/operator.key.pem` | Match the client cert; mode 0o600 |
| Accept unsigned card | Off | Production: always off |

The console persists these as a named "connection profile" so you
don't have to re-enter them each session.

### 4. Connection profile storage

The Tauri shell stores profiles in the platform-standard config dir:

| OS | Path |
|---|---|
| Linux | `$XDG_CONFIG_HOME/nexus-console/profiles.json` (defaults to `~/.config/nexus-console/profiles.json`) |
| macOS | `~/Library/Application Support/nexus-console/profiles.json` |
| Windows | `%APPDATA%\nexus-console\profiles.json` |

Each profile holds the four cert / endpoint fields. The cert + key
material itself stays on disk at the paths you supplied — the
profile only stores the paths.

### 5. Connect

Click **Connect**. The console:

1. Dials the C2 endpoint with the client cert + key as mTLS material.
2. Calls `GetAgentCard` (verifies the Ed25519 signature if the
   server is configured to sign).
3. Calls `ListRegisteredAgents` and renders the agent list.

If any step fails, the dialog reports the failure with a specific
error (`UnknownIssuer`, `BadCertificate`, `PermissionDenied`,
`UnimplementedReflection`, etc.). See
[`local-dev.md`](local-dev.md#troubleshooting) for the troubleshooting
matrix.

### 6. Open a shell

Select an agent → **Open shell**. The console sends a `shell-open`
control frame targeting the agent's peer-id (hex). The C2's
`OperatorRouter` consults the capability matrix
([`production.md`](production.md#capability-matrix)); if allowed, a
new `ShellSession` opens on the agent and an xterm.js terminal
appears in the console.

PTY input → C2 → agent. PTY output → C2 → console. Audit records
land on the server (`shell_session_open`, `shell_session_close`).

## Updating the console

Bundles don't auto-update in v1.2. To upgrade:

1. Quit the running console.
2. Install the new bundle over the old one (overwrites in place).
3. Re-verify the signature per step 2.
4. Relaunch. Connection profiles persist.

For air-gapped operator stations, mirror the CI artifacts internally
and distribute via your normal process.

## Headless operator (CI / scripted)

For CI smoke tests or scripted operator flows, use the headless
example binary instead of the Tauri shell:

```bash
cargo run --release -p nexus-a2a --example headless_operator -- \
    --c2 https://c2.internal:50052 \
    --ca /etc/nexus-console/ca.crt.pem \
    --cert /etc/nexus-console/operator.crt.pem \
    --key /etc/nexus-console/operator.key.pem
```

It exercises the same A2A surface (`get_agent_card_verified`,
`list_registered_agents`, `send_streaming_message` with a shell-open
frame) without the GUI. The v1.1 demo gate (`./scripts/demo.sh`) uses
the same plumbing.
