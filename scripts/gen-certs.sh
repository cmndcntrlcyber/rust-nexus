#!/usr/bin/env bash
# scripts/gen-certs.sh — v1.2 dev/test mTLS cert generator (D-V1.2-mtls).
#
# Produces a self-signed CA + a server cert + a client cert under ./certs/.
# Production deployments should NOT use these — provision your own CA and
# set the NEXUS_*_CERT env vars to point at it.
#
# Usage:
#   ./scripts/gen-certs.sh            # writes ./certs/{ca,server,client}.{crt,key}.pem
#   ./scripts/gen-certs.sh /custom/dir
#
# Required: openssl >= 1.1.1.

set -euo pipefail

OUT_DIR="${1:-./certs}"
SUBJ_BASE="/C=US/ST=DEV/L=DEV/O=rust-nexus/OU=v1.2-dev-certs"
DAYS=365

mkdir -p "$OUT_DIR"
cd "$OUT_DIR"

echo "[gen-certs] writing into $(pwd)"

# CA
echo "[gen-certs] generating CA"
openssl genpkey -algorithm ED25519 -out ca.key.pem
openssl req -x509 -new -key ca.key.pem -days "$DAYS" -out ca.crt.pem \
    -subj "${SUBJ_BASE}/CN=rust-nexus-dev-ca"

# Server
echo "[gen-certs] generating server cert"
openssl genpkey -algorithm ED25519 -out server.key.pem
openssl req -new -key server.key.pem -out server.csr.pem \
    -subj "${SUBJ_BASE}/CN=localhost"
cat > server.ext.cnf <<'EOF'
subjectAltName = DNS:localhost,IP:127.0.0.1
extendedKeyUsage = serverAuth
EOF
openssl x509 -req -in server.csr.pem -CA ca.crt.pem -CAkey ca.key.pem \
    -CAcreateserial -days "$DAYS" -out server.crt.pem \
    -extfile server.ext.cnf
rm server.csr.pem server.ext.cnf

# Client
echo "[gen-certs] generating client cert"
openssl genpkey -algorithm ED25519 -out client.key.pem
openssl req -new -key client.key.pem -out client.csr.pem \
    -subj "${SUBJ_BASE}/CN=operator-dev"
cat > client.ext.cnf <<'EOF'
extendedKeyUsage = clientAuth
EOF
openssl x509 -req -in client.csr.pem -CA ca.crt.pem -CAkey ca.key.pem \
    -CAcreateserial -days "$DAYS" -out client.crt.pem \
    -extfile client.ext.cnf
rm -f client.csr.pem client.ext.cnf ca.srl

chmod 600 ca.key.pem server.key.pem client.key.pem

cat <<EOF

[gen-certs] done. To use, export:

  export NEXUS_CA_CERT=$(pwd)/ca.crt.pem
  export NEXUS_SERVER_CERT=$(pwd)/server.crt.pem
  export NEXUS_SERVER_KEY=$(pwd)/server.key.pem
  export NEXUS_CLIENT_CERT=$(pwd)/client.crt.pem
  export NEXUS_CLIENT_KEY=$(pwd)/client.key.pem

EOF
