# Mesh Networking Troubleshooting

## Common Connectivity Issues

### Nodes cannot discover each other

**Symptom:** `nexus-server` logs show no peer connections.

**Checks:**
1. Verify the mesh port is open (default: 4001):
   ```bash
   ss -tlnp | grep 4001
   sudo ufw status | grep 4001
   ```
2. Confirm bootstrap peers are configured in `nexus.toml`:
   ```toml
   [mesh]
   bootstrap_peers = ["/ip4/<ip>/tcp/4001/p2p/<peer-id>"]
   ```
3. mDNS only works on the same LAN — use explicit bootstrap for
   cross-network deployments.

### Peers connect but messages don't arrive

**Symptom:** Peers appear in the peer table but GossipSub messages
are not received.

**Checks:**
1. Both nodes must subscribe to the same GossipSub topic.
2. Check that sealed-envelope keys match — a key mismatch causes
   silent decryption failures. Verify with:
   ```bash
   RUST_LOG=nexus_infra::mesh_listener=debug nexus-server ...
   ```
3. Ensure the mesh listener is enabled in the server configuration.

## DTN Queue Diagnostics

The DTN (Delay-Tolerant Networking) queue stores messages for
offline peers.

### Check queue depth

```bash
ls -la /var/lib/nexus/dtn-queue/ | wc -l
```

### Queue growing unbounded

The queue has a configurable `max_depth`. When full, the oldest
messages are evicted. If the queue grows faster than it drains:
1. Check if the target peer is actually reachable.
2. Increase `max_depth` or reduce message rate.
3. Review `RUST_LOG=nexus_mesh::dtn=debug` output for eviction
   warnings.

### Clear the queue

```bash
sudo systemctl stop nexus-server
sudo rm -rf /var/lib/nexus/dtn-queue/*
sudo systemctl start nexus-server
```

## libp2p Peer Discovery Debugging

Enable verbose libp2p logging:

```bash
RUST_LOG=libp2p=debug,nexus_mesh=debug nexus-server --config nexus.toml
```

### Key log messages

| Message | Meaning |
|---|---|
| `NewListenAddr` | Mesh node is listening on an address |
| `ConnectionEstablished` | A peer connected |
| `ConnectionClosed` | A peer disconnected |
| `Discovered` (mDNS) | A peer was found on the local network |
| `RoutingUpdated` (Kademlia) | DHT routing table changed |
| `Message` (GossipSub) | A pub/sub message was received |

### Peer ID mismatch

If a node reports "peer ID mismatch" on incoming connections, the
remote node's identity has changed (re-keyed) without updating the
bootstrap peer list. Update `bootstrap_peers` with the new peer ID.
