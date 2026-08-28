//! Compile-time cert embedding with runtime memory-only extraction
//! (ECHOTRIBBLE P3 — WS6).
//!
//! When built with the `embedded-certs` feature **and** `NEXUS_EMBED_CERTS=1`
//! is set at compile time, PEM certificate material is baked into the binary
//! via `include_bytes!()`.  At runtime the bytes are copied into zeroing heap
//! allocations ([`SecretVec<u8>`]) so the material lives only in process
//! memory and is automatically wiped on [`Drop`].  No cert file paths appear
//! in log output on the embedded path.
//!
//! Non-embedded builds (the default) compile without cert files present —
//! [`EmbeddedCerts::from_embedded`] returns empty fields and
//! [`EmbeddedCerts::tls_config`] returns `None`, falling through to the
//! env-var cert resolution path.

use secrecy::{ExposeSecret, SecretVec};

// ---------------------------------------------------------------------------
// Static byte arrays emitted by `build.rs` when `NEXUS_EMBED_CERTS=1`.
// The cfg gate is set by build.rs, independent of the Cargo feature, so that
// the feature can be enabled without requiring cert files to be present
// (build.rs only emits the cfg when it actually finds/copies the files).
// ---------------------------------------------------------------------------
#[cfg(embedded_certs)]
mod statics {
    /// CA certificate (PEM).
    pub static CA_PEM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_ca.pem"));
    /// Agent certificate (PEM).
    pub static CERT_PEM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_cert.pem"));
    /// Agent private key (PEM).
    pub static KEY_PEM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_key.pem"));
}

/// Container for embedded certificate material.
///
/// Each field wraps PEM bytes in a [`SecretVec`] that automatically zeroes
/// its heap allocation on [`Drop`].  The struct can also be explicitly
/// zeroed via [`zeroize_all`](Self::zeroize_all) during self-destruct.
pub struct EmbeddedCerts {
    /// CA certificate PEM (zeroing allocation).
    pub ca: Option<SecretVec<u8>>,
    /// Agent certificate PEM (zeroing allocation).
    pub cert: Option<SecretVec<u8>>,
    /// Agent private key PEM (zeroing allocation).
    pub key: Option<SecretVec<u8>>,
}

impl EmbeddedCerts {
    /// Copy compile-time embedded cert material into zeroing heap buffers.
    ///
    /// When compiled **without** the `embedded_certs` cfg (i.e. certs were not
    /// baked in at build time), all fields are `None`.
    pub fn from_embedded() -> Self {
        #[cfg(embedded_certs)]
        {
            Self {
                ca: Some(SecretVec::new(statics::CA_PEM.to_vec())),
                cert: Some(SecretVec::new(statics::CERT_PEM.to_vec())),
                key: Some(SecretVec::new(statics::KEY_PEM.to_vec())),
            }
        }
        #[cfg(not(embedded_certs))]
        {
            Self {
                ca: None,
                cert: None,
                key: None,
            }
        }
    }

    /// Build a [`tonic_14::transport::ClientTlsConfig`] from the embedded
    /// material without any disk I/O.
    ///
    /// Returns `None` when no CA certificate is available (certs were not
    /// embedded at compile time).
    pub fn tls_config(&self) -> Option<tonic_14::transport::ClientTlsConfig> {
        use tonic_14::transport::{Certificate, ClientTlsConfig, Identity};

        let ca = self.ca.as_ref()?;
        let mut config =
            ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca.expose_secret()));

        if let (Some(cert), Some(key)) = (self.cert.as_ref(), self.key.as_ref()) {
            config = config.identity(Identity::from_pem(
                cert.expose_secret(),
                key.expose_secret(),
            ));
        }

        Some(config)
    }

    /// Explicitly zero all cert buffers by dropping the [`SecretVec`]
    /// allocations.  Called during self-destruct (step 0) before identity
    /// file removal.
    pub fn zeroize_all(&mut self) {
        self.ca = None;
        self.cert = None;
        self.key = None;
    }
}
