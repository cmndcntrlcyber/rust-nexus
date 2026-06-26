# Logging Standard

All production code in the rust-nexus workspace uses the **`tracing`** crate
for structured, levelled logging. The `log` crate has been fully removed
from direct dependencies as of v1.5.

## Usage

```rust
use tracing::{debug, error, info, warn};

info!("server started on {addr}");
warn!(retries = attempt, "connection retry");
debug!(agent_id = %id, "heartbeat received");
error!(?err, "TLS handshake failed");
```

Use structured fields (`key = value`) instead of `format!()` interpolation
where practical. The `%` sigil formats via `Display`, `?` via `Debug`.

## Levels

| Level | Use for |
|-------|---------|
| `error!` | Unrecoverable failures, broken invariants |
| `warn!` | Degraded operation, retries, fallback paths |
| `info!` | Lifecycle events (start, stop, connect, disconnect) |
| `debug!` | Per-request / per-message detail for troubleshooting |
| `trace!` | Byte-level dumps, hot-loop diagnostics (rarely used) |

## Subscriber Configuration

The `init_logging()` function in `nexus-infra/src/lib.rs` initialises
the default subscriber:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    )
    .init();
```

Override at runtime with `RUST_LOG`:

```bash
RUST_LOG=debug ./nexus-server --config nexus.toml
RUST_LOG=nexus_a2a=trace,nexus_infra=debug ./nexus-server --config nexus.toml
```

## Rules

- **Do not add `log` or `env_logger`** as a dependency to any crate.
- New code must use `tracing` exclusively.
- Prefer structured fields over string interpolation.
- Do not log secrets (API tokens, private keys, session cookies).
