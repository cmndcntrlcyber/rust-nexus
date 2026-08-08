//! v1.3 Prometheus `/metrics` HTTP server (Phase 1.3.6).
//! v3.10 WS1: also hosts the REST ferry gateway alongside `/metrics`.
//!
//! Plaintext on a separate port (default `127.0.0.1:9100`). Operators
//! firewall the port; mTLS for the metrics endpoint is v1.4 work.

use std::net::SocketAddr;

use anyhow::Result;
use nexus_a2a::metrics::{gather_text, DEFAULT_METRICS_PORT};
use tracing::{info, warn};

use crate::ferry_gateway::{self, FerryState};

/// Configuration for the metrics + ferry gateway endpoint.
#[derive(Debug, Clone)]
pub struct MetricsServerOptions {
    /// Bind address for `/metrics` and `/ferry/*`.
    pub bind: SocketAddr,
    /// Optional ferry gateway state. When `None`, only `/metrics` is served.
    pub ferry_state: Option<FerryState>,
}

impl Default for MetricsServerOptions {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], DEFAULT_METRICS_PORT)),
            ferry_state: None,
        }
    }
}

/// Run the metrics + ferry gateway server until `shutdown` resolves.
///
/// Endpoints:
/// - `GET /metrics` — Prometheus text exposition
/// - `POST /ferry/task` — submit harness task (v3.10)
/// - `GET /ferry/agents` — list connected agents (v3.10)
/// - `GET /ferry/anomaly` — GML anomaly score (v3.10)
/// - `POST /ferry/telemetry` — ingest telemetry snapshot (v3.10)
/// - `POST /ferry/rate-adjust` — get rate adjustment (v3.10)
/// - `GET /ferry/health` — gateway health check (v3.10)
pub async fn run_metrics(
    opts: MetricsServerOptions,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;

    info!(addr = ?opts.bind, "metrics server starting");

    let metrics_route = Router::new().route(
        "/metrics",
        get(|| async {
            match gather_text() {
                Ok(body) => (StatusCode::OK, body).into_response(),
                Err(err) => {
                    warn!(error = %err, "metrics gather failed");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("metrics gather: {err}"),
                    )
                        .into_response()
                }
            }
        }),
    );

    let app = if let Some(ferry_state) = opts.ferry_state {
        info!("ferry gateway enabled on /ferry/*");
        metrics_route.merge(ferry_gateway::ferry_router(ferry_state))
    } else {
        metrics_route
    };

    let listener = tokio::net::TcpListener::bind(opts.bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
