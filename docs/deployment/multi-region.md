# Multi-Region Deployment

## Topology

```
                    ┌───────────────────┐
                    │   Cloudflare DNS   │
                    │  (GeoDNS failover) │
                    └────┬─────────┬────┘
                         │         │
              ┌──────────┘         └──────────┐
              ▼                               ▼
    ┌──────────────────┐            ┌──────────────────┐
    │  Region A (US)   │            │  Region B (EU)   │
    │  nexus-server    │◄──mesh────►│  nexus-server    │
    │  :50052 (A2A)    │            │  :50052 (A2A)    │
    │  :9100 (metrics) │            │  :9100 (metrics) │
    └────────┬─────────┘            └────────┬─────────┘
             │                               │
        ┌────┴────┐                     ┌────┴────┐
        │ agents  │                     │ agents  │
        └─────────┘                     └─────────┘
```

## DNS Failover with Cloudflare

1. Create an A record per region pointing to each server's public IP.
2. Enable Cloudflare Load Balancing with health checks:
   - Monitor: HTTPS on port 50052 (A2A gRPC) or TCP on port 443.
   - Failover pool: Region B is the fallback when Region A is unhealthy.
3. Agents use the shared domain (`c2.example.com`); Cloudflare routes
   to the nearest healthy region.

## Mesh Node Placement

Each nexus-server runs a libp2p mesh node. Cross-region nodes discover
each other via explicit bootstrap peers (mDNS is LAN-only).

Configure bootstrap in `nexus.toml`:

```toml
[mesh]
bootstrap_peers = [
    "/ip4/203.0.113.10/tcp/4001/p2p/<peer-id-region-a>",
    "/ip4/198.51.100.20/tcp/4001/p2p/<peer-id-region-b>",
]
```

The DTN store-and-forward queue bridges temporary cross-region outages.

## Certificate Considerations

- Each region has its own server cert (SAN includes the region IP).
- All regions share the same CA so agents trust any region's server cert.
- Generate with: `./scripts/gen-certs-prod.sh --domain c2.example.com --ip <region-ip>`
