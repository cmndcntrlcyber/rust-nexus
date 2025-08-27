# Certificate Management Deep Dive

Comprehensive guide to certificate management in Rust-Nexus, covering Let's Encrypt automation, Cloudflare origin certificates, and TLS security best practices.

## Certificate Architecture

```
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│   Let's Encrypt     │    │  Cloudflare Origin  │    │     Client Certs   │
│                     │    │                     │    │                     │
│ • DNS-01 Challenge  │    │ • Backend Security  │    │ • Agent mTLS        │
│ • Wildcard Support  │    │ • 15-year validity  │    │ • Certificate Auth  │
│ • Auto-Renewal      │    │ • ECC/RSA Support   │    │ • Identity Validation│
│ • Public Trust      │    │ • Easy Management   │    │ • Access Control    │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
           │                          │                          │
           ▼                          ▼                          ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         TLS Security Layer                              │
│                                                                         │
│ • End-to-End Encryption    • Certificate Pinning    • Perfect Forward  │
│ • Multiple Certificate     • Automatic Rotation     • Secrecy Support  │
│ • Validation Layers       • Transparency Logs      • Strong Ciphers    │
└─────────────────────────────────────────────────────────────────────────┘
```

## Let's Encrypt Integration

### Automated Certificate Provisioning

The Rust-Nexus certificate manager provides fully automated Let's Encrypt certificate provisioning using DNS-01 challenges through Cloudflare.

#### Configuration
```toml
[letsencrypt]
# ACME account details
contact_email = "admin@yourdomain.com"
challenge_type = "Dns01"  # DNS-01 preferred for security

# Certificate management
cert_renewal_days = 30     # Renew 30 days before expiry
wildcard_enabled = true    # Enable *.yourdomain.com certificates

# ACME directory
acme_directory_url = "https://acme-v02.api.letsencrypt.org/directory"
# Use staging for testing: "https://acme-staging-v02.api.letsencrypt.org/directory"

# Storage
cert_storage_dir = "/opt/nexus/certs"
```

#### Certificate Request Process
```rust
use nexus_infra::{CertificateManager, LetsEncryptConfig, CloudflareManager};

async fn provision_certificates() -> InfraResult<()> {
    // Initialize managers
    let cf_manager = CloudflareManager::new(cloudflare_config)?;
    let le_config = LetsEncryptConfig::default();
    let mut cert_manager = CertificateManager::new(le_config, cf_manager);
    
    // Initialize ACME account
    cert_manager.initialize().await?;
    
    // Request wildcard certificate
    let cert_info = cert_manager.request_certificate(
        "*.yourdomain.com",
        &["yourdomain.com", "api.yourdomain.com"]
    ).await?;
    
    println!("Certificate provisioned: {}", cert_info.domain);
    println!("Expires: {}", cert_info.expires_at);
    println!("Certificate path: {:?}", cert_info.cert_path);
    println!("Private key path: {:?}", cert_info.key_path);
    
    Ok(())
}
```

### DNS-01 Challenge Process

The DNS-01 challenge workflow:

1. **ACME Order Creation**: Create certificate order with Let's Encrypt
2. **Challenge Generation**: Receive DNS challenge from ACME server
3. **TXT Record Creation**: Create `_acme-challenge` TXT record via Cloudflare API
4. **DNS Propagation**: Wait for global DNS propagation
5. **Challenge Validation**: Let's Encrypt validates DNS record
6. **Certificate Issuance**: Receive signed certificate
7. **Record Cleanup**: Remove temporary TXT record

```rust
// Detailed challenge handling
async fn handle_dns_challenge(
    cert_manager: &mut CertificateManager,
    domain: &str,
) -> InfraResult<()> {
    info!("Starting DNS-01 challenge for: {}", domain);
    
    // The certificate manager handles the full workflow:
    let cert_info = cert_manager.request_certificate(domain, &[]).await?;
    
    // Certificate is ready for use
    info!("Certificate ready: {:?}", cert_info.cert_path);
    
    Ok(())
}
```

### Automatic Renewal

