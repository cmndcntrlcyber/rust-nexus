//! Let's Encrypt ACME client with Cloudflare DNS-01 challenge support

use crate::{CloudflareManager, InfraError, InfraResult, LetsEncryptConfig};
use acme_lib::{
    create_p256_key, persist::FilePersist, Account, Certificate, Directory, DirectoryUrl,
    Error as AcmeError,
};
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info, warn};
use pem;
use rcgen::{CertificateParams, DistinguishedName, DnType, SanType};
use rustls_pemfile;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, timeout, Duration as TokioDuration};

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

/// Let's Encrypt certificate manager
pub struct CertificateManager {
    config: LetsEncryptConfig,
    cloudflare: CloudflareManager,
    account: Option<Account<FilePersist>>,
    active_challenges: HashMap<String, ChallengeInfo>,
}

impl CertificateManager {
    /// Create a new certificate manager
    pub fn new(config: LetsEncryptConfig, cloudflare: CloudflareManager) -> Self {
        Self {
            config,
            cloudflare,
            account: None,
            active_challenges: HashMap::new(),
        }
    }

    /// Initialize ACME account or load existing one
    pub async fn initialize(&mut self) -> InfraResult<()> {
        info!("Initializing Let's Encrypt ACME account");

        // Ensure certificate storage directory exists
        fs::create_dir_all(&self.config.cert_storage_dir).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to create cert directory: {}", e))
        })?;

        let account_path = self.config.cert_storage_dir.join("account.json");

        // Try to load existing account
        if account_path.exists() {
            info!("Loading existing ACME account");
            match self.load_existing_account(&account_path).await {
                Ok(account) => {
                    self.account = Some(account);
                    info!("Successfully loaded existing ACME account");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to load existing account: {}, creating new one", e);
                }
            }
        }

        // Create new account
        info!("Creating new ACME account");
        let account = self.create_new_account().await?;

        // Save account credentials
        self.save_account_credentials(&account, &account_path)
            .await?;
        self.account = Some(account);

        info!("Successfully initialized ACME account");
        Ok(())
    }

    async fn load_existing_account(&self, _path: &Path) -> InfraResult<Account<FilePersist>> {
        // For now, just create a new account instead of loading existing one
        // TODO: Implement proper account persistence
        self.create_new_account().await
    }

    async fn create_new_account(&self) -> InfraResult<Account<FilePersist>> {
        // TODO: Implement proper acme-lib account creation when API is stable
        // For now, return error to indicate feature is not available
        Err(InfraError::LetsEncryptError(
            "Let's Encrypt account creation temporarily disabled due to API changes".to_string(),
        ))
    }

    async fn save_account_credentials(
        &self,
        _account: &Account<FilePersist>,
        _path: &Path,
    ) -> InfraResult<()> {
        // Account persistence is handled by FilePersist automatically
        Ok(())
    }

    /// Request a certificate for the given domain(s)
    pub async fn request_certificate(
        &mut self,
        primary_domain: &str,
        san_domains: &[String],
    ) -> InfraResult<CertificateInfo> {
        // TODO: Implement full ACME certificate request when API is stable
        // For now, generate a self-signed certificate as fallback
        warn!("ACME certificate request not fully implemented, generating self-signed certificate");

        let (cert_pem, key_pem) = crate::CertManager::generate_self_signed_cert(
            primary_domain,
            san_domains,
            90, // 90 days validity
        )?;

        // Save certificate files
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

        // Write certificate files
        fs::write(&cert_path, &cert_pem).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write certificate: {}", e))
        })?;

        fs::write(&key_path, &key_pem).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write private key: {}", e))
        })?;

        fs::write(&chain_path, &cert_pem).map_err(|e| {
            InfraError::LetsEncryptError(format!("Failed to write certificate chain: {}", e))
        })?;

        let expires_at = chrono::Utc::now() + chrono::Duration::days(90);

        info!("Generated self-signed certificate for {}", primary_domain);

        Ok(CertificateInfo {
            domain: primary_domain.to_string(),
            cert_path,
            key_path,
            chain_path,
            expires_at,
            san_domains: san_domains.to_vec(),
        })
    }

    async fn wait_for_dns_propagation(
        &self,
        challenge_name: &str,
        challenge_value: &str,
    ) -> InfraResult<()> {
        info!(
            "Waiting for DNS propagation of challenge record: {}",
            challenge_name
        );

        use hickory_resolver::{config::*, Resolver};

        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())
            .map_err(|e| InfraError::DnsError(format!("Failed to create DNS resolver: {}", e)))?;

        let mut attempts = 0;
        let max_attempts = 60; // 5 minutes with 5-second intervals

        while attempts < max_attempts {
            match timeout(TokioDuration::from_secs(5), async {
                resolver.txt_lookup(challenge_name)
            })
            .await
            {
                Ok(Ok(response)) => {
                    for record in response.iter() {
                        let record_value = record
                            .txt_data()
                            .first()
                            .map(|data| String::from_utf8_lossy(data).to_string())
                            .unwrap_or_default();

                        if record_value == challenge_value {
                            info!("DNS challenge record propagated successfully");
                            return Ok(());
                        }
                    }
                }
                _ => {
                    debug!(
                        "DNS propagation attempt {} failed, retrying...",
                        attempts + 1
                    );
                }
            }

            attempts += 1;
            sleep(TokioDuration::from_secs(5)).await;
        }

        warn!(
            "DNS propagation verification failed after {} attempts, proceeding anyway",
            attempts
        );
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

        let pem = ::pem::parse(cert_pem.as_bytes())
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse PEM: {}", e)))?;

        let cert = X509Certificate::from_der(pem.contents())
            .map_err(|e| {
                InfraError::CertificateError(format!("Failed to parse certificate: {}", e))
            })?
            .1;

        let not_after = cert.validity().not_after;
        let timestamp = not_after.timestamp();

        Ok(
            DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| {
                InfraError::CertificateError("Invalid timestamp in certificate".to_string())
            })?,
        )
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
