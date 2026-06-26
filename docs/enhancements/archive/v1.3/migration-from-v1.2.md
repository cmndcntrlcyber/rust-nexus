# v1.2 → v1.3 migration

v1.3 is **wire-compatible, config-additive, and API-additive** —
existing v1.2 operators and agents keep working unchanged. New
defenses and infrastructure are opt-in.

## Wire additions

| New surface | Where | Notes |
|---|---|---|
| Prometheus metrics exposition | `nexus_infra::metrics_server` | Plaintext HTTP on `127.0.0.1:9100`; pull-based `/metrics` endpoint |
| SyslogSink (RFC 5424 TCP) | `nexus_a2a::audit::SyslogSink` | Ships audit records to external SIEM |
| MultiSink | `nexus_a2a::audit::MultiSink` | Combines file + syslog + broadcast sinks |

All v1.2 wire format (`shell-open`, `agent-register`, `Message`,
`Part`, audit records, capability JSON) remains unchanged.

## Config additions

| New section / file | Purpose |
|---|---|
| `[metrics] bind_address / port` in `nexus.toml` | Prometheus metrics HTTP server config |
| `deny.toml` | `cargo-deny` supply chain policy (license allowlist, AGPL hard-deny) |
| `.github/workflows/ci.yml` | CI pipeline: check, test, clippy, fmt, mTLS integration, WASM build |
| `.github/workflows/security-audit.yml` | Weekly + push `cargo-deny` audit |
| `Dockerfile` (multi-arch) | `linux/amd64` + `linux/arm64` builds |

## API additions

| Builder / method | Module | Phase |
|---|---|---|
| `nexus_a2a::audit::SyslogSink::new` | `nexus-a2a/src/audit.rs` | 1.3.7 |
| `nexus_a2a::audit::MultiSink::new` | `nexus-a2a/src/audit.rs` | 1.3.7 |
| `nexus_a2a::metrics::init / gather` | `nexus-a2a/src/metrics.rs` | 1.3.6 |
| `nexus_infra::metrics_server::run` | `nexus-infra/src/metrics_server.rs` | 1.3.6 |
| `nexus_infra::mesh_listener::run` | `nexus-infra/src/mesh_listener.rs` | 1.3.3 |
| Per-operator scoping (`verify_with_operator`) | `nexus-a2a/src/interceptors.rs` | 1.3.5 |
| `ShellOpenParams.operator_cn` | `nexus-a2a/src/framing.rs` | 1.3.5 |

## Dependency changes

| Dependency | Change | Reason |
|---|---|---|
| `axum` 0.8 | Added | Prometheus `/metrics` HTTP server |
| `nexus-mesh` (workspace) | Added | libp2p mesh networking layer |
| `libp2p` 0.53 | Added | Kademlia DHT + mDNS + GossipSub |

## Environment variable additions

| Variable | Purpose |
|---|---|
| `NEXUS_CA_CERT` | CA certificate for mTLS (PEM file path or inline blob) |
| `NEXUS_SERVER_CERT` / `NEXUS_SERVER_KEY` | Server TLS identity |
| `NEXUS_CLIENT_CERT` / `NEXUS_CLIENT_KEY` | Client mTLS identity |

## Upgrade procedure

1. Build the new binary: `cargo build --release -p nexus-server`.
2. Stop the running v1.2 server.
3. Install the new binary and restart.
4. Optionally enable `[metrics]` in `nexus.toml` to expose Prometheus.
5. Agents reconnect automatically within ~60 seconds.
