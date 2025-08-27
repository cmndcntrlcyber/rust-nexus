# Cloudflare Integration Setup Guide

Complete guide for configuring Cloudflare DNS integration with Rust-Nexus, including API token creation, zone configuration, and advanced features.

## Prerequisites

### Cloudflare Account Requirements
- **Account Type**: Free tier sufficient for basic usage, Pro+ recommended for production
- **Domain Management**: Domain must be managed by Cloudflare (nameservers pointed to Cloudflare)
- **API Access**: API token with appropriate permissions

### Domain Setup
1. **Add Domain to Cloudflare**:
   - Login to Cloudflare Dashboard
   - Click "Add a Site" and enter your domain
   - Update nameservers at your domain registrar
   - Wait for DNS propagation (24-48 hours)

2. **Verify Domain Active**:
   ```bash
   # Check nameserver propagation
   dig NS yourdomain.com
   
   # Verify Cloudflare is authoritative
   dig SOA yourdomain.com
   ```

## API Token Configuration

### Creating API Token

1. **Navigate to API Tokens**:
   - Cloudflare Dashboard → My Profile → API Tokens
   - Click "Create Token"

2. **Configure Token Permissions**:
   ```
   Token Name: Nexus-C2-DNS-Management
   
   Permissions:
   ✅ Zone - Zone:Edit
   ✅ Zone - DNS:Edit  
   ✅ Zone - Zone Settings:Edit (optional for proxy settings)
   ✅ Zone - SSL and Certificates:Edit (for origin certs)
   
   Zone Resources:
   ✅ Include - Specific zone - yourdomain.com
   
   Client IP Address Filtering: (optional)
   ✅ Is in - 203.0.113.10 (your server IP)
   
   TTL: (optional)
   ✅ Start date: Today
   ✅ End date: 1 year from now
   ```

3. **Save Token Securely**:
   ```bash
   # Store in environment variable
   export CLOUDFLARE_API_TOKEN="your_token_here"
   
   # Or in secure config file
   echo "api_token = \"your_token_here\"" > ~/.nexus/cloudflare.toml
   chmod 600 ~/.nexus/cloudflare.toml
   ```

### Verify Token Permissions

```bash
# Test token validity
curl -X GET "https://api.cloudflare.com/client/v4/user/tokens/verify" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json"

# Expected response:
{
  "success": true,
  "errors": [],
  "messages": [],
  "result": {
    "id": "token_id",
    "status": "active"
  }
}
```

## Zone Configuration

### Get Zone Information

```bash
# List all zones
curl -X GET "https://api.cloudflare.com/client/v4/zones" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json"

# Get specific zone details
curl -X GET "https://api.cloudflare.com/client/v4/zones/ZONE_ID" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json"
```

### Zone Settings Optimization

```bash
# Enable development mode for testing (speeds up cache purging)
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/ZONE_ID/settings/development_mode" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{"value":"on"}'

# Configure SSL settings
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/ZONE_ID/settings/ssl" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{"value":"full"}'

# Enable Always Use HTTPS
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/ZONE_ID/settings/always_use_https" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{"value":"on"}'
```

## DNS Record Management

### Basic DNS Operations

```rust
use nexus_infra::{CloudflareManager, CloudflareConfig, RecordType};

// Initialize Cloudflare manager
let config = CloudflareConfig {
    api_token: "your_token".to_string(),
    zone_id: "your_zone_id".to_string(),
    domain: "yourdomain.com".to_string(),
    proxy_enabled: true,
    ttl: 300,
    geographic_regions: vec!["US".to_string(), "EU".to_string()],
    custom_headers: HashMap::new(),
};

let cf_manager = CloudflareManager::new(config)?;

// Create A record
let record = cf_manager.create_c2_subdomain("c2-server", "203.0.113.10").await?;
println!("Created record: {} -> {}", record.name, record.content);

// Update existing record
let updated = cf_manager.update_c2_subdomain("c2-server", "203.0.113.20").await?;

// Delete record
cf_manager.delete_dns_record(&record.id.unwrap()).await?;
```

### Advanced DNS Features

#### Geographic Load Balancing
```bash
# Create origin pool
curl -X POST "https://api.cloudflare.com/client/v4/user/load_balancers/pools" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "name": "nexus-us-west-pool",
       "origins": [
         {
           "name": "server-1",
           "address": "203.0.113.10",
           "enabled": true,
           "weight": 1
         },
         {
           "name": "server-2", 
           "address": "203.0.113.20",
           "enabled": true,
           "weight": 1
         }
       ],
       "description": "Nexus C2 servers US West",
       "enabled": true,
       "monitor": "health_check_id"
     }'

# Create load balancer
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/load_balancers" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "name": "c2.yourdomain.com",
       "default_pools": ["pool_id_1"],
       "region_pools": {
         "WNAM": ["pool_id_1"],
         "ENAM": ["pool_id_2"]
       },
       "fallback_pool": "pool_id_backup",
       "proxied": true
     }'
```

