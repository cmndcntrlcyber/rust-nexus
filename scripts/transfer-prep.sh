#!/usr/bin/env bash
# scripts/transfer-prep.sh — Stage all files needed to stand up a
# nexus-server on a remote host.
#
# Creates scripts/transfer/ containing:
#   nexus-server          — compiled release binary
#   ca.crt.pem            — custom CA cert (verifies server cert)
#   server.crt.pem        — server TLS cert
#   server.key.pem        — server TLS key (mode 0600)
#   nexus.toml            — server config (bind address pre-patched if --ip given)
#   capabilities.json     — capability matrix template
#   nexus-server.service  — systemd unit
#   remote-host-prep.sh   — one-shot setup script to run on the remote host
#
# Usage:
#   ./scripts/transfer-prep.sh [OPTIONS]
#
# Options:
#   --ip IP            Public IP of the remote server. When provided:
#                        - The SCP/SSH commands in the output use this address
#                        - nexus.toml is pre-patched with the correct bind IP
#                        - remote-host-prep.sh bakes in the IP for its summary
#                        - UFW firewall rules are pre-configured in remote-host-prep.sh
#   --user USER        SSH user on the remote host (default: ubuntu)
#   --ssh-key FILE     SSH private key file (passed as -i to scp/ssh)
#   --certs-dir DIR    Cert directory (default: ./certs/prod if present,
#                        else ./certs/nexus-agent)
#   --no-build         Skip binary build even if binary is missing (will fail
#                        if target/release/nexus-server does not exist)
#   -h|--help          Show this help
#
# Examples:
#   # Basic (no IP known yet)
#   ./scripts/transfer-prep.sh
#
#   # With server IP — output shows ready-to-run scp + ssh commands
#   ./scripts/transfer-prep.sh --ip <your-server-ip> --user ubuntu
#
#   # With SSH key
#   ./scripts/transfer-prep.sh --ip <your-server-ip> --user ubuntu --ssh-key ~/.ssh/c2-key.pem
#
#   # Custom cert dir
#   ./scripts/transfer-prep.sh --ip <your-server-ip> --certs-dir ./certs/prod

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRANSFER_DIR="${REPO_ROOT}/scripts/transfer"

log()  { printf '\033[1;34m[transfer-prep]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[transfer-prep]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[transfer-prep]\033[0m %s\n' "$*" >&2; exit 1; }

# ---------- defaults ----------
CERTS_DIR=""
REMOTE_IP=""
REMOTE_USER="ubuntu"
SSH_KEY=""
NO_BUILD=0

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --ip)         REMOTE_IP="$2";   shift 2 ;;
        --user)       REMOTE_USER="$2"; shift 2 ;;
        --ssh-key)    SSH_KEY="$2";     shift 2 ;;
        --certs-dir)  CERTS_DIR="$2";   shift 2 ;;
        --no-build)   NO_BUILD=1;       shift ;;
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
log "certs:  ${CERTS_DIR}"

# Validate IP format if provided
if [[ -n "$REMOTE_IP" ]]; then
    if ! [[ "$REMOTE_IP" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]]; then
        die "--ip looks malformed: ${REMOTE_IP} (expected dotted-quad IPv4)"
    fi
    log "remote: ${REMOTE_USER}@${REMOTE_IP}"
fi

# Validate SSH key if provided
if [[ -n "$SSH_KEY" ]]; then
    [[ -f "$SSH_KEY" ]] || die "--ssh-key file not found: ${SSH_KEY}"
    log "ssh-key: ${SSH_KEY}"
fi

# ---------- binary ----------
BINARY="${REPO_ROOT}/target/release/nexus-server"
if [[ ! -f "$BINARY" ]]; then
    if [[ "$NO_BUILD" -eq 1 ]]; then
        die "binary not found and --no-build was set: ${BINARY}"
    fi
    warn "nexus-server binary not found — building now (this may take a few minutes)..."
    (cd "${REPO_ROOT}" && cargo build --release -p nexus-infra --bin nexus-server)
fi
[[ -f "$BINARY" ]] || die "binary still missing after build — check cargo output"
log "binary: ${BINARY}"

