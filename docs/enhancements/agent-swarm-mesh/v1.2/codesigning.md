# v1.2 Tauri bundle codesigning (D-V1.2-F)

> See also: [`../deployment/operator-console.md`](../deployment/operator-console.md)
> for the operator-facing install + signature verification flow.

The operator console ships as a Tauri bundle. v1.2 wires CI codesigning
for **macOS** and **Windows**; the Linux bundle is shipped unsigned (no
platform requirement). Driven by `.github/workflows/tauri-build.yml`.

## CI secrets required

Set these as GitHub Actions repository secrets:

### macOS

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | Password unlocking the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `"Developer ID Application: Acme Inc (TEAMID)"` |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_PASSWORD` | App-specific password for notarization |
| `APPLE_TEAM_ID` | Developer Team ID |

### Windows

| Secret | Purpose |
|---|---|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` codesigning certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password unlocking the `.pfx` |

## How signing happens

Tauri's `cargo tauri build` step reads the env vars set in the workflow.
The relevant Tauri config keys (in `nexus-console/src-tauri/tauri.conf.json`):

- `bundle.macOS.signingIdentity` — Tauri picks up `APPLE_SIGNING_IDENTITY`
  automatically.
- `bundle.macOS.providerShortName` — populated from `APPLE_TEAM_ID`.
- `bundle.windows.certificateThumbprint` — Tauri imports the cert via
  `WINDOWS_CERTIFICATE` + `WINDOWS_CERTIFICATE_PASSWORD`.

The build script does **not** hard-code identities — operators
rotating certs only need to rotate the GitHub secret values.

## Local dev path (unsigned)

```bash
cd nexus-console/src-tauri
cargo tauri build
```

Without the env vars, the bundle is produced **unsigned**. macOS will
warn the user when launching ("can't be verified"). Distribute via
internal channels only; for external release always run through CI.

## Verifying a signed bundle

### macOS

```bash
spctl --assess --type execute --verbose ./nexus-console.app
# expect: "source=Notarized Developer ID"
```

### Windows

```powershell
signtool verify /v /pa .\nexus-console.exe
```

## Cert rotation

When a signing cert nears expiry:

1. Generate / acquire a new cert.
2. Encode to base64 (for `APPLE_CERTIFICATE` / `WINDOWS_CERTIFICATE`):
   ```bash
   base64 < new-cert.p12 | tr -d '\n' > new-cert.b64
   ```
3. Update the corresponding GitHub secret.
4. Tag-push a new release to trigger a signed build.

No code changes are needed — the workflow always reads from secrets.