#### Health Monitoring
```bash
# Create health check monitor
curl -X POST "https://api.cloudflare.com/client/v4/user/load_balancers/monitors" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "type": "https",
       "description": "Nexus gRPC Health Check",
       "method": "GET",
       "path": "/health",
       "header": {
         "Host": ["yourdomain.com"],
         "X-App-ID": ["nexus-health-check"]
       },
       "port": 443,
       "timeout": 5,
       "retries": 2,
       "interval": 60,
       "expected_codes": "200",
       "follow_redirects": true,
       "allow_insecure": false
     }'
```

## Proxy Configuration

### Orange Cloud vs Gray Cloud

```rust
// Configure proxy settings per record type
let proxy_settings = match record_type {
    RecordType::A | RecordType::Aaaa => true,  // Proxy HTTP traffic
    RecordType::Txt => false,                  // Never proxy TXT records
    RecordType::Cname => true,                 // Can proxy CNAME
    _ => false,
};

let record_request = CreateDnsRecordRequest {
    record_type: RecordType::A,
    name: "c2.yourdomain.com".to_string(),
    content: "203.0.113.10".to_string(),
    ttl: 1, // TTL=1 for proxied records (auto)
    proxied: proxy_settings,
};
```

### Custom Page Rules
```bash
# Create page rule for C2 traffic
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/pagerules" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "targets": [
         {
           "target": "url",
           "constraint": {
             "operator": "matches",
             "value": "c2.yourdomain.com/*"
           }
         }
       ],
       "actions": [
         {
           "id": "ssl",
           "value": "full"
         },
         {
           "id": "cache_level", 
           "value": "bypass"
         },
         {
           "id": "security_level",
           "value": "medium"
         }
       ],
       "priority": 1,
       "status": "active"
     }'
```

## Domain Fronting Configuration

### CDN Domain Selection
Choose appropriate CDN domains for fronting:
```rust
// Popular CDN domains for fronting
let cdn_domains = vec![
    "d1234567890123.cloudfront.net",  // AWS CloudFront
    "assets.example.com",             // Generic asset domain
    "cdn.jquery.com",                 // Popular CDN
    "fonts.googleapis.com",           // Google Fonts CDN
    "cdnjs.cloudflare.com",          // Cloudflare CDN
];

// Configure fronting in Rust-Nexus
let fronting_config = DomainFrontingConfig {
    real_domain: "c2.yourdomain.com".to_string(),
    front_domain: "assets.example.com".to_string(),
    host_header: "c2.yourdomain.com".to_string(),
    sni_domain: "assets.example.com".to_string(),
};
```

### Host Header Configuration
```toml
# Configure domain fronting headers
[cloudflare.custom_headers]
"Host" = "c2.yourdomain.com"
"X-Real-IP" = "203.0.113.10"
"X-Forwarded-For" = "203.0.113.10"
"X-Forwarded-Proto" = "https"

# Traffic obfuscation headers
[grpc_client.headers] 
"User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
"Accept" = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
"Accept-Language" = "en-US,en;q=0.5"
"Accept-Encoding" = "gzip, deflate"
"Referer" = "https://www.google.com/"
```

## Advanced Features

### Cloudflare Workers Integration
```javascript
// Cloudflare Worker for additional obfuscation
export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    
    // Route C2 traffic to real backend
    if (url.pathname.startsWith('/api/')) {
      // Forward to real C2 server
      const backendUrl = `https://real-c2-backend.yourdomain.com${url.pathname}`;
      const backendRequest = new Request(backendUrl, {
        method: request.method,
        headers: request.headers,
        body: request.body,
      });
      
      return fetch(backendRequest);
    }
    
    // Serve legitimate content for other requests
    return new Response('Welcome to our service', { status: 200 });
  }
}
```

### DNS Security Configuration
```bash
# Enable DNSSEC
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dnssec" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{"status":"active"}'

# Configure CAA records for certificate authority authorization
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dns_records" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "type": "CAA",
       "name": "yourdomain.com",
       "content": "0 issue \"letsencrypt.org\"",
       "ttl": 3600
     }'
```

## Rate Limiting and Optimization

### API Rate Limits
Cloudflare API rate limits (per token):
- **Free**: 1,200 requests per 5 minutes
- **Pro**: 1,200 requests per 5 minutes  
- **Business**: 1,200 requests per 5 minutes
- **Enterprise**: 4,000 requests per 5 minutes

```rust
// Implement rate limiting in client
pub struct RateLimitedCloudflareManager {
    inner: CloudflareManager,
    last_request: Arc<Mutex<Instant>>,
    min_interval: Duration,
}

