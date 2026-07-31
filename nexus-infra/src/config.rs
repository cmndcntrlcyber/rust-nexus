//! Configuration management for Nexus infrastructure components

use crate::{InfraError, InfraResult};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

fn serialize_redacted<S: serde::Serializer>(_: &SecretString, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("[REDACTED]")
}

/// Main configuration structure for Nexus infrastructure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NexusConfig {
    /// Cloudflare configuration
    pub cloudflare: CloudflareConfig,

    /// Let's Encrypt configuration
    pub letsencrypt: LetsEncryptConfig,

    /// Origin certificate configuration
    pub origin_cert: OriginCertConfig,

    /// gRPC server configuration
    pub grpc_server: GrpcServerConfig,

    /// Domain management configuration
    pub domains: DomainConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

/// Cloudflare API and DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareConfig {
    /// Cloudflare API token (zeroized on drop, redacted in Debug/Serialize)
    #[serde(serialize_with = "serialize_redacted")]
    pub api_token: SecretString,

    /// Zone ID for the domain
    pub zone_id: String,

    /// Base domain name
    pub domain: String,

    /// Enable Cloudflare proxy (orange cloud)
    pub proxy_enabled: bool,

    /// TTL for DNS records (seconds)
    pub ttl: u32,

    /// Geographic regions for load balancing
    pub geographic_regions: Vec<String>,

    /// Custom headers for domain fronting
    pub custom_headers: HashMap<String, String>,
}

/// Let's Encrypt ACME configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetsEncryptConfig {
    /// Contact email for ACME account
    pub contact_email: String,

    /// ACME challenge type
    pub challenge_type: ChallengeType,

    /// Days before expiration to renew certificates
    pub cert_renewal_days: u32,

    /// Enable wildcard certificate support
    pub wildcard_enabled: bool,

    /// ACME directory URL (Let's Encrypt production/staging)
    pub acme_directory_url: String,

    /// Certificate storage directory
    pub cert_storage_dir: PathBuf,
}

/// ACME challenge types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChallengeType {
    /// DNS-01 challenge (preferred for stealth)
    Dns01,
    /// HTTP-01 challenge
    Http01,
    /// TLS-ALPN-01 challenge
    TlsAlpn01,
}

/// Named HTTPS profile with its own origin cert, key, and CA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpsProfile {
    /// Profile identifier (e.g. "primary", "fallback", "fronting")
    pub name: String,
    /// SNI domains this profile handles
    pub domains: Vec<String>,
    /// Path to origin certificate
    pub cert_path: PathBuf,
    /// Path to origin private key
    pub key_path: PathBuf,
    /// Path to CA certificate
    pub ca_cert_path: PathBuf,
}

/// Origin certificate configuration for Cloudflare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginCertConfig {
    /// Path to origin certificate (single-profile legacy)
    pub cert_path: PathBuf,

    /// Path to origin private key (single-profile legacy)
    pub key_path: PathBuf,

    /// Path to CA certificate (single-profile legacy)
    pub ca_cert_path: PathBuf,

    /// Enable certificate pinning validation
    pub pin_validation: bool,

    /// Certificate validity period in days
    pub validity_days: u32,

    /// Named HTTPS profiles (overrides single-cert fields when present)
    #[serde(default)]
    pub profiles: Option<Vec<HttpsProfile>>,
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerConfig {
    /// Server bind address
    pub bind_address: String,

    /// Server port
    pub port: u16,

    /// Enable mutual TLS
    pub mutual_tls: bool,

    /// Maximum concurrent connections
    pub max_connections: u32,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Keep-alive interval in seconds
    pub keepalive_interval: u64,

    /// Maximum message size in bytes
    pub max_message_size: usize,
}

/// Domain management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Primary C2 domains
    pub primary_domains: Vec<String>,

    /// Backup C2 domains
    pub backup_domains: Vec<String>,

    /// Domain rotation interval in hours
    pub rotation_interval: u64,

    /// Maximum number of subdomains to create
    pub max_subdomains: u32,

    /// Subdomain naming pattern
    pub subdomain_pattern: SubdomainPattern,

    /// Enable domain health monitoring
    pub health_monitoring: bool,

    /// DNS resolution timeout in seconds
    pub dns_timeout: u64,
}