```rust
// Set up automatic certificate renewal
use tokio::time::{interval, Duration};

async fn certificate_renewal_service(
    mut cert_manager: CertificateManager,
) -> InfraResult<()> {
    let mut renewal_check = interval(Duration::from_hours(12)); // Check twice daily
    
    loop {
        renewal_check.tick().await;
        
        info!("Checking certificate renewal status");
        
        // Get all certificates
        let certificates = cert_manager.list_certificates()?;
        
        for cert_info in certificates {
            // Check if renewal is needed
            let status = cert_manager.check_renewal_status(&cert_info);
            
            if status == RenewalStatus::Needed {
                info!("Renewing certificate: {}", cert_info.domain);
                
                match cert_manager.renew_certificate_if_needed(&cert_info).await? {
                    Some(new_cert) => {
                        info!("Certificate renewed successfully: {}", new_cert.domain);
                        
                        // Notify other components of certificate update
                        notify_certificate_update(&new_cert).await?;
                    }
                    None => {
                        debug!("Certificate renewal not needed: {}", cert_info.domain);
                    }
                }
            }
        }
    }
}

async fn notify_certificate_update(cert_info: &CertificateInfo) -> InfraResult<()> {
    // Reload gRPC server with new certificates
    // Update agent configurations
    // Send notifications to monitoring systems
    Ok(())
}
```

## Cloudflare Origin Certificates

### Origin Certificate Benefits
- **Extended Validity**: 15-year certificate lifetime
- **Backend Security**: Encrypt traffic between Cloudflare and your server
- **Easy Management**: No ACME challenges required
- **High Performance**: ECC certificates for better performance

### Generating Origin Certificates

#### Via Cloudflare Dashboard
1. **Navigate to SSL/TLS → Origin Server**
2. **Click "Create Certificate"**
3. **Configure Certificate**:
   ```
   Key Type: ECDSA (recommended) or RSA
   Hostnames: *.yourdomain.com, yourdomain.com
   Certificate Validity: 15 years
   ```
4. **Download Certificate and Key**

#### Via API
```bash
# Generate origin certificate via API
curl -X POST "https://api.cloudflare.com/client/v4/certificates" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "hostnames": ["*.yourdomain.com", "yourdomain.com"],
       "requested_validity": 5475,
       "request_type": "origin-ecc"
     }'
```

### Origin Certificate Integration

```rust
// Use origin certificates in Rust-Nexus
use nexus_infra::{CertManager, OriginCertConfig};

let origin_config = OriginCertConfig {
    cert_path: PathBuf::from("/opt/nexus/certs/origin.crt"),
    key_path: PathBuf::from("/opt/nexus/certs/origin.key"),
    ca_cert_path: PathBuf::from("/opt/nexus/certs/origin_ca.crt"),
    pin_validation: true,
    validity_days: 5475, // 15 years
};

let cert_manager = CertManager::new(origin_config)?;

// Create TLS configuration for gRPC server
let tls_acceptor = cert_manager.create_tls_acceptor()?;

// Use in gRPC server
let server = tonic::transport::Server::builder()
    .tls_config(tls_config)?
    .serve(addr);
```

## Certificate Validation and Pinning

### Certificate Pinning Implementation

```rust
// Comprehensive certificate pinning
use sha2::{Sha256, Digest};

pub struct CertificatePinner {
    pinned_fingerprints: HashMap<String, Vec<String>>, // domain -> fingerprints
    backup_fingerprints: HashMap<String, Vec<String>>, // emergency pins
}

impl CertificatePinner {
    pub fn new() -> Self {
        let mut pinner = Self {
            pinned_fingerprints: HashMap::new(),
            backup_fingerprints: HashMap::new(),
        };
        
        // Load known good fingerprints
        pinner.load_pinned_certificates();
        pinner
    }
    
    fn load_pinned_certificates(&mut self) {
        // Primary certificate fingerprints
        self.pinned_fingerprints.insert(
            "c2.yourdomain.com".to_string(),
            vec![
                "sha256:1234567890abcdef...".to_string(), // Current cert
                "sha256:fedcba0987654321...".to_string(), // Backup cert
            ]
        );
        
        // Emergency backup fingerprints (for certificate rotation)
        self.backup_fingerprints.insert(
            "c2.yourdomain.com".to_string(),
            vec![
                "sha256:abcdef1234567890...".to_string(),
            ]
        );
    }
    
    pub fn validate_certificate(&self, domain: &str, cert_der: &[u8]) -> InfraResult<bool> {
        // Calculate certificate fingerprint
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        let fingerprint = format!("sha256:{:x}", hasher.finalize());
        
        // Check against pinned fingerprints
        if let Some(pins) = self.pinned_fingerprints.get(domain) {
            if pins.contains(&fingerprint) {
                return Ok(true);
            }
        }
        
        // Check backup fingerprints
        if let Some(backup_pins) = self.backup_fingerprints.get(domain) {
            if backup_pins.contains(&fingerprint) {
                warn!("Using backup certificate pin for: {}", domain);
                return Ok(true);
            }
        }
        
        error!("Certificate pinning validation failed for: {}", domain);
        Ok(false)
    }
    
    pub fn update_pins(&mut self, domain: &str, new_pins: Vec<String>) {
        self.pinned_fingerprints.insert(domain.to_string(), new_pins);
    }
}
```