impl RateLimitedCloudflareManager {
    pub async fn create_dns_record(&self, record: CreateDnsRecordRequest) -> InfraResult<DnsRecord> {
        // Implement rate limiting
        let mut last_request = self.last_request.lock().await;
        let elapsed = last_request.elapsed();
        
        if elapsed < self.min_interval {
            let sleep_duration = self.min_interval - elapsed;
            tokio::time::sleep(sleep_duration).await;
        }
        
        let result = self.inner.create_dns_record(record).await;
        *last_request = Instant::now();
        
        result
    }
}
```

### Bulk Operations
```rust
// Efficient bulk DNS operations
pub async fn bulk_create_subdomains(
    cf_manager: &CloudflareManager,
    subdomains: Vec<(String, String)>, // (name, ip) pairs
) -> InfraResult<Vec<DnsRecord>> {
    let mut results = Vec::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(5)); // Max 5 concurrent

    let futures: Vec<_> = subdomains.into_iter().map(|(name, ip)| {
        let cf_manager = cf_manager.clone();
        let semaphore = semaphore.clone();
        
        async move {
            let _permit = semaphore.acquire().await.unwrap();
            cf_manager.create_c2_subdomain(&name, &ip).await
        }
    }).collect();
    
    for future in futures {
        match future.await {
            Ok(record) => results.push(record),
            Err(e) => warn!("Failed to create subdomain: {}", e),
        }
    }
    
    Ok(results)
}
```

## Security Considerations

### API Token Security
```bash
# Rotate API tokens regularly
#!/bin/bash
OLD_TOKEN="$1"
NEW_TOKEN="$2"

# Test new token
curl -X GET "https://api.cloudflare.com/client/v4/user/tokens/verify" \
     -H "Authorization: Bearer $NEW_TOKEN"

if [ $? -eq 0 ]; then
    # Update configuration files
    sed -i "s/$OLD_TOKEN/$NEW_TOKEN/g" /opt/nexus/config/production.toml
    
    # Restart services
    systemctl reload nexus-server
    
    # Revoke old token
    curl -X DELETE "https://api.cloudflare.com/client/v4/user/tokens/TOKEN_ID" \
         -H "Authorization: Bearer $OLD_TOKEN"
    
    echo "Token rotation completed successfully"
else
    echo "New token verification failed"
    exit 1
fi
```

### Zone Access Control
```toml
# Restrict API token to specific zones and operations
[cloudflare]
api_token = "your_restricted_token"
zone_id = "specific_zone_only"

# Additional security settings
[cloudflare.security]
validate_zone_access = true
enforce_https = true
monitor_api_usage = true
rate_limit_buffer = 0.8  # Use only 80% of rate limit
```

## Integration with Rust-Nexus

### Configuration Integration
```toml
# Complete Cloudflare configuration
[cloudflare]
api_token = "your_api_token_here"
zone_id = "your_zone_id_here"
domain = "yourdomain.com"

# Proxy configuration
proxy_enabled = true
ttl = 300

# Geographic targeting
geographic_regions = ["US", "EU", "APAC"]

# Custom headers for domain fronting
[cloudflare.custom_headers]
"Host" = "legitimate-service.com"
"User-Agent" = "Mozilla/5.0 (compatible; legitimate-bot/1.0)"
"X-Forwarded-For" = "203.0.113.1"
"X-Real-IP" = "203.0.113.1"

# Security settings
[cloudflare.security]
enable_bot_management = true
enable_rate_limiting = true
ddos_protection = "high"
```

### Programmatic Management
```rust
// Advanced Cloudflare integration
use nexus_infra::CloudflareManager;

