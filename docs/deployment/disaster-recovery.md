# Disaster Recovery

## Backup Targets

| Path | Contents | Frequency |
|---|---|---|
| `/var/lib/nexus/identity.bin` | Ed25519 NodeIdentity (server keypair) | Once (at creation) |
| `/var/lib/nexus/audit.log*` | BLAKE3 hash-chained audit records | Daily |
| `/var/lib/nexus/dtn-queue/` | DTN store-and-forward messages | Daily |
| `/etc/nexus/` | Server certs, keys, and `nexus.toml` | After every cert rotation |
| Agent bundles | Per-host cert + key pairs | After every cert rotation |

## Backup Procedure

```bash
BACKUP_DIR="/backup/nexus/$(date +%Y%m%d)"
sudo mkdir -p "$BACKUP_DIR"

sudo cp /var/lib/nexus/identity.bin "$BACKUP_DIR/"
sudo cp /var/lib/nexus/audit.log* "$BACKUP_DIR/"
sudo cp -r /var/lib/nexus/dtn-queue "$BACKUP_DIR/" 2>/dev/null || true
sudo tar czf "$BACKUP_DIR/etc-nexus.tar.gz" /etc/nexus/
```

Store backups off-host (encrypted, access-controlled).

## Restore Procedure

1. **Install binaries** on the replacement host.
2. **Restore identity:**
   ```bash
   sudo cp "$BACKUP_DIR/identity.bin" /var/lib/nexus/
   sudo chown nexus:nexus /var/lib/nexus/identity.bin
   ```
3. **Restore certs:**
   ```bash
   sudo tar xzf "$BACKUP_DIR/etc-nexus.tar.gz" -C /
   ```
4. **Restore audit log** (for chain continuity):
   ```bash
   sudo cp "$BACKUP_DIR/audit.log" /var/lib/nexus/
   ```
5. **Start the server:** `sudo systemctl start nexus-server`
6. **Verify:** agents should reconnect within ~60 seconds.

## RPO / RTO

| Metric | Target | Notes |
|---|---|---|
| RPO (Recovery Point Objective) | 24 hours | Daily backups; audit log is append-only |
| RTO (Recovery Time Objective) | 30 minutes | Binary install + identity restore + restart |

If the identity key is lost, a full re-key is required (see
`credential-rotation.md` > NodeIdentity Re-Key). All agents will need
new client certificates.