/// Subdomain generation patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubdomainPattern {
    /// Random alphanumeric strings
    Random { length: usize },
    /// Dictionary-based words
    Dictionary { wordlist: PathBuf },
    /// Custom pattern template
    Custom { template: String },
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable additional encryption layer beyond TLS
    pub additional_encryption: bool,

    /// Encryption key for additional layer
    pub encryption_key: Option<String>,

    /// Enable traffic obfuscation
    pub traffic_obfuscation: bool,

    /// Anti-analysis detection settings
    pub anti_analysis: AntiAnalysisConfig,

    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
}

/// Anti-analysis detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiAnalysisConfig {
    /// Enable VM detection
    pub vm_detection: bool,

    /// Enable debugger detection
    pub debugger_detection: bool,

    /// Enable sandbox detection
    pub sandbox_detection: bool,

    /// Action to take when analysis detected
    pub detection_action: DetectionAction,
}

/// Actions to take when analysis is detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionAction {
    /// Exit silently
    Exit,
    /// Sleep for extended period
    Sleep { duration_seconds: u64 },
    /// Return fake data
    Deception,
    /// Self-destruct
    SelfDestruct,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub max_requests_per_minute: u32,

    /// Burst size for rate limiting
    pub burst_size: u32,

    /// Enable per-IP rate limiting
    pub per_ip_limiting: bool,
}

impl Default for CloudflareConfig {
    fn default() -> Self {
        Self {
            api_token: SecretString::new(String::new()),
            zone_id: String::new(),
            domain: String::new(),
            proxy_enabled: true,
            ttl: 300, // 5 minutes
            geographic_regions: vec!["US".to_string(), "EU".to_string()],
            custom_headers: HashMap::new(),
        }
    }
}

impl Default for LetsEncryptConfig {
    fn default() -> Self {
        Self {
            contact_email: String::new(),
            challenge_type: ChallengeType::Dns01,
            cert_renewal_days: 30,
            wildcard_enabled: true,
            acme_directory_url: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
            cert_storage_dir: PathBuf::from("./certs"),
        }
    }
}

impl Default for OriginCertConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("./certs/origin.crt"),
            key_path: PathBuf::from("./certs/origin.key"),
            ca_cert_path: PathBuf::from("./certs/origin_ca.crt"),
            pin_validation: true,
            validity_days: 365,
            profiles: None,
        }
    }
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 443,
            mutual_tls: true,
            max_connections: 1000,
            connection_timeout: 30,
            keepalive_interval: 60,
            max_message_size: 16 * 1024 * 1024, // 16MB
        }
    }
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            primary_domains: Vec::new(),
            backup_domains: Vec::new(),
            rotation_interval: 24, // 24 hours
            max_subdomains: 10,
            subdomain_pattern: SubdomainPattern::Random { length: 8 },
            health_monitoring: true,
            dns_timeout: 5,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            additional_encryption: true,
            encryption_key: None,
            traffic_obfuscation: true,
            anti_analysis: AntiAnalysisConfig::default(),
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

impl Default for AntiAnalysisConfig {
    fn default() -> Self {
        Self {
            vm_detection: true,
            debugger_detection: true,
            sandbox_detection: true,
            detection_action: DetectionAction::Exit,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 60,
            burst_size: 10,
            per_ip_limiting: true,
        }
    }
}

