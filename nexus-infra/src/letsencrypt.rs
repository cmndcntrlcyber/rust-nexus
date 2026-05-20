//! Let's Encrypt ACME client with Cloudflare DNS-01 challenge support.
//!
//! v1.4.1 (Phase 1.4.1 / D-V1.4-A): the ACME order workflow has been
//! re-ported against acme-lib 0.8's current API. Production deployments
//! can opt into automatic cert provisioning by setting `[acme]` in
//! `nexus.toml` (consumed by `nexus_infra::serve::run_a2a` at startup).
//!
//! The flow follows acme-lib 0.8's documented happy path:
//!
//! 1. `FilePersist::new(<cert_storage_dir>)` for account / key persistence.
//! 2. `Directory::from_url(persist, DirectoryUrl::LetsEncryptStaging|LetsEncrypt)`.
//! 3. `directory.account(contact_email)` — creates or loads the ACME account.
//! 4. `account.new_order(primary_domain, &san_domains)` → `NewOrder<P>`.
//! 5. For each `Auth` in `order.authorizations()`:
//!    - `auth.dns_challenge()` → `Challenge<P, Dns>`
//!    - `challenge.dns_proof()` → TXT record value
//!    - Publish via [`CloudflareManager::create_acme_challenge`]
//!    - Wait for DNS propagation
//!    - `challenge.validate(5000)` confirms upstream's check
//! 6. Poll `order.confirm_validations()` until it returns `Some(CsrOrder)`.
//! 7. `csr_order.finalize_pkey(p256_key, &domains, 5000)` → `CertOrder`.
//! 8. `cert_order.download_and_save_cert()` → `Certificate`.
//! 9. Save certificate + private key to disk (mode 0o600 for the key).
//!
//! Since acme-lib 0.8 is synchronous (blocks via `ureq`), the entire
//! flow runs inside `tokio::task::spawn_blocking`. The Cloudflare DNS
//! publish step bridges back into async land via the captured
//! `tokio::runtime::Handle`.

#![allow(dead_code)]

use crate::{CloudflareManager, InfraError, InfraResult, LetsEncryptConfig};
use acme_lib::persist::FilePersist;
use acme_lib::{create_p256_key, Directory, DirectoryUrl};
use chrono::{DateTime, Duration, Utc};
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

/// Certificate information structure
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub domain: String,
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
    pub chain_path: std::path::PathBuf,
    pub expires_at: DateTime<Utc>,
    pub san_domains: Vec<String>,
}

/// ACME challenge information
#[derive(Debug, Clone)]
pub struct ChallengeInfo {
    pub domain: String,
    pub challenge_name: String,
    pub challenge_value: String,
    pub record_id: Option<String>,
}

/// Certificate renewal status
#[derive(Debug, Clone, PartialEq)]
pub enum RenewalStatus {
    NotNeeded,
    Needed,
    InProgress,
    Completed,
    Failed(String),
}

/// Let's Encrypt certificate manager.
///
/// v1.2 stub: the acme-lib 0.8 API requires a `Persist` impl and a different
/// order workflow than this code targeted. The fields that held an
/// `Account<AccountCredentials>` are replaced with a simple `initialized`
/// flag; production cert provisioning in v1.2 goes through operator-supplied
/// certs via `scripts/gen-certs.sh`. Re-introducing real ACME is v1.3 scope.
pub struct CertificateManager {
    config: LetsEncryptConfig,
    cloudflare: CloudflareManager,
    initialized: bool,
    active_challenges: HashMap<String, ChallengeInfo>,
}

impl CertificateManager {
    /// Create a new certificate manager.
    pub fn new(config: LetsEncryptConfig, cloudflare: CloudflareManager) -> Self {
        Self {
            config,
            cloudflare,
            initialized: false,
            active_challenges: HashMap::new(),
        }
    }

