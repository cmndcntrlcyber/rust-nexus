# rust-nexus Deployment Quickstart

## Certificate Architecture

Two separate TLS layers serve different purposes:

| Layer | Cert Source | Purpose | Traffic Path |
|---|---|---|---|
| **Origin TLS** | Cloudflare Origin CA | HTTPS between Cloudflare edge and your server | Browser → CF Edge → Origin |
| **Agent mTLS** | Self-signed PKI (`pki init`) | Mutual TLS between agents/operator and A2A gRPC | Agent → Server (direct) |

```
certs/
├── server.crt.pem          # Cloudflare Origin CA (*.onoiroi.us, valid to 2041)
├── server.key.pem          # Origin server private key
├── server-ca.crt           # Cloudflare Origin CA root (verifies server.crt.pem)
├── origin_ca_ecc_root.pem  # Cloudflare Origin CA ECC root
├── operator.crt.pem        # Cloudflare Client Certificate (for edge mTLS)
├── operator.key.pem        # Cloudflare Client Certificate key
├── managed-ca.pem          # Cloudflare Managed CA (for edge client cert verification)
└── pki/                    # Self-signed PKI for direct agent mTLS
    ├── ca.crt.pem           # PKI CA root — ALL parties trust this
    ├── ca.key.pem           # PKI CA private key (keep secure, offline after gen)
    ├── server.crt.pem       # Server cert (SAN: c2.onoiroi.us, 127.0.0.1)
    ├── server.key.pem       # Server private key
    ├── operator/
    │   ├── client.crt.pem   # Operator/RTPI client cert
    │   └── client.key.pem   # Operator client key
    ├── console/
    │   ├── client.crt.pem   # nexus-console client cert
    │   └── client.key.pem   # Console client key
    ├── agent-001.crt.pem    # Agent 1 client cert
    ├── agent-001.key.pem    # Agent 1 client key
    ├── agent-002.crt.pem    # Agent 2 client cert
    ├── agent-002.key.pem
    ├── agent-003.crt.pem    # Agent 3 client cert
    └── agent-003.key.pem
```

## 1. Start rust-nexus A2A Server

### Local Development (plaintext on loopback)

```bash
cd ~/code/rust-nexus
./target/release/nexus-server --config nexus.toml
```

`nexus.toml` must contain:
```toml
[a2a]
bind = "127.0.0.1:50052"
insecure_network = true
identity_path = "./server-identity.bin"
```

Server starts on `:50052` (gRPC) and `:9100` (metrics/ferry gateway).

### Production (mTLS with PKI certs)

```bash
cd ~/code/rust-nexus

# Set insecure_network = false in nexus.toml, then:
NEXUS_CA_CERT=./certs/pki/ca.crt.pem \
NEXUS_SERVER_CERT=./certs/pki/server.crt.pem \
NEXUS_SERVER_KEY=./certs/pki/server.key.pem \
./target/release/nexus-server --config nexus.toml
```

Verify:
```bash
# Metrics endpoint (always plaintext HTTP)
curl -s http://localhost:9100/metrics | head -5

# gRPC endpoint (with mTLS client cert)
curl -s --http2-prior-knowledge -X POST \
  --cert ./certs/pki/operator/client.crt.pem \
  --key ./certs/pki/operator/client.key.pem \
  --cacert ./certs/pki/ca.crt.pem \
  -H "content-type: application/grpc" \
  -H "te: trailers" \
  -o /dev/null -w "HTTP status: %{http_code}\n" \
  https://localhost:50052/a2a.v1.A2aService/GetAgentCard
```

### Production (Docker)

```bash
cd ~/code/rust-nexus
./scripts/gen-certs.sh          # OR use pki init certs
docker compose up -d
```

Docker Compose maps certs from `./certs:/etc/nexus:ro`.

## 2. Start rust-nexus Agent

### Local Development (connect to loopback server)

```bash
cd ~/code/rust-nexus
NEXUS_SERVER_ADDR=http://127.0.0.1:50052 \
NEXUS_INSECURE_NETWORK=1 \
NEXUS_OP_MODE=lab \
./target/release/nexus-agent
```

### Production (mTLS to remote server)

```bash
NEXUS_SERVER_ADDR=https://c2.onoiroi.us:50052 \
NEXUS_CA_CERT=/path/to/ca.crt.pem \
NEXUS_CLIENT_CERT=/path/to/agent-001.crt.pem \
NEXUS_CLIENT_KEY=/path/to/agent-001.key.pem \
NEXUS_OP_MODE=field \
NEXUS_AGENT_TAG=target-01 \
./target/release/nexus-agent
```

### Agent Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `NEXUS_SERVER_ADDR` | Yes | (first CLI arg) | C2 server address |
| `NEXUS_CA_CERT` | mTLS | — | CA cert to verify server |
| `NEXUS_CLIENT_CERT` | mTLS | — | Agent client cert |
| `NEXUS_CLIENT_KEY` | mTLS | — | Agent client key |
| `NEXUS_AGENT_TAG` | No | `""` | Human-readable agent identifier |
| `NEXUS_OP_MODE` | No | `lab` | `lab` (no self-destruct) or `field` (auto-destruct enabled) |
| `NEXUS_INSECURE_NETWORK` | No | unset | Set to bypass loopback gate (dev only) |
| `NEXUS_IDENTITY_PATH` | No | `/var/lib/nexus-agent/identity.bin` | Agent identity file path |
| `RUST_LOG` | No | `info` | Log level (`debug`, `info`, `warn`, `error`) |

