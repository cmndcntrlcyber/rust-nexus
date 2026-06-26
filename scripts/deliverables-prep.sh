#!/usr/bin/env bash
# scripts/deliverables-prep.sh — Build and stage all agent deliverables
# (Linux and/or Windows) in one command.
#
# Wraps scripts/build-agent-bundles.sh for both OS targets and collects
# the resulting zips into dist/agents/ (or --out DIR).
#
# Usage:
#   ./scripts/deliverables-prep.sh [OPTIONS]
#
# Options:
#   --lin-count N      Number of Linux agent bundles to produce (default: 0)
#   --win-count N      Number of Windows agent bundles to produce (default: 0)
#   --ip IP            Public IP or hostname of the C2 server.
#                        Constructs --server-addr https://<ip>:<port>.
#                        Ignored if --server-addr is given explicitly.
#   --port PORT        Port used with --ip (default: 443)
#   --server-addr URL  Full C2 URL (overrides --ip/--port).
#                        Default: $NEXUS_C2_ADDR or https://c2.example.com:50052
#   --certs-dir DIR    CA cert directory (default: ./certs/prod if present,
#                        else ./certs/nexus-agent)
#   --out DIR          Output directory for zips (default: ./dist/agents)
#   --lin-start N      Start index for Linux bundle numbering (default: 1)
#   --win-start N      Start index for Windows bundle numbering (default: 1)
#   --days N           Client cert validity in days (default: 365)
#   --no-build         Skip cargo build; use existing binaries
#   --force            Overwrite existing bundle zips
#   -h|--help          Show this help
#
# Examples:
#   # 3 Linux + 3 Windows bundles connecting to a known C2
#   ./scripts/deliverables-prep.sh --lin-count 3 --win-count 3 --ip c2.example.com
#
#   # Linux only, custom port, fresh certs
#   ./scripts/deliverables-prep.sh --lin-count 5 --ip <your-server-ip> --port 50052
#
#   # Add 2 more Linux agents starting at index 4 (avoids CN collision)
#   ./scripts/deliverables-prep.sh --lin-count 2 --lin-start 4 --ip c2.example.com --force
#
#   # Windows only, custom cert dir
#   ./scripts/deliverables-prep.sh --win-count 4 --certs-dir ./certs/prod --ip c2.example.com

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_SCRIPT="${REPO_ROOT}/scripts/build-agent-bundles.sh"

log()  { printf '\033[1;34m[deliverables-prep]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[deliverables-prep]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[deliverables-prep]\033[0m %s\n' "$*" >&2; exit 1; }

# ---------- defaults ----------
LIN_COUNT=0
WIN_COUNT=0
LIN_START=1
WIN_START=1
C2_HOST=""
C2_PORT="50052"
SERVER_ADDR=""
CERTS_DIR=""
OUT_DIR="${REPO_ROOT}/dist/agents"
DAYS=365
NO_BUILD=0
FORCE=0

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --lin-count)   LIN_COUNT="$2";   shift 2 ;;
        --win-count)   WIN_COUNT="$2";   shift 2 ;;
        --lin-start)   LIN_START="$2";   shift 2 ;;
        --win-start)   WIN_START="$2";   shift 2 ;;
        --ip)          C2_HOST="$2";     shift 2 ;;
        --port)        C2_PORT="$2";     shift 2 ;;
        --server-addr) SERVER_ADDR="$2"; shift 2 ;;
        --certs-dir)   CERTS_DIR="$2";   shift 2 ;;
        --out)         OUT_DIR="$2";     shift 2 ;;
        --days)        DAYS="$2";        shift 2 ;;
        --no-build)    NO_BUILD=1;       shift ;;
        --force)       FORCE=1;          shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) die "unknown argument: $1 (try --help)" ;;
    esac
done

# ---------- validation ----------
[[ "$LIN_COUNT" =~ ^[0-9]+$ ]] || die "--lin-count must be a non-negative integer"
[[ "$WIN_COUNT" =~ ^[0-9]+$ ]] || die "--win-count must be a non-negative integer"
[[ "$LIN_COUNT" -gt 0 || "$WIN_COUNT" -gt 0 ]] || \
    die "nothing to do — specify --lin-count and/or --win-count"

[[ -x "$BUNDLE_SCRIPT" ]] || die "bundle script not found or not executable: ${BUNDLE_SCRIPT}"

