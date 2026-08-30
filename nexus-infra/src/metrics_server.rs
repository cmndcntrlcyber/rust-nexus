//! v1.3 Prometheus `/metrics` HTTP server (Phase 1.3.6).
//! v3.10 WS1: also hosts the REST ferry gateway alongside `/metrics`.
//!
//! Plaintext on a separate port (default `127.0.0.1:9100`). When
//! `NEXUS_FERRY_TLS_CERT` and `NEXUS_FERRY_TLS_KEY` are set, the
//! server uses TLS (optionally requiring client certs for mTLS).

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
    /// TLS certificate PEM path (env: `NEXUS_FERRY_TLS_CERT`).
    pub tls_cert: Option<String>,
    /// TLS private key PEM path (env: `NEXUS_FERRY_TLS_KEY`).
    pub tls_key: Option<String>,
    /// CA cert PEM path for client verification / mTLS (env: `NEXUS_FERRY_TLS_CA`).
    pub tls_ca: Option<String>,
}

impl Default for MetricsServerOptions {
    fn default() -> Self {
        let bind = std::env::var("NEXUS_FERRY_BIND")
            .ok()
            .and_then(|s| s.parse::<SocketAddr>().ok())
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], DEFAULT_METRICS_PORT)));
        Self {
            bind,
            ferry_state: None,
            tls_cert: std::env::var("NEXUS_FERRY_TLS_CERT").ok(),
            tls_key: std::env::var("NEXUS_FERRY_TLS_KEY").ok(),
            tls_ca: std::env::var("NEXUS_FERRY_TLS_CA").ok(),
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

    if let (Some(cert_path), Some(key_path)) = (&opts.tls_cert, &opts.tls_key) {
        info!(cert = %cert_path, key = %key_path, "ferry gateway TLS enabled");
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &cert_path, &key_path,
        )
        .await
        .map_err(|e| anyhow::anyhow!("TLS config: {e}"))?;

        axum_server::bind_rustls(opts.bind, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(opts.bind).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await?;
    }
    Ok(())
}
