//! Let's Encrypt ACME client with Cloudflare DNS-01 challenge support

use crate::{InfraError, InfraResult, LetsEncryptConfig, CloudflareManager};
use acme_lib::{Account, Directory, DirectoryUrl, persist::FilePersist, create_p384_key};
use chrono::{DateTime, Utc, Duration};
use log::{info, warn, error, debug};
use rcgen::{CertificateParams, DistinguishedName, DnType, SanType};
use sha2::Digest;
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
    persist: Option<FilePersist>,
    active_challenges: HashMap<String, ChallengeInfo>,
}

impl CertificateManager {
    /// Create a new certificate manager
    pub fn new(config: LetsEncryptConfig, cloudflare: CloudflareManager) -> Self {
        Self {
            config,
            cloudflare,
            persist: None,
            active_challenges: HashMap::new(),
        }
    }

    /// Initialize ACME account or load existing one
    pub async fn initialize(&mut self) -> InfraResult<()> {
        info!("Initializing Let's Encrypt ACME account");

        // Ensure certificate storage directory exists
        fs::create_dir_all(&self.config.cert_storage_dir)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to create cert directory: {}", e)))?;

        // Create file-based persistence for ACME account
        let persist = FilePersist::new(&self.config.cert_storage_dir);
        self.persist = Some(persist);

        info!("Successfully initialized ACME persistence");
        Ok(())
    }

    /// Get the directory based on configuration
    fn get_directory(&self) -> InfraResult<Directory<FilePersist>> {
        let persist = self.persist.as_ref()
            .ok_or_else(|| InfraError::LetsEncryptError("ACME not initialized".to_string()))?
            .clone();

        let directory_url = if self.config.acme_directory_url.contains("staging") {
            DirectoryUrl::LetsEncryptStaging
        } else {
            DirectoryUrl::LetsEncrypt
        };

        Directory::from_url(persist, directory_url)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to create directory: {}", e)))
    }

    /// Get or create ACME account
    fn get_account(&self) -> InfraResult<Account<FilePersist>> {
        let directory = self.get_directory()?;

        directory.account(&self.config.contact_email)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to get/create account: {}", e)))
    }
    
