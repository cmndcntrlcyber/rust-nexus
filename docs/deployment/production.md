# Production deployment

This guide covers a production rollout of rust-nexus: a C2 server with
full mTLS, agents distributed to endpoint hosts, and an operator console
on operator workstations.

---

## Architecture

```
+--------------------+        mTLS :50052          +--------------------+
|  nexus-console     | ──── signed card ─────────► |  nexus-infra       |
|  Tauri + Leptos    |   (custom CA, client cert)   |  C2 server         |
+--------------------+                             |  (NodeIdentity,    |
                                                   |   AgentChannels,   |
+--------------------+    v1.2 agent-mode           |   capability       |
|  nexus-agent       | ── AgentRegister ──────────► |   matrix, audit    |
|  (PTY shell,       |   mTLS :50052               |   sink, rate       |
|   OS-aware shell)  |   (NEXUS_CA_CERT +           |   limiter)         |
+--------------------+    NEXUS_CLIENT_CERT/KEY)    +────────────────────+
```

All clients connect **directly** to the C2 server on port **50052** over
mTLS using a custom CA. The server verifies each client's certificate;
agents and operators each have a unique cert signed by the same CA.

---

## Just the Commands

Minimum copy-paste path. All steps run from the repo root on the build
machine. For the **why**, see the detailed sections below.

### Prerequisites

- C2 server host with a public IP. `c2.example.com` in DNS pointing at it.
- Port **50052** open inbound (agents + operators). Port 22 for SSH.
- SSH access to the C2 server host.

### 1. Generate certs

```bash
./scripts/gen-certs-prod.sh \
  --domain c2.example.com \
  --ip <server-public-ip> \
  --out ./certs/prod
```

Produces `ca`, `server`, `operator`, and `agent` certs in `./certs/prod/`.
Store `ca.key.pem` offline after this step — it is only needed to mint
new agent certs.

### 2. Stage and deploy the server

```bash
./scripts/transfer-prep.sh --ip <server-public-ip> --user ubuntu
```

The script builds the binary (if needed), packages everything into
`scripts/transfer/`, and prints the exact commands to run. Copy-paste
the **one-liner** from its output:

```bash
# Example output — copy the line printed by transfer-prep.sh:
scp -r scripts/transfer/ ubuntu@<server-public-ip>:~/nexus-transfer/ && \
  ssh ubuntu@<server-public-ip> 'sudo bash ~/nexus-transfer/remote-host-prep.sh'
```

`remote-host-prep.sh` creates the `nexus` user, installs certs and
config, enables the systemd unit, and starts the service.

### 3. Build and package agent deliverables

```bash
./scripts/deliverables-prep.sh \
  --lin-count 3 \
  --win-count 3 \
  --ip c2.example.com
```

Builds Linux and Windows bundles and places the zips in `dist/agents/`.
Each zip is self-contained: binary, `agent.env` (with C2 URL pre-set),
and an install script.

### 4. Deploy agents

**Linux** (on each target host):
```bash
unzip agent-lin01.zip && cd agent-lin01
sudo bash install-linux.sh
```

**Windows** (elevated `cmd.exe`):
```bat
agent-win01\install.bat
```

### 6. Package and launch the operator console

```bash
# Build + launch on the operator workstation (auto-detects certs/prod/)
NEXUS_SERVER_ADDR=https://c2.example.com:50052 \
  ./scripts/deploy-operator-console.sh

# Or package it for distribution to another operator
./scripts/operator-package.sh --ip c2.example.com --zip
# → scripts/operator-package.zip  (send to operator; they run launch.sh)
```

### 7. Wire agent peer-ids into the capability matrix

After the agent starts, its peer-id appears in the server journal:

```bash
sudo journalctl -u nexus-server | grep "agent registered"
```

Add it to `/etc/nexus/capabilities.json` on the server:

```json
{
  "agents": {
    "<64-hex-peer-id>": { "skills": ["shell-session"], "label": "agent-lin01" }
  }
}
```

