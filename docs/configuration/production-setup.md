# Production Deployment Guide

This guide covers production deployment of Rust-Nexus with enterprise-grade infrastructure, security hardening, and operational best practices.

## Pre-Deployment Checklist

### Infrastructure Requirements
- [ ] **Cloudflare Account** with Pro plan or higher
- [ ] **Domain Registration** with DNS managed by Cloudflare
- [ ] **API Token** with Zone:Edit and DNS:Edit permissions
- [ ] **TLS Certificates** (Let's Encrypt or Cloudflare Origin)
- [ ] **Production Server** with public IP address
- [ ] **Monitoring System** for health checks and alerting

### Security Prerequisites
- [ ] **Certificate Storage** with proper filesystem permissions
- [ ] **API Key Security** using environment variables or key vault
- [ ] **Network Segmentation** isolating C2 infrastructure
- [ ] **Log Aggregation** for audit trails and monitoring
- [ ] **Incident Response** procedures and playbooks

## Step-by-Step Deployment

### 1. Infrastructure Preparation

#### Server Setup
```bash
# Update system and install dependencies
sudo apt update && sudo apt upgrade -y
sudo apt install build-essential git curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Create nexus user and directories
sudo useradd -r -s /bin/false nexus
sudo mkdir -p /opt/nexus/{config,certs,logs}
sudo chown -R nexus:nexus /opt/nexus
```

#### Firewall Configuration
```bash
# Configure firewall for gRPC server
sudo ufw allow 443/tcp comment "Nexus gRPC Server"
sudo ufw allow 80/tcp comment "Let's Encrypt HTTP-01 (if needed)"

# Block direct access to management ports
sudo ufw deny 22/tcp
sudo ufw enable
```

### 2. Cloudflare Configuration

#### API Token Setup
1. Navigate to Cloudflare Dashboard → My Profile → API Tokens
2. Create token with permissions:
   - **Zone Resources**: Include - Specific zone - `yourdomain.com`
   - **Permissions**: Zone:Read, Zone:Edit, DNS:Read, DNS:Edit

#### DNS Zone Configuration
```bash
# Verify zone configuration
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/zones/YOUR_ZONE_ID"

# Test DNS record creation
curl -X POST "https://api.cloudflare.com/client/v4/zones/YOUR_ZONE_ID/dns_records" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{"type":"A","name":"test.yourdomain.com","content":"203.0.113.1","ttl":300}'
```

### 3. Certificate Deployment

#### Production Configuration
```toml
# /opt/nexus/config/production.toml
[cloudflare]
api_token = "YOUR_PRODUCTION_API_TOKEN"
zone_id = "YOUR_ZONE_ID"
domain = "yourdomain.com"
proxy_enabled = true
ttl = 300

[letsencrypt]
contact_email = "admin@yourdomain.com"
challenge_type = "Dns01"
cert_renewal_days = 30
wildcard_enabled = true
acme_directory_url = "https://acme-v02.api.letsencrypt.org/directory"
cert_storage_dir = "/opt/nexus/certs"

[grpc_server]
bind_address = "0.0.0.0"
port = 443
mutual_tls = true
max_connections = 5000

[domains]
primary_domains = ["c2.yourdomain.com", "api.yourdomain.com"]
backup_domains = ["backup.yourdomain.com", "cdn.yourdomain.com"]
rotation_interval = 12  # 12-hour rotation for production
max_subdomains = 20

[security]
additional_encryption = true
traffic_obfuscation = true

[security.anti_analysis]
vm_detection = true
debugger_detection = true
sandbox_detection = true

[security.anti_analysis.detection_action]
type = "Exit"
```

#### Certificate Generation
```bash
# Build and run infrastructure setup
cd /opt/nexus
git clone https://github.com/your-org/rust-nexus.git .
cargo build --release

# Initialize certificates
sudo -u nexus ./target/release/nexus-infra setup \
  --config /opt/nexus/config/production.toml

# Verify certificate generation
sudo -u nexus ls -la /opt/nexus/certs/
```

### 4. Service Configuration

#### Systemd Service
```ini
# /etc/systemd/system/nexus-server.service
[Unit]
Description=Nexus C2 Server
After=network.target
Wants=network.target

[Service]
Type=simple
User=nexus
Group=nexus
WorkingDirectory=/opt/nexus
ExecStart=/opt/nexus/target/release/nexus-server --config /opt/nexus/config/production.toml
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/nexus/certs /opt/nexus/logs
PrivateTmp=true

# Environment
Environment=RUST_LOG=info
Environment=NEXUS_CONFIG=/opt/nexus/config/production.toml

[Install]
WantedBy=multi-user.target
```

#### Enable and Start Service
```bash
# Install and start service
sudo systemctl daemon-reload
sudo systemctl enable nexus-server
sudo systemctl start nexus-server

# Verify status
sudo systemctl status nexus-server
sudo journalctl -u nexus-server -f
```

### 5. Load Balancing & High Availability

#### Multiple Server Configuration
```toml
# Server 1: /opt/nexus/config/server1.toml
[grpc_server]
bind_address = "10.0.1.10"
port = 443

[domains]
primary_domains = ["c2-1.yourdomain.com"]

# Server 2: /opt/nexus/config/server2.toml  
[grpc_server]
bind_address = "10.0.1.20"
port = 443

[domains]
primary_domains = ["c2-2.yourdomain.com"]
```

#### Cloudflare Load Balancing
```bash
# Configure geographic load balancing
# Cloudflare Dashboard → Traffic → Load Balancing
# Create origin pools for each server region
# Configure health checks for gRPC endpoints
```

### 6. Monitoring Setup

#### Health Check Endpoints
```rust
// Add health check to gRPC server
use tonic_health::server::HealthReporter;

let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
health_reporter.set_serving::<NexusC2Server<_>>().await;

let server = Server::builder()
    .add_service(health_service)
    .add_service(nexus_service)
    .serve(addr);
```

#### Monitoring Configuration
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'nexus-server'
    static_configs:
      - targets: ['server1.yourdomain.com:9090', 'server2.yourdomain.com:9090']
    scheme: https
    tls_config:
      ca_file: /etc/ssl/certs/nexus-ca.pem
```

### 7. Certificate Management

#### Automated Renewal
```bash
# Create certificate renewal script
cat > /opt/nexus/scripts/renew-certs.sh << 'EOF'
#!/bin/bash
cd /opt/nexus
./target/release/nexus-infra certificates renew --config config/production.toml
sudo systemctl reload nexus-server
EOF

chmod +x /opt/nexus/scripts/renew-certs.sh

# Add to crontab for automatic renewal
echo "0 2 * * 0 /opt/nexus/scripts/renew-certs.sh" | sudo crontab -u nexus -
```

#### Certificate Validation
```bash
# Verify certificate chain
openssl x509 -in /opt/nexus/certs/yourdomain.com.crt -text -noout

# Check certificate expiration
openssl x509 -in /opt/nexus/certs/yourdomain.com.crt -noout -dates

# Validate certificate chain
openssl verify -CAfile /opt/nexus/certs/chain.pem /opt/nexus/certs/yourdomain.com.crt
```

## Security Hardening

### File System Security
```bash
# Set proper permissions
sudo chmod 600 /opt/nexus/config/production.toml
sudo chmod 600 /opt/nexus/certs/*.key
sudo chmod 644 /opt/nexus/certs/*.crt

# SELinux/AppArmor policies (if applicable)
sudo setsebool -P httpd_can_network_connect 1
```

### Network Security
```bash
# Configure iptables for additional security
sudo iptables -A INPUT -p tcp --dport 443 -m state --state NEW,ESTABLISHED -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 80 -j DROP  # Block HTTP
sudo iptables -A INPUT -p tcp --dport 22 -s MANAGEMENT_IP -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 22 -j DROP  # Block SSH from other IPs

# Save iptables rules
sudo iptables-save > /etc/iptables/rules.v4
```

### Application Security
```toml
# Enhanced security configuration
[security]
additional_encryption = true
traffic_obfuscation = true

[security.rate_limiting]
max_requests_per_minute = 100
burst_size = 20
per_ip_limiting = true

[security.anti_analysis]
vm_detection = true
debugger_detection = true
sandbox_detection = true
detection_action = { type = "Exit" }
```

## Production Monitoring

### Key Metrics to Monitor

#### Infrastructure Metrics
- **Domain Health**: DNS resolution success rate
- **Certificate Status**: Days until expiration
- **API Rate Limits**: Cloudflare API usage
- **Server Resources**: CPU, memory, disk usage

#### Security Metrics  
- **Connection Attempts**: Failed authentication attempts
- **Geographic Distribution**: Connection source countries
- **Certificate Validation**: Failed certificate validations
- **Domain Reputation**: Domain reputation scores

#### Operational Metrics
- **Agent Connectivity**: Active agent count and health
- **Task Execution**: Task success/failure rates
- **Response Times**: gRPC request/response latencies
- **Error Rates**: Application and infrastructure error rates

### Alerting Configuration

```yaml
# AlertManager configuration
groups:
  - name: nexus-alerts
    rules:
      - alert: NexusCertificateExpiry
        expr: nexus_certificate_expiry_days < 7
        for: 1h
        annotations:
          summary: "Nexus certificate expiring soon"

      - alert: NexusDomainHealthLow
        expr: nexus_domain_health_percentage < 90
        for: 5m
        annotations:
          summary: "Nexus domain health degraded"

      - alert: NexusAgentDisconnected
        expr: nexus_active_agents < 1
        for: 10m
        annotations:
          summary: "No active Nexus agents"
```

## Backup and Recovery

### Configuration Backup
```bash
# Automated configuration backup
#!/bin/bash
BACKUP_DIR="/opt/nexus/backups/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup configuration
cp /opt/nexus/config/production.toml "$BACKUP_DIR/"

# Backup certificates
cp -r /opt/nexus/certs "$BACKUP_DIR/"

# Backup logs (last 7 days)
find /opt/nexus/logs -name "*.log" -mtime -7 -exec cp {} "$BACKUP_DIR/" \;

# Upload to secure storage
aws s3 sync "$BACKUP_DIR" s3://nexus-backups/$(date +%Y%m%d)/
```

### Disaster Recovery
```bash
# Recovery procedure
#!/bin/bash
RESTORE_DATE="$1"

# Stop services
sudo systemctl stop nexus-server

# Restore from backup
aws s3 sync "s3://nexus-backups/$RESTORE_DATE/" /opt/nexus/restore/
sudo -u nexus cp /opt/nexus/restore/production.toml /opt/nexus/config/
sudo -u nexus cp -r /opt/nexus/restore/certs/* /opt/nexus/certs/

# Set permissions
sudo chmod 600 /opt/nexus/config/production.toml
sudo chmod 600 /opt/nexus/certs/*.key

# Restart services
sudo systemctl start nexus-server
```

## Deployment Validation

### Infrastructure Testing
```bash
# Test Cloudflare API connectivity
./target/release/nexus-infra cloudflare verify --config config/production.toml

# Test certificate provisioning
./target/release/nexus-infra certificates test --config config/production.toml

# Test domain creation
./target/release/nexus-infra domains create test123 --config config/production.toml
```

### gRPC Testing
```bash
# Test gRPC server connectivity
grpcurl -insecure yourdomain.com:443 describe

# Test agent registration (with test client)
./target/release/nexus-agent --test-mode --config config/agent-test.toml

# Load testing
./scripts/load-test.sh --concurrent-agents 100 --duration 300s
```

### Security Testing
```bash
# Test TLS configuration
testssl.sh --protocols --ciphers yourdomain.com:443

# Test certificate validation
openssl s_client -connect yourdomain.com:443 -verify_return_error

# Test domain fronting
curl -H "Host: yourdomain.com" https://legitimate-cdn.com/
```

## Operational Procedures

### Daily Operations
```bash
# Daily health check
./scripts/daily-health-check.sh

# Certificate status
./target/release/nexus-infra certificates status --config config/production.toml

# Domain rotation (if scheduled)
./target/release/nexus-infra domains rotate --config config/production.toml

# Log rotation
sudo logrotate /etc/logrotate.d/nexus
```

### Weekly Maintenance
```bash
# Infrastructure health report
./scripts/weekly-health-report.sh

# Certificate renewal check
./target/release/nexus-infra certificates check-renewal --config config/production.toml

# Domain cleanup
./target/release/nexus-infra domains cleanup --older-than 7d --config config/production.toml

# Security audit
./scripts/security-audit.sh
```

### Monthly Tasks
```bash
# API key rotation
./scripts/rotate-api-keys.sh

# Certificate backup verification
./scripts/verify-backups.sh

# Performance analysis
./scripts/performance-report.sh

# Infrastructure cost analysis
./scripts/cost-analysis.sh
```

## Scaling Considerations

### Horizontal Scaling
```bash
# Deploy additional server instances
for i in {2..5}; do
    ./scripts/deploy-server.sh --instance $i --region us-west-$((i-1))
done

# Configure load balancing
./scripts/setup-load-balancer.sh --instances 5
```

### Geographic Distribution
```yaml
# Multi-region deployment
regions:
  us-west:
    servers: 2
    domains: ["c2-west.yourdomain.com"]
  eu-central:
    servers: 2  
    domains: ["c2-eu.yourdomain.com"]
  asia-pacific:
    servers: 1
    domains: ["c2-apac.yourdomain.com"]
```

### Database Scaling
```bash
# Configure agent data storage
# Redis for session management
redis-server /etc/redis/nexus.conf

# PostgreSQL for task results and audit logs  
sudo -u postgres createdb nexus_production
psql nexus_production < schemas/nexus.sql
```

## Security Best Practices

### Certificate Management
- **Use Origin Certificates**: Cloudflare origin certs for backend security
- **Implement Certificate Pinning**: Validate specific certificate fingerprints
- **Automate Renewal**: Certificates renewed 30+ days before expiration
- **Monitor Transparency Logs**: Watch for unauthorized certificate issuance

### API Security
- **Rotate API Keys**: Monthly rotation of Cloudflare API tokens
- **Least Privilege**: Minimal required permissions for API tokens
- **Rate Limiting**: Implement API rate limiting and monitoring
- **Audit Logging**: Log all API calls with timestamps and results

### Network Security
- **Domain Fronting**: Route traffic through legitimate CDN endpoints
- **Geographic Distribution**: Use multiple regions for resilience
- **Connection Encryption**: Multiple layers of encryption (TLS + AES)
- **Traffic Analysis**: Monitor for unusual patterns or behaviors

### Operational Security
- **Infrastructure Rotation**: Regular domain and certificate rotation
- **Monitoring**: Comprehensive logging and alerting
- **Incident Response**: Prepared procedures for compromise scenarios
- **Access Control**: Role-based access to management interfaces

## Compliance & Auditing

### Audit Requirements
- **Access Logs**: All administrative access logged
- **Configuration Changes**: Version controlled configuration
- **Certificate Lifecycle**: Complete certificate management audit trail
- **Task Execution**: Detailed logs of all agent task execution

### Compliance Framework
```bash
# Generate compliance reports
./scripts/compliance-report.sh --framework SOC2 --period monthly

# Export audit logs
./scripts/export-audit-logs.sh --start-date 2024-01-01 --format json

# Certificate compliance check
./scripts/cert-compliance.sh --standards "TLS1.2,TLS1.3"
```

## Troubleshooting Production Issues

### Common Problems

#### Certificate Issues
```bash
# Certificate validation failing
openssl x509 -in cert.pem -text -noout | grep -A 2 "Validity"

# Check certificate chain
openssl verify -CAfile chain.pem cert.pem

# Regenerate if necessary
./target/release/nexus-infra certificates regenerate --force
```

#### DNS Problems
```bash
# DNS propagation issues
dig @8.8.8.8 subdomain.yourdomain.com
dig @1.1.1.1 subdomain.yourdomain.com

# Cloudflare API debugging
curl -H "Authorization: Bearer TOKEN" \
     "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dns_records"
```

#### gRPC Connectivity
```bash
# Test gRPC connectivity
grpcurl -insecure yourdomain.com:443 list

# Check TLS handshake
openssl s_client -connect yourdomain.com:443 -debug

# Analyze traffic
tcpdump -i eth0 port 443 -w grpc-debug.pcap
```

## Performance Optimization

### Server Tuning
```toml
# High-performance configuration
[grpc_server]
max_connections = 10000
connection_timeout = 60
keepalive_interval = 30
max_message_size = 33554432  # 32MB

# Linux kernel optimizations
echo 'net.core.somaxconn = 65535' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 65535' >> /etc/sysctl.conf
sysctl -p
```

### Database Optimization
```sql
-- PostgreSQL performance tuning
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';
SELECT pg_reload_conf();
```

## Maintenance Schedules

### Automated Tasks
```bash
# Certificate renewal (weekly)
0 2 * * 0 /opt/nexus/scripts/renew-certificates.sh

# Domain rotation (daily)  
0 3 * * * /opt/nexus/scripts/rotate-domains.sh

# Health checks (hourly)
0 * * * * /opt/nexus/scripts/health-check.sh

# Log cleanup (daily)
0 4 * * * /opt/nexus/scripts/cleanup-logs.sh
```

### Manual Tasks
- **Monthly**: API key rotation and security audit
- **Quarterly**: Infrastructure cost analysis and optimization
- **Semi-annually**: Disaster recovery testing
- **Annually**: Security assessment and penetration testing

This production deployment guide ensures enterprise-grade reliability, security, and maintainability for the Rust-Nexus C2 framework.
