//! Cloudflare API client for DNS management and domain fronting

use crate::{CloudflareConfig, InfraError, InfraResult};
use log::{debug, error, info, warn};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloudflare DNS record types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordType {
    #[serde(rename = "A")]
    A,
    #[serde(rename = "AAAA")]
    Aaaa,
    #[serde(rename = "CNAME")]
    Cname,
    #[serde(rename = "TXT")]
    Txt,
    #[serde(rename = "MX")]
    Mx,
    #[serde(rename = "NS")]
    Ns,
    #[serde(rename = "SRV")]
    Srv,
    #[serde(rename = "CAA")]
    Caa,
}

/// DNS record structure for Cloudflare API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub record_type: RecordType,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub created_on: Option<String>,
    pub modified_on: Option<String>,
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

/// Cloudflare API response structure
#[derive(Debug, Clone, Deserialize)]
pub struct CloudflareResponse<T> {
    pub success: bool,
    pub errors: Vec<CloudflareError>,
    pub messages: Vec<CloudflareMessage>,
    pub result: Option<T>,
    pub result_info: Option<ResultInfo>,
}

/// Cloudflare error structure
#[derive(Debug, Clone, Deserialize)]
pub struct CloudflareError {
    pub code: u32,
    pub message: String,
}

/// Cloudflare message structure
#[derive(Debug, Clone, Deserialize)]
pub struct CloudflareMessage {
    pub code: u32,
    pub message: String,
}

/// Result info for paginated responses
#[derive(Debug, Clone, Deserialize)]
pub struct ResultInfo {
    pub page: u32,
    pub per_page: u32,
    pub count: u32,
    pub total_count: u32,
    pub total_pages: u32,
}

/// Request structure for creating/updating DNS records
#[derive(Debug, Clone, Serialize)]
pub struct CreateDnsRecordRequest {
    #[serde(rename = "type")]
    pub record_type: RecordType,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
}

/// Zone information from Cloudflare
#[derive(Debug, Clone, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: String,
    pub paused: bool,
    pub development_mode: u32,
    pub name_servers: Vec<String>,
}

/// Cloudflare manager for DNS operations
pub struct CloudflareManager {
    client: Client,
    config: CloudflareConfig,
    base_url: String,
}

