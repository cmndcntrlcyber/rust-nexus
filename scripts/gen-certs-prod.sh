#!/usr/bin/env bash
# scripts/gen-certs-prod.sh — production mTLS cert generator + optional
#   Let's Encrypt ACME via automated Cloudflare DNS-01 challenge.
#
# ---- mTLS self-signed CA (default mode) ----
# Issues a dedicated CA, a server cert with proper DNS+IP SANs, and
# separate operator + agent client certs from the same CA. ED25519 keys.
#
# Unlike scripts/gen-certs.sh (which hardcodes CN=localhost for dev), this
# script takes the C2's public hostname and IP as arguments so the server
# cert's SubjectAltName actually matches what operators and agents will dial.
#
# Usage:
#   ./scripts/gen-certs-prod.sh --domain c2.example.com --ip 1.2.3.4
#   ./scripts/gen-certs-prod.sh --domain c2.example.com --ip 1.2.3.4 --out ./certs/prod
#   ./scripts/gen-certs-prod.sh --domain c2.example.com --ip 1.2.3.4 --org "YourCo" --days 365
#
# Produces (under $OUT_DIR):
#   ca.crt.pem        ca.key.pem
#   server.crt.pem    server.key.pem      (SAN: DNS:<domain>, IP:<ip>)
#   operator.crt.pem  operator.key.pem    (clientAuth, CN=operator-001)
#   agent.crt.pem     agent.key.pem       (clientAuth, CN=agent-001)
#
# ---- Let's Encrypt ACME + Cloudflare DNS-01 (--acme / --acme-only) ----
# Automates the certbot DNS-01 challenge by posting/deleting Cloudflare
# TXT records via the Cloudflare API. Runs certbot as a sub-process with
# --manual-auth-hook and --manual-cleanup-hook; no manual intervention needed.
#
# All domains must be in the same Cloudflare zone. Certbot requires root.
# Run with sudo when using --acme or --acme-only.
#
# Usage (ACME alongside mTLS certs):
#   sudo ./scripts/gen-certs-prod.sh \
#     --domain hr.attck-deploy.net --ip 1.2.3.4 \
#     --acme --email admin@example.com \
#     --cf-api-token <TOKEN> --cf-zone-id <ZONE_ID> \
#     --le-domains "hr.attck-deploy.net mail.attck-deploy.net"
#
# Usage (ACME only, skip mTLS CA generation):
#   sudo ./scripts/gen-certs-prod.sh \
#     --acme-only --email admin@example.com \
#     --cf-api-token <TOKEN> --cf-zone-id <ZONE_ID> \
#     --le-domains "hr.attck-deploy.net mail.attck-deploy.net" \
#     --out ./certs/prod
#
# Let's Encrypt certs are placed in $OUT_DIR alongside mTLS certs:
#   le-cert.pem     (fullchain — use for public HTTPS/TLS listeners)
#   le-key.pem      (privkey   — mode 600)
#
# CF_API_TOKEN must have Zone:DNS:Edit permission for the target zone.
# To look up ZONE_ID: curl -sH "Authorization: Bearer $TOKEN" \
#   "https://api.cloudflare.com/client/v4/zones?name=attck-deploy.net" | jq -r '.result[0].id'
#
# Optional flags:
#   --dns-wait <seconds>   Seconds to wait after adding TXT for propagation (default: 30)
#
# Requires: openssl >= 1.1.1, certbot, curl, jq (for ACME mode).

set -euo pipefail

# ---------- defaults ----------
DOMAIN=""
IP=""
OUT_DIR="./certs/prod"
ORG="rust-nexus"
DAYS_CA=1825        # 5 years for the CA
DAYS_LEAF=365       # 1 year for leaf certs
OPERATOR_CN="operator-001"
AGENT_CN="agent-001"

# ACME / Cloudflare options
DO_ACME=false
ACME_ONLY=false
ACME_EMAIL=""
CF_API_TOKEN=""
CF_ZONE_ID=""
LE_DOMAINS=()
DNS_WAIT=30         # seconds to wait after TXT record creation