### Certificate Transparency Monitoring

```rust
// Monitor certificate transparency logs
use reqwest::Client;

pub struct CertTransparencyMonitor {
    client: Client,
    monitored_domains: Vec<String>,
}

impl CertTransparencyMonitor {
    pub async fn check_for_unauthorized_certificates(&self) -> InfraResult<Vec<String>> {
        let mut unauthorized_certs = Vec::new();
        
        for domain in &self.monitored_domains {
            // Query crt.sh for new certificates
            let url = format!("https://crt.sh/?q={}&output=json", domain);
            let response = self.client.get(&url).send().await?;
            
            if response.status().is_success() {
                let certs: Vec<serde_json::Value> = response.json().await?;
                
                // Check for certificates issued in the last 24 hours
                let yesterday = chrono::Utc::now() - chrono::Duration::hours(24);
                
                for cert in certs {
                    if let Some(not_before) = cert.get("not_before") {
                        let issued_date = chrono::DateTime::parse_from_rfc3339(not_before.as_str().unwrap_or(""))?;
                        
                        if issued_date.with_timezone(&chrono::Utc) > yesterday {
                            // Check if this is an expected certificate
                            if !self.is_expected_certificate(&cert) {
                                unauthorized_certs.push(format!("{}: {}", domain, cert["id"]));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(unauthorized_certs)
    }
    
    fn is_expected_certificate(&self, cert: &serde_json::Value) -> bool {
        // Check against known certificate issuers and patterns
        if let Some(issuer) = cert.get("issuer_name") {
            let issuer_str = issuer.as_str().unwrap_or("");
            
            // Expected issuers
            return issuer_str.contains("Let's Encrypt") || 
                   issuer_str.contains("Cloudflare Origin");
        }
        
        false
    }
}
```

## TLS Configuration

### Strong TLS Settings

```toml
# Production TLS configuration
[tls]
min_version = "TLS1.3"
max_version = "TLS1.3"

# Cipher suite preferences (TLS 1.3)
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_CHACHA20_POLY1305_SHA256",
    "TLS_AES_128_GCM_SHA256"
]

# Key exchange preferences
key_exchange = ["X25519", "secp256r1"]

# Certificate preferences
[tls.certificates]
signature_algorithms = ["ecdsa_secp256r1_sha256", "rsa_pss_rsae_sha256"]
key_types = ["ecdsa", "rsa"]
min_key_size = 256  # For ECDSA, 2048 for RSA
```

### Rust TLS Implementation