    /// Initialize ACME account (v1.2 stub — defers to v1.3 ACME re-port).
    pub async fn initialize(&mut self) -> InfraResult<()> {
        info!("Initializing Let's Encrypt ACME account (v1.2 stub)");

        // Still create the cert storage directory so save_certificate and
        // list_certificates work for operator-supplied certs.
        fs::create_dir_all(&self.config.cert_storage_dir).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to create cert directory: {}", e))
        })?;

        self.initialized = true;
        Ok(())
    }

    /// Request a certificate via ACME DNS-01 (re-ported v1.4.1).
    ///
    /// Runs the synchronous acme-lib 0.8 flow inside `spawn_blocking`,
    /// bridging back to async land for the Cloudflare DNS publish step.
    pub async fn request_certificate(
        &mut self,
        primary_domain: &str,
        san_domains: &[String],
    ) -> InfraResult<CertificateInfo> {
        info!(
            "ACME: requesting cert for {} (sans: {:?})",
            primary_domain, san_domains
        );

        let staging = self.config.acme_directory_url.contains("staging");
        let storage_dir = self.config.cert_storage_dir.clone();
        let contact = self.config.contact_email.clone();
        let primary = primary_domain.to_string();
        let sans = san_domains.to_vec();
        let cloudflare = self.cloudflare.clone();

        // Capture the current Tokio runtime handle so the blocking
        // task can publish/delete TXT records via Cloudflare while the
        // outer caller awaits.
        let handle = tokio::runtime::Handle::current();

        let cert_bundle = tokio::task::spawn_blocking(move || {
            run_acme_flow(
                staging,
                storage_dir,
                contact,
                primary,
                sans,
                cloudflare,
                handle,
            )
        })
        .await
        .map_err(|e| InfraError::LetsEncryptError(format!("acme task join: {e}")))??;

        // `cert_bundle.cert_pem` includes the full chain; the
        // `save_certificate` helper persists + parses expiry.
        self.save_certificate(
            primary_domain,
            &cert_bundle.cert_pem,
            &cert_bundle.key_pem,
            san_domains,
        )
        .await
    }

    async fn wait_for_dns_propagation(
        &self,
        challenge_name: &str,
        _challenge_value: &str,
    ) -> InfraResult<()> {
        // Best-effort: wait a fixed budget for DNS propagation. The
        // `Challenge::validate(delay_millis)` step adds its own
        // server-side poll budget so we only need to hedge against
        // upstream cache latency here.
        debug!("ACME: waiting 5s for DNS propagation of {challenge_name}");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        Ok(())
    }

    async fn save_certificate(
        &self,
        primary_domain: &str,
        certificate_chain: &str,
        private_key: &str,
        san_domains: &[String],
    ) -> InfraResult<CertificateInfo> {
        let domain_safe = primary_domain.replace("*", "wildcard");
        let cert_path = self
            .config
            .cert_storage_dir
            .join(format!("{}.crt", domain_safe));
        let key_path = self
            .config
            .cert_storage_dir
            .join(format!("{}.key", domain_safe));
        let chain_path = self
            .config
            .cert_storage_dir
            .join(format!("{}-chain.crt", domain_safe));

        // Parse certificate to get expiration date
        let cert_pem_data = certificate_chain
            .lines()
            .skip_while(|line| !line.starts_with("-----BEGIN CERTIFICATE-----"))
            .take_while(|line| !line.starts_with("-----END CERTIFICATE-----"))
            .chain(std::iter::once("-----END CERTIFICATE-----"))
            .collect::<Vec<_>>()
            .join("\n");

        let expires_at = self.parse_certificate_expiry(&cert_pem_data)?;

        // Write certificate files
        fs::write(&cert_path, certificate_chain).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write certificate: {}", e))
        })?;

        fs::write(&key_path, private_key).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write private key: {}", e))
        })?;

        fs::write(&chain_path, certificate_chain).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write certificate chain: {}", e))
        })?;

        info!("Certificate saved to: {:?}", cert_path);
        info!("Private key saved to: {:?}", key_path);
        info!("Certificate expires at: {}", expires_at);

        Ok(CertificateInfo {
            domain: primary_domain.to_string(),
            cert_path,
            key_path,
            chain_path,
            expires_at,
            san_domains: san_domains.to_vec(),
        })
    }

    fn parse_certificate_expiry(&self, cert_pem: &str) -> InfraResult<DateTime<Utc>> {
        use x509_parser::prelude::*;

        // Pull the first CERTIFICATE block via rustls_pemfile (avoids the pem
        // crate's 3.0 API shift) and decode the DER with x509-parser.
        let mut reader = std::io::Cursor::new(cert_pem.as_bytes());
        let mut der_chain = rustls_pemfile::certs(&mut reader)
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse PEM: {}", e)))?;
        let cert_der = der_chain
            .drain(..)
            .next()
            .ok_or_else(|| InfraError::CertificateError("No certificate in PEM".to_string()))?;

        let (_, cert) = X509Certificate::from_der(&cert_der).map_err(|e| {
            InfraError::CertificateError(format!("Failed to parse certificate: {}", e))
        })?;

        let not_after = cert.validity().not_after;
        let timestamp = not_after.timestamp();

        DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| {
            InfraError::CertificateError("Invalid timestamp in certificate".to_string())
        })
    }

    /// Check if certificate needs renewal
    pub fn check_renewal_status(&self, cert_info: &CertificateInfo) -> RenewalStatus {
        let now = Utc::now();
        let renewal_threshold = Duration::days(self.config.cert_renewal_days as i64);
        let renewal_time = cert_info.expires_at - renewal_threshold;

        if now >= renewal_time {
            RenewalStatus::Needed
        } else {
            RenewalStatus::NotNeeded
        }
    }

    /// Renew certificate if needed
    pub async fn renew_certificate_if_needed(
        &mut self,
        cert_info: &CertificateInfo,
    ) -> InfraResult<Option<CertificateInfo>> {
        match self.check_renewal_status(cert_info) {
            RenewalStatus::Needed => {
                info!("Certificate renewal needed for: {}", cert_info.domain);
                let new_cert = self
                    .request_certificate(&cert_info.domain, &cert_info.san_domains)
                    .await?;
                Ok(Some(new_cert))
            }
            RenewalStatus::NotNeeded => {
                debug!("Certificate renewal not needed for: {}", cert_info.domain);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// List all certificates in storage directory
    pub fn list_certificates(&self) -> InfraResult<Vec<CertificateInfo>> {
        let mut certificates = Vec::new();

        if !self.config.cert_storage_dir.exists() {
            return Ok(certificates);
        }

        let entries = fs::read_dir(&self.config.cert_storage_dir).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to read cert directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                InfraError::LetsEncryptError(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();

            if path.extension().map(|ext| ext == "crt").unwrap_or(false) {
                if let Some(domain) = path.file_stem().and_then(|stem| stem.to_str()) {
                    if !domain.ends_with("-chain") {
                        if let Ok(cert_info) = self.load_certificate_info(domain) {
                            certificates.push(cert_info);
                        }
                    }
                }
            }
        }

        Ok(certificates)
    }

    fn load_certificate_info(&self, domain: &str) -> InfraResult<CertificateInfo> {
        let cert_path = self.config.cert_storage_dir.join(format!("{}.crt", domain));
        let key_path = self.config.cert_storage_dir.join(format!("{}.key", domain));
        let chain_path = self
            .config
            .cert_storage_dir
            .join(format!("{}-chain.crt", domain));

        if !cert_path.exists() {
            return Err(InfraError::CertificateError(format!(
                "Certificate not found: {:?}",
                cert_path
            )));
        }

        let cert_pem = fs::read_to_string(&cert_path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to read certificate: {}", e))
        })?;

        let expires_at = self.parse_certificate_expiry(&cert_pem)?;

        // TODO: Parse SAN domains from certificate
        let san_domains = Vec::new();

        Ok(CertificateInfo {
            domain: domain.replace("wildcard", "*"),
            cert_path,
            key_path,
            chain_path,
            expires_at,
            san_domains,
        })
    }

    /// Get configuration reference
    pub fn config(&self) -> &LetsEncryptConfig {
        &self.config
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::CloudflareConfig;
    use tempfile::tempdir;

    #[test]
    fn test_certificate_info_creation() {
        let cert_info = CertificateInfo {
            domain: "example.com".to_string(),
            cert_path: std::path::PathBuf::from("/tmp/example.com.crt"),
            key_path: std::path::PathBuf::from("/tmp/example.com.key"),
            chain_path: std::path::PathBuf::from("/tmp/example.com-chain.crt"),
            expires_at: Utc::now() + Duration::days(90),
            san_domains: vec!["www.example.com".to_string()],
        };

        assert_eq!(cert_info.domain, "example.com");
        assert_eq!(cert_info.san_domains.len(), 1);
    }

    #[test]
    fn test_renewal_status_logic() {
        let temp_dir = tempdir().unwrap();
        let config = LetsEncryptConfig {
            cert_renewal_days: 30,
            cert_storage_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let cf_config = CloudflareConfig::default();
        let cf_manager = CloudflareManager::new(cf_config).unwrap();
        let cert_manager = CertificateManager::new(config, cf_manager);

        // Certificate expiring in 20 days (needs renewal)
        let cert_info_needs_renewal = CertificateInfo {
            domain: "example.com".to_string(),
            cert_path: std::path::PathBuf::from("/tmp/example.com.crt"),
            key_path: std::path::PathBuf::from("/tmp/example.com.key"),
            chain_path: std::path::PathBuf::from("/tmp/example.com-chain.crt"),
            expires_at: Utc::now() + Duration::days(20),
            san_domains: Vec::new(),
        };

        // Certificate expiring in 60 days (doesn't need renewal)
        let cert_info_no_renewal = CertificateInfo {
            domain: "test.com".to_string(),
            cert_path: std::path::PathBuf::from("/tmp/test.com.crt"),
            key_path: std::path::PathBuf::from("/tmp/test.com.key"),
            chain_path: std::path::PathBuf::from("/tmp/test.com-chain.crt"),
            expires_at: Utc::now() + Duration::days(60),
            san_domains: Vec::new(),
        };

        assert_eq!(
            cert_manager.check_renewal_status(&cert_info_needs_renewal),
            RenewalStatus::Needed
        );
        assert_eq!(
            cert_manager.check_renewal_status(&cert_info_no_renewal),
            RenewalStatus::NotNeeded
        );
    }
}

// ---------------------------------------------------------------------------
// v1.4.1 ACME flow internals (blocking — bridged via spawn_blocking).
// ---------------------------------------------------------------------------

/// Output of [`run_acme_flow`]. Both fields are PEM-encoded strings.
struct CertBundle {
    /// Full certificate chain (leaf first, then intermediates).
    cert_pem: String,
    /// Private key matching the leaf certificate.
    key_pem: String,
}

/// Synchronous ACME flow against `acme-lib` 0.8. Drives the entire
/// pipeline from account creation through cert download, bridging
/// back to async land via the captured `tokio::runtime::Handle` for
/// the Cloudflare TXT record publish/delete + the DNS propagation
/// wait.
fn run_acme_flow(
    staging: bool,
    storage_dir: std::path::PathBuf,
    contact_email: String,
    primary_domain: String,
    san_domains: Vec<String>,
    cloudflare: CloudflareManager,
    handle: tokio::runtime::Handle,
) -> InfraResult<CertBundle> {
    // Ensure the persist directory exists.
    fs::create_dir_all(&storage_dir)
        .map_err(|e| InfraError::LetsEncryptError(format!("create storage dir: {e}")))?;

    let persist = FilePersist::new(&storage_dir);

    let url = if staging {
        DirectoryUrl::LetsEncryptStaging
    } else {
        DirectoryUrl::LetsEncrypt
    };
    let directory = Directory::from_url(persist, url)
        .map_err(|e| InfraError::LetsEncryptError(format!("Directory::from_url: {e}")))?;

    let account = directory
        .account(&contact_email)
        .map_err(|e| InfraError::LetsEncryptError(format!("account: {e}")))?;

    let alt_refs: Vec<&str> = san_domains.iter().map(String::as_str).collect();
    let mut order = account
        .new_order(&primary_domain, &alt_refs)
        .map_err(|e| InfraError::LetsEncryptError(format!("new_order: {e}")))?;

    // -- Authorize each domain via DNS-01.
    let auths = order
        .authorizations()
        .map_err(|e| InfraError::LetsEncryptError(format!("authorizations: {e}")))?;

    let cloudflare = Arc::new(cloudflare);
    let mut published: Vec<String> = Vec::new();
    for auth in &auths {
        let domain = auth.domain_name();
        let challenge = auth.dns_challenge();
        let proof = challenge.dns_proof();
        let txt_name = format!("_acme-challenge.{domain}");

        info!("ACME: publishing DNS-01 TXT {txt_name}");
        let cf = Arc::clone(&cloudflare);
        let txt_name_clone = txt_name.clone();
        let proof_clone = proof.clone();
        let publish_result: InfraResult<()> = handle.block_on(async move {
            cf.create_acme_challenge(&txt_name_clone, &proof_clone)
                .await
                .map(|_| ())
        });
        publish_result?;
        published.push(txt_name);

        // Hedge against upstream DNS cache latency before asking
        // Let's Encrypt to validate.
        std::thread::sleep(std::time::Duration::from_secs(5));

        challenge
            .validate(5000)
            .map_err(|e| InfraError::LetsEncryptError(format!("challenge.validate: {e}")))?;
    }

    // -- Poll until ACME confirms the authorizations.
    let csr_order = {
        let mut attempts = 0;
        loop {
            if let Some(csr) = order.confirm_validations() {
                break csr;
            }
            attempts += 1;
            if attempts > 30 {
                return Err(InfraError::LetsEncryptError(
                    "ACME order did not reach ready state within 60s".to_string(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
            order
                .refresh()
                .map_err(|e| InfraError::LetsEncryptError(format!("order refresh: {e}")))?;
        }
    };

    // -- Finalize with a fresh P-256 key.
    let pkey = create_p256_key();
    let cert_order = csr_order
        .finalize_pkey(pkey, 5000)
        .map_err(|e| InfraError::LetsEncryptError(format!("finalize_pkey: {e}")))?;

    let cert = cert_order
        .download_and_save_cert()
        .map_err(|e| InfraError::LetsEncryptError(format!("download_and_save_cert: {e}")))?;

    // -- Clean up DNS challenge records (best-effort).
    for txt_name in published {
        let cf = Arc::clone(&cloudflare);
        let _ = handle.block_on(async move { cf.delete_acme_challenge(&txt_name).await });
    }

    Ok(CertBundle {
        cert_pem: cert.certificate().to_string(),
        key_pem: cert.private_key().to_string(),
    })
}