usage() {
    grep '^#' "$0" | sed 's/^# \{0,1\}//'
    exit 0
}

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --domain)         DOMAIN="$2";          shift 2 ;;
        --ip)             IP="$2";              shift 2 ;;
        --out)            OUT_DIR="$2";         shift 2 ;;
        --org)            ORG="$2";             shift 2 ;;
        --days)           DAYS_LEAF="$2";       shift 2 ;;
        --ca-days)        DAYS_CA="$2";         shift 2 ;;
        --operator-cn)    OPERATOR_CN="$2";     shift 2 ;;
        --agent-cn)       AGENT_CN="$2";        shift 2 ;;
        --acme)           DO_ACME=true;         shift   ;;
        --acme-only)      DO_ACME=true; ACME_ONLY=true; shift ;;
        --email)          ACME_EMAIL="$2";      shift 2 ;;
        --cf-api-token)   CF_API_TOKEN="$2";    shift 2 ;;
        --cf-zone-id)     CF_ZONE_ID="$2";      shift 2 ;;
        --le-domains)
            IFS=' ' read -r -a LE_DOMAINS <<< "$2"; shift 2 ;;
        --le-domain)
            LE_DOMAINS+=("$2"); shift 2 ;;
        --dns-wait)       DNS_WAIT="$2";        shift 2 ;;
        -h|--help)        usage ;;
        *) echo "unknown arg: $1" >&2; usage ;;
    esac
done

