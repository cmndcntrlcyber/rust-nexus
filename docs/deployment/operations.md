# Operations runbook

Day-2 procedures for a rust-nexus v1.2 deployment. Use alongside
[`production.md`](production.md) (the initial rollout doc).

> All commands assume the filesystem layout from
> [`production.md` § Server provisioning](production.md#server-provisioning):
> `/etc/nexus/`, `/var/lib/nexus/`, `/var/log/nexus/`. Adjust paths
> for your environment.

---

## Cert rotation

Recommended cadence: **quarterly** for leaf certs (server + operator
+ agent), **before-expiry** for the CA (typically 5-year validity).

### Zero-downtime server cert rotation

1. **Issue the new server cert** signed by the existing CA. SAN must
   still cover `c2.internal,IP:1.2.3.4`. Save as
   `/etc/nexus/server-2026Q3.crt.pem` + `server-2026Q3.key.pem`.

2. **Bundle both old + new CA** in `NEXUS_CA_CERT` if you're also
   rotating the CA in the same pass — otherwise leave `ca.crt.pem`
   untouched. Operators and agents read the CA bundle, so adding
   the new root before flipping the leaf gives them time to trust
   it.

3. **Atomically swap the new cert / key** into place:

   ```bash
   sudo install -m 644 -o root -g nexus /tmp/server-2026Q3.crt.pem /etc/nexus/server.crt.pem
   sudo install -m 600 -o root -g nexus /tmp/server-2026Q3.key.pem /etc/nexus/server.key.pem
   ```

4. **Reload the server.** v1.2 doesn't support hot cert reload; restart:

   ```bash
   sudo systemctl restart nexus-server
   sudo journalctl -u nexus-server --since "1 minute ago" -n 20
   ```

5. **Verify** with `openssl s_client`:

   ```bash
   openssl s_client -connect c2.internal:50052 -servername c2.internal \
       -CAfile /etc/nexus/ca.crt.pem </dev/null | grep "subject="
   ```

   The subject should match the new cert's CN.

6. **Decommission the old cert / key** files only after operators and
   agents have all successfully reconnected.

### Operator client cert rotation

Issue a new client cert. Each operator copies the new cert + key to
their local cert store, updates their Connect profile, and reconnects.
The audit log records `operator=<new CN>` going forward.

When an operator leaves the team: **revoke their client cert**
(typically by removing it from the CA's CRL or short-lived OCSP
response — whatever your PKI offers). If you don't have a CRL, the
short-term mitigation is to rotate the CA.

### Agent client cert rotation

For each agent host:

```bash
# Copy the new cert+key into place.
sudo install -m 644 /tmp/agent-new.crt.pem /etc/nexus-agent/client.crt.pem
sudo install -m 600 /tmp/agent-new.key.pem /etc/nexus-agent/client.key.pem
sudo systemctl restart nexus-agent
```

The agent reconnects with the new cert. No capability-matrix change
needed (the matrix keys on the agent's NodeIdentity peer-id, not its
TLS cert CN).

### CA rotation

Multi-step, lasts as long as your longest-lived leaf cert:

1. Stand up the new issuing CA.
2. Distribute the new CA cert to all operators + agents + the C2
   server. Bundle old + new CA in everyone's `NEXUS_CA_CERT` for the
   transition window.
3. Issue new leaf certs from the new CA. Distribute to all parties.
4. After every leaf cert has been rotated to one issued by the new
   CA, remove the old CA from the bundles.

---

## Server NodeIdentity rotation

When to rotate the server's `NodeIdentity` (`/var/lib/nexus/server-identity.bin`):

- Suspected compromise of the host
- Hardware swap / disk failure / restore from old backup
- Annual hygiene (recommended for high-assurance deployments)

### Procedure

1. **Stop the server**:

   ```bash
   sudo systemctl stop nexus-server
   ```

2. **Back up the old identity** (forensics):

   ```bash
   sudo cp /var/lib/nexus/server-identity.bin \
           /var/lib/nexus/server-identity.bin.retired-2026-08-15
   ```

3. **Generate a new identity** (the server does this automatically
   on first run if the file is missing — or use the `--init-identity`
   flag explicitly):

   ```bash
   sudo rm /var/lib/nexus/server-identity.bin
   sudo -u nexus nexus-server --init-identity /var/lib/nexus/server-identity.bin
   sudo chmod 600 /var/lib/nexus/server-identity.bin
   ```

4. **Restart**. The server signs a new AgentCard with the fresh
   identity:

   ```bash
   sudo systemctl start nexus-server
   sudo journalctl -u nexus-server -n 30 | grep 'AgentCard signed'
   ```

5. **Notify operators**. The first time each operator console reconnects,
   `get_agent_card_verified()` will return a card with a new
   `signer_peer_id`. If your operators are configured to pin a specific
   `signer_peer_id` out-of-band (recommended for high-assurance), they
   need to be told the new value.

---

## Agent NodeIdentity rotation

When an agent's identity is compromised — for example, the endpoint
host was reimaged or the `identity.bin` was exfiltrated:

1. **Stop the agent** on the affected host (or accept that it's no
   longer trustworthy — see [Incident response](#incident-response)).
2. **Remove the old peer-id from the capability matrix**:

   ```bash
   sudo sed -i.bak '/ab12cd34/d' /etc/nexus/capabilities.json
   ```

   (Replace `ab12cd34` with the actual peer-id prefix.)

3. **Generate a new identity** by deleting `identity.bin` and
   restarting the agent — `NodeIdentity::load_or_create` creates a
   fresh one.
4. **Add the new peer-id** to `capabilities.json`. The C2 server has
   to be restarted for the change to take effect (v1.2 reads the file
   at startup; hot-reload is v1.3+).
5. **Audit recent sessions** with the old peer-id — see
   [Incident response](#incident-response).

---

## Audit log retention

`/var/lib/nexus/audit.log` accumulates one JSON-per-line record per
operator action + server-side registry event. The hash chain
(`BLAKE3(prev_hash || record_bytes)`) makes after-the-fact tampering
detectable but does not bound the file's size.

### Recommended rotation

- **Cadence**: daily.
- **Retention**: 90 days compressed.
- **Verifier cadence**: once per day, after rotation, before
  compression of the previous day.

### logrotate config

See [`examples/audit-rotation.conf`](examples/audit-rotation.conf).
Install:

```bash
sudo install -m 644 \
    docs/deployment/examples/audit-rotation.conf \
    /etc/logrotate.d/nexus-audit
sudo logrotate -d /etc/logrotate.d/nexus-audit   # dry run
sudo logrotate /etc/logrotate.d/nexus-audit      # commit
```

### Daily verification

Add a cron entry to verify the chain integrity:

```cron
# /etc/cron.d/nexus-audit-verify
0 3 * * *  nexus  /usr/local/bin/nexus-a2a-audit-verify /var/lib/nexus/audit.log \
                  || logger -t nexus-audit -p user.crit "audit log TAMPER detected"
```

The `audit_verify` CLI from `nexus-a2a/src/bin/audit_verify.rs`
returns exit code 0 on intact chain, non-zero on tamper. Wire the
non-zero exit to your alerting system (e.g. push to PagerDuty,
post to your incident Slack).

### Why a hash chain helps

If an attacker compromises the C2 host and tries to **edit** an audit
record (e.g. delete their own session), the recomputed `record_hash`
won't match the stored value and the chain breaks. If they **append**
fake records, the next genuine record's `prev_hash` won't link to
their fake one. The chain doesn't prevent tampering — it just makes
silent tampering impossible.

### What a hash chain doesn't help with

- An attacker who controls the C2 process can write arbitrary new
  records before tampering becomes detectable. Mitigation: ship audit
  records to an external sink (Splunk, syslog) for a second copy. v1.2
  ships file-only; remote sinks are v1.3+.
- An attacker who controls the host can rewrite the entire file from
  scratch, recomputing every hash. Mitigation: pin the latest
  `record_hash` externally (e.g. publish daily to an immutable log).

---

## Capability matrix updates

`/etc/nexus/capabilities.json` is read at server startup. To change
authorization rules:

1. **Edit the file**:

   ```bash
   sudo $EDITOR /etc/nexus/capabilities.json
   ```

2. **Validate JSON**:

   ```bash
   sudo -u nexus python3 -c "import json,sys; json.load(open('/etc/nexus/capabilities.json'))"
   ```

3. **Restart the server**:

   ```bash
   sudo systemctl restart nexus-server
   ```

4. **Audit the change**. Note who made the edit + which agents
   gained/lost which skills. In v1.3+ the server itself will emit
   an `audit_record action=capability_change` event — for now, track
   in your change management system.

> **v1.3+ work**: hot-reload via SIGHUP, per-operator scoping based
> on client cert CN, JSON-schema validation at load. v1.2 keeps the
> matrix simple.

---

## Rate limit tuning

Default cap: 100 RPS per peer (`nexus_a2a::interceptors::DEFAULT_RATE_LIMIT_RPS`).
A peer in v1.2 is the operator's TLS endpoint (transport-level).
Tune in `nexus.toml`:

```toml
[rate_limit]
requests_per_second = 200
```

Then `systemctl restart nexus-server`.

### Recognizing over-limit traffic

The C2 logs `Status::ResourceExhausted` responses with the message
`"rate limit exceeded for <peer> (<rps> req/s)"`. Tail the journal:

```bash
sudo journalctl -u nexus-server -f | grep -i "rate limit"
```

If you see legitimate operators hitting the cap, raise it
incrementally. If you see a single peer hitting the cap repeatedly,
investigate for misbehaving client (or active probing).

---

## Incident response

### Suspected operator compromise

1. **Revoke** the operator's client cert at your PKI (CRL / OCSP
   short-lived response). If you don't have CRL infrastructure, the
   short-term fix is to **rotate the CA** (see [CA rotation](#ca-rotation)).
2. **Audit** recent sessions by that operator's cert CN. Even though
   the v1.2 capability matrix doesn't key on operator cert, the
   audit log records the operator's CN in the `actor` field:

   ```bash
   jq -c 'select(.actor=="operator-suspect")' /var/lib/nexus/audit.log
   ```

3. **Rotate** all certs the operator may have known passphrases for.

### Suspected agent compromise

1. **Identify the agent**: its peer-id from the audit log.
2. **Remove** the peer-id from `/etc/nexus/capabilities.json`.
3. **Audit** recent shell sessions on that agent:

   ```bash
   jq -c 'select(.resource=="ab12cd34…64-char-peer-id")' /var/lib/nexus/audit.log
   ```

4. **Rebuild** the agent host from a known-good image. Do not assume
   that just rotating the agent's NodeIdentity is sufficient.

### Audit log tamper detection

Triggered by the daily `audit_verify` cron returning non-zero.

1. **Don't restart anything yet** — preserve the file as-is for
   forensics.
2. **Confirm** the tamper:

   ```bash
   sudo nexus-a2a-audit-verify /var/lib/nexus/audit.log
   # Note the "record N: prev_hash mismatch" / "record_hash tampered" message.
   ```

3. **Compare** against the previous day's rotated archive:

   ```bash
   sudo zcat /var/lib/nexus/audit.log.1.gz | head -3
   ```

   If the previous archive's tail records appear with different
   content vs the current file's head, an attacker partially
   overwrote the live file.

4. **Escalate**. Assume the host was compromised and treat per your
   IR playbook.

---

## Monitoring / alerting integration

v1.2 ships **structured `tracing`** output (`tracing-subscriber` with
JSON formatter). v1.3 will add a Prometheus exporter and OTel traces.

### Suggested log fields to scrape

| Field | Source | Why |
|---|---|---|
| `mtls=true` on startup | `nexus-a2a::server` | Confirms production hardening is on |
| `A2A AgentCard signed` | `nexus-infra::serve` | Confirms NodeIdentity is configured |
| `A2A agent registered peer_id=…` | `nexus-infra::a2a_router::AgentRegistrar` | Track agent fleet inventory |
| `A2A agent deregistered` | same | Detect unexpected disconnects |
| `rate limit exceeded for…` | `nexus-a2a::interceptors` | Abuse detection |
| `capability denied: skill … for agent …` | `nexus-infra::a2a_router::OperatorRouter` | Track denied actions |

### Example Loki / Promtail target

```yaml
# /etc/promtail/config.yml (excerpt)
scrape_configs:
  - job_name: nexus-server
    journal:
      max_age: 24h
      labels:
        job: nexus-server
    relabel_configs:
      - source_labels: ['__journal__systemd_unit']
        regex: nexus-server\.service
        action: keep
```

### Recommended alerts

| Alert | Condition | Why |
|---|---|---|
| `audit log tamper` | `nexus-a2a-audit-verify` cron exits non-zero | Critical |
| `agent dropped` | `A2A agent deregistered` not followed by re-`registered` within 5 min | Connectivity or compromise |
| `rate limit sustained` | `rate limit exceeded` > 10/min for one peer | Abuse / runaway client |
| `mTLS handshake failure` | `tonic` error rate > 1/min | Cert / network / attack |
| `server unsigned card` | `ships unsigned (no server identity)` at startup | Misconfiguration |

---

## Backup / disaster recovery

### What to back up

| Path | Frequency | Storage |
|---|---|---|
| `/var/lib/nexus/server-identity.bin` | On change (rotation only) | Offline, encrypted |
| `/etc/nexus/nexus.toml` | On change | Version control |
| `/etc/nexus/capabilities.json` | On change | Version control |
| `/etc/nexus/{ca,server}.crt.pem` | On rotation | Same as identity |
| `/var/lib/nexus/audit.log` (rotated archives) | Daily after rotation | Hot tier for 30 days, cold after |

### Restore drill (recommended quarterly)

On a spare host:

1. Provision per [`production.md`](production.md).
2. Restore `server-identity.bin` from backup.
3. Restore `nexus.toml` + `capabilities.json` from version control.
4. Restore certs.
5. Start the server. Confirm:
   - `A2A AgentCard signed with server identity` (identity restored).
   - One test agent connects and registers.
   - Operator console connects and lists the agent.

If any step fails, document the gap and update the backup procedure
before the real DR event.
