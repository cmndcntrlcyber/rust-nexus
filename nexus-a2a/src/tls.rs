//! v1.2 mTLS plumbing for the A2A plane (D-V1.2-mtls / D-V1-E reversal).
//!
//! Loads cert + key + CA material from the reserved environment variables:
//!
//! - Server side: `NEXUS_SERVER_CERT`, `NEXUS_SERVER_KEY`, `NEXUS_CA_CERT`
//! - Client side: `NEXUS_CLIENT_CERT`, `NEXUS_CLIENT_KEY`, `NEXUS_CA_CERT`
//!
//! Each variable accepts **either** a filesystem path (PEM-on-disk) **or**
//! an inline PEM blob (detected by the presence of `"-----BEGIN"`).
//! Operators provide certs via `scripts/gen-certs.sh` (dev/test) or their
//! own CA (production).

use std::env;
use std::path::Path;

use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};

/// Env var carrying the CA bundle (PEM, path or inline).
pub const ENV_CA_CERT: &str = "NEXUS_CA_CERT";
/// Env var carrying the server certificate (PEM, path or inline).
pub const ENV_SERVER_CERT: &str = "NEXUS_SERVER_CERT";
/// Env var carrying the server private key (PEM, path or inline).
pub const ENV_SERVER_KEY: &str = "NEXUS_SERVER_KEY";
/// Env var carrying the client certificate (PEM, path or inline).
pub const ENV_CLIENT_CERT: &str = "NEXUS_CLIENT_CERT";
/// Env var carrying the client private key (PEM, path or inline).
pub const ENV_CLIENT_KEY: &str = "NEXUS_CLIENT_KEY";

/// Errors from cert/key loading.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    /// Required env var was missing or empty.
    #[error("environment variable `{0}` is not set")]
    Missing(&'static str),
    /// PEM material couldn't be read from disk.
    #[error("read PEM from {path}: {err}")]
    ReadFile {
        /// File path.
        path: String,
        /// Inner io error.
        err: std::io::Error,
    },
}

/// Materialize the contents of an env var. If the value contains
/// `"-----BEGIN"`, treat it as inline PEM; otherwise treat it as a file
/// path on disk.
fn load_pem(var: &'static str) -> Result<Vec<u8>, TlsError> {
    let value = env::var(var).map_err(|_| TlsError::Missing(var))?;
    if value.is_empty() {
        return Err(TlsError::Missing(var));
    }
    if value.contains("-----BEGIN") {
        Ok(value.into_bytes())
    } else {
        std::fs::read(Path::new(&value)).map_err(|err| TlsError::ReadFile { path: value, err })
    }
}

/// Build a [`ServerTlsConfig`] for the A2A gRPC server from the reserved
/// env vars. Requires `NEXUS_SERVER_CERT`, `NEXUS_SERVER_KEY`; if
/// `NEXUS_CA_CERT` is also set, enforces client-cert verification (mTLS).
pub fn load_server_config_from_env() -> Result<ServerTlsConfig, TlsError> {
    let cert = load_pem(ENV_SERVER_CERT)?;
    let key = load_pem(ENV_SERVER_KEY)?;
    let identity = Identity::from_pem(&cert, &key);

    let mut config = ServerTlsConfig::new().identity(identity);
    if let Ok(ca) = load_pem(ENV_CA_CERT) {
        let ca_cert = Certificate::from_pem(ca);
        config = config.client_ca_root(ca_cert);
    }
    Ok(config)
}

/// Build a [`ClientTlsConfig`] for the A2A gRPC client from the reserved
/// env vars. Requires `NEXUS_CA_CERT`; if `NEXUS_CLIENT_CERT` and
/// `NEXUS_CLIENT_KEY` are also set, presents a client identity (mTLS).
pub fn load_client_config_from_env() -> Result<ClientTlsConfig, TlsError> {
    let ca = load_pem(ENV_CA_CERT)?;
    let mut config = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
    let cert = load_pem(ENV_CLIENT_CERT).ok();
    let key = load_pem(ENV_CLIENT_KEY).ok();
    if let (Some(c), Some(k)) = (cert, key) {
        config = config.identity(Identity::from_pem(&c, &k));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INLINE_PEM: &str = "-----BEGIN CERTIFICATE-----\nABC\n-----END CERTIFICATE-----";

    /// `load_pem` returns `Missing` when the variable isn't set.
    #[test]
    fn load_pem_missing() {
        // Use a deterministic var name unlikely to be set by the host shell.
        let var = "NEXUS_TLS_TEST_NEVER_SET_42";
        std::env::remove_var(var);
        // SAFETY: we leak `var` so the &'static lifetime requirement is
        // satisfied for the test. Test-only.
        let leaked: &'static str = Box::leak(var.to_string().into_boxed_str());
        let err = load_pem(leaked).expect_err("must be missing");
        matches!(err, TlsError::Missing(_));
    }

    /// Inline PEM detection — when the value contains `-----BEGIN`,
    /// `load_pem` returns the value verbatim (no filesystem touch).
    #[test]
    fn load_pem_inline() {
        let var = "NEXUS_TLS_TEST_INLINE";
        std::env::set_var(var, INLINE_PEM);
        let leaked: &'static str = Box::leak(var.to_string().into_boxed_str());
        let bytes = load_pem(leaked).expect("inline pem");
        assert_eq!(bytes, INLINE_PEM.as_bytes());
        std::env::remove_var(var);
    }
}