# ---------- validation ----------
if [[ "$ACME_ONLY" == false ]]; then
    [[ -n "$DOMAIN" ]] || { echo "--domain is required (e.g. c2.example.com)" >&2; exit 2; }
    [[ -n "$IP" ]]     || { echo "--ip is required (e.g. 34.228.6.154)" >&2; exit 2; }
    if ! [[ "$IP" =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
        echo "--ip looks malformed: $IP" >&2; exit 2
    fi
    command -v openssl >/dev/null || { echo "openssl not on PATH" >&2; exit 2; }
fi

if [[ "$DO_ACME" == true ]]; then
    [[ -n "$ACME_EMAIL" ]]        || { echo "--email is required for ACME mode" >&2; exit 2; }
    [[ -n "$CF_API_TOKEN" ]]      || { echo "--cf-api-token is required for ACME mode" >&2; exit 2; }
    [[ -n "$CF_ZONE_ID" ]]        || { echo "--cf-zone-id is required for ACME mode" >&2; exit 2; }
    [[ ${#LE_DOMAINS[@]} -gt 0 ]] || { echo "--le-domains (or --le-domain) required for ACME mode" >&2; exit 2; }
    command -v certbot >/dev/null || { echo "certbot not on PATH — install with: apt install certbot" >&2; exit 2; }
    command -v curl    >/dev/null || { echo "curl not on PATH" >&2; exit 2; }
    command -v jq      >/dev/null || { echo "jq not on PATH — install with: apt install jq" >&2; exit 2; }
fi

mkdir -p "$OUT_DIR"
ABSOLUTE_OUT="$(cd "$OUT_DIR" && pwd)"
echo "[gen-certs-prod] output dir: $ABSOLUTE_OUT"

# ==========================================================================
#  mTLS self-signed CA + leaf certs
# ==========================================================================
if [[ "$ACME_ONLY" == false ]]; then
    cd "$ABSOLUTE_OUT"
    echo "[gen-certs-prod] domain=$DOMAIN ip=$IP org=$ORG"

    SUBJ_BASE="/C=US/O=${ORG}/OU=v1.2-prod-certs"

    # --- CA ---
    echo "[gen-certs-prod] generating CA (Ed25519, ${DAYS_CA} days)"
    openssl genpkey -algorithm ED25519 -out ca.key.pem
    openssl req -x509 -new -key ca.key.pem -days "$DAYS_CA" -out ca.crt.pem \
        -subj "${SUBJ_BASE}/CN=${ORG}-issuing-ca"

    # --- server ---
    echo "[gen-certs-prod] generating server cert for ${DOMAIN}, ${IP}"
    openssl genpkey -algorithm ED25519 -out server.key.pem
    openssl req -new -key server.key.pem -out server.csr.pem \
        -subj "${SUBJ_BASE}/CN=${DOMAIN}"
    cat > server.ext.cnf <<EOF
subjectAltName = DNS:${DOMAIN}, IP:${IP}
extendedKeyUsage = serverAuth
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature, keyEncipherment
EOF
    openssl x509 -req -in server.csr.pem -CA ca.crt.pem -CAkey ca.key.pem \
        -CAcreateserial -days "$DAYS_LEAF" -out server.crt.pem \
        -extfile server.ext.cnf
    rm -f server.csr.pem server.ext.cnf

    # --- operator client ---
    echo "[gen-certs-prod] generating operator client cert (CN=${OPERATOR_CN})"
    openssl genpkey -algorithm ED25519 -out operator.key.pem
    openssl req -new -key operator.key.pem -out operator.csr.pem \
        -subj "${SUBJ_BASE}/CN=${OPERATOR_CN}"
    cat > client.ext.cnf <<EOF
extendedKeyUsage = clientAuth
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
EOF
    openssl x509 -req -in operator.csr.pem -CA ca.crt.pem -CAkey ca.key.pem \
        -CAcreateserial -days "$DAYS_LEAF" -out operator.crt.pem \
        -extfile client.ext.cnf
    rm -f operator.csr.pem

    # --- agent client ---
    echo "[gen-certs-prod] generating agent client cert (CN=${AGENT_CN})"
    openssl genpkey -algorithm ED25519 -out agent.key.pem
    openssl req -new -key agent.key.pem -out agent.csr.pem \
        -subj "${SUBJ_BASE}/CN=${AGENT_CN}"
    openssl x509 -req -in agent.csr.pem -CA ca.crt.pem -CAkey ca.key.pem \
        -CAcreateserial -days "$DAYS_LEAF" -out agent.crt.pem \
        -extfile client.ext.cnf
    rm -f agent.csr.pem client.ext.cnf ca.srl

    chmod 600 ca.key.pem server.key.pem operator.key.pem agent.key.pem
    chmod 644 ca.crt.pem server.crt.pem operator.crt.pem agent.crt.pem
fi

# ==========================================================================
#  Let's Encrypt ACME via Cloudflare DNS-01
# ==========================================================================
if [[ "$DO_ACME" == true ]]; then
    PRIMARY_DOMAIN="${LE_DOMAINS[0]}"
    echo ""
    echo "[gen-certs-prod] === Let's Encrypt ACME (Cloudflare DNS-01) ==="
    echo "[gen-certs-prod] domains:   ${LE_DOMAINS[*]}"
    echo "[gen-certs-prod] email:     $ACME_EMAIL"
    echo "[gen-certs-prod] zone_id:   $CF_ZONE_ID"
    echo "[gen-certs-prod] dns-wait:  ${DNS_WAIT}s"

    # --- write hook scripts into a temp dir (cleaned up on exit) ---
    HOOK_DIR="$(mktemp -d)"
    trap 'rm -rf "$HOOK_DIR"' EXIT

    # auth hook: POST a _acme-challenge TXT record to Cloudflare
    # Variables expanded at runtime via certbot env + exported CF_* vars
    cat > "${HOOK_DIR}/cf-auth-hook.sh" <<'HOOK_EOF'
#!/usr/bin/env bash
# Certbot manual-auth-hook: adds Cloudflare DNS TXT record for DNS-01 challenge.
# Runtime env (set by certbot):   CERTBOT_DOMAIN, CERTBOT_VALIDATION
# Runtime env (exported by caller): CF_API_TOKEN, CF_ZONE_ID, DNS_WAIT
set -euo pipefail

RECORD_NAME="_acme-challenge.${CERTBOT_DOMAIN}"
echo "[cf-auth-hook] adding TXT ${RECORD_NAME} = ${CERTBOT_VALIDATION}"

RESPONSE=$(curl -s -X POST \
    "https://api.cloudflare.com/client/v4/zones/${CF_ZONE_ID}/dns_records" \
    -H "Authorization: Bearer ${CF_API_TOKEN}" \
    -H "Content-Type: application/json" \
    --data "{\"type\":\"TXT\",\"name\":\"${RECORD_NAME}\",\"content\":\"${CERTBOT_VALIDATION}\",\"ttl\":120}")

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
if [[ "$SUCCESS" != "true" ]]; then
    echo "[cf-auth-hook] ERROR: Cloudflare API call failed:" >&2
    echo "$RESPONSE" | jq . >&2
    exit 1
fi

RECORD_ID=$(echo "$RESPONSE" | jq -r '.result.id')
echo "[cf-auth-hook] created record id=$RECORD_ID"
# Emit record ID so the cleanup hook receives it via CERTBOT_AUTH_OUTPUT
echo "$RECORD_ID"

echo "[cf-auth-hook] waiting ${DNS_WAIT}s for DNS propagation..."
sleep "${DNS_WAIT}"
HOOK_EOF
    chmod +x "${HOOK_DIR}/cf-auth-hook.sh"

    # cleanup hook: DELETE the TXT record by ID (from auth-hook stdout) or by lookup
    cat > "${HOOK_DIR}/cf-cleanup-hook.sh" <<'HOOK_EOF'
#!/usr/bin/env bash
# Certbot manual-cleanup-hook: removes Cloudflare DNS TXT record after challenge.
# Runtime env (set by certbot):   CERTBOT_DOMAIN, CERTBOT_VALIDATION, CERTBOT_AUTH_OUTPUT
# Runtime env (exported by caller): CF_API_TOKEN, CF_ZONE_ID
set -euo pipefail

RECORD_NAME="_acme-challenge.${CERTBOT_DOMAIN}"

if [[ -n "${CERTBOT_AUTH_OUTPUT:-}" ]]; then
    # Prefer the record ID echoed by the auth hook
    RECORD_ID="${CERTBOT_AUTH_OUTPUT}"
    echo "[cf-cleanup-hook] deleting TXT ${RECORD_NAME} id=${RECORD_ID}"
else
    # Fall back: look up by name + content
    echo "[cf-cleanup-hook] looking up TXT ${RECORD_NAME} = ${CERTBOT_VALIDATION}"
    RECORD_ID=$(curl -s -X GET \
        "https://api.cloudflare.com/client/v4/zones/${CF_ZONE_ID}/dns_records?type=TXT&name=${RECORD_NAME}&content=${CERTBOT_VALIDATION}" \
        -H "Authorization: Bearer ${CF_API_TOKEN}" \
        -H "Content-Type: application/json" \
        | jq -r '.result[0].id // empty')
    if [[ -z "$RECORD_ID" ]]; then
        echo "[cf-cleanup-hook] TXT record not found, skipping delete"
        exit 0
    fi
fi

RESPONSE=$(curl -s -X DELETE \
    "https://api.cloudflare.com/client/v4/zones/${CF_ZONE_ID}/dns_records/${RECORD_ID}" \
    -H "Authorization: Bearer ${CF_API_TOKEN}" \
    -H "Content-Type: application/json")

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
if [[ "$SUCCESS" != "true" ]]; then
    echo "[cf-cleanup-hook] WARNING: delete may have failed:" >&2
    echo "$RESPONSE" | jq . >&2
fi
echo "[cf-cleanup-hook] removed TXT ${RECORD_NAME}"
HOOK_EOF
    chmod +x "${HOOK_DIR}/cf-cleanup-hook.sh"

    # --- build certbot -d args from LE_DOMAINS array ---
    CERTBOT_D_ARGS=()
    for d in "${LE_DOMAINS[@]}"; do
        CERTBOT_D_ARGS+=("-d" "$d")
    done

    # --- run certbot; export CF_* and DNS_WAIT so hooks inherit them ---
    echo "[gen-certs-prod] running certbot (non-interactive)..."
    export CF_API_TOKEN CF_ZONE_ID DNS_WAIT
    certbot certonly \
        --manual \
        --preferred-challenges dns \
        --manual-auth-hook    "${HOOK_DIR}/cf-auth-hook.sh" \
        --manual-cleanup-hook "${HOOK_DIR}/cf-cleanup-hook.sh" \
        --email "$ACME_EMAIL" \
        --server https://acme-v02.api.letsencrypt.org/directory \
        --agree-tos \
        --non-interactive \
        "${CERTBOT_D_ARGS[@]}"

    # --- copy LE certs into OUT_DIR ---
    LE_LIVE="/etc/letsencrypt/live/${PRIMARY_DOMAIN}"
    if [[ -d "$LE_LIVE" ]]; then
        cp "${LE_LIVE}/fullchain.pem" "${ABSOLUTE_OUT}/le-cert.pem"
        cp "${LE_LIVE}/privkey.pem"   "${ABSOLUTE_OUT}/le-key.pem"
        chmod 600 "${ABSOLUTE_OUT}/le-key.pem"
        chmod 644 "${ABSOLUTE_OUT}/le-cert.pem"
        echo "[gen-certs-prod] LE certs written to $ABSOLUTE_OUT"
    else
        echo "[gen-certs-prod] WARNING: expected $LE_LIVE not found — check /etc/letsencrypt/live/ manually" >&2
    fi
fi
# HOOK_DIR cleaned by trap EXIT

# ==========================================================================
#  Summary
# ==========================================================================

if [[ "$ACME_ONLY" == false ]]; then
    cat <<EOF

[gen-certs-prod] mTLS certs done. Files in: ${ABSOLUTE_OUT}

Server cert SANs:
$(openssl x509 -in "${ABSOLUTE_OUT}/server.crt.pem" -noout -ext subjectAltName | tail -n +2)

# ---- Deploy to the Remote Server (C2 server) ----
scp ${ABSOLUTE_OUT}/ca.crt.pem ${ABSOLUTE_OUT}/server.crt.pem ${ABSOLUTE_OUT}/server.key.pem ubuntu@${IP}:/tmp/

# Then on the Remote Server:
sudo systemctl stop nexus-server
sudo install -m 0644 -o root  -g nexus  /tmp/ca.crt.pem      /etc/nexus/ca.crt.pem
sudo install -m 0644 -o root  -g nexus  /tmp/server.crt.pem  /etc/nexus/server.crt.pem
sudo install -m 0600 -o nexus -g nexus  /tmp/server.key.pem  /etc/nexus/server.key.pem
shred -u /tmp/server.key.pem
sudo systemctl start nexus-server

# ---- Operator workstation ----
export NEXUS_CA_CERT=${ABSOLUTE_OUT}/ca.crt.pem
export NEXUS_CLIENT_CERT=${ABSOLUTE_OUT}/operator.crt.pem
export NEXUS_CLIENT_KEY=${ABSOLUTE_OUT}/operator.key.pem
export NEXUS_SERVER_ADDR=https://${DOMAIN}:50052

# ---- Agent host ----
# Copy ca.crt.pem + agent.crt.pem + agent.key.pem (mode 600 for the key)
# to /etc/nexus-agent/ on the agent host.

# ---- Verify end-to-end (from any client box with these certs) ----
openssl s_client -4 -connect ${DOMAIN}:50052 -servername ${DOMAIN} \\
    -CAfile ${ABSOLUTE_OUT}/ca.crt.pem \\
    -cert   ${ABSOLUTE_OUT}/operator.crt.pem \\
    -key    ${ABSOLUTE_OUT}/operator.key.pem </dev/null 2>&1 | tail -15
# Expect: "Verify return code: 0 (ok)" and NO "certificate required" alert.

EOF
fi

if [[ "$DO_ACME" == true ]]; then
    cat <<EOF

[gen-certs-prod] Let's Encrypt certs done.
  ${ABSOLUTE_OUT}/le-cert.pem  (fullchain — deploy to public HTTPS/TLS listeners)
  ${ABSOLUTE_OUT}/le-key.pem   (privkey   — mode 600)

# ---- Renewal ----
# certbot auto-renews via /etc/cron.d/certbot or systemd timer 'certbot.timer'.
# After each renewal, re-copy certs or configure a deploy-hook:
#
#   Add to /etc/letsencrypt/renewal/${PRIMARY_DOMAIN}.conf:
#     post_hook = cp /etc/letsencrypt/live/${PRIMARY_DOMAIN}/fullchain.pem ${ABSOLUTE_OUT}/le-cert.pem && \\
#                 cp /etc/letsencrypt/live/${PRIMARY_DOMAIN}/privkey.pem ${ABSOLUTE_OUT}/le-key.pem && \\
#                 chmod 600 ${ABSOLUTE_OUT}/le-key.pem

EOF
fi
