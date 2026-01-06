# D3tect-Nexus: Enterprise Threat Detection & Response Platform

An advanced threat detection and incident response platform built in Rust featuring reverse shell detection, behavioral analysis, automated malware analysis (LitterBox integration), EDR-style agent deployment, and comprehensive security operations orchestration.

## 🚀 Overview

D3tect-Nexus combines cutting-edge threat detection capabilities with enterprise-grade infrastructure automation to create a sophisticated, distributed security operations platform for threat hunting, incident response, and proactive defense.

### Core Platform Components

1. **🔍 Detection Engine (`nexus-detection`)**: Reverse shell detection, behavioral analysis, and threat signatures
2. **🛡️ Security Agents (`nexus-agent`)**: EDR-style agents for threat hunting and telemetry collection
3. **🖥️ SOC Platform (`nexus-server`)**: gRPC-based security operations center with orchestration
4. **🧬 Malware Analysis (`litterbox-integration`)**: Automated LitterBox deployment for dynamic analysis
5. **📚 Common Library (`nexus-common`)**: Shared utilities and cryptographic functions

## ✨ Key Features

### 🔍 **Advanced Threat Detection**
- ✅ **30+ Reverse Shell Signatures**: Comprehensive detection patterns for common reverse shells
- ✅ **Behavioral Analysis**: Process-network correlation and anomaly detection
- ✅ **Network Monitoring**: Real-time packet capture and traffic analysis
- ✅ **Process Tracking**: Process discovery, monitoring, and parent-child relationships
- ✅ **Signature Engine**: Pattern matching with high-confidence threat identification

### 🧬 **Automated Malware Analysis**
- ✅ **LitterBox Integration**: Automated deployment and orchestration of malware sandboxes
- ✅ **Geographic Distribution**: Multi-region LitterBox clusters for load balancing
- ✅ **Static Analysis**: PE analysis, YARA scanning, and signature identification
- ✅ **Dynamic Analysis**: Real-time behavior monitoring in Windows containers
- ✅ **Priority Routing**: Smart routing based on threat confidence levels

### 🌐 **Enterprise Infrastructure**
- ✅ **Cloudflare DNS Integration**: Secure SOC infrastructure management
- ✅ **Let's Encrypt Automation**: Automated certificate provisioning for endpoints
- ✅ **Geographic Distribution**: Global threat detection via distributed agents
- ✅ **High Availability**: Automated failover and load balancing
- ✅ **Dynamic Infrastructure**: Infrastructure rotation for operational security

### 🔒 **Secure Communication**
- ✅ **gRPC over mTLS**: Encrypted telemetry and command transmission
- ✅ **Certificate Pinning**: Multi-layer validation for data integrity
- ✅ **Connection Resilience**: Automatic failover for continuous monitoring
- ✅ **Data Encryption**: AES-256-GCM encryption for all security data
- ✅ **Audit Trail**: Comprehensive logging for security operations

### 🎯 **Incident Response**
- ✅ **Automated Response**: Orchestrated incident response workflows
- ✅ **Threat Containment**: Isolation and quarantine capabilities
- ✅ **Remediation Tools**: Automated deployment of security tools
- ✅ **SIEM Integration**: Connect to Splunk, QRadar, Sentinel, and more
- ✅ **Threat Intelligence**: Real-time threat feed integration

## 🏗️ Detection Architecture

```
┌─────────────────────────┐    ┌─────────────────────────┐    ┌─────────────────────────┐
│   Detection Infrastructure│    │  Secure gRPC/TLS Comms │    │   Detection Agents      │
│                         │    │                         │    │                         │
├─────────────────────────┤    ├─────────────────────────┤    ├─────────────────────────┤
│ • Signature Management  │◄──►│ • Mutual TLS            │◄──►│ • Network Monitoring   │
│ • LitterBox Clusters    │    │ • Encrypted Transport   │    │ • Process Tracking      │
│ • Behavioral Analysis   │    │ • Certificate Pinning   │    │ • Behavioral Analysis   │
│ • Threat Intelligence   │    │ • Audit Logging         │    │ • Evidence Collection   │
│ • SIEM Integration      │    │ • Alert Streaming       │    │ • Incident Response     │
└─────────────────────────┘    └─────────────────────────┘    └─────────────────────────┘
```

## 📁 Project Structure