## 3. Start RTPI

### Prerequisites

```bash
cd ~/code/rtpi
docker compose up -d            # PostgreSQL, Redis, core services
npm run db:push                  # Apply schema (if first run)
npm run db:create-admin          # Create admin user (if first run)
```

### Development

```bash
# Terminal 1: Backend
npm run dev                      # Express on :3001

# Terminal 2: Frontend
npm run dev:frontend             # Vite on :5000
```

### Key Environment Variables (in `.env`)

| Variable | Value | Description |
|---|---|---|
| `DATABASE_URL` | `postgresql://rtpi:rtpi@localhost:5434/rtpi_main` | PostgreSQL connection |
| `REDIS_URL` | `redis://localhost:6381` | Redis connection |
| `OLLAMA_HOST` | `http://192.168.1.124:11434` | Ollama LLM server |
| `NEXUS_FERRY_URL` | `http://127.0.0.1:9100` | rust-nexus REST ferry gateway (v3.10) |
| `RUST_NEXUS_PATH` | `/opt/rust-nexus` | Path to rust-nexus repo |
| `RUST_NEXUS_HMAC_KEY` | (change me) | HMAC for WebSocket controller auth |

### Verify

```bash
# Backend health
curl -s http://localhost:3001/api/v1/health | python3 -m json.tool

# rust-nexus integration
curl -s http://localhost:3001/api/v1/rust-nexus/stats | python3 -m json.tool

# Frontend
open http://localhost:5000
```

## 4. Start nexus-harness

### Prerequisites

- Ollama running at `192.168.1.124:11434` with `qwen14b-v9:latest` and `qwen3-embedding:4b`

### Development (standalone)

```bash
cd ~/code/nexus-harness
./target/release/nexus status    # Verify config and connectivity
./target/release/nexus run "echo hello"  # Test agent loop
```

### As MCP Server (for RTPI consumption)

```bash
./target/release/nexus serve     # Stdio mode (pipe-based)
```

### Docker (Kali container with all offense tools)

```bash
cd ~/code/nexus-harness/docker
docker compose up -d             # Builds nexus-offense:latest + nexus-kali
```

### Key Config (`.nexus/config.toml`)

```toml
[ollama]
base_url = "http://192.168.1.124:11434"

[models]
reasoning = "qwen14b-v9:latest"
execution = "qwen14b-v9:latest"
embedding = "qwen3-embedding:4b"
```

### Optional: pgvector Intelligence Store

```bash
export NEXUS_DATABASE_URL="postgresql://rtpi:rtpi@localhost:5434/nexus_intel"
./target/release/nexus migrate-to-pg   # Migrate from SQLite
```

## 5. Mint Additional Agent Certs

```bash
cd ~/code/rust-nexus
./target/release/nexus-server pki agent \
  --certs-dir ./certs/pki \
  --name agent-target-dc01
```

## 6. Full Stack Verification

```bash
# 1. Server running?
curl -s http://localhost:9100/metrics | head -1

# 2. gRPC responding?
curl -s --http2-prior-knowledge -X POST \
  -H "content-type: application/grpc" -H "te: trailers" \
  -o /dev/null -w "%{http_code}" http://localhost:50052/a2a.v1.A2aService/GetAgentCard

# 3. RTPI healthy?
curl -s http://localhost:3001/api/v1/health | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'DB: {d[\"checks\"][\"database\"][\"ok\"]}, Redis: {d[\"checks\"][\"redis\"][\"ok\"]}')"

# 4. Ollama reachable?
curl -s http://192.168.1.124:11434/api/tags | python3 -c "import sys,json; [print(m['name']) for m in json.load(sys.stdin)['models'][:3]]"

# 5. nexus-harness built?
~/code/nexus-harness/target/release/nexus --version
```

## Port Summary

| Port | Service | Protocol | Binding |
|---|---|---|---|
| 3001 | RTPI Express backend | HTTP | `0.0.0.0` |
| 5000 | RTPI Vite frontend | HTTP | `0.0.0.0` |
| 5434 | PostgreSQL (RTPI) | TCP | `0.0.0.0` |
| 6381 | Redis (RTPI) | TCP | `0.0.0.0` |
| 9100 | rust-nexus metrics + ferry gateway | HTTP | `127.0.0.1` |
| 50052 | rust-nexus A2A gRPC | HTTP/2 | `127.0.0.1` |
| 11434 | Ollama (LAN) | HTTP | `192.168.1.124` |

## Domain Architecture

| Subdomain | Service | Through Cloudflare? |
|---|---|---|
| `onoiroi.us` | RTPI backend API | Yes (Origin CA) |
| `c3s.onoiroi.us` | RTPI frontend | Yes (Origin CA) |
| `c3s-workbench.onoiroi.us` | ATT&CK Workbench | Yes (Origin CA) |
| `c3s-kasm.onoiroi.us` | Kasm Workspaces | Yes (Origin CA) |
| `c2.onoiroi.us` | rust-nexus A2A gRPC | Optional (self-signed PKI for direct) |
| `metrics.onoiroi.us` | Metrics/ferry gateway | Optional |