```rust
// Advanced TLS configuration
use rustls::{ServerConfig, ClientConfig, Certificate, PrivateKey};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub struct AdvancedTlsConfig {
    server_config: Arc<ServerConfig>,
    client_config: Arc<ClientConfig>,
}

impl AdvancedTlsConfig {
    pub fn new(
        cert_chain: Vec<Certificate>,
        private_key: PrivateKey,
        ca_certs: Vec<Certificate>,
    ) -> InfraResult<Self> {
        // Server configuration with strong security
        let server_config = ServerConfig::builder()
            .with_cipher_suites(&[
                rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
            ])
            .with_kx_groups(&[
                &rustls::kx_group::X25519,
                &rustls::kx_group::SECP256R1,
            ])
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| InfraError::TlsError(format!("Server config error: {}", e)))?
            .with_no_client_auth()
            .with_single_cert(cert_chain.clone(), private_key.clone())
            .map_err(|e| InfraError::TlsError(format!("Server cert error: {}", e)))?;
        
        // Client configuration with certificate verification
        let mut root_store = rustls::RootCertStore::empty();
        for ca_cert in ca_certs {
            root_store.add(&ca_cert)
                .map_err(|e| InfraError::TlsError(format!("CA cert error: {:?}", e)))?;
        }
        
        let client_config = ClientConfig::builder()
            .with_cipher_suites(&[
                rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
            ])
            .with_kx_groups(&[
                &rustls::kx_group::X25519,
                &rustls::kx_group::SECP256R1,
            ])
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| InfraError::TlsError(format!("Client config error: {}", e)))?
            .with_root_certificates(root_store)
            .with_no_client_auth();
        
        Ok(Self {
            server_config: Arc::new(server_config),
            client_config: Arc::new(client_config),
        })
    }
}
```

## Certificate Lifecycle Management

### Renewal Automation

```rust
// Comprehensive renewal management
pub struct CertificateLifecycleManager {
    cert_manager: CertificateManager,
    renewal_schedule: HashMap<String, chrono::DateTime<chrono::Utc>>,
    notification_handlers: Vec<Box<dyn Fn(&str) -> InfraResult<()>>>,
}

impl CertificateLifecycleManager {
    pub async fn start_renewal_service(&mut self) -> InfraResult<()> {
        let mut check_interval = tokio::time::interval(Duration::from_hours(6));
        
        loop {
            check_interval.tick().await;
            
            let certificates = self.cert_manager.list_certificates()?;
            
            for cert_info in certificates {
                if self.should_renew(&cert_info) {
                    match self.renew_certificate(&cert_info).await {
                        Ok(new_cert) => {
                            self.handle_successful_renewal(&new_cert).await?;
                        }
                        Err(e) => {
                            self.handle_renewal_failure(&cert_info.domain, e).await?;
                        }
                    }
                }
            }
        }
    }
    
    fn should_renew(&self, cert_info: &CertificateInfo) -> bool {
        let status = self.cert_manager.check_renewal_status(cert_info);
        matches!(status, RenewalStatus::Needed)
    }
    
    async fn handle_successful_renewal(&self, cert_info: &CertificateInfo) -> InfraResult<()> {
        info!("Certificate renewed successfully: {}", cert_info.domain);
        
        // Update gRPC server configuration
        self.reload_server_certificates(cert_info).await?;
        
        // Update agent configurations
        self.update_agent_certificates(cert_info).await?;
        
        // Send success notification
        self.send_notification(&format!("Certificate renewed: {}", cert_info.domain))?;
        
        Ok(())
    }
    
    async fn handle_renewal_failure(&self, domain: &str, error: InfraError) -> InfraResult<()> {
        error!("Certificate renewal failed for {}: {}", domain, error);
        
        // Send alert notification
        self.send_alert(&format!("Certificate renewal failed: {} - {}", domain, error))?;
        
        // Schedule retry with backoff
        self.schedule_retry(domain).await?;
        
        Ok(())
    }
}
```

### Certificate Validation

