# Cross-Platform Deployment Guide

This guide covers the complete process of deploying Rust-Nexus C2 framework across different platforms, with a focus on Ubuntu 22.04 server deployment and Windows/Linux agent compilation.

## Overview

The Rust-Nexus framework now supports streamlined cross-platform deployment with:

- **Server**: Ubuntu 22.04 optimized deployment
- **Agents**: Windows and Linux with platform-specific configurations
- **Build System**: Enhanced cross-compilation support
- **Configuration**: Platform-specific TOML configurations
- **Deployment**: Automated installation scripts

## Platform-Specific Configurations

### Agent Configurations

#### Linux Agent (`config/agent-linux.toml`)
- **Persistence**: systemd, cron, bashrc, systemd_user
- **Evasion**: process_hollowing, ld_preload, namespace_escape
- **Execution**: ELF loading, script execution, privilege escalation
- **System Info**: Linux-specific reconnaissance
- **Capabilities**: Container escape, namespace manipulation, SELinux/AppArmor bypass

#### Windows Agent (`config/agent-windows.toml`)
- **Persistence**: Windows Service, Registry, Startup, WMI, Scheduled Tasks
- **Evasion**: DLL injection, AMSI bypass, ETW bypass, process hollowing
- **Execution**: PowerShell, BOF loading, WMI execution
- **System Info**: WMI queries, Registry collection
- **Capabilities**: Token manipulation, UAC bypass, reflective DLL loading

### Server Configuration
- **Platform**: Ubuntu 22.04 LTS optimized
- **Protocol**: gRPC with mutual TLS
- **Certificates**: Let's Encrypt integration
- **Security**: Hardened systemd service, fail2ban protection
- **Monitoring**: Comprehensive logging and metrics

## Build System Usage

### Enhanced Build Script (`scripts/build.sh`)

```bash
# Build all platforms (agents + server)
./scripts/build.sh

# Build specific platform
./scripts/build.sh release server    # Server only
./scripts/build.sh release agents    # Agents only

# Build with debug information
./scripts/build.sh debug all
```

### Platform-Specific Build Scripts

#### Linux Agents
```bash
# Build Linux agent (native)
cargo build --release --bin nexus-agent-linux

# Output: target/release/nexus-agent-linux
```

#### Windows Agents (Cross-compilation from Linux)
```bash
# Build Windows agent with cross-compilation
cargo build --release --bin nexus-agent-windows --target x86_64-pc-windows-gnu

# Output: target/x86_64-pc-windows-gnu/release/nexus-agent-windows.exe
```

#### Using Build Scripts
```bash
# Use enhanced build script for all platforms
./scripts/build.sh release agents

# Outputs:
# - target/builds/nexus-agent-linux
# - target/builds/nexus-agent-windows-x86_64-pc-windows-gnu.exe
# - Platform-specific configuration files
```

### Cross-Compilation Setup

#### Prerequisites
```bash
# Install Rust targets
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf

# For Windows hosts
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc
```

#### Feature Flags
The build system automatically applies platform-specific features:

**Linux Features:**
- `linux-specific`: Linux-only code paths
- `elf-loading`: ELF binary execution
- `systemd-integration`: systemd service management
- `anti-debug`: Linux debugging detection
- `anti-vm`: Virtual machine detection

**Windows Features:**
- `windows-specific`: Windows-only code paths
- `bof-loading`: Beacon Object File execution
- `wmi-execution`: WMI query execution
- `process-injection`: Process injection techniques
- `anti-debug`: Windows debugging detection

## Ubuntu 22.04 Server Deployment

### Automated Deployment

```bash
# Deploy complete server environment
sudo ./scripts/deploy-ubuntu-server.sh
```

### Manual Deployment Steps

1. **System Preparation**
```bash
# Update system
apt update && apt upgrade -y

# Install dependencies
apt install -y curl wget git build-essential pkg-config libssl-dev
```

2. **Install Rust Toolchain**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

3. **Build Server**
```bash
cd nexus-server
cargo build --release
```

4. **Deploy Server**
```bash
# Copy binary
sudo cp target/release/nexus-server /opt/nexus/bin/

# Create configuration
sudo cp nexus.toml.example /etc/nexus/nexus.toml
sudo nano /etc/nexus/nexus.toml

# Start service
sudo systemctl start nexus-server
sudo systemctl enable nexus-server
```

### Server Management

