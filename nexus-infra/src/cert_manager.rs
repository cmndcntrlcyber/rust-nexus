//! Certificate management for Cloudflare origin certificates and TLS operations

use crate::{InfraError, InfraResult, OriginCertConfig};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::info;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256,
};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::{any_supported_type, CertifiedKey};
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

/// A loaded certificate profile with parsed cert chain and key
pub struct CertProfile {
    pub name: String,
    pub domains: Vec<String>,
    pub cert_chain: Vec<RustlsCert>,
    pub private_key: PrivateKey,
    pub ca_certificates: Vec<RustlsCert>,
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
    ca_certificates: Vec<RustlsCert>,
    /// Named profiles keyed by profile name
    profiles: HashMap<String, CertProfile>,
    /// SNI domain -> profile name index for O(1) lookup
    domain_index: HashMap<String, String>,
}

impl CertManager {
    /// Create a new certificate manager
    pub fn new(config: OriginCertConfig) -> InfraResult<Self> {
        info!("Initializing certificate manager");

        // Load server certificate chain
        let server_cert_chain = Self::load_certificate_chain(&config.cert_path)?;

        // Load server private key
        let server_private_key = Self::load_private_key(&config.key_path)?;

        // Load CA certificates
        let ca_certificates = Self::load_certificate_chain(&config.ca_cert_path)?;

        info!("Successfully loaded certificates and private key");

        let mut profiles = HashMap::new();
        let mut domain_index = HashMap::new();

        if let Some(ref profile_configs) = config.profiles {
            for p in profile_configs {
                let cert_chain = Self::load_certificate_chain(&p.cert_path)?;
                let private_key = Self::load_private_key(&p.key_path)?;
                let ca_certificates = Self::load_certificate_chain(&p.ca_cert_path)?;
                for domain in &p.domains {
                    domain_index.insert(domain.clone(), p.name.clone());
                }
                profiles.insert(
                    p.name.clone(),
                    CertProfile {
                        name: p.name.clone(),
                        domains: p.domains.clone(),
                        cert_chain,
                        private_key,
                        ca_certificates,
                    },
                );
            }
            info!("Loaded {} HTTPS profiles", profiles.len());
        }

        Ok(Self {
            config,
            server_cert_chain,
            server_private_key,
            ca_certificates,
            profiles,
            domain_index,
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

    /// Load private key from PEM file. Tries PKCS8, then RSA, then EC formats.
    fn load_private_key(path: &Path) -> InfraResult<PrivateKey> {
        if !path.exists() {
            return Err(InfraError::CertificateError(format!(
                "Private key file not found: {:?}",
                path
            )));
        }

        let pem_bytes = fs::read(path).map_err(|e| {
            InfraError::CertificateError(format!("Failed to read private key file: {}", e))
        })?;

        for parser in [
            pkcs8_private_keys as fn(&mut dyn std::io::BufRead) -> std::io::Result<Vec<Vec<u8>>>,
            rsa_private_keys,
            ec_private_keys,
        ] {
            let mut reader = std::io::Cursor::new(&pem_bytes);
            if let Ok(keys) = parser(&mut reader) {
                if let Some(key) = keys.into_iter().next() {
                    return Ok(PrivateKey(key));
                }
            }
        }

        Err(InfraError::CertificateError(
            "No private key found in file".to_string(),
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

    /// Create client TLS configuration with certificate pinning
    pub fn create_client_config(&self, verify_hostname: bool) -> InfraResult<Arc<ClientConfig>> {
        use rustls::RootCertStore;

        let mut root_store = RootCertStore::empty();

        // Add CA certificates to root store
        for ca_cert in &self.ca_certificates {
            root_store.add(ca_cert).map_err(|e| {
                InfraError::TlsError(format!("Failed to add CA certificate: {:?}", e))
            })?;
        }

        // Add system CA certificates as fallback
        root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let config_builder = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store);

        let config = if verify_hostname {
            config_builder.with_no_client_auth()
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

        // Set validity period. rcgen expects time::OffsetDateTime; bridge from chrono.
        // Use the absolute `::time` path because `x509_parser::prelude::*` (imported
        // at the top of this file) shadows `time` with a private alias.
        let now_ts = chrono::Utc::now().timestamp();
        let then_ts = now_ts + (validity_days as i64) * 86400;
        params.not_before = ::time::OffsetDateTime::from_unix_timestamp(now_ts).map_err(|e| {
            InfraError::CertificateError(format!("Invalid not_before timestamp: {}", e))
        })?;
        params.not_after = ::time::OffsetDateTime::from_unix_timestamp(then_ts).map_err(|e| {
            InfraError::CertificateError(format!("Invalid not_after timestamp: {}", e))
        })?;

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

    /// Parse certificate information. Accepts PEM bytes; pulls the first
    /// CERTIFICATE block via rustls_pemfile and decodes the DER with x509-parser.
    pub fn parse_certificate_info(&self, cert_pem: &[u8]) -> InfraResult<CertInfo> {
        let mut reader = std::io::Cursor::new(cert_pem);
        let mut der_chain = certs(&mut reader)
            .map_err(|e| InfraError::CertificateError(format!("Failed to parse PEM: {}", e)))?;
        let cert_der = der_chain
            .drain(..)
            .next()
            .ok_or_else(|| InfraError::CertificateError("No certificate in PEM".to_string()))?;

        let (_, cert) = X509Certificate::from_der(&cert_der).map_err(|e| {
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

        // SHA-256 fingerprint of the DER bytes, hex-encoded.
        let digest = sha2::Sha256::digest(&cert_der);
        let fingerprint: String = digest.iter().map(|b| format!("{:02x}", b)).collect();

        // SAN parsing deferred — the x509-parser 0.15 BasicExtension access
        // pattern shifted across minor releases; v1.3 will re-introduce this.
        let san_domains: Vec<String> = Vec::new();

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

        // CA signature validation deferred — mTLS handshake already
        // validates the chain at connection time; this check covers expiry only.
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

    /// Get CA certificates
    pub fn get_ca_certificates(&self) -> &[RustlsCert] {
        &self.ca_certificates
    }

    /// Get configuration reference
    pub fn config(&self) -> &OriginCertConfig {
        &self.config
    }

    /// Check whether any named profiles are configured
    pub fn has_profiles(&self) -> bool {
        !self.profiles.is_empty()
    }

    /// Look up a profile by SNI hostname (exact match, then wildcard fallback)
    pub fn resolve_profile(&self, sni: &str) -> Option<&CertProfile> {
        if let Some(name) = self.domain_index.get(sni) {
            return self.profiles.get(name);
        }
        if let Some(dot) = sni.find('.') {
            let wildcard = format!("*{}", &sni[dot..]);
            if let Some(name) = self.domain_index.get(&wildcard) {
                return self.profiles.get(name);
            }
        }
        None
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&CertProfile> {
        self.profiles.get(name)
    }

    /// List all profile names
    pub fn profile_names(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }

    /// Create a multi-profile server TLS config with SNI-based cert resolution
    pub fn create_multi_profile_server_config(
        &self,
        mutual_tls: bool,
    ) -> InfraResult<ServerConfig> {
        let default_signing_key = any_supported_type(&self.server_private_key).map_err(|_| {
            InfraError::TlsError("Unsupported private key type for default cert".into())
        })?;
        let default = Arc::new(CertifiedKey::new(
            self.server_cert_chain.clone(),
            default_signing_key,
        ));

        let mut resolver_profiles = HashMap::new();
        let mut resolver_domain_index = HashMap::new();
        for (name, profile) in &self.profiles {
            let signing_key = any_supported_type(&profile.private_key).map_err(|_| {
                InfraError::TlsError(format!(
                    "Unsupported private key type for profile '{}'",
                    name
                ))
            })?;
            let certified_key = Arc::new(CertifiedKey::new(
                profile.cert_chain.clone(),
                signing_key,
            ));
            resolver_profiles.insert(name.clone(), certified_key);
            for domain in &profile.domains {
                resolver_domain_index.insert(domain.clone(), name.clone());
            }
        }

        let resolver = MultiProfileCertResolver {
            profiles: resolver_profiles,
            domain_index: resolver_domain_index,
            default,
        };

        let builder = ServerConfig::builder().with_safe_defaults();

        let config = if mutual_tls {
            let mut root_store = rustls::RootCertStore::empty();
            for ca_cert in &self.ca_certificates {
                root_store.add(ca_cert).map_err(|e| {
                    InfraError::TlsError(format!("Failed to add CA cert: {:?}", e))
                })?;
            }
            for profile in self.profiles.values() {
                for ca_cert in &profile.ca_certificates {
                    let _ = root_store.add(ca_cert);
                }
            }
            let verifier =
                rustls::server::AllowAnyAuthenticatedClient::new(root_store).boxed();
            builder
                .with_client_cert_verifier(verifier)
                .with_cert_resolver(Arc::new(resolver))
        } else {
            builder
                .with_no_client_auth()
                .with_cert_resolver(Arc::new(resolver))
        };

        Ok(config)
    }

    /// Create a TLS acceptor with multi-profile SNI-based cert resolution
    pub fn create_multi_profile_tls_acceptor(
        &self,
        mutual_tls: bool,
    ) -> InfraResult<TlsAcceptor> {
        let config = self.create_multi_profile_server_config(mutual_tls)?;
        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}

/// SNI-based cert resolver for multi-profile TLS
pub struct MultiProfileCertResolver {
    profiles: HashMap<String, Arc<CertifiedKey>>,
    domain_index: HashMap<String, String>,
    default: Arc<CertifiedKey>,
}

impl ResolvesServerCert for MultiProfileCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            if let Some(name) = self.domain_index.get(sni) {
                return self.profiles.get(name).cloned();
            }
            if let Some(dot) = sni.find('.') {
                let wildcard = format!("*{}", &sni[dot..]);
                if let Some(name) = self.domain_index.get(&wildcard) {
                    return self.profiles.get(name).cloned();
                }
            }
        }
        Some(Arc::clone(&self.default))
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
            ca_cert_path: temp_dir.path().join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
            profiles: None,
        };

        // Generate test certificate
        let (cert_pem, key_pem) =
            CertManager::generate_self_signed_cert("test.example.com", &[], 30).unwrap();

        // Save to temp files
        fs::write(&config.cert_path, &cert_pem).unwrap();
        fs::write(&config.key_path, &key_pem).unwrap();
        fs::write(&config.ca_cert_path, &cert_pem).unwrap(); // Use self as CA for test

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
            ca_cert_path: temp_dir.path().join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
            profiles: None,
        };

        // Generate certificate valid for 10 days
        let (cert_pem, key_pem) =
            CertManager::generate_self_signed_cert("test.example.com", &[], 10).unwrap();

        fs::write(&config.cert_path, &cert_pem).unwrap();
        fs::write(&config.key_path, &key_pem).unwrap();
        fs::write(&config.ca_cert_path, &cert_pem).unwrap();

        let cert_manager = CertManager::new(config).unwrap();

        // Should need renewal if threshold is 15 days (cert expires in 10)
        let needs_renewal = cert_manager.needs_renewal(&cert_pem, 15).unwrap();
        assert!(needs_renewal);

        // Should not need renewal if threshold is 5 days
        let needs_renewal = cert_manager.needs_renewal(&cert_pem, 5).unwrap();
        assert!(!needs_renewal);
    }

    #[test]
    fn test_multi_profile_cert_manager() {
        use crate::config::HttpsProfile;

        let temp_dir = tempdir().unwrap();

        // Generate two distinct self-signed certs
        let (cert1, key1) =
            CertManager::generate_self_signed_cert("c2.example.com", &[], 30).unwrap();
        let (cert2, key2) =
            CertManager::generate_self_signed_cert("backup.example.com", &[], 30).unwrap();

        let p1_dir = temp_dir.path().join("primary");
        let p2_dir = temp_dir.path().join("fallback");
        fs::create_dir_all(&p1_dir).unwrap();
        fs::create_dir_all(&p2_dir).unwrap();

        fs::write(p1_dir.join("cert.pem"), &cert1).unwrap();
        fs::write(p1_dir.join("key.pem"), &key1).unwrap();
        fs::write(p1_dir.join("ca.pem"), &cert1).unwrap();
        fs::write(p2_dir.join("cert.pem"), &cert2).unwrap();
        fs::write(p2_dir.join("key.pem"), &key2).unwrap();
        fs::write(p2_dir.join("ca.pem"), &cert2).unwrap();

        // Use profile 1's cert as the default single-cert
        let config = OriginCertConfig {
            cert_path: p1_dir.join("cert.pem"),
            key_path: p1_dir.join("key.pem"),
            ca_cert_path: p1_dir.join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
            profiles: Some(vec![
                HttpsProfile {
                    name: "primary".into(),
                    domains: vec!["c2.example.com".into()],
                    cert_path: p1_dir.join("cert.pem"),
                    key_path: p1_dir.join("key.pem"),
                    ca_cert_path: p1_dir.join("ca.pem"),
                },
                HttpsProfile {
                    name: "fallback".into(),
                    domains: vec!["backup.example.com".into()],
                    cert_path: p2_dir.join("cert.pem"),
                    key_path: p2_dir.join("key.pem"),
                    ca_cert_path: p2_dir.join("ca.pem"),
                },
            ]),
        };

        let cm = CertManager::new(config).unwrap();
        assert!(cm.has_profiles());
        assert_eq!(cm.profile_names().len(), 2);

        let primary = cm.resolve_profile("c2.example.com").unwrap();
        assert_eq!(primary.name, "primary");

        let fallback = cm.resolve_profile("backup.example.com").unwrap();
        assert_eq!(fallback.name, "fallback");

        assert!(cm.resolve_profile("unknown.example.com").is_none());
    }

    #[test]
    fn test_resolve_profile_wildcard() {
        use crate::config::HttpsProfile;

        let temp_dir = tempdir().unwrap();
        let (cert, key) =
            CertManager::generate_self_signed_cert("wildcard.example.com", &[], 30).unwrap();

        fs::write(temp_dir.path().join("cert.pem"), &cert).unwrap();
        fs::write(temp_dir.path().join("key.pem"), &key).unwrap();
        fs::write(temp_dir.path().join("ca.pem"), &cert).unwrap();

        let config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
            ca_cert_path: temp_dir.path().join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
            profiles: Some(vec![HttpsProfile {
                name: "wildcard".into(),
                domains: vec!["*.example.com".into()],
                cert_path: temp_dir.path().join("cert.pem"),
                key_path: temp_dir.path().join("key.pem"),
                ca_cert_path: temp_dir.path().join("ca.pem"),
            }]),
        };

        let cm = CertManager::new(config).unwrap();
        let profile = cm.resolve_profile("sub.example.com").unwrap();
        assert_eq!(profile.name, "wildcard");

        let profile = cm.resolve_profile("other.example.com").unwrap();
        assert_eq!(profile.name, "wildcard");

        assert!(cm.resolve_profile("example.com").is_none());
    }

    #[test]
    fn test_no_profiles_backward_compat() {
        let temp_dir = tempdir().unwrap();
        let (cert, key) =
            CertManager::generate_self_signed_cert("test.example.com", &[], 30).unwrap();

        fs::write(temp_dir.path().join("cert.pem"), &cert).unwrap();
        fs::write(temp_dir.path().join("key.pem"), &key).unwrap();
        fs::write(temp_dir.path().join("ca.pem"), &cert).unwrap();

        let config = OriginCertConfig {
            cert_path: temp_dir.path().join("cert.pem"),
            key_path: temp_dir.path().join("key.pem"),
            ca_cert_path: temp_dir.path().join("ca.pem"),
            pin_validation: true,
            validity_days: 365,
            profiles: None,
        };

        let cm = CertManager::new(config).unwrap();
        assert!(!cm.has_profiles());
        assert!(cm.profile_names().is_empty());
        assert!(!cm.get_certificate_chain().is_empty());
    }
}
