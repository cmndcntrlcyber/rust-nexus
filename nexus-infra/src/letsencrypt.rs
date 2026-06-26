//! Let's Encrypt ACME client with Cloudflare DNS-01 challenge support.
//!
//! v1.5 — migrated from `acme-lib` 0.8 (unmaintained since 2021) to
//! `instant-acme` 0.8 (async, pure-Rust, actively maintained). The
//! synchronous `spawn_blocking` bridge is no longer needed.

// ACME flow is opt-in; some methods are only called when [acme] config is present.

use crate::{CloudflareManager, InfraError, InfraResult, LetsEncryptConfig};
use chrono::{DateTime, Duration, Utc};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus, RetryPolicy,
};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tracing::{debug, info};

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
pub struct CertificateManager {
    config: LetsEncryptConfig,
    cloudflare: CloudflareManager,
    initialized: bool,
    #[allow(dead_code)] // populated during ACME flow, read during cleanup
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

    /// Initialize ACME account.
    pub async fn initialize(&mut self) -> InfraResult<()> {
        info!("Initializing Let's Encrypt ACME account");

        fs::create_dir_all(&self.config.cert_storage_dir).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to create cert directory: {}", e))
        })?;

        self.initialized = true;
        Ok(())
    }

    /// Request a certificate via ACME DNS-01.
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
        let server_url = if staging {
            LetsEncrypt::Staging.url()
        } else {
            LetsEncrypt::Production.url()
        };

        let contact = format!("mailto:{}", self.config.contact_email);
        let (account, _credentials) = Account::builder()
            .map_err(|e| InfraError::LetsEncryptError(format!("Account::builder: {e}")))?
            .create(
                &NewAccount {
                    contact: &[&contact],
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                server_url.to_owned(),
                None,
            )
            .await
            .map_err(|e| InfraError::LetsEncryptError(format!("Account::create: {e}")))?;

        let mut identifiers = vec![Identifier::Dns(primary_domain.to_string())];
        for san in san_domains {
            identifiers.push(Identifier::Dns(san.clone()));
        }

        let mut order = account
            .new_order(&NewOrder::new(&identifiers))
            .await
            .map_err(|e| InfraError::LetsEncryptError(format!("new_order: {e}")))?;

        let cloudflare = Arc::new(self.cloudflare.clone());
        let mut published: Vec<String> = Vec::new();

        let mut authorizations = order.authorizations();
        while let Some(result) = authorizations.next().await {
            let mut authz = result
                .map_err(|e| InfraError::LetsEncryptError(format!("authorization: {e}")))?;

            if matches!(authz.status, AuthorizationStatus::Valid) {
                continue;
            }

            let mut challenge = authz
                .challenge(ChallengeType::Dns01)
                .ok_or_else(|| {
                    InfraError::LetsEncryptError("no DNS-01 challenge found".to_string())
                })?;

            let proof = challenge.key_authorization().dns_value();
            let ident = challenge.identifier().to_string();
            let txt_name = format!("_acme-challenge.{ident}");

            info!("ACME: publishing DNS-01 TXT {txt_name}");
            cloudflare
                .create_acme_challenge(&txt_name, &proof)
                .await
                .map_err(|e| {
                    InfraError::LetsEncryptError(format!("publish TXT {txt_name}: {e}"))
                })?;
            published.push(txt_name.clone());

            debug!("ACME: waiting 5s for DNS propagation of {txt_name}");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            challenge
                .set_ready()
                .await
                .map_err(|e| InfraError::LetsEncryptError(format!("set_ready: {e}")))?;
        }

        let status = order
            .poll_ready(&RetryPolicy::default())
            .await
            .map_err(|e| InfraError::LetsEncryptError(format!("poll_ready: {e}")))?;

        if status != OrderStatus::Ready {
            return Err(InfraError::LetsEncryptError(format!(
                "unexpected order status: {status:?}"
            )));
        }

        let private_key_pem = order
            .finalize()
            .await
            .map_err(|e| InfraError::LetsEncryptError(format!("finalize: {e}")))?;

        let cert_chain_pem = order
            .poll_certificate(&RetryPolicy::default())
            .await
            .map_err(|e| InfraError::LetsEncryptError(format!("poll_certificate: {e}")))?;

        // Clean up DNS challenge records (best-effort).
        for txt_name in published {
            let _ = cloudflare.delete_acme_challenge(&txt_name).await;
        }

        self.save_certificate(primary_domain, &cert_chain_pem, &private_key_pem, san_domains)
            .await
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

        let cert_pem_data = certificate_chain
            .lines()
            .skip_while(|line| !line.starts_with("-----BEGIN CERTIFICATE-----"))
            .take_while(|line| !line.starts_with("-----END CERTIFICATE-----"))
            .chain(std::iter::once("-----END CERTIFICATE-----"))
            .collect::<Vec<_>>()
            .join("\n");

        let expires_at = self.parse_certificate_expiry(&cert_pem_data)?;

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

        let cert_info_needs_renewal = CertificateInfo {
            domain: "example.com".to_string(),
            cert_path: std::path::PathBuf::from("/tmp/example.com.crt"),
            key_path: std::path::PathBuf::from("/tmp/example.com.key"),
            chain_path: std::path::PathBuf::from("/tmp/example.com-chain.crt"),
            expires_at: Utc::now() + Duration::days(20),
            san_domains: Vec::new(),
        };

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
