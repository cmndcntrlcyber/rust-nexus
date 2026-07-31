#!/usr/bin/env bash
# scripts/operator-package.sh — Build a self-contained operator console
# package that can be sent to an operator workstation.
#
# Creates scripts/operator-package/ containing:
#   nexus-console         — Tauri desktop binary
#   ca.crt.pem            — CA cert (operator uses this to verify the server)
#   client.crt.pem        — Operator client cert (mTLS identity)
#   client.key.pem        — Operator client key  (mode 0600)
#   launch.sh             — Pre-configured launcher (sets env vars, runs binary)
#   README.txt            — Quick-start instructions
#
# Usage:
#   ./scripts/operator-package.sh [OPTIONS]
#
# Options:
#   --ip IP            Public IP or hostname of the C2 server.
#                        Baked into launch.sh as NEXUS_SERVER_ADDR.
#                        Accepts hostnames (e.g. c2.example.com) or IPs.
#   --port PORT        A2A port (default: 443 when --ip is given, else prompted)
#   --certs-dir DIR    Cert directory with ca.crt.pem + client.crt.pem +
#                        client.key.pem (default: ./certs/prod if present,
#                        else ./certs/nexus-agent)
#   --no-build         Skip console build; fail if binary is missing
#   --zip              Also produce scripts/operator-package.zip
#   -h|--help          Show this help
#
# Examples:
#   # Basic
#   ./scripts/operator-package.sh
#
#   # With C2 address baked in (operator just runs launch.sh)
#   ./scripts/operator-package.sh --ip c2.example.com
#
#   # With non-default port
#   ./scripts/operator-package.sh --ip c2.example.com --port 50052
#
#   # Build + zip for distribution
#   ./scripts/operator-package.sh --ip c2.example.com --zip

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG_DIR="${REPO_ROOT}/scripts/operator-package"

log()  { printf '\033[1;34m[operator-package]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[operator-package]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[operator-package]\033[0m %s\n' "$*" >&2; exit 1; }

# ---------- defaults ----------
CERTS_DIR=""
C2_HOST=""
C2_PORT="50052"
NO_BUILD=0
MAKE_ZIP=0

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --ip)         C2_HOST="$2";  shift 2 ;;
        --port)       C2_PORT="$2";  shift 2 ;;
        --certs-dir)  CERTS_DIR="$2"; shift 2 ;;
        --no-build)   NO_BUILD=1;    shift ;;
        --zip)        MAKE_ZIP=1;    shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) die "unknown argument: $1 (try --help)" ;;
    esac
done

# ---------- cert dir resolution ----------
if [[ -z "$CERTS_DIR" ]]; then
    if [[ -f "${REPO_ROOT}/certs/prod/ca.crt.pem" ]]; then
        CERTS_DIR="${REPO_ROOT}/certs/prod"
    elif [[ -f "${REPO_ROOT}/certs/nexus-agent/ca.crt.pem" ]]; then
        CERTS_DIR="${REPO_ROOT}/certs/nexus-agent"
    else
        die "no cert directory found; run scripts/gen-certs-prod.sh first or pass --certs-dir"
    fi
fi
log "certs: ${CERTS_DIR}"

for f in ca.crt.pem client.crt.pem client.key.pem; do
    [[ -f "${CERTS_DIR}/${f}" ]] || \
        die "missing cert: ${CERTS_DIR}/${f} (run scripts/gen-certs-prod.sh)"
done

# ---------- C2 address ----------
if [[ -n "$C2_HOST" ]]; then
    SERVER_ADDR="https://${C2_HOST}:${C2_PORT}"
    log "C2 address: ${SERVER_ADDR}"
else
    SERVER_ADDR=""
    warn "no --ip provided; launch.sh will prompt the operator for the C2 address"
fi

# ---------- console binary ----------
BINARY="${REPO_ROOT}/target/release/nexus-console"
if [[ ! -f "$BINARY" ]]; then
    if [[ "$NO_BUILD" -eq 1 ]]; then
        die "console binary not found and --no-build was set: ${BINARY}"
    fi
    warn "nexus-console binary not found — building now..."
    (cd "${REPO_ROOT}/nexus-console/src-tauri" && cargo tauri build)
fi
[[ -f "$BINARY" ]] || die "binary still missing after build — check cargo output"
log "binary: ${BINARY} ($(du -sh "$BINARY" | cut -f1))"

# ---------- stage package directory ----------
log "staging ${PKG_DIR}"
rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}"

install -m 0755 "${BINARY}"                        "${PKG_DIR}/nexus-console"
install -m 0644 "${CERTS_DIR}/ca.crt.pem"          "${PKG_DIR}/ca.crt.pem"
install -m 0644 "${CERTS_DIR}/client.crt.pem"      "${PKG_DIR}/client.crt.pem"
install -m 0600 "${CERTS_DIR}/client.key.pem"      "${PKG_DIR}/client.key.pem"