# ---------- validate certs ----------
for f in ca.crt.pem server.crt.pem server.key.pem; do
    [[ -f "${CERTS_DIR}/${f}" ]] || \
        die "missing cert: ${CERTS_DIR}/${f} — run scripts/gen-certs-prod.sh first"
done

# ---------- stage transfer directory ----------
log "staging ${TRANSFER_DIR}"
rm -rf "${TRANSFER_DIR}"
mkdir -p "${TRANSFER_DIR}"

# Binary
install -m 0755 "${BINARY}" "${TRANSFER_DIR}/nexus-server"

# Certs
install -m 0644 "${CERTS_DIR}/ca.crt.pem"     "${TRANSFER_DIR}/ca.crt.pem"
install -m 0644 "${CERTS_DIR}/server.crt.pem"  "${TRANSFER_DIR}/server.crt.pem"
install -m 0600 "${CERTS_DIR}/server.key.pem"  "${TRANSFER_DIR}/server.key.pem"

# nexus.toml — patch bind IP if --ip was given
TOML_SRC="${REPO_ROOT}/docs/deployment/examples/nexus.toml.example"
if [[ -n "$REMOTE_IP" ]]; then
    # Bind on all interfaces but record the public IP in a comment
    sed "s|bind = \"0.0.0.0:50052\"|bind = \"0.0.0.0:50052\"  # public: ${REMOTE_IP}|g" \
        "$TOML_SRC" > "${TRANSFER_DIR}/nexus.toml"
else
    install -m 0644 "$TOML_SRC" "${TRANSFER_DIR}/nexus.toml"
fi

install -m 0644 \
    "${REPO_ROOT}/config/capabilities.example.json" \
    "${TRANSFER_DIR}/capabilities.json"
install -m 0644 \
    "${REPO_ROOT}/docs/deployment/examples/nexus-server.service" \
    "${TRANSFER_DIR}/nexus-server.service"

# ---------- generate remote-host-prep.sh (bake in IP if known) ----------
log "writing remote-host-prep.sh"

# Build the IP block for the summary and firewall section
if [[ -n "$REMOTE_IP" ]]; then
    BAKED_IP="$REMOTE_IP"
else
    BAKED_IP="<not provided>"
fi

cat > "${TRANSFER_DIR}/remote-host-prep.sh" <<REMOTE_EOF
#!/usr/bin/env bash
# remote-host-prep.sh — One-shot nexus-server setup for a fresh Linux host.
# Run as root (or via sudo) from the directory containing the transfer files.
#
# What it does:
#   1. Creates the 'nexus' system user and required directories
#   2. Installs the nexus-server binary to /usr/local/bin/
#   3. Installs certs and config to /etc/nexus/
#   4. Writes /etc/nexus/server.env with NEXUS_* environment variables
#   5. Installs and enables the systemd service unit
#   6. Configures UFW firewall rules (if ufw is present)
#   7. Initialises the server NodeIdentity on first run
#   8. Starts the service and tails the journal
#
# Generated by transfer-prep.sh on $(date -u '+%Y-%m-%d %H:%M UTC')
# Remote IP at generation time: ${BAKED_IP}
#
# Usage:
#   sudo bash remote-host-prep.sh
#
# Env overrides:
#   NEXUS_BIND      — A2A bind address (default: 0.0.0.0:50052)
#   RESTART_ONLY    — set to 1 to skip file installation and only restart
#   SKIP_FIREWALL   — set to 1 to skip ufw configuration

set -euo pipefail

SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
NEXUS_BIND="\${NEXUS_BIND:-0.0.0.0:50052}"
RESTART_ONLY="\${RESTART_ONLY:-0}"
SKIP_FIREWALL="\${SKIP_FIREWALL:-0}"
PUBLIC_IP="${BAKED_IP}"

log()  { printf '\033[1;34m[remote-host-prep]\033[0m %s\n' "\$*"; }
warn() { printf '\033[1;33m[remote-host-prep]\033[0m %s\n' "\$*" >&2; }
die()  { printf '\033[1;31m[remote-host-prep]\033[0m %s\n' "\$*" >&2; exit 1; }

[[ "\$(id -u)" -eq 0 ]] || die "must be run as root (use sudo)"