impl CloudflareManager {
    /// Create a new Cloudflare manager
    pub fn new(config: CloudflareConfig) -> InfraResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_token))
                .map_err(|e| InfraError::CloudflareError(format!("Invalid API token: {}", e)))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                InfraError::CloudflareError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            base_url: "https://api.cloudflare.com/client/v4".to_string(),
        })
    }

    /// Verify API token and zone access
    pub async fn verify_access(&self) -> InfraResult<Zone> {
        let url = format!("{}/zones/{}", self.base_url, self.config.zone_id);

        debug!(
            "Verifying Cloudflare access for zone: {}",
            self.config.zone_id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<Zone> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        let zone = api_response
            .result
            .ok_or_else(|| InfraError::CloudflareError("No zone data returned".to_string()))?;

        info!(
            "Successfully verified access to zone: {} ({})",
            zone.name, zone.id
        );
        Ok(zone)
    }

    /// List all DNS records for the zone
    pub async fn list_dns_records(&self) -> InfraResult<Vec<DnsRecord>> {
        let url = format!(
            "{}/zones/{}/dns_records",
            self.base_url, self.config.zone_id
        );

        debug!("Listing DNS records for zone: {}", self.config.zone_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<Vec<DnsRecord>> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        let records = api_response.result.unwrap_or_default();
        info!("Found {} DNS records", records.len());

        Ok(records)
    }

    /// Create a new DNS record
    pub async fn create_dns_record(
        &self,
        record: CreateDnsRecordRequest,
    ) -> InfraResult<DnsRecord> {
        let url = format!(
            "{}/zones/{}/dns_records",
            self.base_url, self.config.zone_id
        );

        info!(
            "Creating DNS record: {} {} {}",
            record.record_type, record.name, record.content
        );

        let response = self
            .client
            .post(&url)
            .json(&record)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<DnsRecord> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        let created_record = api_response
            .result
            .ok_or_else(|| InfraError::CloudflareError("No record data returned".to_string()))?;

        info!(
            "Successfully created DNS record: {}",
            created_record.id.as_ref().unwrap_or(&"unknown".to_string())
        );
        Ok(created_record)
    }

    /// Update an existing DNS record
    pub async fn update_dns_record(
        &self,
        record_id: &str,
        record: CreateDnsRecordRequest,
    ) -> InfraResult<DnsRecord> {
        let url = format!(
            "{}/zones/{}/dns_records/{}",
            self.base_url, self.config.zone_id, record_id
        );

        info!(
            "Updating DNS record {}: {} {} {}",
            record_id, record.record_type, record.name, record.content
        );

        let response = self
            .client
            .put(&url)
            .json(&record)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<DnsRecord> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        let updated_record = api_response
            .result
            .ok_or_else(|| InfraError::CloudflareError("No record data returned".to_string()))?;

        info!("Successfully updated DNS record: {}", record_id);
        Ok(updated_record)
    }

    /// Delete a DNS record
    pub async fn delete_dns_record(&self, record_id: &str) -> InfraResult<()> {
        let url = format!(
            "{}/zones/{}/dns_records/{}",
            self.base_url, self.config.zone_id, record_id
        );

        info!("Deleting DNS record: {}", record_id);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        info!("Successfully deleted DNS record: {}", record_id);
        Ok(())
    }

    /// Find DNS records by name and type
    pub async fn find_dns_records(
        &self,
        name: &str,
        record_type: Option<RecordType>,
    ) -> InfraResult<Vec<DnsRecord>> {
        let mut url = format!(
            "{}/zones/{}/dns_records?name={}",
            self.base_url, self.config.zone_id, name
        );

        if let Some(rt) = record_type {
            url.push_str(&format!("&type={:?}", rt));
        }

        debug!("Finding DNS records: {}", name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("API request failed: {}", e)))?;

        let api_response: CloudflareResponse<Vec<DnsRecord>> = response
            .json()
            .await
            .map_err(|e| InfraError::CloudflareError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(InfraError::CloudflareError(format!(
                "API error: {}",
                error_msg
            )));
        }

        let records = api_response.result.unwrap_or_default();
        debug!("Found {} matching DNS records", records.len());

        Ok(records)
    }

    /// Create a C2 subdomain with A record
    pub async fn create_c2_subdomain(
        &self,
        subdomain: &str,
        ip_address: &str,
    ) -> InfraResult<DnsRecord> {
        let full_name = format!("{}.{}", subdomain, self.config.domain);

        let record_request = CreateDnsRecordRequest {
            record_type: RecordType::A,
            name: full_name,
            content: ip_address.to_string(),
            ttl: self.config.ttl,
            proxied: self.config.proxy_enabled,
        };

        self.create_dns_record(record_request).await
    }

    /// Create a TXT record for ACME challenge
    pub async fn create_acme_challenge(
        &self,
        challenge_name: &str,
        challenge_value: &str,
    ) -> InfraResult<DnsRecord> {
        let record_request = CreateDnsRecordRequest {
            record_type: RecordType::Txt,
            name: challenge_name.to_string(),
            content: challenge_value.to_string(),
            ttl: 120,       // Short TTL for challenges
            proxied: false, // Never proxy TXT records
        };

        self.create_dns_record(record_request).await
    }

    /// Delete ACME challenge TXT record
    pub async fn delete_acme_challenge(&self, challenge_name: &str) -> InfraResult<()> {
        let records = self
            .find_dns_records(challenge_name, Some(RecordType::Txt))
            .await?;

        for record in records {
            if let Some(record_id) = record.id {
                self.delete_dns_record(&record_id).await?;
            }
        }

        Ok(())
    }

    /// Update subdomain to point to new IP address
    pub async fn update_c2_subdomain(
        &self,
        subdomain: &str,
        new_ip: &str,
    ) -> InfraResult<DnsRecord> {
        let full_name = format!("{}.{}", subdomain, self.config.domain);
        let records = self
            .find_dns_records(&full_name, Some(RecordType::A))
            .await?;

        if let Some(existing_record) = records.first() {
            if let Some(record_id) = &existing_record.id {
                let update_request = CreateDnsRecordRequest {
                    record_type: RecordType::A,
                    name: full_name,
                    content: new_ip.to_string(),
                    ttl: self.config.ttl,
                    proxied: self.config.proxy_enabled,
                };

                self.update_dns_record(record_id, update_request).await
            } else {
                Err(InfraError::CloudflareError(
                    "Record ID not found".to_string(),
                ))
            }
        } else {
            // Create new record if it doesn't exist
            self.create_c2_subdomain(subdomain, new_ip).await
        }
    }

    /// Clean up old C2 subdomains
    pub async fn cleanup_old_subdomains(
        &self,
        keep_patterns: &[String],
    ) -> InfraResult<Vec<String>> {
        let all_records = self.list_dns_records().await?;
        let mut deleted_records = Vec::new();

        for record in all_records {
            if record.record_type == RecordType::A && record.name.ends_with(&self.config.domain) {
                let subdomain = record
                    .name
                    .strip_suffix(&format!(".{}", self.config.domain))
                    .unwrap_or(&record.name);

                // Check if this subdomain matches any keep pattern
                let should_keep = keep_patterns
                    .iter()
                    .any(|pattern| subdomain.contains(pattern) || pattern.contains(subdomain));

                if !should_keep {
                    if let Some(record_id) = record.id {
                        match self.delete_dns_record(&record_id).await {
                            Ok(_) => {
                                deleted_records.push(record.name.clone());
                                info!("Cleaned up old subdomain: {}", record.name);
                            }
                            Err(e) => {
                                warn!("Failed to delete record {}: {}", record.name, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_records)
    }

    /// Get configuration reference
    pub fn config(&self) -> &CloudflareConfig {
        &self.config
    }
}

impl std::fmt::Display for RecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordType::A => write!(f, "A"),
            RecordType::Aaaa => write!(f, "AAAA"),
            RecordType::Cname => write!(f, "CNAME"),
            RecordType::Txt => write!(f, "TXT"),
            RecordType::Mx => write!(f, "MX"),
            RecordType::Ns => write!(f, "NS"),
            RecordType::Srv => write!(f, "SRV"),
            RecordType::Caa => write!(f, "CAA"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_type_display() {
        assert_eq!(RecordType::A.to_string(), "A");
        assert_eq!(RecordType::Txt.to_string(), "TXT");
        assert_eq!(RecordType::Cname.to_string(), "CNAME");
    }

    #[test]
    fn test_create_dns_record_request() {
        let request = CreateDnsRecordRequest {
            record_type: RecordType::A,
            name: "test.example.com".to_string(),
            content: "192.168.1.1".to_string(),
            ttl: 300,
            proxied: true,
        };

        assert_eq!(request.record_type, RecordType::A);
        assert_eq!(request.name, "test.example.com");
        assert!(request.proxied);
    }

    #[test]
    fn test_cloudflare_config_default() {
        let config = CloudflareConfig::default();
        assert_eq!(config.ttl, 300);
        assert!(config.proxy_enabled);
        assert_eq!(
            config.geographic_regions,
            vec!["US".to_string(), "EU".to_string()]
        );
    }
}