#[tokio::main]
async fn main() -> InfraResult<()> {
    let cf_manager = CloudflareManager::new(config.cloudflare)?;
    
    // Verify access and get zone info
    let zone = cf_manager.verify_access().await?;
    println!("Managing zone: {} ({})", zone.name, zone.status);
    
    // Create multiple C2 subdomains
    let subdomains = vec![
        ("primary", "203.0.113.10"),
        ("backup", "203.0.113.20"), 
        ("fallback", "203.0.113.30"),
    ];
    
    for (name, ip) in subdomains {
        let record = cf_manager.create_c2_subdomain(name, ip).await?;
        println!("Created: {} -> {}", record.name, record.content);
        
        // Wait between requests to respect rate limits
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // Set up automated cleanup
    let cleanup_patterns = vec!["temp-".to_string(), "test-".to_string()];
    let removed = cf_manager.cleanup_old_subdomains(&cleanup_patterns).await?;
    println!("Cleaned up {} old records", removed.len());
    
    Ok(())
}
```

## Monitoring and Alerting

### API Usage Monitoring
```rust
// Monitor API usage and rates
pub struct ApiUsageMonitor {
    request_count: Arc<AtomicUsize>,
    last_reset: Arc<Mutex<Instant>>,
    rate_limit_window: Duration,
    rate_limit_max: usize,
}

impl ApiUsageMonitor {
    pub fn new(window: Duration, max_requests: usize) -> Self {
        Self {
            request_count: Arc::new(AtomicUsize::new(0)),
            last_reset: Arc::new(Mutex::new(Instant::now())),
            rate_limit_window: window,
            rate_limit_max: max_requests,
        }
    }
    
    pub async fn check_rate_limit(&self) -> bool {
        let mut last_reset = self.last_reset.lock().await;
        
        // Reset counter if window elapsed
        if last_reset.elapsed() >= self.rate_limit_window {
            self.request_count.store(0, Ordering::Relaxed);
            *last_reset = Instant::now();
        }
        
        let current_count = self.request_count.fetch_add(1, Ordering::Relaxed);
        current_count < self.rate_limit_max
    }
}
```

### Zone Health Monitoring
```bash
# Monitor zone health script
#!/bin/bash
ZONE_ID="your_zone_id"
API_TOKEN="your_token"

# Check zone status
ZONE_STATUS=$(curl -s -H "Authorization: Bearer $API_TOKEN" \
  "https://api.cloudflare.com/client/v4/zones/$ZONE_ID" | \
  jq -r '.result.status')

if [ "$ZONE_STATUS" != "active" ]; then
    echo "ALERT: Zone status is $ZONE_STATUS"
    # Send alert notification
fi

# Check nameserver configuration
NS_COUNT=$(dig NS yourdomain.com +short | wc -l)
if [ "$NS_COUNT" -lt 2 ]; then
    echo "ALERT: Insufficient nameservers configured"
fi

# Monitor API rate limits
RATE_LIMIT_REMAINING=$(curl -s -I -H "Authorization: Bearer $API_TOKEN" \
  "https://api.cloudflare.com/client/v4/zones/$ZONE_ID" | \
  grep -i "x-ratelimit-remaining" | cut -d: -f2 | tr -d ' \r')

if [ "$RATE_LIMIT_REMAINING" -lt 100 ]; then
    echo "ALERT: API rate limit running low: $RATE_LIMIT_REMAINING"
fi
```

## Troubleshooting

### Common Issues

#### Authentication Errors
```bash
# Debug authentication issues
curl -v -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/verify"

# Check token format
echo "Bearer YOUR_TOKEN" | wc -c  # Should be reasonable length
echo "YOUR_TOKEN" | grep -E '^[A-Za-z0-9_-]+$'  # Should contain valid characters
```

#### Zone Access Issues
```bash
# Verify zone ownership
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/zones" | \
     jq '.result[] | select(.name=="yourdomain.com")'

# Check zone status
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/zones/ZONE_ID" | \
     jq '.result.status'
```

#### DNS Propagation Issues
```bash
# Check DNS propagation globally
dig @8.8.8.8 subdomain.yourdomain.com
dig @1.1.1.1 subdomain.yourdomain.com  
dig @208.67.222.222 subdomain.yourdomain.com

# Use online tools
curl "https://dns.google/resolve?name=subdomain.yourdomain.com&type=A"
```

### Debug Mode
```rust
// Enable detailed Cloudflare API debugging
use log::LevelFilter;

env_logger::Builder::new()
    .filter_module("nexus_infra::cloudflare", LevelFilter::Debug)
    .init();

// All Cloudflare API calls will be logged with full request/response details
```

## Best Practices

### **Security**
- Use separate API tokens for different environments (dev/staging/prod)
- Implement IP restrictions on API tokens when possible
- Monitor certificate transparency logs for unauthorized certificates
- Regular rotation of API tokens and certificates
- Use Cloudflare's security features (bot management, DDoS protection)

### **Performance**
- Cache DNS responses appropriately (300-3600 second TTLs)
- Use batch operations for multiple record changes
- Implement retry logic with exponential backoff
- Monitor API usage to stay within rate limits
- Use geographic load balancing for global deployments

### **Operations**
- Automate all DNS operations to reduce human error
- Implement comprehensive logging and audit trails
- Use Infrastructure as Code principles for reproducibility
- Have rollback procedures for failed changes
- Test all operations in staging environment first

### **Monitoring**
- Monitor zone health and nameserver configuration
- Alert on API rate limit consumption
- Track DNS record changes and modifications
- Monitor certificate expiration dates
- Set up health checks for all critical records

This Cloudflare setup guide provides comprehensive coverage of integrating Cloudflare DNS services with Rust-Nexus for enterprise-grade infrastructure automation.
