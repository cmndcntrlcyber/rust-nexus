//! v1.3 Prometheus `/metrics` HTTP server (Phase 1.3.6).
//!
//! Plaintext on a separate port (default `127.0.0.1:9100`). Operators
//! firewall the port; mTLS for the metrics endpoint is v1.4 work.

use std::net::SocketAddr;

use anyhow::Result;
use nexus_a2a::metrics::{gather_text, DEFAULT_METRICS_PORT};
use tracing::{info, warn};

/// Configuration for the metrics endpoint.
#[derive(Debug, Clone)]
pub struct MetricsServerOptions {
    /// Bind address for `/metrics`.
    pub bind: SocketAddr,
}

impl Default for MetricsServerOptions {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], DEFAULT_METRICS_PORT)),
        }
    }
}

/// Run the metrics server until `shutdown` resolves. Single endpoint:
/// `GET /metrics` returning Prometheus text exposition. Any other
/// path returns `404`.
pub async fn run_metrics(
    opts: MetricsServerOptions,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;

    info!(addr = ?opts.bind, "metrics server starting");

    let app = Router::new().route(
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

    let listener = tokio::net::TcpListener::bind(opts.bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