# ---------- verify transfer files ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    for f in nexus-server ca.crt.pem server.crt.pem server.key.pem \\
              nexus.toml capabilities.json nexus-server.service; do
        [[ -f "\${SCRIPT_DIR}/\${f}" ]] || \\
            die "missing transfer file: \${f} — re-run transfer-prep.sh on the build host"
    done
fi

# ---------- 1. system user + directories ----------
log "creating nexus user and directories"
id nexus >/dev/null 2>&1 || \\
    useradd --system --no-create-home --shell /usr/sbin/nologin nexus

mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus
chown -R nexus:nexus /var/lib/nexus /var/log/nexus
chown root:nexus /etc/nexus && chmod 750 /etc/nexus

# ---------- 2. binary ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    log "installing nexus-server binary"
    install -m 0755 -o root -g root \\
        "\${SCRIPT_DIR}/nexus-server" /usr/local/bin/nexus-server
fi

# ---------- 3. certs ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    log "installing certs"
    install -m 0644 -o root  -g nexus "\${SCRIPT_DIR}/ca.crt.pem"     /etc/nexus/ca.crt.pem
    install -m 0644 -o root  -g nexus "\${SCRIPT_DIR}/server.crt.pem" /etc/nexus/server.crt.pem
    install -m 0600 -o nexus -g nexus "\${SCRIPT_DIR}/server.key.pem" /etc/nexus/server.key.pem
fi

# ---------- 4. config ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    log "installing nexus.toml"
    sed "s|bind = \"0.0.0.0:50052\".*|bind = \"\${NEXUS_BIND}\"|g" \\
        "\${SCRIPT_DIR}/nexus.toml" > /etc/nexus/nexus.toml
    chown root:nexus /etc/nexus/nexus.toml
    chmod 640 /etc/nexus/nexus.toml

    log "installing capabilities.json"
    install -m 0644 -o root -g nexus \\
        "\${SCRIPT_DIR}/capabilities.json" /etc/nexus/capabilities.json
fi

# ---------- 5. server env file ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    log "writing /etc/nexus/server.env"
    cat > /etc/nexus/server.env <<EOF
NEXUS_CA_CERT=/etc/nexus/ca.crt.pem
NEXUS_SERVER_CERT=/etc/nexus/server.crt.pem
NEXUS_SERVER_KEY=/etc/nexus/server.key.pem
RUST_LOG=info
EOF
    chown root:nexus /etc/nexus/server.env
    chmod 640 /etc/nexus/server.env
fi

# ---------- 6. systemd unit ----------
if [[ "\$RESTART_ONLY" -ne 1 ]]; then
    log "installing systemd unit"
    install -m 0644 -o root -g root \\
        "\${SCRIPT_DIR}/nexus-server.service" \\
        /etc/systemd/system/nexus-server.service
fi

systemctl daemon-reload
systemctl enable nexus-server

# ---------- 7. firewall (ufw, best-effort) ----------
if [[ "\$SKIP_FIREWALL" -ne 1 ]] && command -v ufw >/dev/null 2>&1; then
    log "configuring UFW firewall rules"
    ufw allow 22/tcp   comment "SSH"         || warn "ufw: failed to add SSH rule"
    ufw allow 50052/tcp comment "nexus A2A"  || warn "ufw: failed to add A2A rule"
    ufw --force enable                        || warn "ufw: enable failed"
    log "UFW status:"
    ufw status numbered | sed 's/^/  /'
else
    if [[ "\$SKIP_FIREWALL" -eq 1 ]]; then
        log "firewall configuration skipped (SKIP_FIREWALL=1)"
    else
        warn "ufw not found — configure your firewall manually:"
        warn "  allow TCP 22    (SSH)"
        warn "  allow TCP 50052 (nexus A2A)"
    fi
fi

# ---------- 8. NodeIdentity (first-run only) ----------
IDENTITY_PATH="/var/lib/nexus/server-identity.bin"
if [[ ! -f "\$IDENTITY_PATH" ]]; then
    log "initialising server NodeIdentity (first run)"
    sudo -u nexus /usr/local/bin/nexus-server \\
        --init-identity "\$IDENTITY_PATH" 2>&1 | \\
        sed 's/^/[nexus-server] /' || true
    if [[ -f "\$IDENTITY_PATH" ]]; then
        chmod 600 "\$IDENTITY_PATH"
        log "identity created: \${IDENTITY_PATH}"
    else
        warn "identity init may have failed — the server will generate one on first start"
    fi
