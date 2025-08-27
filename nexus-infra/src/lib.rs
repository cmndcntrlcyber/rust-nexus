//! Nexus Infrastructure Management
//! 
//! This crate provides infrastructure management capabilities for the Nexus C2 framework,
//! including Cloudflare DNS management, Let's Encrypt certificate automation,
//! gRPC communication layers, and enhanced BOF/COFF execution support.

use nexus_common::*;

pub mod config;
pub mod cloudflare;
pub mod letsencrypt;
pub mod grpc_client;
pub mod grpc_server;
pub mod bof_loader;
pub mod domain_manager;
pub mod cert_manager;

// Re-export generated gRPC code
pub mod proto {
    tonic::include_proto!("nexus.v1");
    
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("nexus_descriptor");
}

// Common types and utilities
pub use config::{NexusConfig, CloudflareConfig, LetsEncryptConfig, OriginCertConfig};
pub use cloudflare::CloudflareManager;
pub use letsencrypt::CertificateManager;
pub use grpc_client::GrpcClient;
pub use grpc_server::GrpcServer;
pub use bof_loader::BOFLoader;
pub use domain_manager::DomainManager;
pub use cert_manager::CertManager;

/// Infrastructure error types
#[derive(thiserror::Error, Debug)]
pub enum InfraError {
    #[error("Cloudflare API error: {0}")]
    CloudflareError(String),
    
    #[error("Let's Encrypt error: {0}")]
    LetsEncryptError(String),
    
    #[error("Certificate error: {0}")]
    CertificateError(String),
    
    #[error("DNS error: {0}")]
    DnsError(String),
    
    #[error("gRPC error: {0}")]
    GrpcError(String),
    
    #[error("BOF loading error: {0}")]
    BofError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("TLS error: {0}")]
    TlsError(String),
}

pub type InfraResult<T> = std::result::Result<T, InfraError>;

/// Initialize logging for the infrastructure components
pub fn init_logging() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
}

/// Utility function to generate secure random strings for subdomains
pub fn generate_subdomain(length: usize) -> String {
    use rand::{distributions::Alphanumeric, Rng};
    
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

/// Utility function to validate domain names
pub fn validate_domain(domain: &str) -> bool {
    // Basic domain validation - could be enhanced with more sophisticated checks
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    
    // Check for valid characters and structure
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    
    for part in parts {
        if part.is_empty() || part.len() > 63 {
            return false;
        }
        
        if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
        
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }
    }
    
    true
}

/// Utility function to create gRPC channel with TLS
pub async fn create_grpc_channel(
    endpoint: &str,
    ca_cert_pem: Option<&str>,
) -> InfraResult<tonic::transport::Channel> {
    let mut endpoint = tonic::transport::Endpoint::from_shared(endpoint.to_string())
        .map_err(|e| InfraError::GrpcError(format!("Invalid endpoint: {}", e)))?;
    
    // Configure TLS
    if let Some(ca_cert) = ca_cert_pem {
        let ca_cert = tonic::transport::Certificate::from_pem(ca_cert);
        let tls_config = tonic::transport::ClientTlsConfig::new()
            .ca_certificate(ca_cert);
        
        endpoint = endpoint.tls_config(tls_config)
            .map_err(|e| InfraError::TlsError(format!("TLS config error: {}", e)))?;
    }
    
    endpoint.connect()
        .await
        .map_err(|e| InfraError::GrpcError(format!("Connection error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_subdomain() {
        let subdomain = generate_subdomain(8);
        assert_eq!(subdomain.len(), 8);
        assert!(subdomain.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_validate_domain() {
        assert!(validate_domain("example.com"));
        assert!(validate_domain("sub.example.com"));
        assert!(validate_domain("test-domain.example.com"));
        
        assert!(!validate_domain(""));
        assert!(!validate_domain("invalid"));
        assert!(!validate_domain("invalid..com"));
        assert!(!validate_domain("-invalid.com"));
        assert!(!validate_domain("invalid-.com"));
    }
}