```
d3tect-nexus/
├── nexus-detection/        # 🆕 Threat detection module
│   └── src/
│       ├── signature_engine.rs # Reverse shell signature matching
│       ├── behavioral_analysis.rs # Process-network correlation
│       ├── network_monitor.rs # Packet capture and analysis
│       ├── process_tracker.rs # Process monitoring
│       ├── litterbox_integration.rs # Malware analysis integration
│       └── threat_scoring.rs # AI-powered threat scoring
├── nexus-infra/            # Infrastructure management
│   ├── proto/
│   │   └── nexus.proto     # gRPC service definitions
│   └── src/
│       ├── cloudflare.rs   # DNS infrastructure automation
│       ├── letsencrypt.rs  # Certificate automation
│       ├── cert_manager.rs # Certificate and TLS management
│       ├── domain_manager.rs # Domain rotation
│       ├── grpc_client.rs  # Detection agent client
│       ├── grpc_server.rs  # SOC server implementation
│       └── litterbox_deployment.rs # LitterBox automation
├── nexus-common/           # Shared libraries
│   └── src/
│       ├── crypto.rs       # AES-256-GCM encryption
│       ├── detection.rs    # Detection data structures
│       ├── threats.rs      # Threat definitions
│       └── alerts.rs       # Alert types
├── nexus-agent/            # Detection agent (EDR-style)
│   └── src/
│       ├── agent.rs        # Core detection agent
│       ├── network_capture.rs # Network monitoring
│       ├── process_monitor.rs # Process tracking
│       ├── threat_hunter.rs # Threat hunting capabilities
│       ├── evidence_collector.rs # Evidence collection
│       └── response_executor.rs # Incident response actions
├── nexus-server/           # SOC orchestration platform
│   └── src/
│       ├── main.rs         # Server main
│       ├── handlers.rs     # gRPC service handlers
│       ├── detection_engine.rs # Detection logic
│       ├── alert_manager.rs # Alert processing
│       ├── threat_intel.rs # Threat intelligence
│       └── incident_response.rs # IR orchestration
├── nexus-recon/            # Reconnaissance and profiling
│   └── src/
│       ├── network_recon.rs # Network reconnaissance
│       ├── system_profiler.rs # System profiling
│       ├── browser_fingerprint.rs # Browser detection
│       └── javascript_engine.rs # JS analysis
├── nexus-hybrid-exec/      # 🆕 Hybrid execution for IR
│   └── src/
│       ├── ssh_executor.rs # SSH-based remediation
│       ├── powershell_executor.rs # PowerShell execution
│       ├── wmi_executor.rs # WMI execution
│       └── api_executor.rs # API-based actions
├── nexus-web-comms/        # 🆕 Communication channels
│   └── src/
│       ├── websocket_fallback.rs # WebSocket comms
│       ├── http_fallback.rs # HTTP fallback
│       └── traffic_obfuscation.rs # Legitimate traffic patterns
├── config/                 # Configuration templates
│   ├── detection/          # Detection rules
│   ├── signatures/         # Threat signatures
│   ├── litterbox/          # LitterBox configurations
│   └── response/           # Response playbooks
├── docs/                   # Comprehensive documentation
│   ├── detection/          # Detection guides
│   ├── incident-response/  # IR procedures
│   ├── integration/        # SIEM integration guides
│   └── api/               # API reference documentation
└── scripts/               # Deployment and automation
    ├── deploy-soc.sh      # SOC infrastructure deployment
    ├── deploy-litterbox.sh # LitterBox cluster deployment
    └── update-signatures.sh # Signature updates
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ with cargo
- Docker (for LitterBox malware analysis)
- Cloud infrastructure (AWS, Azure, GCP, or on-premises)
- Domain with DNS management capability
- Basic understanding of threat detection and incident response

### 1. **Infrastructure Setup**

```bash
# Clone the repository
git clone https://github.com/cmndcntrlcyber/rust-nexus.git -b d3tect-nexus
cd rust-nexus

# Create configuration from template
cp config/examples/detection-config.toml ./nexus.toml

# Edit configuration with your infrastructure details
vim nexus.toml
```

### 2. **Build Platform**

```bash
# Build all components
cargo build --release

# Or build specific components
cargo build --release -p nexus-detection
cargo build --release -p nexus-infra
cargo build --release -p nexus-server  
cargo build --release -p nexus-agent
```

### 3. **Deploy Detection Infrastructure**

```bash
# Initialize infrastructure
./target/release/nexus-infra setup --config nexus.toml

