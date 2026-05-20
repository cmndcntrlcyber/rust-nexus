# CI guide (scaffold, Phase 1.3.8)

Two workflows in `.github/workflows/`:

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | every PR + push to main | `cargo check`, `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, mTLS integration test, wasm32 build |
| `security-audit.yml` | every PR + weekly cron + manual dispatch | `cargo audit` (RustSec advisories) + `cargo deny check` (license + bans, primarily for AGPL) |
| `tauri-build.yml` | tag push (`v*`) + manual dispatch | Codesigned Tauri operator console bundles for macOS / Windows / Linux |

## Debugging a failing run

1. **clippy fails**: run `cargo clippy --workspace --exclude nexus-console --all-targets -- -D warnings` locally. The CI run uses the same invocation.
2. **fmt fails**: run `cargo fmt --all`. Commit the diff.
3. **mTLS integration test fails on CI but passes locally**: confirm the CI workflow's `./scripts/gen-certs.sh` step is producing certs in the right paths; the env vars in the test step are case-sensitive.
4. **cargo deny check fails**: usually a transitive dep pulled in an AGPL crate. Run `cargo deny check` locally for the same report. Either replace the offending dep or add a tightly-scoped exception in `deny.toml` (with justification in the diff message).

## What runs on dispatch vs cron

`cargo-audit` runs on every PR — fast (~10 s). `cargo-deny` runs both
on PR and weekly to catch DB-drift cases where a new advisory lands
for a dep we already have.

(Full content lands in Phase 1.3.8.)