#### Using Management Scripts
```bash
# Server control
/opt/nexus/bin/nexus-ctl start|stop|restart|status|logs

# Monitoring
/opt/nexus/bin/monitor-nexus

# Backup
/opt/nexus/bin/backup-nexus

# Updates
/opt/nexus/bin/update-nexus
```

#### Manual Management
```bash
# Service management
systemctl status nexus-server
systemctl start nexus-server
systemctl stop nexus-server

# Log monitoring
journalctl -u nexus-server -f
tail -f /var/log/nexus/nexus.log

# Configuration
nano /etc/nexus/nexus.toml
```

## Agent Deployment

### Linux Agent Deployment

#### Using Deployment Package
```bash
# Extract package
tar -xzf nexus-linux-YYYYMMDD_HHMMSS.tar.gz
cd nexus-linux-YYYYMMDD_HHMMSS

# Run installation script
sudo ./install.sh
```

#### Manual Installation
```bash
# Copy Linux agent binary (use correct path from build)
sudo cp target/release/nexus-agent-linux /opt/nexus/nexus-agent
# Or if using cross-compilation build output:
# sudo cp target/builds/nexus-agent-linux /opt/nexus/nexus-agent

# Copy platform-specific configuration
sudo cp config/agent-linux.toml.example /opt/nexus/agent.toml

# Configure
sudo nano /opt/nexus/agent.toml

# Create systemd service
sudo systemctl enable nexus-agent
sudo systemctl start nexus-agent
```

### Windows Agent Deployment

#### Using Deployment Package
```batch
REM Extract nexus-windows-YYYYMMDD_HHMMSS.zip
REM Right-click install.bat -> "Run as administrator"
```

#### Using PowerShell
```powershell
# Extract package
Expand-Archive nexus-windows-YYYYMMDD_HHMMSS.zip
cd nexus-windows-YYYYMMDD_HHMMSS

# Run installation script as Administrator
.\install.ps1
```

#### Manual Installation
```batch
REM Copy Windows agent binary (use correct path from cross-compilation build)
copy target\x86_64-pc-windows-gnu\release\nexus-agent-windows.exe "C:\Program Files\Nexus\nexus-agent.exe"
REM Or if using build script output:
REM copy target\builds\nexus-agent-windows-x86_64-pc-windows-gnu.exe "C:\Program Files\Nexus\nexus-agent.exe"

REM Copy platform-specific configuration
copy config\agent-windows.toml.example "C:\Program Files\Nexus\agent.toml"

REM Create service
sc create "NexusAgent" binPath= "\"C:\Program Files\Nexus\nexus-agent.exe\" --config \"C:\Program Files\Nexus\agent.toml\"" start= auto
sc start "NexusAgent"
```

## Cross-Platform Connectivity Verification

### Server Verification

1. **Check Server Status**
```bash
# Service status
systemctl status nexus-server

# Network listening
ss -tlnp | grep :8443

# Logs
journalctl -u nexus-server -n 50
```

2. **Certificate Verification**
```bash
# Check certificate validity
openssl s_client -connect localhost:8443 -servername your-domain.com

# Verify mutual TLS
openssl s_client -connect localhost:8443 -cert /opt/nexus/certs/client.crt -key /opt/nexus/certs/client.key
```

### Agent Connectivity Testing

#### Linux Agent
```bash
# Check agent status
systemctl status nexus-agent

# Test connectivity
telnet your-c2-domain.com 8443

# Check logs
journalctl -u nexus-agent -f
```

#### Windows Agent
```batch
REM Check service status
sc query NexusAgent

REM Test connectivity
telnet your-c2-domain.com 8443

REM Check event logs
eventvwr.msc
```

### Protocol Testing

1. **gRPC Health Check**
```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# Test server health
grpcurl -insecure localhost:8443 grpc.health.v1.Health/Check
```

2. **Manual Connection Test**
```rust
// Test code for direct connection
use tonic::transport::{Channel, ClientTlsConfig};

let channel = Channel::from_static("https://your-c2-domain.com:8443")
    .tls_config(ClientTlsConfig::new().domain_name("your-c2-domain.com"))?
    .connect()
    .await?;
```

## Security Considerations

### Server Hardening

1. **Firewall Configuration**
```bash
# UFW rules (automatically configured by deployment script)
ufw allow ssh
ufw allow 8443/tcp
ufw allow 443/tcp
ufw allow 80/tcp
ufw default deny incoming
```