```rust
// Comprehensive certificate validation
use x509_parser::prelude::*;

pub struct CertificateValidator;

impl CertificateValidator {
    pub fn validate_certificate_chain(cert_chain: &[u8]) -> InfraResult<ValidationResult> {
        let (_, cert) = X509Certificate::from_der(cert_chain)
            .map_err(|e| InfraError::CertificateError(format!("Parse error: {}", e)))?;
        
        let mut validation = ValidationResult::new();
        
        // Check certificate validity period
        let now = chrono::Utc::now().timestamp();
        let not_before = cert.validity().not_before.timestamp();
        let not_after = cert.validity().not_after.timestamp();
        
        if now < not_before {
            validation.add_error("Certificate not yet valid");
        }
        
        if now > not_after {
            validation.add_error("Certificate expired");
        }
        
        // Check key usage
        if let Ok(Some(key_usage)) = cert.key_usage() {
            if !key_usage.digital_signature() {
                validation.add_warning("Digital signature not enabled");
            }
            
            if !key_usage.key_encipherment() {
                validation.add_warning("Key encipherment not enabled");
            }
        }
        
        // Check extended key usage
        if let Ok(Some(ext_key_usage)) = cert.extended_key_usage() {
            if !ext_key_usage.server_auth {
                validation.add_error("Server authentication not enabled");
            }
        }
        
        // Check subject alternative names
        if let Ok(Some(san)) = cert.subject_alternative_name() {
            validation.san_domains = san.general_names.iter()
                .filter_map(|gn| match gn {
                    x509_parser::extensions::GeneralName::DNSName(name) => Some(name.to_string()),
                    _ => None,
                })
                .collect();
        }
        
        // Check certificate chain (if multiple certificates provided)
        // Implementation would validate the full chain up to root CA
        
        Ok(validation)
    }
    
    pub fn check_certificate_strength(cert: &X509Certificate) -> SecurityLevel {
        // Analyze key size and algorithm
        let public_key = cert.public_key();
        
        match public_key.algorithm.algorithm {
            // RSA key analysis
            _ if public_key.algorithm.algorithm == x509_parser::oid_registry::OID_PKCS1_RSAENCRYPTION => {
                let key_size = Self::get_rsa_key_size(&public_key);
                match key_size {
                    size if size >= 4096 => SecurityLevel::High,
                    size if size >= 2048 => SecurityLevel::Medium,
                    _ => SecurityLevel::Low,
                }
            }
            
            // ECDSA key analysis
            _ if public_key.algorithm.algorithm == x509_parser::oid_registry::OID_EC_PUBLICKEY => {
                SecurityLevel::High // ECDSA generally considered strong
            }
            
            _ => SecurityLevel::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub san_domains: Vec<String>,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Unknown,
}
```

## Certificate Storage and Security

### Secure Certificate Storage

```rust
// Secure certificate storage implementation
use std::os::unix::fs::PermissionsExt;

pub struct SecureCertificateStorage {
    storage_dir: PathBuf,
    encryption_key: [u8; 32],
}

impl SecureCertificateStorage {
    pub fn new(storage_dir: PathBuf) -> InfraResult<Self> {
        // Ensure storage directory exists with proper permissions
        std::fs::create_dir_all(&storage_dir)?;
        
        // Set restrictive permissions (700 - owner only)
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&storage_dir)?.permissions();
            perms.set_mode(0o700);
            std::fs::set_permissions(&storage_dir, perms)?;
        }
        
        // Generate or load encryption key for certificate storage
        let encryption_key = Self::get_or_create_storage_key(&storage_dir)?;
        
        Ok(Self {
            storage_dir,
            encryption_key,
        })
    }
    
    pub fn store_certificate(&self, domain: &str, cert_pem: &[u8], key_pem: &[u8]) -> InfraResult<()> {
        let domain_safe = domain.replace("*", "wildcard").replace("/", "_");
        
        // Encrypt certificate and key
        let encrypted_cert = self.encrypt_data(cert_pem)?;
        let encrypted_key = self.encrypt_data(key_pem)?;
        
        // Store with secure permissions
        let cert_path = self.storage_dir.join(format!("{}.crt.enc", domain_safe));
        let key_path = self.storage_dir.join(format!("{}.key.enc", domain_safe));
        
        std::fs::write(&cert_path, encrypted_cert)?;
        std::fs::write(&key_path, encrypted_key)?;
        
        // Set restrictive permissions
        #[cfg(unix)]
        {
            let mut cert_perms = std::fs::metadata(&cert_path)?.permissions();
            cert_perms.set_mode(0o600);
            std::fs::set_permissions(&cert_path, cert_perms)?;
            
            let mut key_perms = std::fs::metadata(&key_path)?.permissions();
            key_perms.set_mode(0o600);
            std::fs::set_permissions(&key_path, key_perms)?;
        }
        
        info!("Certificate stored securely: {}", domain);
        Ok(())
    }
    
    fn encrypt_data(&self, data: &[u8]) -> InfraResult<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce, AeadInPlace};
        
        let key = Key::<Aes256Gcm>::from_slice(&self.encryption_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&rand::random::<[u8; 12]>());
        
        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(nonce, b"", &mut buffer)
            .map_err(|e| InfraError::CryptographicError(format!("Encryption failed: {}", e)))?;
        
        // Prepend nonce and tag
        let mut result = nonce.to_vec();
        result.extend_from_slice(&tag);
        result.extend_from_slice(&buffer);
        
        Ok(result)
    }
}
```

