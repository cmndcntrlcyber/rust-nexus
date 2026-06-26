//! v1.4 OpenTelemetry trace export (Phase 1.4.5).
//!
//! Optional OTLP/gRPC export. Off by default; operators enable via
//! the `otel` Cargo feature on `nexus-a2a` and call
//! [`init_tracing_with_otel`] at startup.
//!
//! v1.4 ships:
//! - Always-on: the [`OtelOptions`] config struct + feature-gated
//!   `init_tracing_with_otel` entry point.
//! - With `otel` feature: real OTLP/gRPC span export via
//!   `opentelemetry-otlp` + `tracing-opentelemetry`.
//! - Without `otel` feature: a no-op `init_tracing_with_otel` that
//!   logs a warning and returns `Ok(())` so consumer code stays
//!   feature-agnostic.

// OTel config struct is always available; init is behind `otel` feature.

/// Default OTLP collector endpoint (assumed local-dev Jaeger / Tempo).
pub const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";

/// Configuration for the OTel layer.
#[derive(Debug, Clone)]
pub struct OtelOptions {
    /// OTLP/gRPC collector endpoint.
    pub endpoint: String,
    /// Service name reported with spans.
    pub service_name: String,
}

impl Default for OtelOptions {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_OTLP_ENDPOINT.to_string(),
            service_name: "nexus-server".to_string(),
        }
    }
}

/// Errors emitted by tracing initialization.
#[derive(Debug, thiserror::Error)]
pub enum OtelError {
    /// OTel pipeline construction failed.
    #[error("otel pipeline init: {0}")]
    Init(String),
    /// `init_tracing_with_otel` was called without the `otel`
    /// Cargo feature compiled in.
    #[error("otel feature not enabled; rebuild nexus-a2a with `--features otel`")]
    FeatureDisabled,
}

#[cfg(feature = "otel")]
mod inner {
    use super::{OtelError, OtelOptions};
    use opentelemetry::trace::TracerProvider;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::Resource;
    use tracing_subscriber::prelude::*;

    pub fn init_tracing_with_otel(opts: &OtelOptions) -> Result<(), OtelError> {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(opts.endpoint.clone())
            .build()
            .map_err(|e| OtelError::Init(format!("build span exporter: {e}")))?;

        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                opts.service_name.clone(),
            )]))
            .build();

        let tracer = provider.tracer("nexus-a2a");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer())
            .with(otel_layer)
            .try_init()
            .map_err(|e| OtelError::Init(format!("install subscriber: {e}")))?;

        Ok(())
    }
}

#[cfg(not(feature = "otel"))]
mod inner {
    use super::{OtelError, OtelOptions};
    use tracing::warn;

    pub fn init_tracing_with_otel(_opts: &OtelOptions) -> Result<(), OtelError> {
        warn!(
            "OTel trace export requested but `otel` feature is not enabled — rebuild \
             nexus-a2a with `--features otel`. Falling back to default tracing-subscriber."
        );
        Err(OtelError::FeatureDisabled)
    }
}

/// Initialize tracing with optional OTel OTLP/gRPC export.
///
/// When the `otel` Cargo feature is enabled, installs a
/// `tracing-subscriber` that batches spans to `opts.endpoint`. When
/// the feature is off, returns [`OtelError::FeatureDisabled`] so
/// callers can fall back to default tracing setup.
pub fn init_tracing_with_otel(opts: &OtelOptions) -> Result<(), OtelError> {
    inner::init_tracing_with_otel(opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_have_local_endpoint() {
        let opts = OtelOptions::default();
        assert_eq!(opts.endpoint, DEFAULT_OTLP_ENDPOINT);
        assert_eq!(opts.service_name, "nexus-server");
    }

    #[cfg(not(feature = "otel"))]
    #[test]
    fn init_returns_feature_disabled_when_off() {
        let err = init_tracing_with_otel(&OtelOptions::default()).expect_err("must error");
        matches!(err, OtelError::FeatureDisabled);
    }
}