```bash
sudo systemctl restart nexus-server
```

---

## Server provisioning

### Filesystem layout

```
/etc/nexus/
  nexus.toml              # Server config
  capabilities.json       # Capability matrix (mode 0644)
  server.crt.pem          # Server cert (mode 0644, custom CA-signed)
  server.key.pem          # Server key (mode 0600, owned by nexus user)
  ca.crt.pem              # Custom CA cert (mode 0644)
  server.env              # Env var file for systemd

/var/lib/nexus/
  server-identity.bin     # NodeIdentity blob (mode 0600)
  audit.log               # Hash-chained audit log

/var/log/nexus/           # Optional; journald is the primary sink
```

### Create the server user

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin nexus
sudo mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus
sudo chown -R nexus:nexus /var/lib/nexus /var/log/nexus
sudo chown root:nexus /etc/nexus && sudo chmod 750 /etc/nexus
```

### NodeIdentity

Generated automatically on first server start from
`nexus.toml`'s `identity_path`. Back it up — if lost, all signed
AgentCards must be re-issued.

### Server config (`nexus.toml`)

Key sections:

```toml
[a2a]
bind = "0.0.0.0:50052"
insecure_network = false
identity_path = "/var/lib/nexus/server-identity.bin"

[capabilities]
file = "/etc/nexus/capabilities.json"

[audit]
sink = "file"
path = "/var/lib/nexus/audit.log"
```

### Server env file

```ini
# /etc/nexus/server.env
NEXUS_CA_CERT=/etc/nexus/ca.crt.pem
NEXUS_SERVER_CERT=/etc/nexus/server.crt.pem
NEXUS_SERVER_KEY=/etc/nexus/server.key.pem
RUST_LOG=info
```

```bash
sudo chown root:nexus /etc/nexus/server.env
sudo chmod 640 /etc/nexus/server.env
```

### Start the server

```bash
sudo cp docs/deployment/examples/nexus-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nexus-server
sudo systemctl status nexus-server
```

Expected journal: `A2A server starting addr=0.0.0.0:50052`.

---

## Agent provisioning

### How agents connect

Agents connect **directly** to the C2 server on port **50052** using mTLS.
The `agent.env` installed by each bundle includes:

```ini
# /etc/nexus-agent/agent.env
NEXUS_CA_CERT=/etc/nexus-agent/ca.crt.pem
NEXUS_CLIENT_CERT=/etc/nexus-agent/client.crt.pem
NEXUS_CLIENT_KEY=/etc/nexus-agent/client.key.pem
NEXUS_SERVER_ADDR=https://c2.example.com:50052
RUST_LOG=info
```

The agent loads `NEXUS_CA_CERT` to verify the server cert, and presents
`NEXUS_CLIENT_CERT` + `NEXUS_CLIENT_KEY` as its mTLS identity.

### Agent identity

Each agent generates a `NodeIdentity` on first run at:
- Linux: `/var/lib/nexus-agent/identity.bin` (mode 0600)
- Windows: `%PROGRAMDATA%\nexus-agent\identity.bin` (ACL-restricted)

The 64-hex peer-id is what you register in `capabilities.json`. Back it
up if you want the peer-id to survive a reinstall.

### Building agent bundles

```bash
# Linux bundles
./scripts/build-agent-bundles.sh \
  --os lin \
  --count 3 \
  --certs-dir ./certs/nexus-agent \
  --server-addr https://c2.example.com:50052

# Windows bundles
./scripts/build-agent-bundles.sh \
  --os win \
  --count 3 \
  --certs-dir ./certs/nexus-agent \
  --server-addr https://c2.example.com:50052

# Or use deliverables-prep.sh for both in one command:
./scripts/deliverables-prep.sh \
  --lin-count 3 --win-count 3 --ip c2.example.com --port 50052
