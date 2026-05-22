# Production deployment

This guide walks through a production rollout of rust-nexus v1.2: a
hardened C2 server, N agents distributed across endpoint hosts, and
operator consoles installed on operator workstations.

> **Scope.** v1.2 covers single-region single-server deployments with
> operator-provided CA, mTLS, signed AgentCards, capability matrix,
> and a hash-chained audit log. Multi-region / HA / Docker / k8s are
> v1.3+ work. The legacy Cloudflare + ACME pipeline still exists in
> the code (`nexus-infra::{cloudflare,letsencrypt,domain_manager}`)
> and is partially stubbed; see the [Cloudflare/ACME
> appendix](#cloudflareacme-appendix) for current status.

---

## Just the Commands

Minimum copy-paste path for a first production rollout on an EC2 box
(domain → public IP, BYO CA certs in hand, building from source on the
host). Each step is the smallest thing that has to happen; for the
**why**, see the detailed sections below.

**Assumptions before step 1**

- EC2 security group: inbound TCP **50052** allowed from your operator
  and agent CIDRs. Outbound unrestricted.
- DNS: `c2.example.com` A record points at the EC2 public IP.
- You have a CA and have already issued: `ca.crt.pem`, `server.crt.pem`
  + `server.key.pem` (server cert SANs include
  `DNS:c2.example.com,IP:<ec2-public-ip>`), plus per-agent and
  per-operator client cert/key pairs. If you don't have a CA, see
  [CA strategy](#ca-strategy) and `scripts/gen-certs.sh` first.
- NTP enabled: `sudo timedatectl set-ntp true`.

### 1. Host prep + build (on the EC2 box)

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev git curl protobuf-compiler
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

git clone https://github.com/cmndcntrlcyber/rust-nexus.git
cd rust-nexus
cargo build --release -p nexus-infra --bin nexus-server
cargo build --release -p nexus-agent --bin nexus-agent
sudo install -m 0755 target/release/nexus-server /usr/local/bin/
sudo install -m 0755 target/release/nexus-agent  /usr/local/bin/
```

> `nexus-server` is a `[[bin]]` inside the `nexus-infra` crate, not a
> top-level package — `-p nexus-server` will error with
> `package ID specification did not match any packages`.

### 2. Users, directories, and cert drop

```bash
# Server side
sudo useradd --system --no-create-home --shell /usr/sbin/nologin nexus
sudo mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus
sudo chown -R nexus:nexus /var/lib/nexus /var/log/nexus
sudo chown root:nexus /etc/nexus && sudo chmod 750 /etc/nexus

# Agent side (same box for a smoke test, or each endpoint host)
sudo useradd --system --no-create-home --shell /usr/sbin/nologin nexus-agent
sudo mkdir -p /etc/nexus-agent /var/lib/nexus-agent
sudo chown -R nexus-agent:nexus-agent /var/lib/nexus-agent
sudo chown root:nexus-agent /etc/nexus-agent && sudo chmod 750 /etc/nexus-agent
```

Copy your BYO CA-signed PEMs into place:

```
/etc/nexus/ca.crt.pem            root:nexus      0644
/etc/nexus/server.crt.pem        root:nexus      0644
/etc/nexus/server.key.pem        nexus:nexus     0600
/etc/nexus-agent/ca.crt.pem      root:nexus-agent  0644
/etc/nexus-agent/client.crt.pem  root:nexus-agent  0644
/etc/nexus-agent/client.key.pem  nexus-agent:nexus-agent  0600
```

### 3. Server NodeIdentity, config, capabilities

```bash
sudo -u nexus /usr/local/bin/nexus-server --init-identity /var/lib/nexus/server-identity.bin
sudo chmod 600 /var/lib/nexus/server-identity.bin

sudo cp docs/deployment/examples/nexus.toml.example /etc/nexus/nexus.toml
sudo cp config/capabilities.example.json /etc/nexus/capabilities.json
```

Leave `capabilities.json` as-is for now; you'll add the agent's peer-id
in step 6.

### 4. Server env file (the three required `NEXUS_*` vars)

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

### 5. Start the server

```bash
sudo cp docs/deployment/examples/nexus-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nexus-server
sudo systemctl status nexus-server
```

Look for `A2A server starting addr=0.0.0.0:50052 insecure_network=false mtls=true`
in `journalctl -u nexus-server`.

### 6. Start the agent and wire its peer-id into the capability matrix

Agent env file:

```ini
# /etc/nexus-agent/agent.env
NEXUS_CA_CERT=/etc/nexus-agent/ca.crt.pem
NEXUS_CLIENT_CERT=/etc/nexus-agent/client.crt.pem
NEXUS_CLIENT_KEY=/etc/nexus-agent/client.key.pem
NEXUS_SERVER_ADDR=https://c2.example.com:50052
RUST_LOG=info
```

```bash
sudo chown root:nexus-agent /etc/nexus-agent/agent.env
sudo chmod 640 /etc/nexus-agent/agent.env
sudo cp docs/deployment/examples/nexus-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nexus-agent

# Grab the agent's peer-id from the server journal
sudo journalctl -u nexus-server --since "5 min ago" | grep "agent registered"
# Add that peer_id to /etc/nexus/capabilities.json with ["shell-session"]
sudoedit /etc/nexus/capabilities.json
sudo systemctl restart nexus-server
```

### 7. Operator console (on the operator workstation, not EC2)

Set these env vars before launching the Tauri console (see
[`operator-console.md`](operator-console.md) for the install bundle):

```ini
NEXUS_CA_CERT=...            # operator's copy of ca.crt.pem
NEXUS_CLIENT_CERT=...        # operator-<name>.crt.pem from your CA
NEXUS_CLIENT_KEY=...         # matching key
NEXUS_SERVER_ADDR=https://c2.example.com:50052
```

### 8. Smoke test

- Operator console sees the agent in `ListRegisteredAgents`.
- Open a shell on the agent, run `whoami`, see output round-trip.
- `sudo tail /var/lib/nexus/audit.log` shows fresh signed records.

For deeper verification (mTLS handshake errors, AgentCard signature,
audit-log chain), see [First-run verification](#first-run-verification).

---

## Pre-flight checklist

### Infrastructure

- [ ] **C2 server host.** Dedicated Linux box, ≥2 vCPU / ≥4 GB RAM. No
  other services on it. Public IPv4 (or behind a TLS-terminating
  reverse proxy if you front it).
- [ ] **Operator workstations.** macOS / Windows / Linux. Tauri
  console bundles distributed per [`operator-console.md`](operator-console.md).
- [ ] **Endpoint hosts.** Where the agents run. The agent is a single
  static binary; cross-compile with `cross` for the target triple.
- [ ] **Time sync.** All hosts on NTP within 5 seconds — mTLS
  validation and audit-log timestamps depend on it.

### Network

- [ ] **Inbound to C2 server, port 50052** (A2A plane): allow from
  operator nets and agent nets. TCP.
- [ ] **Inbound to C2 server, port 50051** (legacy NexusC2 plane,
  optional): allow only if you're keeping the v1.0 task-pull lane.
- [ ] **Outbound from agents**: allow to C2 on 50052.
- [ ] **Egress from operators**: allow to C2 on 50052.

### Security prerequisites

- [ ] **CA strategy.** Bring your own PKI (offline root + intermediate
  issuing CA). See [CA strategy](#ca-strategy) below.
- [ ] **Server NodeIdentity persistence.** Stable host directory with
  filesystem-level 0o600. Backups + rotation procedure agreed.
- [ ] **Capability matrix file.** Decide who can target which agents.
  Write `capabilities.json` ahead of first deploy.
- [ ] **Audit log retention.** Decide retention window (default
  recommendation: 90 days compressed). See
  [`operations.md`](operations.md) § Audit log retention.

---

## CA strategy

v1.2 deployments use mTLS via the five reserved env vars:
`NEXUS_CA_CERT`, `NEXUS_SERVER_CERT`, `NEXUS_SERVER_KEY`,
`NEXUS_CLIENT_CERT`, `NEXUS_CLIENT_KEY`. Each accepts either a
filesystem path or inline PEM (see `nexus-a2a/src/tls.rs`).

### Recommended: two-tier PKI

```
[ Offline Root CA ]                              (paper-cold, never online)
        │
        ▼
[ Online Issuing CA ]                            (your existing PKI)
        │
        ├─ server.crt    (CN=c2.internal, SAN=c2.internal,1.2.3.4)
        ├─ operator-<name>.crt   (CN per operator)
        └─ agent-<hostname>.crt   (CN per agent host)
```

The operator console authenticates each operator by their client cert
CN/SAN. The capability matrix is keyed by the **agent's** peer-id
(BLAKE3 of the agent's Ed25519 public key), not the operator's cert —
this is consistent with `nexus-a2a/src/capabilities.rs`. Per-operator
authorization is queued for v1.3.

### Minimum-viable: openssl-issued certs

For environments without an existing PKI, use `scripts/gen-certs.sh`
as a starting template — it produces ED25519-keyed certs in
`./certs/`. For production, regenerate with:

- A real CA CN (`O=YourCo, CN=YourCo Issuing CA 2026`)
- Proper SANs on server cert (`DNS:c2.internal,IP:1.2.3.4`)
- Per-operator client certs with distinct CNs (used in v1.3
  per-operator capability matrices)
- Validity: 1 year for leaf certs, 5 years for CA

### Production cert rotation

See [`operations.md`](operations.md) § Cert rotation.

---

## Server provisioning

### Filesystem layout (recommended)

```
/etc/nexus/
  nexus.toml              # Server config (see Example below)
  capabilities.json       # Capability matrix (mode 0o644)
  server.crt.pem          # Server cert (mode 0o644)
  server.key.pem          # Server key (mode 0o600, owned by nexus user)
  ca.crt.pem              # CA bundle (mode 0o644)

/var/lib/nexus/
  server-identity.bin     # 72-byte NXS_ID01 NodeIdentity (mode 0o600)
  audit.log               # Hash-chained audit log (FileSink)

/var/log/nexus/
  server.log              # tracing-subscriber JSON output (rotated by journald or logrotate)
```

Create the dedicated user:

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin nexus
sudo mkdir -p /etc/nexus /var/lib/nexus /var/log/nexus
sudo chown -R nexus:nexus /var/lib/nexus /var/log/nexus
sudo chown root:nexus /etc/nexus
sudo chmod 750 /etc/nexus
```

### NodeIdentity persistence (D-DEPLOY-D)

The C2 server's `NodeIdentity` is a 72-byte `NXS_ID01` blob:

- 8 bytes magic / version (`b"NXS_ID01"`)
- 32 bytes Ed25519 signing seed
- 32 bytes X25519 secret

Persist it to `/var/lib/nexus/server-identity.bin` with **mode 0o600**.
The file backs `A2aSharedState::with_server_identity()` which signs
the AgentCard at server startup (see `nexus-infra/src/serve.rs`).

```bash
# First-run generation. Restart-safe; will NOT overwrite if the file exists.
sudo -u nexus nexus-server --init-identity /var/lib/nexus/server-identity.bin
sudo chmod 600 /var/lib/nexus/server-identity.bin
```

Back up `server-identity.bin` to a secure offline location. If lost,
all signed AgentCards must be re-signed and operators will see a
fresh `signer_peer_id` once.

### Server config (`nexus.toml`)

See [`examples/nexus.toml.example`](examples/nexus.toml.example) for
the full reference. Key sections:

```toml
[a2a]
bind = "0.0.0.0:50052"
insecure_network = false                  # production: always false
identity_path = "/var/lib/nexus/server-identity.bin"

[a2a.tls]
# v1.2 reads certs from env vars (NEXUS_CA_CERT, NEXUS_SERVER_CERT,
# NEXUS_SERVER_KEY) — but you can pin paths here as well; systemd
# EnvironmentFile= is the recommended path.

[capabilities]
file = "/etc/nexus/capabilities.json"

[audit]
sink = "file"
path = "/var/lib/nexus/audit.log"

[rate_limit]
requests_per_second = 100                 # nexus_a2a::interceptors

[message_size]
max_bytes = 4194304                       # 4 MiB, nexus_a2a::interceptors::MAX_MESSAGE_SIZE
```

### Capability matrix

Edit `/etc/nexus/capabilities.json` based on the
[`config/capabilities.example.json`](../../config/capabilities.example.json)
template. Example for two production agents:

```json
{
  "agents": {
    "ab12cd34ef…1234": {"skills": ["shell-session"]},
    "ff99aa00bb…5678": {"skills": ["shell-session"]}
  }
}
```

The 64-character hex keys are the agents' peer-ids (BLAKE3 of the
agent's Ed25519 public key). To discover them, run the agent once and
look for the `A2A agent registered peer_id=<hex>` log line.

Wildcard `"*"` allows any agent; wildcard skill `"*"` allows any
skill. **Avoid both wildcards in production** — they reduce the
capability matrix to a no-op.

### mTLS env vars (D-V1.2-mtls)

Inject the five env vars via a systemd `EnvironmentFile=`:

```ini
# /etc/nexus/server.env
NEXUS_CA_CERT=/etc/nexus/ca.crt.pem
NEXUS_SERVER_CERT=/etc/nexus/server.crt.pem
NEXUS_SERVER_KEY=/etc/nexus/server.key.pem
```

```bash
sudo chmod 640 /etc/nexus/server.env
sudo chown root:nexus /etc/nexus/server.env
```

### Systemd unit

See [`examples/nexus-server.service`](examples/nexus-server.service)
for the full reference. Install:

```bash
sudo cp docs/deployment/examples/nexus-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nexus-server
sudo systemctl status nexus-server
```

Expected output: `Active: active (running)` and journal log lines
showing `A2A server starting addr=0.0.0.0:50052 insecure_network=false
mtls=true`.

---

## Agent provisioning

### Cross-compile the agent binary

On a build host with `cross` installed:

```bash
cross build --release --target x86_64-unknown-linux-musl -p nexus-agent
cross build --release --target x86_64-pc-windows-gnu -p nexus-agent
cross build --release --target aarch64-apple-darwin -p nexus-agent
```

The resulting binaries are in `target/<triple>/release/nexus-agent`.

### Per-agent NodeIdentity

Each agent host generates its own NodeIdentity on first run:

- Linux: `/var/lib/nexus-agent/identity.bin`, mode 0o600, owned by a
  dedicated `nexus-agent` user.
- Windows: `%PROGRAMDATA%\nexus-agent\identity.bin`, ACL restricted to
  SYSTEM + Administrators.
- macOS: `/var/db/nexus-agent/identity.bin`, mode 0o600.

The agent calls `NodeIdentity::load_or_create(path)` from
`nexus-common::identity` — first run writes the file, subsequent
runs read it.

### Per-agent client cert

Each agent host needs its own client cert from your CA:

```bash
openssl req -new -key /etc/nexus-agent/client.key.pem \
    -out /tmp/agent-csr.pem \
    -subj "/CN=agent-$(hostname -s)"
# Sign /tmp/agent-csr.pem on your offline / online CA, install the
# resulting cert as /etc/nexus-agent/client.crt.pem (mode 0o644).
```

Per-agent env vars (systemd `EnvironmentFile=`):

```ini
# /etc/nexus-agent/agent.env
NEXUS_CA_CERT=/etc/nexus-agent/ca.crt.pem
NEXUS_CLIENT_CERT=/etc/nexus-agent/client.crt.pem
NEXUS_CLIENT_KEY=/etc/nexus-agent/client.key.pem
```

### Systemd unit

See [`examples/nexus-agent.service`](examples/nexus-agent.service).

```bash
sudo cp docs/deployment/examples/nexus-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nexus-agent
```

The agent dials `https://<c2-host>:50052`, sends an `agent-register`
first frame, and stays connected. The C2 logs:

```
A2A agent registered peer_id=ab12cd34... os=linux version=0.2.0 tag=prod-host-1
```

Record the `peer_id` and add it to `/etc/nexus/capabilities.json` on
the C2 server.

---

## Operator console

See [`operator-console.md`](operator-console.md) for the full
distribution + first-run setup.

---

## First-run verification

After installing the C2, one agent, and the operator console:

1. **mTLS handshake**. Operator → C2 connection succeeds with TLS
   handshake completing (`tracing` log shows `mtls=true`). If you see
   `UnknownIssuer` or `BadCertificate`, the operator's `NEXUS_CA_CERT`
   doesn't match the CA that signed the server cert.

2. **Signed AgentCard**. The operator console calls
   `get_agent_card_verified()` (or `get_agent_card()` + verify) and
   accepts the response. If the server has no NodeIdentity configured,
   the card is unsigned — production deployments must have one.

3. **Agent registration**. C2 log line `A2A agent registered peer_id=…`
   appears after `systemctl start nexus-agent`. Without it, the
   agent's first frame is failing — check the agent's journal for
   TLS errors.

4. **Operator sees agent in list**. `ListRegisteredAgents` returns
   the agent's row.

5. **Capability check**. Open a shell on the agent. If denied with
   `PermissionDenied`, the capability matrix doesn't list this
   agent's peer-id for the `shell-session` skill — edit
   `/etc/nexus/capabilities.json` and restart the server.

6. **Interactive shell**. Type `whoami` → output round-trips.

7. **Audit log**. Inspect `/var/lib/nexus/audit.log`:

   ```bash
   tail -3 /var/lib/nexus/audit.log
   nexus-a2a-audit-verify /var/lib/nexus/audit.log
   # audit_verify: OK (3 records)
   ```

---

## Upgrade procedure

Rust-nexus v1.2 supports zero-downtime upgrades for agents (rolling
restart) and brief-outage upgrades for the C2 server.

### Agent rolling restart

```bash
# On each agent host, in parallel batches (e.g. 10% at a time):
sudo systemctl restart nexus-agent
sudo journalctl -u nexus-agent --since "1 minute ago" --no-pager
```

The agent reconnects to the C2 with the same peer-id, picks up its
slot in `AgentChannels`, and resumes serving operator sessions.

### C2 server upgrade

The server holds operator sessions in memory; restarting it drops
in-flight shells. Coordinate with operators or do the upgrade during
off-hours.

```bash
sudo systemctl stop nexus-server
# Install the new binary (e.g. via your package manager or `cp`).
sudo systemctl start nexus-server
sudo systemctl status nexus-server
```

Verify post-restart:

- All previously-connected agents reconnect within 60 seconds (their
  retry loop is in `nexus-agent/src/transports/grpc.rs`).
- Operator console reconnects and sees the agent list populate again.
- `tail /var/lib/nexus/audit.log` shows the chain continues
  uninterrupted across the restart (the `FileSink::open` constructor
  scans existing records to compute the last hash).

### Mixed-version rollout

v1.2 agents talk to v1.2 servers. v1.1 agents (no `agent-register`
frame) still register via the legacy NexusC2 lane (port 50051) and
appear in `ListRegisteredAgents` read-only — interactive shells
require the v1.2 agent-side bidi.

If you're upgrading from v1.1, the order is:

1. Upgrade the C2 server first (it accepts both lanes).
2. Roll the agents one at a time. Each newly-upgraded agent appears
   in the operator console with full v1.2 features (interactive
   shells, mTLS, capability gating).

---

## Cloudflare/ACME appendix

The v1.0 overlay shipped a Cloudflare DNS automation +
Let's Encrypt ACME pipeline (`nexus-infra/src/{cloudflare,
letsencrypt, domain_manager}.rs`). **v1.2 makes this optional** and
v1.2.1 stubbed the ACME `request_certificate` path — it now returns
`InfraError::LetsEncryptError("ACME order workflow deferred to v1.3
— use scripts/gen-certs.sh + NEXUS_*_CERT env vars in v1.2")`.

If you want to keep using the Cloudflare side for domain rotation /
DNS-fronting, the code is still in place and the `CloudflareManager`
APIs still work. But cert provisioning has to come from elsewhere
(your CA, or external ACME via certbot). The path back to fully
in-process ACME is queued for v1.3 — see
`nexus-infra/src/letsencrypt.rs` and the
`docs/configuration/production-setup.md` redirect stub for the
historical pipeline doc.

---

## Hardening checklist

- [ ] `insecure_network = false` in `nexus.toml` (D-V1-E reversal).
- [ ] All five `NEXUS_*_CERT` env vars set; server refuses to start
  without them in release builds.
- [ ] Server `NodeIdentity` persisted with mode 0o600.
- [ ] Capability matrix lists explicit agent peer-ids (no `"*"`
  wildcards for agents in production).
- [ ] Audit log path is on a non-tmpfs filesystem with at least 7
  days of retention; verify cadence ≥ daily.
- [ ] Rate limit configured (default 100 RPS per peer; raise only if
  you have measured load).
- [ ] `dev-reflection` Cargo feature is **off** (it's off by default
  in release builds).
- [ ] Filesystem perms: `chmod 600 /var/lib/nexus/server-identity.bin
  /var/lib/nexus/audit.log /etc/nexus/server.key.pem`.
- [ ] Tauri console bundles installed from CI artifacts (signed); see
  [`operator-console.md`](operator-console.md).
- [ ] Audit log verification cron in place; alerts on non-zero exit.
- [ ] Backup procedure for `server-identity.bin`, `capabilities.json`,
  and `audit.log` documented in [`operations.md`](operations.md).