else
    log "NodeIdentity already exists — skipping"
fi

# ---------- 9. start service ----------
log "starting nexus-server"
systemctl restart nexus-server
sleep 2

systemctl is-active nexus-server >/dev/null && \\
    log "nexus-server is active" || \\
    warn "service may not have started — check: journalctl -u nexus-server -n 50"

# ---------- 10. summary ----------
NEXUS_PORT="\${NEXUS_BIND##*:}"
cat <<EOF

╔══════════════════════════════════════════════════════════════╗
║  nexus-server setup complete                                  ║
╠══════════════════════════════════════════════════════════════╣
║  Public IP:  ${BAKED_IP}
║  A2A bind:   \${NEXUS_BIND}
║  Service:    systemctl status nexus-server                    ║
║  Logs:       journalctl -u nexus-server -f                    ║
║  Config:     /etc/nexus/nexus.toml                            ║
║  Certs:      /etc/nexus/{ca,server}.crt.pem                   ║
║  Identity:   /var/lib/nexus/server-identity.bin               ║
║  Audit log:  /var/lib/nexus/audit.log                         ║
╠══════════════════════════════════════════════════════════════╣
║  Next steps:                                                  ║
║  1. Copy peer-ids from agent journals into capabilities.json  ║
║  2. sudo systemctl restart nexus-server after editing caps    ║
║  3. Open port \${NEXUS_PORT} inbound in your firewall/security group
╚══════════════════════════════════════════════════════════════╝

EOF

log "tailing journal (ctrl-c to stop):"
journalctl -u nexus-server -f
REMOTE_EOF

chmod 755 "${TRANSFER_DIR}/remote-host-prep.sh"

# ---------- build transfer commands ----------
SSH_OPTS=""
if [[ -n "$SSH_KEY" ]]; then
    SSH_OPTS="-i ${SSH_KEY} "
fi

if [[ -n "$REMOTE_IP" ]]; then
    SCP_CMD="scp ${SSH_OPTS}-r ${TRANSFER_DIR}/ ${REMOTE_USER}@${REMOTE_IP}:~/nexus-transfer/"
    SSH_CMD="ssh ${SSH_OPTS}${REMOTE_USER}@${REMOTE_IP} 'sudo bash ~/nexus-transfer/remote-host-prep.sh'"
    ONE_LINER="${SCP_CMD} && ${SSH_CMD}"
else
    SCP_CMD="scp -r ${TRANSFER_DIR}/ <user>@<remote-ip>:~/nexus-transfer/"
    SSH_CMD="ssh <user>@<remote-ip> 'sudo bash ~/nexus-transfer/remote-host-prep.sh'"
    ONE_LINER="${SCP_CMD} && ${SSH_CMD}"
fi

# ---------- summary ----------
log "transfer directory ready: ${TRANSFER_DIR}"
echo ""
echo "  Contents:"
ls -lh "${TRANSFER_DIR}" | awk 'NR>1 {printf "    %s\n", $0}'
echo ""
if [[ -n "$REMOTE_IP" ]]; then
    echo "  ── Ready-to-run commands ──────────────────────────────────────"
    echo ""
    echo "  1. Transfer files:"
    echo "     ${SCP_CMD}"
    echo ""
    echo "  2. Run setup:"
    echo "     ${SSH_CMD}"
    echo ""
    echo "  One-liner:"
    echo "     ${ONE_LINER}"
else
    echo "  ── Next steps (re-run with --ip to get exact commands) ────────"
    echo ""
    echo "  1. Transfer:  ${SCP_CMD}"
    echo "  2. Setup:     ${SSH_CMD}"
fi
echo ""
echo "  To skip firewall config on the remote host:"
if [[ -n "$REMOTE_IP" ]]; then
    echo "     ssh ${SSH_OPTS}${REMOTE_USER}@${REMOTE_IP} 'sudo SKIP_FIREWALL=1 bash ~/nexus-transfer/remote-host-prep.sh'"
else
    echo "     ssh <user>@<remote-ip> 'sudo SKIP_FIREWALL=1 bash ~/nexus-transfer/remote-host-prep.sh'"
fi
echo ""