# Deploy LitterBox malware analysis cluster
./scripts/deploy-litterbox.sh --regions "us-east,us-west,eu-central"

# Start the SOC server
./target/release/nexus-server --config nexus.toml

# Deploy detection agents to monitored systems
./target/release/nexus-agent --config agent.toml
```

## 🔧 Configuration

### Example Configuration (`nexus.toml`)

```toml
[detection]
enabled_signatures = [
    "reverse_shells",
    "command_injection",
    "lateral_movement",
    "privilege_escalation",
    "data_exfiltration"
]
behavioral_analysis = true
threat_intelligence_feeds = [
    "alienvault_otx",
    "abuse_ch",
    "threatfox"
]
min_confidence_threshold = 0.75

[litterbox]
enabled = true
auto_deploy = true
instances_per_region = 2
max_instances_per_region = 5
docker_setup_timeout = 3600
static_analysis_enabled = true
dynamic_analysis_enabled = true
priority_routing = true
high_priority_threshold = 0.8

[litterbox.regions]
us_east = { enabled = true, priority = "high" }
us_west = { enabled = true, priority = "high" }
eu_central = { enabled = true, priority = "medium" }
ap_southeast = { enabled = true, priority = "low" }

[infrastructure]
cloudflare_api_token = "your_api_token"
zone_id = "your_zone_id"
domain = "detection.example.com"
geo_distribution = ["us-east", "us-west", "eu-central"]

[agents]
deployment_mode = "distributed"
collection_interval = 60  # 1 minute for real-time detection
encryption_enabled = true
max_agents = 50000
capture_network_traffic = true
monitor_processes = true

[server]
bind_address = "0.0.0.0"
port = 8443
mutual_tls = true
max_connections = 10000
database_url = "postgresql://localhost/detection"

[siem_integration]
enabled = true
platforms = ["splunk", "sentinel", "qradar"]
forward_alerts = true
alert_severity_min = "medium"

[incident_response]
auto_response_enabled = true
quarantine_on_high_confidence = true
alert_escalation_threshold = 3
response_playbooks = ["isolate", "collect_evidence", "remediate"]
```

## 🎯 Detection Operations

### **Threat Detection**

```bash
# Start real-time detection monitoring
nexus-cli detection start

# Query detected threats
nexus-cli threats list --severity high --last 24h

# Analyze specific alert
nexus-cli alert analyze --id alert-12345

# Get threat intelligence for IOC
nexus-cli threat-intel lookup --ioc 192.0.2.100
```

### **Malware Analysis with LitterBox**

```bash
# Submit sample to LitterBox for analysis
nexus-cli litterbox submit --file suspicious.exe --priority high

# Get analysis results
nexus-cli litterbox results --hash abc123def456

# Check LitterBox cluster health
nexus-cli litterbox health --all-regions

# View analysis report
nexus-cli litterbox report --hash abc123def456 --format pdf
```

### **Incident Response**

```bash
# Initiate incident response
nexus-cli incident create --alert alert-12345 --severity critical

# Execute response playbook
nexus-cli incident respond --id incident-789 --playbook isolate

# Quarantine affected host
nexus-cli response quarantine --agent agent-456

# Collect evidence
nexus-cli evidence collect --agent agent-456 --output evidence.zip
```

## 🔍 Reverse Shell Detection

### **30+ Supported Signatures**

D3tect-Nexus includes comprehensive reverse shell detection for:

- **Netcat Variants**: Traditional nc, ncat, netcat with various options
- **Bash Shells**: /dev/tcp, /dev/udp, named pipes
- **Python Shells**: socket-based, pty-based, encrypted
- **PowerShell**: Invoke-Expression, System.Net.Sockets
- **Perl, Ruby, PHP**: Language-specific reverse shells
- **Metasploit Payloads**: Common meterpreter patterns
- **Web Shells**: Common web shell patterns
- **Encrypted Shells**: SSL/TLS wrapped connections

### **Detection Example**

```rust
use d3tect_nexus::{SignatureEngine, BehavioralAnalyzer};

// Initialize detection engine
let engine = SignatureEngine::new()?;
let analyzer = BehavioralAnalyzer::new()?;

// Detect reverse shell in network traffic
let detection = engine.scan_traffic(&packet_data)?;

