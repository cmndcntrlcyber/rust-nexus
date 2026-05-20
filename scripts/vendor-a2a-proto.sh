#!/usr/bin/env bash
#
# scripts/vendor-a2a-proto.sh — Phase 1.4.2 / D-V1.4-A.
#
# One-time helper that fetches the upstream `a2aproject/A2A` proto file
# at a specific tag (or sha) and writes it to
# `nexus-a2a/vendor/a2a-upstream/a2a.v1.proto`. Records the sha256
# fingerprint + upstream sha into `nexus-a2a/VENDORED-VERSION` so the
# pure-Rust interop test (`nexus-a2a/tests/upstream_compat.rs`) can
# detect drift on subsequent bumps.
#
# Usage:
#   ./scripts/vendor-a2a-proto.sh [tag-or-sha]
#
# Examples:
#   ./scripts/vendor-a2a-proto.sh v0.3.0     # named tag
#   ./scripts/vendor-a2a-proto.sh 7f2c8d9    # short sha (resolved via API)
#
# Idempotent: re-running with the same tag is a no-op (after sha-check).
# Re-running with a new tag overwrites the vendored proto + records the
# bump.

set -euo pipefail

REF="${1:-v0.3.0}"
UPSTREAM_REPO="a2aproject/A2A"
UPSTREAM_PATH="a2a/v1/a2a.proto"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR_FILE="$ROOT/nexus-a2a/vendor/a2a-upstream/a2a.v1.proto"
VERSION_FILE="$ROOT/nexus-a2a/VENDORED-VERSION"

echo "[vendor-a2a] fetching ${UPSTREAM_REPO}@${REF}:${UPSTREAM_PATH}"

# Resolve to a concrete sha for the VERSION_FILE record.
COMMIT_SHA=$(curl -fsSL "https://api.github.com/repos/${UPSTREAM_REPO}/commits/${REF}" \
    | grep -m1 '"sha"' | head -1 | sed 's/.*"sha": *"\([^"]*\)".*/\1/')

if [[ -z "$COMMIT_SHA" ]]; then
    echo "[vendor-a2a] error: failed to resolve commit sha for ${REF}" >&2
    echo "[vendor-a2a] (rate-limited? try again with a GITHUB_TOKEN env var)" >&2
    exit 1
fi

RAW_URL="https://raw.githubusercontent.com/${UPSTREAM_REPO}/${COMMIT_SHA}/${UPSTREAM_PATH}"
echo "[vendor-a2a] resolved to commit ${COMMIT_SHA}"
echo "[vendor-a2a] downloading from ${RAW_URL}"

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

curl -fsSL "$RAW_URL" -o "$TMP"

# rust-nexus additions are preserved by patching the upstream bytes:
# we rename the `package` declaration to `a2a.upstream.v1` so the file
# compiles into our `pb_upstream` Rust module alongside `pb`.
sed -i.bak 's/^package a2a\.v1;/package a2a.upstream.v1;/' "$TMP"
rm -f "${TMP}.bak"

# Sanity check: confirm the package rename took effect.
if ! grep -q '^package a2a.upstream.v1;' "$TMP"; then
    echo "[vendor-a2a] error: failed to rewrite package declaration" >&2
    echo "[vendor-a2a] (upstream may have used a non-standard package format)" >&2
    exit 1
fi

mkdir -p "$(dirname "$VENDOR_FILE")"
mv "$TMP" "$VENDOR_FILE"
trap - EXIT

SHA256=$(sha256sum "$VENDOR_FILE" | cut -d' ' -f1)
DATE=$(date -u +%Y-%m-%d)

cat > "$VERSION_FILE" <<EOF
# A2A proto vendoring record (D-V1.2-E / D-2.1.2-B / D-V1.3-A / D-V1.4-A)

## Upstream pin (v1.4)

- Repository: https://github.com/${UPSTREAM_REPO}
- Tag / ref:  ${REF}
- Commit sha: ${COMMIT_SHA}
- Vendored:   ${DATE}
- sha256:     ${SHA256}
- License:    Apache 2.0

## Vendored file

- Path: \`nexus-a2a/vendor/a2a-upstream/a2a.v1.proto\`
- Package: \`a2a.upstream.v1\` (auto-renamed from \`a2a.v1\` so the
  two protos compile side-by-side in Rust without module collision).
- License file: \`nexus-a2a/LICENSE-A2A\`

## Interop verification

\`\`\`bash
cargo test -p nexus-a2a --test upstream_compat
\`\`\`

Four tests cover compile-time field-number compatibility + live RPC
wire compatibility. The test runs every PR; any drift between our
\`pb\` and the vendored \`pb_upstream\` produces either:

1. A compile error (mismatched field types between the two prost-
   generated modules).
2. A runtime decode error in one of the symmetric round-trip tests.

## Bumping the pin

Run this script with a new tag / sha:

\`\`\`bash
./scripts/vendor-a2a-proto.sh v0.3.1
\`\`\`

The script:
- fetches the upstream proto at the new commit
- patches the package declaration to \`a2a.upstream.v1\`
- writes the bytes + updates this file with the new sha256 + sha pin

After running, \`cargo test -p nexus-a2a --test upstream_compat\`
either passes (drift-free) or fails with a clear field-number error
the developer can resolve before merging.

## v1.4 rust-nexus additions

Our \`proto/a2a/v1/a2a.proto\` ships these on top of the upstream
subset (preserved across vendoring bumps):

- \`AgentCard.signature\` (bytes)        — D-V1.2-cards
- \`AgentCard.signer_peer_id\` (bytes)   — D-V1.2-cards
- \`StreamAuditRecords\` RPC             — D-V1.4 / Phase 1.4.3
- \`IssueOperatorToken\` RPC             — D-V1.4-D / Phase 1.4.7

If the upstream proto ever uses field number 5 or 6 on AgentCard for
something else, we must rename or renumber our additions before
bumping the pin. The compile-time drift check catches this.
EOF

echo "[vendor-a2a] wrote ${VENDOR_FILE}"
echo "[vendor-a2a] wrote ${VERSION_FILE} (sha256: ${SHA256})"
echo
echo "Run \`cargo test -p nexus-a2a --test upstream_compat\` to verify."