    /// Request a certificate for the given domain(s)
    pub async fn request_certificate(&mut self, primary_domain: &str, san_domains: &[String]) -> InfraResult<CertificateInfo> {
        let account = self.get_account()?;

        info!("Requesting certificate for domain: {} with SANs: {:?}", primary_domain, san_domains);

        // Prepare domain list
        let mut all_domains = vec![primary_domain.to_string()];
        all_domains.extend(san_domains.iter().cloned());

        // Create certificate order
        let mut order = account.new_order(primary_domain, &all_domains.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to create order: {}", e)))?;

        // Process authorizations for each domain
        let authorizations = order.authorizations()
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to get authorizations: {}", e)))?;

        for auth in authorizations {
            let domain = auth.domain_name().to_string();
            info!("Processing authorization for domain: {}", domain);

            // Get DNS challenge
            let dns_challenge = auth.dns_challenge();

            // Create DNS record for challenge
            let challenge_name = format!("_acme-challenge.{}", domain);
            let challenge_value = dns_challenge.dns_proof();

            info!("Creating DNS challenge record: {} = {}", challenge_name, challenge_value);

            let dns_record = self.cloudflare.create_acme_challenge(&challenge_name, &challenge_value).await?;

            // Store challenge info for cleanup
            let challenge_info = ChallengeInfo {
                domain: domain.clone(),
                challenge_name: challenge_name.clone(),
                challenge_value: challenge_value.clone(),
                record_id: dns_record.id.clone(),
            };
            self.active_challenges.insert(domain.clone(), challenge_info);

            // Wait for DNS propagation
            info!("Waiting for DNS propagation...");
            self.wait_for_dns_propagation(&challenge_name, &challenge_value).await?;

            // Validate challenge
            info!("Validating DNS challenge for {}", domain);
            dns_challenge.validate(5000) // 5 second timeout
                .map_err(|e| InfraError::LetsEncryptError(format!("Challenge validation failed: {}", e)))?;
        }

        // Wait for order to be ready and finalize
        info!("Waiting for certificate order to be ready...");

        // Generate private key for CSR
        let pkey = create_p384_key();

        // Finalize the order with CSR
        let ord_cert = order.finalize_pkey(pkey, 5000)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to finalize order: {}", e)))?;

        // Download certificate
        let certificate_chain = ord_cert.download_and_save_cert()
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to download certificate: {}", e)))?;

        // Clean up DNS challenges
        for challenge_info in self.active_challenges.values() {
            if let Err(e) = self.cloudflare.delete_acme_challenge(&challenge_info.challenge_name).await {
                warn!("Failed to clean up challenge record {}: {}", challenge_info.challenge_name, e);
            }
        }
        self.active_challenges.clear();

        // Get the private key PEM
        let private_key_pem = ord_cert.private_key();

        // Save certificate and key to files
        let cert_info = self.save_certificate(
            primary_domain,
            &certificate_chain.certificate(),
            private_key_pem,
            san_domains,
        ).await?;

        info!("Successfully obtained certificate for {}", primary_domain);
        Ok(cert_info)
    }
    
    async fn wait_for_dns_propagation(&self, challenge_name: &str, challenge_value: &str) -> InfraResult<()> {
        info!("Waiting for DNS propagation of challenge record: {}", challenge_name);
        
        use hickory_resolver::{Resolver, config::*};
        
        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())
            .map_err(|e| InfraError::DnsError(format!("Failed to create DNS resolver: {}", e)))?;
        
        let mut attempts = 0;
        let max_attempts = 60; // 5 minutes with 5-second intervals
        
        while attempts < max_attempts {
            match timeout(TokioDuration::from_secs(5), async {
                resolver.txt_lookup(challenge_name)
            }).await {
                Ok(Ok(response)) => {
                    for record in response.iter() {
                        let record_value = record.txt_data().first()
                            .map(|data| String::from_utf8_lossy(data).to_string())
                            .unwrap_or_default();
                        
                        if record_value == challenge_value {
                            info!("DNS challenge record propagated successfully");
                            return Ok(());
                        }
                    }
                }
                _ => {
                    debug!("DNS propagation attempt {} failed, retrying...", attempts + 1);
                }
            }
            
            attempts += 1;
            sleep(TokioDuration::from_secs(5)).await;
        }
        
        warn!("DNS propagation verification failed after {} attempts, proceeding anyway", attempts);
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
        let cert_path = self.config.cert_storage_dir.join(format!("{}.crt", domain_safe));
        let key_path = self.config.cert_storage_dir.join(format!("{}.key", domain_safe));
        let chain_path = self.config.cert_storage_dir.join(format!("{}-chain.crt", domain_safe));
        
        // Parse certificate to get expiration date
        let cert_pem_data = certificate_chain.lines()
            .skip_while(|line| !line.starts_with("-----BEGIN CERTIFICATE-----"))
            .take_while(|line| !line.starts_with("-----END CERTIFICATE-----"))
            .chain(std::iter::once("-----END CERTIFICATE-----"))
            .collect::<Vec<_>>()
            .join("\n");
        
        let expires_at = self.parse_certificate_expiry(&cert_pem_data)?;
        
        // Write certificate files
        fs::write(&cert_path, certificate_chain)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to write certificate: {}", e)))?;
        
        fs::write(&key_path, private_key)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to write private key: {}", e)))?;
        
        fs::write(&chain_path, certificate_chain)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to write certificate chain: {}", e)))?;
        
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

        let pem = pem::parse(cert_pem)
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse PEM: {}", e)))?;

        let cert = X509Certificate::from_der(pem.contents())
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse certificate: {}", e)))?
            .1;

        let not_after = cert.validity().not_after;
        let timestamp = not_after.timestamp();

        Ok(DateTime::<Utc>::from_timestamp(timestamp, 0)
            .ok_or_else(|| InfraError::CertificateError("Invalid timestamp in certificate".to_string()))?)
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
    pub async fn renew_certificate_if_needed(&mut self, cert_info: &CertificateInfo) -> InfraResult<Option<CertificateInfo>> {
        match self.check_renewal_status(cert_info) {
            RenewalStatus::Needed => {
                info!("Certificate renewal needed for: {}", cert_info.domain);
                let new_cert = self.request_certificate(&cert_info.domain, &cert_info.san_domains).await?;
                Ok(Some(new_cert))
            }
            RenewalStatus::NotNeeded => {
                debug!("Certificate renewal not needed for: {}", cert_info.domain);
                Ok(None)
            }
            _ => Ok(None)
        }
    }
    
    /// List all certificates in storage directory
    pub fn list_certificates(&self) -> InfraResult<Vec<CertificateInfo>> {
        let mut certificates = Vec::new();
        
        if !self.config.cert_storage_dir.exists() {
            return Ok(certificates);
        }
        
        let entries = fs::read_dir(&self.config.cert_storage_dir)
            .map_err(|e| InfraError::LetsEncryptError(format!("Failed to read cert directory: {}", e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| InfraError::LetsEncryptError(format!("Failed to read directory entry: {}", e)))?;
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
        let chain_path = self.config.cert_storage_dir.join(format!("{}-chain.crt", domain));
        
        if !cert_path.exists() {
            return Err(InfraError::CertificateError(format!("Certificate not found: {:?}", cert_path)));
        }
        
        let cert_pem = fs::read_to_string(&cert_path)
            .map_err(|e| InfraError::CertificateError(format!("Failed to read certificate: {}", e)))?;
        
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
    use crate::{CloudflareConfig};
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
        
        assert_eq!(cert_manager.check_renewal_status(&cert_info_needs_renewal), RenewalStatus::Needed);
        assert_eq!(cert_manager.check_renewal_status(&cert_info_no_renewal), RenewalStatus::NotNeeded);
    }
}