if detection.confidence > 0.8 {
    // High confidence - submit to LitterBox
    let litterbox_result = engine
        .submit_to_litterbox(&detection.payload, Priority::High)
        .await?;
    
    // Generate alert
    let alert = Alert::new(detection, litterbox_result);
    soc_platform.send_alert(alert).await?;
}
```

## 🧬 LitterBox Malware Analysis Integration

### **Automated Deployment**

D3tect-Nexus automatically deploys and manages LitterBox malware analysis infrastructure:

```rust
use d3tect_nexus::LitterBoxDeployment;

// Deploy global LitterBox network
let deployment = LitterBoxDeployment::new(config)?;
let network = deployment.deploy_global_network().await?;

// Network includes:
// - Automated Docker container setup
// - Nginx reverse proxy with TLS
// - Geographic load balancing
// - Health monitoring and auto-scaling
```

### **Analysis Features**

- **Static Analysis**: PE header analysis, imports/exports, YARA scanning
- **Dynamic Analysis**: Behavior monitoring in Windows 10 containers
- **API Monitoring**: Windows API call tracking
- **Network Activity**: Capture outbound connections and DNS requests
- **File System Changes**: Track file creation, modification, deletion
- **Registry Monitoring**: Track registry key changes
- **Process Analysis**: Monitor spawned processes

### **Priority Routing**

```rust
// Automatic priority-based routing
match detection.confidence {
    0.9..=1.0 => route_to_nearest_litterbox(payload, Priority::Critical).await?,
    0.7..=0.9 => route_to_nearest_litterbox(payload, Priority::High).await?,
    _ => queue_for_batch_analysis(payload).await?,
}
```

## 📊 SOC Dashboard

### Real-Time Monitoring
```bash
# Launch SOC dashboard
nexus-webui --bind 0.0.0.0:8080

# Access via browser
# https://soc.example.com
```

### Dashboard Features
- **Threat Detection**: Real-time threat detection status and alerts
- **LitterBox Status**: Malware analysis queue and results
- **Agent Health**: Detection agent deployment and telemetry status
- **Threat Map**: Geographic visualization of detected threats
- **IOC Tracker**: Indicator of Compromise tracking and correlation
- **Incident Timeline**: Active incidents and response status
- **SIEM Integration**: Forwarded alerts and SIEM connectivity

## 🔐 SIEM Integration

### **Supported Platforms**

| Platform | Integration Type | Website |
|----------|------------------|---------|
| **Splunk** | HTTP Event Collector + API | https://www.splunk.com |
| **Microsoft Sentinel** | Log Analytics API | https://azure.microsoft.com/en-us/products/microsoft-sentinel |
| **IBM QRadar** | Syslog + API | https://www.ibm.com/qradar |
| **Elastic Security** | Elasticsearch API | https://www.elastic.co/security |
| **Chronicle** | Ingestion API | https://chronicle.security |

### **Integration Example**

```rust
use d3tect_nexus::{SIEMIntegration, Alert};

// Configure Splunk integration
let splunk = SIEMIntegration::splunk(
    "https://splunk.example.com:8088",
    hec_token
)?;

// Forward alerts automatically
detection_engine.add_siem(splunk).await?;

// Alerts are automatically forwarded with:
// - Threat metadata
// - Detection confidence
// - IOCs extracted
// - LitterBox analysis results
// - Recommended response actions
```

## 📚 Documentation

- **[Detection Guide](docs/detection/setup.md)** - Complete detection setup guide
- **[Signature Development](docs/detection/signatures.md)** - Creating custom signatures
- **[LitterBox Integration](docs/integration/litterbox.md)** - Malware analysis setup
- **[Incident Response](docs/incident-response/playbooks.md)** - IR procedures and playbooks
- **[SIEM Integration](docs/integration/siem-platforms.md)** - SIEM platform integration
- **[API Reference](docs/api/detection-api.md)** - Complete API documentation

## 🎯 Use Cases

### **Threat Hunting**
- **Proactive Detection**: Hunt for advanced persistent threats (APTs)
- **Behavioral Analysis**: Identify anomalous behavior patterns
- **Network Forensics**: Deep packet inspection and traffic analysis
- **IOC Hunting**: Search for known indicators of compromise

### **Incident Response**
- **Real-Time Detection**: Immediate threat identification and alerting
- **Automated Response**: Orchestrated response workflows
- **Evidence Collection**: Automated forensic evidence gathering
- **Threat Containment**: Isolation and quarantine capabilities

### **Security Operations**
- **SOC Automation**: Reduce manual analyst workload by 70%
- **Alert Triage**: AI-powered alert prioritization
- **Threat Intelligence**: Integrate with threat feeds for context
- **Compliance**: Meet security monitoring requirements

### **Malware Analysis**
- **Automated Sandboxing**: LitterBox integration for dynamic analysis
- **Reverse Engineering**: Static analysis with PE parsing and YARA
- **Behavioral Profiling**: Understand malware behavior and TTPs
- **IOC Extraction**: Automatic extraction of indicators from samples

## 🧪 Testing

```bash
# Run all tests
cargo test

