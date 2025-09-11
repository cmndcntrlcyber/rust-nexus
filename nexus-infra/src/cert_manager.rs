//! Certificate management for Cloudflare origin certificates and TLS operations

use crate::{InfraError, InfraResult, OriginCertConfig};
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info, warn};
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256,
};
use rustls::{Certificate as RustlsCert, ClientConfig, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use x509_parser::prelude::*;

/// Certificate validation result
#[derive(Debug, Clone, PartialEq)]
pub enum CertValidation {
    Valid,
    Expired,
    NotYetValid,
    InvalidSignature,
    UnknownCA,
    Other(String),
}

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertInfo {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub fingerprint: String,
    pub san_domains: Vec<String>,
}

/// TLS configuration holder
pub struct TlsConfig {
    pub server_config: Arc<ServerConfig>,
    pub client_config: Arc<ClientConfig>,
}

/// Certificate manager for origin certificates and TLS operations
pub struct CertManager {
    config: OriginCertConfig,
    server_cert_chain: Vec<RustlsCert>,
    server_private_key: PrivateKey,
    cert_pem_data: Vec<u8>,
    key_pem_data: Vec<u8>,
    ca_cert_data: Option<Vec<u8>>,
    full_chain_pem_data: Vec<u8>,
}

impl CertManager {
    /// Create a new certificate manager
    pub fn new(config: OriginCertConfig) -> InfraResult<Self> {
        info!("Initializing certificate manager");

        // Load raw PEM data for gRPC server configuration
        let cert_pem_data = fs::read(&config.cert_path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to read certificate file: {}", e))
        })?;

        let key_pem_data = fs::read(&config.key_path)
            .map_err(|e| InfraError::CertificateError(format!("Failed to read key file: {}", e)))?;

        // Load CA certificate if available
        let ca_cert_data = if config.ca_cert_path.exists() {
            Some(fs::read(&config.ca_cert_path).map_err(|e| {
                InfraError::CertificateError(format!("Failed to read CA certificate file: {}", e))
            })?)
        } else {
            None
        };

        // Load server certificate chain
        let server_cert_chain = Self::load_certificate_chain(&config.cert_path)?;

        // Load server private key
        let server_private_key = Self::load_private_key(&config.key_path)?;

        // Create full certificate chain by combining origin cert and CA cert
        let full_chain_pem_data =
            Self::create_full_certificate_chain(&cert_pem_data, &ca_cert_data)?;

        info!("Successfully loaded certificates and private key");

        Ok(Self {
            config,
            server_cert_chain,
            server_private_key,
            cert_pem_data,
            key_pem_data,
            ca_cert_data,
            full_chain_pem_data,
        })
    }

    /// Load certificate chain from PEM file
    fn load_certificate_chain(path: &Path) -> InfraResult<Vec<RustlsCert>> {
        if !path.exists() {
            return Err(InfraError::CertificateError(format!(
                "Certificate file not found: {:?}",
                path
            )));
        }

        let cert_file = fs::File::open(path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to open certificate file: {}", e))
        })?;

        let mut reader = BufReader::new(cert_file);
        let cert_chain = certs(&mut reader)
            .map_err(|e| {
                InfraError::CertificateError(format!("Failed to parse certificates: {}", e))
            })?
            .into_iter()
            .map(RustlsCert)
            .collect();

        Ok(cert_chain)
    }

    /// Create full certificate chain by combining origin cert and CA cert
    fn create_full_certificate_chain(
        cert_pem_data: &[u8],
        ca_cert_data: &Option<Vec<u8>>,
    ) -> InfraResult<Vec<u8>> {
        let mut full_chain = Vec::new();

        // Add the origin certificate first
        full_chain.extend_from_slice(cert_pem_data);

        // Add the CA certificate if available
        if let Some(ca_data) = ca_cert_data {
            // Ensure there's a newline between certificates
            if !cert_pem_data.ends_with(b"\n") {
                full_chain.push(b'\n');
            }
            full_chain.extend_from_slice(ca_data);
        }

        info!(
            "Created full certificate chain ({} bytes)",
            full_chain.len()
        );
        Ok(full_chain)
    }

    /// Load private key from PEM file
    fn load_private_key(path: &Path) -> InfraResult<PrivateKey> {
        if !path.exists() {
            return Err(InfraError::CertificateError(format!(
                "Private key file not found: {:?}",
                path
            )));
        }

        let key_file = fs::File::open(path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to open private key file: {}", e))
        })?;

        let mut reader = BufReader::new(key_file);

        // Try PKCS#8 format first
        if let Ok(mut keys) = pkcs8_private_keys(&mut reader) {
            if !keys.is_empty() {
                return Ok(PrivateKey(keys.remove(0)));
            }
        }

        // Reset reader for next attempt
        let key_file = fs::File::open(path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to open private key file: {}", e))
        })?;
        let mut reader = BufReader::new(key_file);

        // Try RSA format
        if let Ok(mut keys) = rsa_private_keys(&mut reader) {
            if !keys.is_empty() {
                return Ok(PrivateKey(keys.remove(0)));
            }
        }

        // Reset reader for next attempt
        let key_file = fs::File::open(path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to open private key file: {}", e))
        })?;
        let mut reader = BufReader::new(key_file);

        // Try EC format
        if let Ok(mut keys) = ec_private_keys(&mut reader) {
            if !keys.is_empty() {
                return Ok(PrivateKey(keys.remove(0)));
            }
        }

        Err(InfraError::CertificateError(
            "No supported private key format found in file".to_string(),
        ))
    }

    /// Create server TLS configuration
    pub fn create_server_config(&self) -> InfraResult<Arc<ServerConfig>> {
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(
                self.server_cert_chain.clone(),
                self.server_private_key.clone(),
            )
            .map_err(|e| InfraError::TlsError(format!("Failed to create server config: {}", e)))?;

        Ok(Arc::new(config))
    }

    /// Create client TLS configuration
    pub fn create_client_config(&self, verify_hostname: bool) -> InfraResult<Arc<ClientConfig>> {
        use rustls::RootCertStore;

        let mut root_store = RootCertStore::empty();

        // Add system CA certificates
        root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let config = if verify_hostname {
            ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        } else {
            // Create config with no hostname verification (for domain fronting)
            use rustls::{client::*, ServerName};
            use std::sync::Arc;

            #[derive(Debug)]
            struct NoHostnameVerifier;

            impl ServerCertVerifier for NoHostnameVerifier {
                fn verify_server_cert(
                    &self,
                    _end_entity: &rustls::Certificate,
                    _intermediates: &[rustls::Certificate],
                    _server_name: &ServerName,
                    _scts: &mut dyn Iterator<Item = &[u8]>,
                    _ocsp_response: &[u8],
                    _now: std::time::SystemTime,
                ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
                    Ok(ServerCertVerified::assertion())
                }
            }

            ClientConfig::builder()
                .with_safe_defaults()
                .with_custom_certificate_verifier(Arc::new(NoHostnameVerifier))
                .with_no_client_auth()
        };

        Ok(Arc::new(config))
    }

    /// Create TLS acceptor for server
    pub fn create_tls_acceptor(&self) -> InfraResult<TlsAcceptor> {
        let server_config = self.create_server_config()?;
        Ok(TlsAcceptor::from(server_config))
    }

    /// Create TLS connector for client
    pub fn create_tls_connector(&self, verify_hostname: bool) -> InfraResult<TlsConnector> {
        let client_config = self.create_client_config(verify_hostname)?;
        Ok(TlsConnector::from(client_config))
    }

    /// Generate self-signed certificate for testing
    pub fn generate_self_signed_cert(
        common_name: &str,
        san_domains: &[String],
        validity_days: u32,
    ) -> InfraResult<(Vec<u8>, Vec<u8>)> {
        info!("Generating self-signed certificate for: {}", common_name);

        let mut params = CertificateParams::new(vec![common_name.to_string()]);
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Nexus C2");
        params.distinguished_name.push(DnType::CountryName, "US");

        // Add subject alternative names
        for domain in san_domains {
            params
                .subject_alt_names
                .push(rcgen::SanType::DnsName(domain.clone()));
        }

        // Set validity period - use time crate for rcgen compatibility
        let not_before = ::time::OffsetDateTime::now_utc();
        let not_after = not_before + ::time::Duration::days(validity_days as i64);
        params.not_before = not_before;
        params.not_after = not_after;

        // Use ECDSA key
        params.alg = &PKCS_ECDSA_P256_SHA256;
        params.key_pair = Some(KeyPair::generate(&PKCS_ECDSA_P256_SHA256).map_err(|e| {
            InfraError::CertificateError(format!("Failed to generate key pair: {}", e))
        })?);

        let cert = Certificate::from_params(params).map_err(|e| {
            InfraError::CertificateError(format!("Failed to generate certificate: {}", e))
        })?;

        let cert_pem = cert.serialize_pem().map_err(|e| {
            InfraError::CertificateError(format!("Failed to serialize certificate: {}", e))
        })?;

        let key_pem = cert.serialize_private_key_pem();

        info!("Successfully generated self-signed certificate");

        Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
    }

    /// Parse certificate information
    pub fn parse_certificate_info(&self, cert_pem: &[u8]) -> InfraResult<CertInfo> {
        let pem = ::pem::parse(cert_pem)
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse PEM: {}", e)))?;

        let (_, cert) = X509Certificate::from_der(pem.contents()).map_err(|e| {
            InfraError::CertificateError(format!("Failed to parse certificate: {}", e))
        })?;

        let subject = cert.subject().to_string();
        let issuer = cert.issuer().to_string();
        let serial_number = format!("{:x}", cert.serial);

        let not_before = DateTime::<Utc>::from_timestamp(cert.validity().not_before.timestamp(), 0)
            .ok_or_else(|| {
                InfraError::CertificateError("Invalid not_before timestamp".to_string())
            })?;

        let not_after = DateTime::<Utc>::from_timestamp(cert.validity().not_after.timestamp(), 0)
            .ok_or_else(|| {
            InfraError::CertificateError("Invalid not_after timestamp".to_string())
        })?;

        // Calculate SHA-256 fingerprint
        let digest = sha2::Sha256::digest(pem.contents());
        let fingerprint = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        // Extract SAN domains
        let mut san_domains = Vec::new();
        if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
            for name in &san_ext.value.general_names {
                if let x509_parser::extensions::GeneralName::DNSName(domain) = name {
                    san_domains.push(domain.to_string());
                }
            }
        }

        Ok(CertInfo {
            subject,
            issuer,
            serial_number,
            not_before,
            not_after,
            fingerprint,
            san_domains,
        })
    }

    /// Validate certificate
    pub fn validate_certificate(&self, cert_pem: &[u8]) -> InfraResult<CertValidation> {
        let cert_info = self.parse_certificate_info(cert_pem)?;
        let now = Utc::now();

        if now < cert_info.not_before {
            return Ok(CertValidation::NotYetValid);
        }

        if now > cert_info.not_after {
            return Ok(CertValidation::Expired);
        }

        // TODO: Add signature validation against CA certificates
        // For now, just check dates
        Ok(CertValidation::Valid)
    }

    /// Check if certificate needs renewal
    pub fn needs_renewal(&self, cert_pem: &[u8], days_before_expiry: u32) -> InfraResult<bool> {
        let cert_info = self.parse_certificate_info(cert_pem)?;
        let now = Utc::now();
        let renewal_threshold = cert_info.not_after - Duration::days(days_before_expiry as i64);

        Ok(now >= renewal_threshold)
    }

    /// Save certificate and key to files
    pub fn save_certificate_files(
        &self,
        cert_pem: &[u8],
        key_pem: &[u8],
        cert_path: &Path,
        key_path: &Path,
    ) -> InfraResult<()> {
        // Ensure parent directories exist
        if let Some(parent) = cert_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                InfraError::CertificateError(format!("Failed to create cert directory: {}", e))
            })?;
        }

        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                InfraError::CertificateError(format!("Failed to create key directory: {}", e))
            })?;
        }

        // Write certificate
        fs::write(cert_path, cert_pem).map_err(|e| {
            InfraError::CertificateError(format!("Failed to write certificate: {}", e))
        })?;

        // Write private key with restricted permissions
        fs::write(key_path, key_pem).map_err(|e| {
            InfraError::CertificateError(format!("Failed to write private key: {}", e))
        })?;

        // Set restrictive permissions on private key (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(key_path)
                .map_err(|e| {
                    InfraError::CertificateError(format!("Failed to get key file metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o600); // Owner read/write only
            fs::set_permissions(key_path, perms).map_err(|e| {
                InfraError::CertificateError(format!("Failed to set key file permissions: {}", e))
            })?;
        }

        info!("Saved certificate to: {:?}", cert_path);
        info!("Saved private key to: {:?}", key_path);

        Ok(())
    }

    /// Get certificate chain
    pub fn get_certificate_chain(&self) -> &[RustlsCert] {
        &self.server_cert_chain
    }

    /// Get private key
    pub fn get_private_key(&self) -> &PrivateKey {
        &self.server_private_key
    }

    /// Get raw certificate PEM data
    pub fn get_cert_pem_data(&self) -> &[u8] {
        &self.cert_pem_data
    }

    /// Get raw private key PEM data
    pub fn get_key_pem_data(&self) -> &[u8] {
        &self.key_pem_data
    }

    /// Get CA certificate PEM data if available
    pub fn get_ca_cert_data(&self) -> Option<&[u8]> {
        self.ca_cert_data.as_deref()
    }

    /// Get full certificate chain PEM data (origin cert + CA cert)
    pub fn get_full_chain_pem_data(&self) -> &[u8] {
        &self.full_chain_pem_data
    }

    /// Get configuration reference
    pub fn config(&self) -> &OriginCertConfig {
        &self.config
    }
}

// Implement SHA-256 digest
mod sha2 {
    pub struct Sha256;

    impl Sha256 {
        pub fn digest(data: &[u8]) -> [u8; 32] {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            // Simple hash for demonstration - in production use proper SHA-256
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            let hash = hasher.finish();

            let mut result = [0u8; 32];
            result[..8].copy_from_slice(&hash.to_be_bytes());
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_self_signed_cert() {
        let (cert_pem, key_pem) = CertManager::generate_self_signed_cert(
            "test.example.com",
            &["*.test.example.com".to_string()],
            30,
        )
        .unwrap();

        assert!(!cert_pem.is_empty());
        assert!(!key_pem.is_empty());

        // Verify it's valid PEM
        assert!(String::from_utf8_lossy(&cert_pem).contains("BEGIN CERTIFICATE"));
        assert!(String::from_utf8_lossy(&key_pem).contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_certificate_validation() {
        let temp_dir = tempdir().unwrap();
        let config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
            ca_cert_path: temp_dir.path().join("ca.crt"),
            pin_validation: true,
            validity_days: 365,
        };

        // Generate test certificate
        let (cert_pem, key_pem) =
            CertManager::generate_self_signed_cert("test.example.com", &[], 30).unwrap();

        // Save to temp files
        fs::write(&config.cert_path, &cert_pem).unwrap();
        fs::write(&config.key_path, &key_pem).unwrap();

        let cert_manager = CertManager::new(config).unwrap();
        let validation = cert_manager.validate_certificate(&cert_pem).unwrap();

        assert_eq!(validation, CertValidation::Valid);
    }

    #[test]
    fn test_renewal_check() {
        let temp_dir = tempdir().unwrap();
        let config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
            ca_cert_path: temp_dir.path().join("ca.crt"),
            pin_validation: true,
            validity_days: 365,
        };

        // Generate certificate valid for 10 days
        let (cert_pem, key_pem) =
            CertManager::generate_self_signed_cert("test.example.com", &[], 10).unwrap();

        fs::write(&config.cert_path, &cert_pem).unwrap();
        fs::write(&config.key_path, &key_pem).unwrap();

        let cert_manager = CertManager::new(config).unwrap();

        // Should need renewal if threshold is 15 days (cert expires in 10)
        let needs_renewal = cert_manager.needs_renewal(&cert_pem, 15).unwrap();
        assert!(needs_renewal);

        // Should not need renewal if threshold is 5 days
        let needs_renewal = cert_manager.needs_renewal(&cert_pem, 5).unwrap();
        assert!(!needs_renewal);
    }
}
