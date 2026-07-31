#!/usr/bin/env bash
# scripts/deploy-operator-console.sh — Local operator-console deployment.
#
# Builds the Tauri 2 + Leptos operator console from `nexus-console/`,
# wires it up to the mTLS certs in `certs/nexus-agent/`, and launches it
# against a user-supplied C2 endpoint.
#
# Tested on Ubuntu/Debian (Linux). For macOS/Windows bundles use the CI
# workflow at .github/workflows/tauri-build.yml.
#
# Usage:
#   ./scripts/deploy-operator-console.sh              # build + launch
#   ./scripts/deploy-operator-console.sh --dev        # `cargo tauri dev` instead of release bundle
#   ./scripts/deploy-operator-console.sh --no-build   # skip build, just launch
#   ./scripts/deploy-operator-console.sh --build-only # build, don't launch
#
# Env overrides:
#   NEXUS_SERVER_ADDR   — C2 endpoint URL (https://host:50052). If unset, script prompts.
#   CERT_DIR            — Override cert directory (default: ./certs/nexus-agent).

set -euo pipefail

# Ensure cargo is on PATH regardless of how the script is invoked (e.g. via
# sudo where /root/.cargo/bin may not be inherited). Try every common location.
for _cb in \
    "${CARGO_HOME:+${CARGO_HOME}/bin}" \
    "${HOME:-}/.cargo/bin" \
    "/root/.cargo/bin" \
    "/usr/local/cargo/bin"; do
    [[ -n "$_cb" && -d "$_cb" && ":${PATH}:" != *":${_cb}:"* ]] && PATH="${_cb}:${PATH}"
done
export PATH

# ---------- paths ----------
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Prefer prod certs if present; fall back to dev/test certs.
if [[ -d "${REPO_ROOT}/certs/nexus-agent" && -r "${REPO_ROOT}/certs/prod/ca.crt.pem" ]]; then
    CERT_DIR="${CERT_DIR:-${REPO_ROOT}/certs/prod}"
else
    CERT_DIR="${CERT_DIR:-${REPO_ROOT}/certs/nexus-agent}"
fi
CONSOLE_DIR="${REPO_ROOT}/nexus-console"
TAURI_DIR="${CONSOLE_DIR}/src-tauri"

# ---------- flags ----------
MODE="release"   # release | dev
DO_BUILD=1
DO_LAUNCH=1
for arg in "$@"; do
    case "$arg" in
        --dev)        MODE="dev" ;;
        --no-build)   DO_BUILD=0 ;;
        --build-only) DO_LAUNCH=0 ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "unknown flag: $arg" >&2; exit 2 ;;
    esac
done