```

Each zip contains: binary, `agent.env` (with cert paths + server addr),
`ca.crt.pem`, `client.crt.pem`, `client.key.pem`, and install script.

### Deploy a Linux agent bundle

```bash
unzip dist/agents/agent-lin01.zip
cd agent-lin01
sudo bash install-linux.sh
sudo journalctl -u nexus-agent -f
```

Expected journal:
```
INFO identity loaded peer_id=<64-hex>
INFO connecting to C2 addr=https://c2.example.com:50052
INFO A2A agent-mode stream registered
```

### Deploy a Windows agent bundle

Copy the extracted folder to the target host, then from an elevated `cmd.exe`:
```bat
agent-win01\install.bat
```
Logs: `C:\ProgramData\nexus-agent\agent.log`

---

## Operator console

### Launch via deploy script

```bash
NEXUS_SERVER_ADDR=https://c2.example.com:50052 \
  ./scripts/deploy-operator-console.sh

# Or package for distribution to another operator:
./scripts/operator-package.sh --ip c2.example.com --port 50052 --zip
```

The deploy script exports `NEXUS_CA_CERT`, `NEXUS_CLIENT_CERT`,
`NEXUS_CLIENT_KEY` from `certs/prod/` and `NEXUS_SERVER_ADDR`, then
launches the binary. The Connect dialog pre-fills with `NEXUS_SERVER_ADDR`.

See [`operator-console.md`](operator-console.md) for the full console guide.

---

## Cert management

### Generating production certs

```bash
./scripts/gen-certs-prod.sh \
  --domain c2.example.com \
  --ip <public-ip> \
  --out ./certs/prod
```

Produces a self-contained PKI: CA, server cert (SAN: `c2.example.com` +
IP), operator cert, and agent cert template. All keys are ED25519.

### Installing on the server host

```bash
sudo install -m 0644 -o root  -g nexus ./certs/prod/ca.crt.pem     /etc/nexus/ca.crt.pem
sudo install -m 0644 -o root  -g nexus ./certs/prod/server.crt.pem /etc/nexus/server.crt.pem
sudo install -m 0600 -o nexus -g nexus ./certs/prod/server.key.pem /etc/nexus/server.key.pem
sudo systemctl restart nexus-server
```

Verify mTLS is working end-to-end:
```bash
openssl s_client -connect c2.example.com:50052 \
  -servername c2.example.com \
  -CAfile ./certs/prod/ca.crt.pem \
  -cert   ./certs/prod/operator.crt.pem \
  -key    ./certs/prod/operator.key.pem < /dev/null 2>&1 \
  | grep "Verify return code"
# Expected: Verify return code: 0 (ok)
```

### Multi-domain TLS profiles

For deployments that need distinct TLS identities per domain (e.g. primary
C2 + fallback domain, or domain fronting with separate certs), configure
named HTTPS profiles in `nexus.toml`:

```toml
[origin_cert]
# Legacy single-cert fields serve as the fallback when no SNI match
cert_path = "./certs/origin.crt"
key_path  = "./certs/origin.key"
ca_cert_path = "./certs/origin_ca.crt"
pin_validation = true
validity_days = 365

[[origin_cert.profiles]]
name = "primary"
domains = ["c2.example.com"]
cert_path    = "./certs/prod/server.crt.pem"
key_path     = "./certs/prod/server.key.pem"
ca_cert_path = "./certs/prod/ca.crt.pem"