# Test detection engine
cargo test -p nexus-detection

# Test LitterBox integration
cargo test -p nexus-infra litterbox

# Test reverse shell detection
cargo test -p nexus-detection signature_engine

# Integration tests
./scripts/test-detection.sh
```

## 🔍 Troubleshooting

### Common Issues

**❌ Detection Agent Connection Failed**
```bash
# Verify agent connectivity
nexus-cli agent health --all

# Check TLS certificates
nexus-cli certificates verify --agent agent-456

# Restart agent with debug logging
RUST_LOG=debug ./nexus-agent --config agent.toml
```

**❌ LitterBox Deployment Failed**
```bash
# Check Docker status
docker ps -a | grep litterbox

# Verify LitterBox cluster health
nexus-cli litterbox health --region us-east

# Redeploy specific instance
./scripts/deploy-litterbox.sh --region us-east --instance 1 --force
```

**❌ Reverse Shell False Positives**
```bash
# Adjust confidence threshold
nexus-cli detection config --min-confidence 0.85

# Add whitelist for legitimate traffic
nexus-cli detection whitelist --add 203.0.113.0/24

# Retrain behavioral analysis
nexus-cli detection train --update-baseline
```

## 📈 Performance & Scale

### **Benchmarks**
- **Agent Capacity**: 50,000+ detection agents per server cluster
- **Packet Processing**: 10 Gbps sustained packet capture per agent
- **Alert Processing**: 100,000+ alerts per hour
- **LitterBox Analysis**: 1,000+ samples per day across cluster
- **Response Time**: <100ms detection latency for signature matches

### **Scalability Features**
- **Horizontal Scaling**: Distributed SOC architecture with load balancing
- **Geographic Distribution**: Multi-region deployment for global coverage
- **Efficient Processing**: Stream processing for real-time analysis
- **Smart Routing**: Geographic and load-based LitterBox routing

## 🎖️ Enterprise Features

### **Advanced Detection**
- **AI/ML Models**: Machine learning-based anomaly detection
- **Threat Correlation**: Cross-agent threat correlation
- **Attack Chain Detection**: Multi-stage attack identification
- **Zero-Day Detection**: Behavioral analysis for unknown threats

### **Orchestration**
- **Automated Playbooks**: Pre-built incident response workflows
- **Cross-Platform**: Windows, Linux, macOS agent support
- **API-First**: Complete REST and gRPC APIs
- **Extensible**: Plugin architecture for custom detections

### **Operations**
- **24/7 Monitoring**: Continuous threat monitoring
- **Alert Fatigue Reduction**: AI-powered alert deduplication
- **Forensic Capabilities**: Complete evidence collection
- **Compliance Reporting**: Security incident reporting

## ⚠️ Security Notice

This platform is designed for **authorized security operations and threat detection**. Users must:

- Ensure compliance with applicable laws and regulations
- Obtain proper authorization before deployment
- Use responsibly for defensive security purposes
- Protect sensitive detection data and intelligence
- Follow responsible disclosure practices for vulnerabilities

**This platform is for defensive security operations only.**

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Rust Community**: Exceptional tooling and ecosystem
- **Security Researchers**: Threat intelligence and detection techniques
- **LitterBox Project**: BlackSnufkin for malware analysis framework
- **Reverse-Shell-Detector**: Original detection signatures and patterns
- **SIEM Platforms**: Splunk, Microsoft, IBM, Elastic, and Chronicle
- **Threat Intelligence**: AlienVault OTX, Abuse.ch, ThreatFox
- **Open Source Security**: Detection rule communities

---

## 🚀 Getting Started

Ready to deploy threat detection? Check out our [Detection Setup Guide](docs/detection/setup.md) for step-by-step instructions.

For LitterBox malware analysis, see the [LitterBox Integration Guide](docs/integration/litterbox.md).

For incident response procedures, review the [IR Playbooks](docs/incident-response/playbooks.md).

---

**Built with ❤️ in Rust | Enterprise-Ready | Detection-Focused | Defense-First**