# ---------- helpers ----------
log()  { printf '\033[1;34m[console-deploy]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[console-deploy]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[console-deploy]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() { command -v "$1" >/dev/null 2>&1; }

# ---------- 1. cert validation ----------
log "validating certs in ${CERT_DIR}"
_missing_cert=0
for f in ca.crt.pem client.crt.pem client.key.pem; do
    [[ -r "${CERT_DIR}/${f}" ]] || { _missing_cert=1; break; }
done
if [[ $_missing_cert -eq 1 ]]; then
    warn "cert(s) missing from ${CERT_DIR} — generating dev/test certs now"
    warn "For production, provision your own CA and set CERT_DIR accordingly."
    GEN_SCRIPT="${REPO_ROOT}/scripts/gen-certs.sh"
    [[ -x "$GEN_SCRIPT" ]] || die "gen-certs.sh not found or not executable: $GEN_SCRIPT"
    bash "$GEN_SCRIPT" "${CERT_DIR}"
    # gen-certs.sh writes ca/server/client — alias client→expected names if needed.
    for pair in "client.crt.pem:client.crt.pem" "client.key.pem:client.key.pem" "ca.crt.pem:ca.crt.pem"; do
        src="${CERT_DIR}/${pair#*:}"; dst="${CERT_DIR}/${pair%%:*}"
        [[ -f "$src" && ! -f "$dst" ]] && cp "$src" "$dst"
    done
fi
for f in ca.crt.pem client.crt.pem client.key.pem; do
    [[ -r "${CERT_DIR}/${f}" ]] || die "missing or unreadable cert: ${CERT_DIR}/${f}"
done

# Enforce 0600 on the key (rustls doesn't care, but our docs do).
key_mode=$(stat -c '%a' "${CERT_DIR}/client.key.pem")
if [[ "${key_mode}" != "600" ]]; then
    warn "client.key.pem mode is ${key_mode}; tightening to 600"
    chmod 600 "${CERT_DIR}/client.key.pem"
fi

# ---------- 2. system deps (Linux/Debian only) ----------
if [[ "${DO_BUILD}" -eq 1 ]]; then
    log "checking system build deps"
    APT_PKGS=(
        libwebkit2gtk-4.1-dev
        libayatana-appindicator3-dev
        librsvg2-dev
        patchelf
        libssl-dev
        openssl
        build-essential
        pkg-config
        protobuf-compiler
        curl
        git
    )
    missing=()
    for p in "${APT_PKGS[@]}"; do
        dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p")
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        log "installing missing apt packages: ${missing[*]}"
        sudo apt-get update
        sudo apt-get install -y "${missing[@]}"
    else
        log "all apt packages present"
    fi
fi

# ---------- 3. rust toolchain bits ----------
if [[ "${DO_BUILD}" -eq 1 ]]; then
    need_cmd cargo  || die "cargo not on PATH — install rustup first"
    need_cmd rustup || die "rustup not on PATH"

    if ! rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
        log "adding wasm32-unknown-unknown target"
        rustup target add wasm32-unknown-unknown
    fi

    if ! need_cmd trunk; then
        log "installing trunk (Leptos build tool)"
        cargo install trunk --locked
    fi

    if ! need_cmd cargo-tauri; then
        log "installing cargo-tauri CLI"
        cargo install tauri-cli --version '^2' --locked
    fi

    if ! need_cmd npm; then
        warn "npm not on PATH — Trunk pre-build hook vendors xterm.js via npm"
        warn "  install nodejs/npm (e.g. via nvm or apt) and re-run"
        die  "npm required for UI build"
    fi
fi

# ---------- 4. build ----------
BUNDLE_BIN="${REPO_ROOT}/target/release/nexus-console"

if [[ "${DO_BUILD}" -eq 1 ]]; then
    if [[ "${MODE}" == "release" ]]; then
        log "building Tauri release bundle (this takes a while on first build)"
        (cd "${TAURI_DIR}" && cargo tauri build)
        log "bundles in: ${REPO_ROOT}/target/release/bundle/"
    fi
    # In dev mode the build happens at launch time via `cargo tauri dev`.
fi

# ---------- 5. launch ----------
if [[ "${DO_LAUNCH}" -eq 0 ]]; then
    log "build-only: skipping launch"
    exit 0
fi

# Prompt for C2 endpoint if not pre-set.
if [[ -z "${NEXUS_SERVER_ADDR:-}" ]]; then
    printf 'C2 endpoint URL (e.g. https://c2.example.com:50052): '
    read -r NEXUS_SERVER_ADDR
fi
[[ -n "${NEXUS_SERVER_ADDR}" ]] || die "C2 endpoint required"
[[ "${NEXUS_SERVER_ADDR}" == https://* ]] || \
    warn "endpoint is not https:// — mTLS requires TLS; proceeding anyway"

export NEXUS_CA_CERT="${CERT_DIR}/ca.crt.pem"
export NEXUS_CLIENT_CERT="${CERT_DIR}/client.crt.pem"
export NEXUS_CLIENT_KEY="${CERT_DIR}/client.key.pem"
export NEXUS_SERVER_ADDR
export RUST_LOG="${RUST_LOG:-info}"

log "launching operator console"
log "  C2:           ${NEXUS_SERVER_ADDR}"
log "  CA cert:      ${NEXUS_CA_CERT}"
log "  Client cert:  ${NEXUS_CLIENT_CERT}"
log "  Client key:   ${NEXUS_CLIENT_KEY}"

if [[ "${MODE}" == "dev" ]]; then
    exec env -C "${TAURI_DIR}" cargo tauri dev
fi

if [[ ! -x "${BUNDLE_BIN}" ]]; then
    die "release binary not found at ${BUNDLE_BIN}; run without --no-build"
fi

exec "${BUNDLE_BIN}"