2. **Fail2ban Protection**
```bash
# Check fail2ban status
fail2ban-client status nexus-c2

# Unban IP if needed
fail2ban-client set nexus-c2 unbanip <IP>
```

3. **Certificate Security**
```bash
# Verify certificate permissions
ls -la /opt/nexus/certs/
# Should be owned by nexus:nexus with 600 permissions

# Rotate certificates
/opt/nexus/bin/rotate-certificates.sh
```

### Agent Security

#### Linux Agent
- Run as non-privileged user when possible
- Use systemd service hardening
- Monitor system logs for detection

#### Windows Agent
- Use Windows Service for persistence
- Configure Windows Firewall exclusions
- Monitor Windows Event Logs
- Consider antivirus exclusions for testing

## Troubleshooting

### Common Issues

#### Build Failures

**Issue**: Cross-compilation target not available
```bash
error[E0463]: can't find crate for `core`
```
**Solution**: Install the target
```bash
rustup target add x86_64-pc-windows-gnu
```

**Issue**: Missing system dependencies
```bash
error: failed to run custom build command for `openssl-sys`
```
**Solution**: Install development packages
```bash
# Ubuntu/Debian
apt install -y pkg-config libssl-dev

# CentOS/RHEL
yum install -y openssl-devel pkg-config
```

#### Connection Issues

**Issue**: Agent cannot connect to server
1. Check server is listening: `ss -tlnp | grep :8443`
2. Verify DNS resolution: `nslookup your-c2-domain.com`
3. Test connectivity: `telnet your-c2-domain.com 8443`
4. Check certificates: Certificate expiry and validity
5. Verify firewall rules: Both server and client sides

**Issue**: TLS handshake failures
1. Verify certificate chains
2. Check clock synchronization
3. Validate certificate domains
4. Review mutual TLS configuration

#### Performance Issues

**Issue**: High memory usage
- Review agent configuration limits
- Check for memory leaks in logs
- Monitor system resources

**Issue**: High CPU usage
- Adjust agent heartbeat intervals
- Review enabled features
- Check for tight loops in logs

### Logging and Monitoring

#### Server Logs
```bash
# Application logs
tail -f /var/log/nexus/nexus.log

# System logs
journalctl -u nexus-server -f

# Access logs
grep "agent registration" /var/log/nexus/nexus.log
```

#### Agent Logs

**Linux:**
```bash
journalctl -u nexus-agent -f
```

**Windows:**
```powershell
Get-WinEvent -LogName Application | Where-Object {$_.ProviderName -eq "NexusAgent"}
```

### Debug Mode

#### Enable Debug Logging
```toml
# In configuration file
[logging]
level = "debug"
verbose_logging = true
```

#### Build with Debug Information
```bash
# Debug build
./scripts/build.sh debug all

# With debug features
cargo build --features debug-mode
```

## Performance Tuning

### Server Optimization

```toml
[grpc_server]
max_connections = 5000
connection_timeout = 60
keepalive_interval = 30

[performance]
worker_threads = 8
max_memory = "2GB"
```

### Agent Optimization

```toml
[performance]
heartbeat_interval = 60  # Reduce frequency
max_memory_usage = "50MB"
cpu_limit_percent = 5
```

### Network Optimization

```toml
[communication]
connection_timeout = 30
retry_backoff = 5.0
max_retries = 3
```

## Best Practices

### Development
1. Use platform-specific feature flags
2. Test on target platforms regularly
3. Implement proper error handling
4. Use structured logging

### Deployment
1. Always use TLS/mutual TLS
2. Implement proper certificate management
3. Use configuration management
4. Monitor resource usage

### Security
1. Regular security updates
2. Certificate rotation
3. Log monitoring
4. Network segmentation

### Operational
1. Automated backups
2. Health monitoring
3. Performance metrics
4. Incident response procedures

## Support and Resources

### Documentation
- [Server Configuration](../configuration/production-setup.md)
- [Security Hardening](../security/SECURITY_HARDENING.md)
- [Troubleshooting Guide](../troubleshooting/FAQ.md)

### Build Artifacts
- Server binary: `target/builds/nexus-server`
- Linux agents: `target/builds/nexus-agent-linux-*`
- Windows agents: `target/builds/nexus-agent-windows-*`
- Deployment packages: `target/builds/nexus-{linux,windows}-*`

### Community
- GitHub Issues: Report bugs and feature requests
- Security Issues: Use responsible disclosure
- Contributions: Follow contribution guidelines