# ---------- launch.sh ----------
log "writing launch.sh"
if [[ -n "$SERVER_ADDR" ]]; then
    ADDR_LINE="NEXUS_SERVER_ADDR=\"${SERVER_ADDR}\""
    ADDR_NOTE="# C2 address pre-configured at package time"
else
    ADDR_LINE='NEXUS_SERVER_ADDR="${NEXUS_SERVER_ADDR:-}"'
    ADDR_NOTE="# Set NEXUS_SERVER_ADDR before running, or the connect dialog will prompt"
fi

cat > "${PKG_DIR}/launch.sh" <<EOF
#!/usr/bin/env bash
# launch.sh — Launch the nexus-console operator application.
# Run this script from the package directory (or set PKG_DIR explicitly).
# Generated $(date -u '+%Y-%m-%d %H:%M UTC')

set -euo pipefail

PKG_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"

${ADDR_NOTE}
${ADDR_LINE}

export NEXUS_CA_CERT="\${PKG_DIR}/ca.crt.pem"
export NEXUS_CLIENT_CERT="\${PKG_DIR}/client.crt.pem"
export NEXUS_CLIENT_KEY="\${PKG_DIR}/client.key.pem"
export NEXUS_SERVER_ADDR
export RUST_LOG="\${RUST_LOG:-info}"

echo "[nexus-console] C2:          \${NEXUS_SERVER_ADDR:-<will be prompted>}"
echo "[nexus-console] CA cert:     \${NEXUS_CA_CERT}"
echo "[nexus-console] Client cert: \${NEXUS_CLIENT_CERT}"

exec "\${PKG_DIR}/nexus-console"
EOF
chmod 755 "${PKG_DIR}/launch.sh"

# ---------- README.txt ----------
log "writing README.txt"
cat > "${PKG_DIR}/README.txt" <<EOF
nexus-console operator package
Generated: $(date -u '+%Y-%m-%d %H:%M UTC')
$(if [[ -n "$C2_HOST" ]]; then echo "C2 server:  ${SERVER_ADDR}"; else echo "C2 server:  (not pre-configured — enter in the connect dialog)"; fi)
================================================================

QUICK START
-----------
1. Run the launcher:
       bash launch.sh

   Or on Linux, make it executable and double-click:
       chmod +x nexus-console launch.sh
       ./launch.sh

2. The connect dialog opens pre-filled with the C2 address.
   Check "Allow non-loopback address" if the server is remote.
   Click Connect.

3. The agent list populates. Select an agent → open a shell.

FILES IN THIS PACKAGE
---------------------
  nexus-console     Desktop binary (Tauri/Leptos)
  launch.sh         Pre-configured launcher (sets cert env vars)
  ca.crt.pem        CA cert — verifies the C2 server's TLS cert
  client.crt.pem    Your operator identity cert (mTLS)
  client.key.pem    Your operator private key  (keep this secret)
  README.txt        This file

TROUBLESHOOTING
---------------
  "timed out after 15s"   — C2 server is not running or port is blocked
  "connect error"         — Check the C2 address and port
  "TLS handshake failed"  — ca.crt.pem doesn't match the server's cert
  No agents listed        — Agent not registered or not in capabilities.json

SECURITY NOTES
--------------
  - client.key.pem is your private key. Do not share it.
  - Verify the C2 address before connecting.
  - The CA cert in this package must match the one on the C2 server.
================================================================
EOF

# ---------- optional zip ----------
if [[ "$MAKE_ZIP" -eq 1 ]]; then
    ZIP_OUT="${REPO_ROOT}/scripts/operator-package.zip"
    log "creating ${ZIP_OUT}"
    (cd "${REPO_ROOT}/scripts" && zip -qr operator-package.zip operator-package/)
    log "zip: ${ZIP_OUT} ($(du -sh "$ZIP_OUT" | cut -f1))"
fi

# ---------- summary ----------
log "package ready: ${PKG_DIR}"
echo ""
echo "  Contents:"
ls -lh "${PKG_DIR}" | awk 'NR>1 {printf "    %s\n", $0}'
echo ""
if [[ -n "$C2_HOST" ]]; then
    echo "  Operator runs:  bash launch.sh"
    echo "  C2 pre-set to:  ${SERVER_ADDR}"
else
    echo "  Operator runs:  bash launch.sh"
    echo "  Note: no --ip given; operator must enter C2 address in the dialog"
fi
if [[ "$MAKE_ZIP" -eq 1 ]]; then
    echo ""
    echo "  Distributable zip: ${REPO_ROOT}/scripts/operator-package.zip"
fi
echo ""