[[origin_cert.profiles]]
name = "fallback"
domains = ["backup.example.com", "cdn.example.com"]
cert_path    = "./certs/fallback/server.crt.pem"
key_path     = "./certs/fallback/server.key.pem"
ca_cert_path = "./certs/fallback/ca.crt.pem"
```

**How it works:** The server selects the certificate to present based on
the SNI hostname in the client's TLS `ClientHello`. If no profile matches
the requested hostname, the single-cert fields are used as a default.

**Backward compatibility:** When no `[[origin_cert.profiles]]` entries
are present, the server behaves identically to previous versions — a
single certificate is used for all connections.

**Constraints:**

- Each domain may appear in only one profile (validated at startup).
- Wildcard domains (e.g. `*.example.com`) are supported.
- All profiles' CA certificates are trusted for client verification
  when `mutual_tls = true`.

---

## Pre-flight checklist

### Infrastructure

- [ ] C2 server host: public IP, port **50052** open inbound (agents + operators).
- [ ] DNS: `c2.example.com` A record → C2 server public IP.
- [ ] NTP on all hosts (`sudo timedatectl set-ntp true`).

### Network

- [ ] **Inbound port 50052**: agents + operators → C2 server. TCP.
- [ ] **Port 50051** (legacy v1.0 lane): only if overlay agents are in use.

### Security

- [ ] `insecure_network = false` in `nexus.toml`.
- [ ] All three server `NEXUS_*` vars set (`CA_CERT`, `SERVER_CERT`, `SERVER_KEY`).
- [ ] `server-identity.bin` backed up offline.
- [ ] `capabilities.json` has explicit peer-ids (no `"*"` wildcards in production).
- [ ] `server.key.pem` mode 0600, owned by nexus user.
- [ ] Audit log on a persistent filesystem (not tmpfs).

---

## First-run verification

1. **Server healthy**:
   ```bash
   sudo journalctl -u nexus-server | grep "A2A server starting"
   # A2A server starting addr=0.0.0.0:50052
   ```

2. **mTLS handshake**:
   ```bash
   openssl s_client -connect c2.example.com:50052 \
     -CAfile ./certs/prod/ca.crt.pem \
     -cert   ./certs/prod/operator.crt.pem \
     -key    ./certs/prod/operator.key.pem < /dev/null 2>&1 \
     | grep "Verify return code"
   # Verify return code: 0 (ok)
   ```

3. **Agent connects**:
   ```bash
   sudo journalctl -u nexus-agent -f
   # INFO identity loaded peer_id=<hex>
   # INFO connecting to C2 addr=https://c2.example.com:50052
   # INFO A2A agent-mode stream registered
   ```

4. **Agent in capability matrix** — edit `/etc/nexus/capabilities.json`
   with the peer-id, restart nexus-server, then:
   ```bash
   sudo journalctl -u nexus-server | grep "agent registered"
   ```

5. **Console connects** — open console → Connect dialog → click Connect
   → agent list populates.

6. **Shell works** — select agent → open shell → `whoami` returns.

7. **Audit log** — `sudo tail /var/lib/nexus/audit.log` shows records.

---

## Upgrade procedure

### Agent rolling restart

```bash
sudo systemctl restart nexus-agent
sudo journalctl -u nexus-agent --since "1 minute ago" --no-pager
```

### C2 server upgrade (brief outage)

```bash
sudo systemctl stop nexus-server
# Install new binary
sudo systemctl start nexus-server
sudo systemctl status nexus-server
```

In-flight shells drop on restart. All agents reconnect within ~60 s.

### Metrics Endpoint Security

The Prometheus metrics endpoint (`/metrics`) runs on a **separate plaintext
HTTP server** bound to `127.0.0.1:9100` by default.

**Security requirements:**

- **Never expose port 9100 to the WAN.** The endpoint is unauthenticated
  and serves internal counters (active sessions, request rates, rate-limit
  events).
- Firewall the port to localhost or the monitoring VLAN:
  ```bash
  sudo ufw deny in on eth0 to any port 9100
  ```
- If Prometheus runs on a different host, use an **SSH tunnel** rather than
  opening the port:
  ```bash
  ssh -NL 9100:127.0.0.1:9100 nexus@c2.example.com
  ```
- mTLS for the metrics endpoint is planned for v1.5+.

### Cert rotation

1. Re-run `gen-certs-prod.sh` with `--out ./certs/prod`.
2. Install new server certs to `/etc/nexus/` and restart nexus-server.
3. Rebuild agent bundles with `build-agent-bundles.sh --force` and
   redeploy to agent hosts.
4. Distribute new `certs/prod/operator.crt.pem` to operators and
   relaunch the console with the new `CERT_DIR`.