## Certificate Backup and Recovery

### Automated Backup

```bash
#!/bin/bash
# Certificate backup script

BACKUP_DIR="/opt/nexus/backups/certificates/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup certificate files
cp -r /opt/nexus/certs/* "$BACKUP_DIR/"

# Create backup manifest
cat > "$BACKUP_DIR/manifest.json" << EOF
{
    "backup_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "certificate_count": $(find /opt/nexus/certs -name "*.crt" | wc -l),
    "key_count": $(find /opt/nexus/certs -name "*.key" | wc -l),
    "backup_type": "automated",
    "server_hostname": "$(hostname)"
}
EOF

# Encrypt backup for storage
tar czf "$BACKUP_DIR.tar.gz" "$BACKUP_DIR"
gpg --symmetric --cipher-algo AES256 "$BACKUP_DIR.tar.gz"

# Upload to secure storage
aws s3 cp "$BACKUP_DIR.tar.gz.gpg" "s3://nexus-backups/certificates/"

# Clean up local backup
rm -rf "$BACKUP_DIR" "$BACKUP_DIR.tar.gz"

echo "Certificate backup completed: $BACKUP_DIR.tar.gz.gpg"
```

### Disaster Recovery

```bash
#!/bin/bash
# Certificate recovery script

BACKUP_FILE="$1"
RECOVERY_DIR="/opt/nexus/certs-recovery"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup-file.tar.gz.gpg>"
    exit 1
fi

# Download backup from storage
aws s3 cp "s3://nexus-backups/certificates/$BACKUP_FILE" ./

# Decrypt and extract
gpg --decrypt "$BACKUP_FILE" > "${BACKUP_FILE%.gpg}"
mkdir -p "$RECOVERY_DIR"
tar xzf "${BACKUP_FILE%.gpg}" -C "$RECOVERY_DIR" --strip-components=1

# Validate recovered certificates
for cert_file in "$RECOVERY_DIR"/*.crt; do
    if ! openssl x509 -in "$cert_file" -noout -text > /dev/null 2>&1; then
        echo "ERROR: Invalid certificate: $cert_file"
        exit 1
    fi
done

# Replace current certificates
systemctl stop nexus-server
cp "$RECOVERY_DIR"/* /opt/nexus/certs/
chown -R nexus:nexus /opt/nexus/certs/
chmod 600 /opt/nexus/certs/*.key
chmod 644 /opt/nexus/certs/*.crt
systemctl start nexus-server

echo "Certificate recovery completed"
```

## Monitoring and Alerting

### Certificate Expiration Monitoring

```rust
// Automated certificate expiration monitoring
use chrono::{DateTime, Utc, Duration};

pub struct CertificateMonitor {
    warning_days: i64,
    critical_days: i64,
    notification_channels: Vec<NotificationChannel>,
}

impl CertificateMonitor {
    pub async fn monitor_certificates(&self, cert_manager: &CertificateManager) -> InfraResult<()> {
        let certificates = cert_manager.list_certificates()?;
        
        for cert_info in certificates {
            let days_until_expiry = (cert_info.expires_at - Utc::now()).num_days();
            
            match days_until_expiry {
                days if days <= self.critical_days => {
                    self.send_critical_alert(&cert_info, days).await?;
                }
                days if days <= self.warning_days => {
                    self.send_warning_alert(&cert_info, days).await?;
                }
                _ => {
                    // Certificate is healthy
                }
            }
        }
        
        Ok(())
    }
    
    async fn send_critical_alert(&self, cert_info: &CertificateInfo, days: i64) -> InfraResult<()> {
        let message = format!(
            "CRITICAL: Certificate {} expires in {} days ({})",
            cert_info.domain,
            days,
            cert_info.expires_at.format("%Y-%m-%
