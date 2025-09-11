#!/bin/bash
# Generate development certificates for local gRPC server

set -e

CERT_DIR="$(dirname "$0")"
cd "$CERT_DIR"

echo "Generating development certificates for local gRPC server..."

# Generate private key
openssl genrsa -out server.key 2048

# Generate certificate signing request
cat > server.conf <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = California
L = San Francisco
O = Nexus Development
CN = localhost

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.attck-deploy.net
DNS.3 = c2.attck-deploy.net
DNS.4 = api.attck-deploy.net
IP.1 = 127.0.0.1
IP.2 = 0.0.0.0
EOF

# Generate self-signed certificate
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server.conf -extensions v3_req

# Create CA certificate (same as server cert for self-signed)
cp server.crt server-ca.crt

echo "Generated certificates:"
echo "  server.crt - Server certificate"
echo "  server.key - Server private key"
echo "  server-ca.crt - CA certificate (same as server for self-signed)"

# Clean up
rm server.conf

echo "Development certificates generated successfully!"