# ---------- server address ----------
if [[ -z "$SERVER_ADDR" ]]; then
    if [[ -n "$C2_HOST" ]]; then
        SERVER_ADDR="https://${C2_HOST}:${C2_PORT}"
    else
        # Use the default already baked into build-agent-bundles.sh
        SERVER_ADDR="${NEXUS_C2_ADDR:-https://c2.example.com:50052}"
        warn "no --ip or --server-addr given; using default: ${SERVER_ADDR}"
    fi
fi
log "C2 address: ${SERVER_ADDR}"

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

# Verify CA key exists (needed to mint client certs)
[[ -f "${CERTS_DIR}/ca.key.pem" ]] || \
    die "ca.key.pem missing from ${CERTS_DIR} — needed to mint agent certs"

mkdir -p "${OUT_DIR}"

# ---------- build common args ----------
COMMON_ARGS=(
    --certs-dir "${CERTS_DIR}"
    --server-addr "${SERVER_ADDR}"
    --out "${OUT_DIR}"
    --days "${DAYS}"
)
[[ "$NO_BUILD" -eq 1 ]] && COMMON_ARGS+=(--no-build)
[[ "$FORCE"    -eq 1 ]] && COMMON_ARGS+=(--force)

# ---------- build Linux bundles ----------
LIN_ZIPS=()
if [[ "$LIN_COUNT" -gt 0 ]]; then
    log "building ${LIN_COUNT} Linux bundle(s) (start-index=${LIN_START})"
    "${BUNDLE_SCRIPT}" \
        --os lin \
        --count "${LIN_COUNT}" \
        --start-index "${LIN_START}" \
        "${COMMON_ARGS[@]}"

    # Collect produced zip paths
    end_idx=$(( LIN_START + LIN_COUNT - 1 ))
    pad="%02d"
    [[ $end_idx -ge 100 ]] && pad="%03d"
    for i in $(seq "$LIN_START" "$end_idx"); do
        cn="agent-lin$(printf "${pad}" "$i")"
        LIN_ZIPS+=("${OUT_DIR}/${cn}.zip")
    done
fi

# ---------- build Windows bundles ----------
WIN_ZIPS=()
if [[ "$WIN_COUNT" -gt 0 ]]; then
    log "building ${WIN_COUNT} Windows bundle(s) (start-index=${WIN_START})"
    "${BUNDLE_SCRIPT}" \
        --os win \
        --count "${WIN_COUNT}" \
        --start-index "${WIN_START}" \
        "${COMMON_ARGS[@]}"

    end_idx=$(( WIN_START + WIN_COUNT - 1 ))
    pad="%02d"
    [[ $end_idx -ge 100 ]] && pad="%03d"
    for i in $(seq "$WIN_START" "$end_idx"); do
        cn="agent-win$(printf "${pad}" "$i")"
        WIN_ZIPS+=("${OUT_DIR}/${cn}.zip")
    done
fi

# ---------- summary ----------
TOTAL=$(( LIN_COUNT + WIN_COUNT ))
echo ""
printf '\033[1;34m[deliverables-prep]\033[0m %s\n' \
    "────────────────────────────────────────────────────────"
log "done — ${TOTAL} bundle(s) ready in ${OUT_DIR}"
echo ""

if [[ "$LIN_COUNT" -gt 0 ]]; then
    echo "  Linux (${LIN_COUNT}):"
    for z in "${LIN_ZIPS[@]}"; do
        size="$(du -sh "$z" 2>/dev/null | cut -f1 || echo '?')"
        printf "    %-40s  %s\n" "$(basename "$z")" "${size}"
    done
    echo ""
fi

if [[ "$WIN_COUNT" -gt 0 ]]; then
    echo "  Windows (${WIN_COUNT}):"
    for z in "${WIN_ZIPS[@]}"; do
        size="$(du -sh "$z" 2>/dev/null | cut -f1 || echo '?')"
        printf "    %-40s  %s\n" "$(basename "$z")" "${size}"
    done
    echo ""
fi

echo "  C2:   ${SERVER_ADDR}"
echo "  Cert: ${CERTS_DIR}/ca.crt.pem"
echo ""
echo "  Deploy Linux:   unzip <name>.zip && cd <name> && sudo bash install-linux.sh"
echo "  Deploy Windows: extract zip, run install.bat as Administrator"
echo ""
echo "  After first start, grab the peer-id from the server journal:"
echo "    sudo journalctl -u nexus-server | grep 'agent registered'"
echo "  Then add it to /etc/nexus/capabilities.json and restart nexus-server."
echo ""