impl NexusConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> InfraResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| InfraError::ConfigError(format!("Failed to read config file: {}", e)))?;

        let config: NexusConfig = toml::from_str(&content)
            .or_else(|_| serde_json::from_str(&content))
            .or_else(|_| serde_yaml::from_str(&content))
            .map_err(|e| InfraError::ConfigError(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        format: ConfigFormat,
    ) -> InfraResult<()> {
        let content = match format {
            ConfigFormat::Toml => toml::to_string_pretty(self)
                .map_err(|e| InfraError::ConfigError(format!("TOML serialization error: {}", e)))?,
            ConfigFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| InfraError::ConfigError(format!("JSON serialization error: {}", e)))?,
            ConfigFormat::Yaml => serde_yaml::to_string(self)
                .map_err(|e| InfraError::ConfigError(format!("YAML serialization error: {}", e)))?,
        };

        std::fs::write(path, content)
            .map_err(|e| InfraError::ConfigError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate configuration settings
    pub fn validate(&self) -> InfraResult<()> {
        // Validate Cloudflare configuration
        if self.cloudflare.api_token.expose_secret().is_empty() {
            return Err(InfraError::ConfigError(
                "Cloudflare API token is required".to_string(),
            ));
        }

        if self.cloudflare.zone_id.is_empty() {
            return Err(InfraError::ConfigError(
                "Cloudflare zone ID is required".to_string(),
            ));
        }

        if self.cloudflare.domain.is_empty() {
            return Err(InfraError::ConfigError("Domain is required".to_string()));
        }

        if !crate::validate_domain(&self.cloudflare.domain) {
            return Err(InfraError::ConfigError("Invalid domain format".to_string()));
        }

        // Validate Let's Encrypt configuration
        if self.letsencrypt.contact_email.is_empty() {
            return Err(InfraError::ConfigError(
                "Let's Encrypt contact email is required".to_string(),
            ));
        }

        if !self.letsencrypt.contact_email.contains('@') {
            return Err(InfraError::ConfigError("Invalid email format".to_string()));
        }

        // Validate gRPC server configuration
        if self.grpc_server.port == 0 {
            return Err(InfraError::ConfigError(
                "Invalid gRPC server port".to_string(),
            ));
        }

        // Validate domain configuration
        if self.domains.primary_domains.is_empty() {
            return Err(InfraError::ConfigError(
                "At least one primary domain is required".to_string(),
            ));
        }

        for domain in &self.domains.primary_domains {
            if !crate::validate_domain(domain) {
                return Err(InfraError::ConfigError(format!(
                    "Invalid primary domain: {}",
                    domain
                )));
            }
        }

        for domain in &self.domains.backup_domains {
            if !crate::validate_domain(domain) {
                return Err(InfraError::ConfigError(format!(
                    "Invalid backup domain: {}",
                    domain
                )));
            }
        }

        // Validate HTTPS profiles (if present)
        if let Some(profiles) = &self.origin_cert.profiles {
            let mut seen_domains: HashMap<&str, &str> = HashMap::new();
            for profile in profiles {
                if profile.name.is_empty() {
                    return Err(InfraError::ConfigError(
                        "HTTPS profile name cannot be empty".to_string(),
                    ));
                }
                if profile.domains.is_empty() {
                    return Err(InfraError::ConfigError(format!(
                        "HTTPS profile '{}' must have at least one domain",
                        profile.name,
                    )));
                }
                for domain in &profile.domains {
                    if let Some(existing) = seen_domains.get(domain.as_str()) {
                        return Err(InfraError::ConfigError(format!(
                            "Domain '{}' appears in profiles '{}' and '{}'",
                            domain, existing, profile.name,
                        )));
                    }
                    seen_domains.insert(domain, &profile.name);
                }
            }
        }

        Ok(())
    }

    /// Get configuration directory path
    pub fn get_config_dir() -> InfraResult<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            let nexus_config_dir = config_dir.join("nexus");
            std::fs::create_dir_all(&nexus_config_dir).map_err(|e| {
                InfraError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
            Ok(nexus_config_dir)
        } else {
            Err(InfraError::ConfigError(
                "Unable to determine config directory".to_string(),
            ))
        }
    }
}

/// Configuration file formats
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    Toml,
    Json,
    Yaml,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NexusConfig::default();
        assert_eq!(config.letsencrypt.challenge_type, ChallengeType::Dns01);
        assert_eq!(config.grpc_server.port, 443);
        assert!(config.security.additional_encryption);
    }

    #[test]
    fn test_config_validation() {
        let mut config = NexusConfig::default();

        // Should fail validation due to empty required fields
        assert!(config.validate().is_err());

        // Fill in required fields
        config.cloudflare.api_token = SecretString::new("test_token".to_string());
        config.cloudflare.zone_id = "test_zone".to_string();
        config.cloudflare.domain = "example.com".to_string();
        config.letsencrypt.contact_email = "test@example.com".to_string();
        config.domains.primary_domains = vec!["c2.example.com".to_string()];

        // Should now pass validation
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_subdomain_patterns() {
        let random_pattern = SubdomainPattern::Random { length: 8 };
        assert_eq!(random_pattern, SubdomainPattern::Random { length: 8 });

        let dict_pattern = SubdomainPattern::Dictionary {
            wordlist: PathBuf::from("/path/to/wordlist.txt"),
        };
        assert!(matches!(dict_pattern, SubdomainPattern::Dictionary { .. }));
    }

    #[test]
    fn test_detection_actions() {
        let exit_action = DetectionAction::Exit;
        let sleep_action = DetectionAction::Sleep {
            duration_seconds: 300,
        };

        assert_eq!(exit_action, DetectionAction::Exit);
        assert_eq!(
            sleep_action,
            DetectionAction::Sleep {
                duration_seconds: 300
            }
        );
    }

    #[test]
    fn test_https_profiles_config() {
        let toml_str = r#"
            cert_path = "./certs/origin.crt"
            key_path = "./certs/origin.key"
            ca_cert_path = "./certs/origin_ca.crt"
            pin_validation = true
            validity_days = 365

            [[profiles]]
            name = "primary"
            domains = ["c2.example.com"]
            cert_path = "./certs/prod/server.crt.pem"
            key_path = "./certs/prod/server.key.pem"
            ca_cert_path = "./certs/prod/ca.crt.pem"

            [[profiles]]
            name = "fallback"
            domains = ["backup.example.com", "cdn.example.com"]
            cert_path = "./certs/fallback/server.crt.pem"
            key_path = "./certs/fallback/server.key.pem"
            ca_cert_path = "./certs/fallback/ca.crt.pem"
        "#;

        let config: OriginCertConfig = toml::from_str(toml_str).unwrap();
        let profiles = config.profiles.unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "primary");
        assert_eq!(profiles[0].domains, vec!["c2.example.com"]);
        assert_eq!(profiles[1].name, "fallback");
        assert_eq!(profiles[1].domains, vec!["backup.example.com", "cdn.example.com"]);
    }

    #[test]
    fn test_backward_compat_no_profiles() {
        let toml_str = r#"
            cert_path = "./certs/origin.crt"
            key_path = "./certs/origin.key"
            ca_cert_path = "./certs/origin_ca.crt"
            pin_validation = true
            validity_days = 365
        "#;

        let config: OriginCertConfig = toml::from_str(toml_str).unwrap();
        assert!(config.profiles.is_none());
    }

    #[test]
    fn test_duplicate_domain_rejected() {
        let mut config = NexusConfig::default();
        config.cloudflare.api_token = SecretString::new("test_token".to_string());
        config.cloudflare.zone_id = "test_zone".to_string();
        config.cloudflare.domain = "example.com".to_string();
        config.letsencrypt.contact_email = "test@example.com".to_string();
        config.domains.primary_domains = vec!["c2.example.com".to_string()];

        config.origin_cert.profiles = Some(vec![
            HttpsProfile {
                name: "primary".to_string(),
                domains: vec!["c2.example.com".to_string()],
                cert_path: PathBuf::from("./certs/a.crt"),
                key_path: PathBuf::from("./certs/a.key"),
                ca_cert_path: PathBuf::from("./certs/a_ca.crt"),
            },
            HttpsProfile {
                name: "fallback".to_string(),
                domains: vec!["c2.example.com".to_string()],
                cert_path: PathBuf::from("./certs/b.crt"),
                key_path: PathBuf::from("./certs/b.key"),
                ca_cert_path: PathBuf::from("./certs/b_ca.crt"),
            },
        ]);

        let err = config.validate().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("c2.example.com"));
        assert!(msg.contains("primary"));
        assert!(msg.contains("fallback"));
    }

    #[test]
    fn test_toml_roundtrip_profiles() {
        let mut config = NexusConfig::default();
        config.origin_cert.profiles = Some(vec![
            HttpsProfile {
                name: "primary".to_string(),
                domains: vec!["c2.example.com".to_string()],
                cert_path: PathBuf::from("./certs/prod/server.crt.pem"),
                key_path: PathBuf::from("./certs/prod/server.key.pem"),
                ca_cert_path: PathBuf::from("./certs/prod/ca.crt.pem"),
            },
            HttpsProfile {
                name: "fallback".to_string(),
                domains: vec!["backup.example.com".to_string()],
                cert_path: PathBuf::from("./certs/fallback/server.crt.pem"),
                key_path: PathBuf::from("./certs/fallback/server.key.pem"),
                ca_cert_path: PathBuf::from("./certs/fallback/ca.crt.pem"),
            },
        ]);

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let roundtripped: NexusConfig = toml::from_str(&toml_str).unwrap();

        let profiles = roundtripped.origin_cert.profiles.unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "primary");
        assert_eq!(profiles[0].domains, vec!["c2.example.com"]);
        assert_eq!(profiles[1].name, "fallback");
        assert_eq!(profiles[1].domains, vec!["backup.example.com"]);
    }
}
