#!/usr/bin/env bash
# scripts/gen-certs-prod.sh — production mTLS cert generator.
#
# Issues a dedicated CA, a server cert with proper DNS+IP SANs, and
# separate operator + agent client certs from the same CA. ED25519 keys.
#
# Unlike scripts/gen-certs.sh (which hardcodes CN=localhost for dev), this
# script takes the C2's public hostname and IP as arguments so the server
# cert's SubjectAltName actually matches what operators and agents will
# dial.
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
# Requires: openssl >= 1.1.1.

set -euo pipefail

# ---------- defaults ----------
DOMAIN=""
IP=""
OUT_DIR="./certs/nexus-agent"  # default output dir (matches where deploy-operator-console.sh looks for certs)
ORG="rust-nexus"
DAYS_CA=1825        # 5 years for the CA
DAYS_LEAF=365       # 1 year for leaf certs
OPERATOR_CN="operator-001"
AGENT_CN="agent-001"

usage() {
    grep '^#' "$0" | sed 's/^# \{0,1\}//'
    exit 0
}

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --domain)        DOMAIN="$2"; shift 2 ;;
        --ip)            IP="$2"; shift 2 ;;
        --out)           OUT_DIR="$2"; shift 2 ;;
        --org)           ORG="$2"; shift 2 ;;
        --days)          DAYS_LEAF="$2"; shift 2 ;;
        --ca-days)       DAYS_CA="$2"; shift 2 ;;
        --operator-cn)   OPERATOR_CN="$2"; shift 2 ;;
        --agent-cn)      AGENT_CN="$2"; shift 2 ;;
        -h|--help)       usage ;;
        *) echo "unknown arg: $1" >&2; usage ;;
    esac
done

# ---------- validation ----------
[[ -n "$DOMAIN" ]] || { echo "--domain is required (e.g. c2.example.com)" >&2; exit 2; }
[[ -n "$IP" ]]     || { echo "--ip is required (e.g. 34.228.6.154)" >&2; exit 2; }

if ! [[ "$IP" =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
    echo "--ip looks malformed: $IP" >&2; exit 2
fi

command -v openssl >/dev/null || { echo "openssl not on PATH" >&2; exit 2; }

mkdir -p "$OUT_DIR"
umask 077
cd "$OUT_DIR"
echo "[gen-certs-prod] writing into $(pwd)"
echo "[gen-certs-prod] domain=$DOMAIN ip=$IP org=$ORG"

SUBJ_BASE="/C=US/O=${ORG}/OU=v1.2-prod-certs"

# ---------- CA ----------
echo "[gen-certs-prod] generating CA (Ed25519, ${DAYS_CA} days)"
openssl genpkey -algorithm ED25519 -out ca.key.pem
openssl req -x509 -new -key ca.key.pem -days "$DAYS_CA" -out ca.crt.pem \
    -subj "${SUBJ_BASE}/CN=${ORG}-issuing-ca"

# ---------- server ----------
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

# ---------- operator client ----------
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
# Alias: deploy-operator-console.sh expects client.{crt,key}.pem
ln -sf operator.crt.pem client.crt.pem
ln -sf operator.key.pem client.key.pem

# ---------- agent client ----------
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

# ---------- summary ----------
cat <<EOF

[gen-certs-prod] done. Files in: $(pwd)

Server cert SANs:
$(openssl x509 -in server.crt.pem -noout -ext subjectAltName | tail -n +2)

# ---- Deploy to the Remote Server (C2 server) ----
scp ca.crt.pem server.crt.pem server.key.pem ubuntu@${IP}:/tmp/

# Then on the Remote Server:
sudo systemctl stop nexus-server
sudo install -m 0644 -o root  -g nexus  /tmp/ca.crt.pem      /etc/nexus/ca.crt.pem
sudo install -m 0644 -o root  -g nexus  /tmp/server.crt.pem  /etc/nexus/server.crt.pem
sudo install -m 0600 -o nexus -g nexus  /tmp/server.key.pem  /etc/nexus/server.key.pem
shred -u /tmp/server.key.pem
sudo systemctl start nexus-server

# ---- Operator workstation ----
export NEXUS_CA_CERT=$(pwd)/ca.crt.pem
export NEXUS_CLIENT_CERT=$(pwd)/operator.crt.pem
export NEXUS_CLIENT_KEY=$(pwd)/operator.key.pem
export NEXUS_SERVER_ADDR=https://${DOMAIN}:50052

# ---- Agent host ----
# Copy ca.crt.pem + agent.crt.pem + agent.key.pem (mode 600 for the key)
# to /etc/nexus-agent/ on the agent host.

# ---- Verify end-to-end (from any client box with these certs) ----
openssl s_client -4 -connect ${DOMAIN}:50052 -servername ${DOMAIN} \\
    -CAfile $(pwd)/ca.crt.pem \\
    -cert   $(pwd)/operator.crt.pem \\
    -key    $(pwd)/operator.key.pem </dev/null 2>&1 | tail -15
# Expect: "Verify return code: 0 (ok)" and NO "certificate required" alert.

EOF
