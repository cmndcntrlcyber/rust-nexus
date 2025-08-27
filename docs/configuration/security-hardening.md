# Security Hardening Guide

This guide provides comprehensive security hardening recommendations for production Rust-Nexus deployments, covering infrastructure, application, and operational security.

## Infrastructure Security

### Certificate Management

#### TLS Configuration
```toml
# Use strong TLS configuration
[grpc_server]
mutual_tls = true
min_tls_version = "TLS1.3"
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_CHACHA20_POLY1305_SHA256",
    "TLS_AES_128_GCM_SHA256"
]

[origin_cert]
pin_validation = true
key_algorithm = "ECDSA-P256"  # Use ECDSA for better performance
```

#### Certificate Pinning Implementation
```rust
// Implement strict certificate pinning
use sha2::{Sha256, Digest};

pub fn validate_certificate_pin(cert_der: &[u8], expected_pins: &[String]) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(cert_der);
    let fingerprint = format!("sha256:{:x}", hasher.finalize());
    
    expected_pins.contains(&fingerprint)
}

// Use in gRPC client
let pinned_fingerprints = vec![
    "sha256:1234567890abcdef...".to_string(),
    "sha256:fedcba0987654321...".to_string(), // Backup certificate
];

if !validate_certificate_pin(&peer_cert, &pinned_fingerprints) {
    return Err(InfraError::TlsError("Certificate pinning validation failed"));
}
```

### Domain Security

#### Advanced Domain Generation
```toml
# Use sophisticated domain generation patterns
[domains.subdomain_pattern]
type = "Custom"
template = "{word}-{random}-{hash}"

# Where:
# {word} = legitimate-looking word from dictionary
# {random} = random alphanumeric string
# {hash} = hash of current timestamp
```

#### Domain Reputation Monitoring
```rust
// Monitor domain reputation
use reqwest::Client;

async fn check_domain_reputation(domain: &str) -> InfraResult<bool> {
    let client = Client::new();
    
    // Check against multiple reputation services
    let services = vec![
        format!("https://safebrowsing.googleapis.com/v4/threatMatches:find?key={}", api_key),
        format!("https://www.virustotal.com/vtapi/v2/domain/report?apikey={}&domain={}", vt_key, domain),
    ];
    
    for service_url in services {
        let response = client.get(&service_url).send().await?;
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            // Parse response and check reputation
        }
    }
    
    Ok(true) // Domain reputation is clean
}
```

### Network Security

#### Traffic Obfuscation
```rust
// Implement traffic obfuscation
pub struct TrafficObfuscator {
    fake_headers: Vec<(String, String)>,
    timing_jitter: std::ops::Range<u64>,
    size_padding: std::ops::Range<usize>,
}

impl TrafficObfuscator {
    pub fn new() -> Self {
        Self {
            fake_headers: vec![
                ("X-Requested-With".to_string(), "XMLHttpRequest".to_string()),
                ("Accept".to_string(), "application/json, text/plain, */*".to_string()),
                ("Cache-Control".to_string(), "no-cache".to_string()),
            ],
            timing_jitter: 100..2000,  // 100ms to 2s jitter
            size_padding: 0..512,      // Up to 512 bytes padding
        }
    }
    
    pub async fn obfuscate_request(&self, mut request: tonic::Request<T>) -> tonic::Request<T> {
        // Add fake headers
        for (key, value) in &self.fake_headers {
            request.metadata_mut().insert(key, value.parse().unwrap());
        }
        
        // Add timing jitter
        let jitter = rand::thread_rng().gen_range(self.timing_jitter.clone());
        tokio::time::sleep(Duration::from_millis(jitter)).await;
        
        request
    }
}
```

## Application Security

### Memory Protection

#### Secure Memory Allocation
```rust
// Implement secure memory management for sensitive data
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr;

pub struct SecureMemory {
    ptr: *mut u8,
    size: usize,
    layout: Layout,
}

impl SecureMemory {
    pub fn new(size: usize) -> Result<Self, std::alloc::AllocError> {
        let layout = Layout::from_size_align(size, 16)?;
        
        unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                return Err(std::alloc::AllocError);
            }
            
            // Lock memory to prevent swapping (Unix-like systems)
            #[cfg(unix)]
            libc::mlock(ptr as *const libc::c_void, size);
            
            Ok(Self { ptr, size, layout })
        }
    }
    
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for SecureMemory {
    fn drop(&mut self) {
        unsafe {
            // Zero memory before deallocation
            ptr::write_bytes(self.ptr, 0, self.size);
            
            #[cfg(unix)]
            libc::munlock(self.ptr as *const libc::c_void, self.size);
            
            dealloc(self.ptr, self.layout);
        }
    }
}
```

#### BOF Sandbox Security
```rust
// Enhanced BOF security sandbox
pub struct BofSandbox {
    allowed_apis: HashSet<String>,
    memory_limit: usize,
    execution_timeout: Duration,
    syscall_filter: Vec<u32>,
}

impl BofSandbox {
    pub fn new() -> Self {
        Self {
            allowed_apis: Self::create_api_allowlist(),
            memory_limit: 50 * 1024 * 1024, // 50MB limit
            execution_timeout: Duration::from_secs(30),
            syscall_filter: Self::create_syscall_filter(),
        }
    }
    
    fn create_api_allowlist() -> HashSet<String> {
        vec![
            "kernel32.dll!GetCurrentProcess",
            "kernel32.dll!GetCurrentThread", 
            "kernel32.dll!VirtualAlloc",
            "kernel32.dll!VirtualFree",
            "kernel32.dll!GetLastError",
            // Add other safe APIs
        ].into_iter().map(String::from).collect()
    }
    
    pub fn validate_api_call(&self, api_name: &str) -> bool {
        self.allowed_apis.contains(api_name)
    }
    
    pub fn execute_with_limits<F, R>(&self, f: F) -> InfraResult<R>
    where
        F: FnOnce() -> R,
    {
        // Set memory limits, execution timeout, etc.
        // Implementation depends on platform
        Ok(f())
    }
}
```

### Anti-Analysis Enhancements

#### Advanced VM Detection
```rust
// Enhanced VM detection techniques
pub struct EnhancedVMDetection;

impl EnhancedVMDetection {
    pub async fn is_virtual_environment() -> bool {
        // Multiple detection vectors
        Self::check_hypervisor_vendors() ||
        Self::check_system_artifacts() ||
        Self::check_performance_indicators() ||
        Self::check_network_indicators() ||
        Self::check_hardware_indicators()
    }
    
    fn check_hypervisor_vendors() -> bool {
        // Check for hypervisor vendor strings
        let vendors = ["VMware", "VirtualBox", "QEMU", "Microsoft Corporation", "Xen"];
        
        #[cfg(windows)]
        {
            // Check registry for VM indicators
            use winreg::RegKey;
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            
            if let Ok(system_key) = hklm.open_subkey("HARDWARE\\DESCRIPTION\\System") {
                if let Ok(bios_vendor: String) = system_key.get_value("SystemBiosVendor") {
                    return vendors.iter().any(|&v| bios_vendor.contains(v));
                }
            }
        }
        
        false
    }
    
    fn check_performance_indicators() -> bool {
        // Timing-based detection
        use std::time::Instant;
        
        let start = Instant::now();
        
        // CPU-intensive operation that shows timing differences in VMs
        let mut sum = 0u64;
        for i in 0..1000000 {
            sum = sum.wrapping_add(i);
        }
        
        let duration = start.elapsed();
        
        // VMs typically show slower performance for CPU-bound operations
        duration.as_millis() > 50 // Threshold indicating VM
    }
    
    fn check_network_indicators() -> bool {
        // Check for VM-specific network configurations
        #[cfg(unix)]
        {
            use std::fs;
            if let Ok(contents) = fs::read_to_string("/proc/net/arp") {
                return contents.contains("00:50:56") ||  // VMware MAC prefix
                       contents.contains("08:00:27");   // VirtualBox MAC prefix
            }
        }
        
        false
    }
    
    fn check_hardware_indicators() -> bool {
        // Check CPU features and hardware configuration
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;
            unsafe {
                let cpuid = __cpuid(0x40000000);
                // Check hypervisor bit and vendor signatures
                return cpuid.ecx != 0 || cpuid.edx != 0;
            }
        }
        
        false
    }
    
    fn check_system_artifacts() -> bool {
        // Check for VM-specific files and registry entries
        let vm_artifacts = vec![
            "/proc/vz/version",           // OpenVZ
            "/proc/xen/capabilities",     // Xen
            "/sys/hypervisor/uuid",       // Various hypervisors
            "/dev/vmware",                // VMware tools
        ];
        
        vm_artifacts.iter().any(|&path| std::path::Path::new(path).exists())
    }
}
```

#### Debugger Detection
```rust
// Multi-vector debugger detection
pub struct DebuggerDetection;

impl DebuggerDetection {
    pub fn is_debugger_present() -> bool {
        Self::check_debug_flags() ||
        Self::check_debug_ports() ||
        Self::check_debug_processes() ||
        Self::check_timing_attacks()
    }
    
    #[cfg(windows)]
    fn check_debug_flags() -> bool {
        use windows_sys::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
        unsafe { IsDebuggerPresent() != 0 }
    }
    
    fn check_debug_processes() -> bool {
        // Check for known debugger processes
        let debugger_processes = [
            "windbg.exe", "x64dbg.exe", "x32dbg.exe", "ollydbg.exe",
            "gdb", "lldb", "ida.exe", "ida64.exe", "ghidra",
        ];
        
        // Implementation to check running processes
        // Platform-specific process enumeration required
        false
    }
    
    fn check_timing_attacks() -> bool {
        use std::time::Instant;
        
        let start = Instant::now();
        
        // Operation that should take consistent time
        let mut result = 0u32;
        for i in 0..10000 {
            result ^= i;
        }
        
        let duration = start.elapsed();
        
        // Debuggers often cause timing anomalies
        duration.as_micros() > 1000 // Threshold indicating debugging
    }
    
    #[cfg(windows)]
    fn check_debug_ports() -> bool {
        use std::mem;
        use windows_sys::Win32::Foundation::*;
        use windows_sys::Win32::System::Diagnostics::Debug::*;
        
        unsafe {
            let mut debug_port = 0u32;
            let status = NtQueryInformationProcess(
                GetCurrentProcess(),
                7, // ProcessDebugPort
                &mut debug_port as *mut _ as *mut _,
                mem::size_of::<u32>() as u32,
                ptr::null_mut(),
            );
            
            status == 0 && debug_port != 0
        }
    }
}
```

## Communication Security

### Enhanced Encryption
```rust
// Multi-layer encryption implementation
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rsa::{RsaPublicKey, RsaPrivateKey, PaddingScheme};

pub struct EnhancedCrypto {
    aes_key: Key<Aes256Gcm>,
    rsa_keypair: (RsaPrivateKey, RsaPublicKey),
    cipher: Aes256Gcm,
}

impl EnhancedCrypto {
    pub fn new() -> InfraResult<Self> {
        // Generate strong AES key
        let aes_key = Aes256Gcm::generate_key(rand::thread_rng());
        
        // Generate RSA keypair for key exchange
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 4096)?;
        let public_key = private_key.to_public_key();
        
        let cipher = Aes256Gcm::new(&aes_key);
        
        Ok(Self {
            aes_key,
            rsa_keypair: (private_key, public_key),
            cipher,
        })
    }
    
    pub fn encrypt_message(&self, plaintext: &[u8]) -> InfraResult<Vec<u8>> {
        let nonce = Nonce::from_slice(&rand::random::<[u8; 12]>());
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| InfraError::CryptographicError(format!("Encryption failed: {}", e)))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    pub fn decrypt_message(&self, encrypted_data: &[u8]) -> InfraResult<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(InfraError::CryptographicError("Invalid encrypted data".to_string()));
        }
        
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| InfraError::CryptographicError(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }
}
```

### Domain Fronting Security
```rust
// Advanced domain fronting with traffic mimicking
pub struct DomainFrontingClient {
    legitimate_headers: HashMap<String, String>,
    user_agents: Vec<String>,
    referrers: Vec<String>,
}

impl DomainFrontingClient {
    pub fn new() -> Self {
        Self {
            legitimate_headers: Self::create_legitimate_headers(),
            user_agents: Self::create_user_agent_list(),
            referrers: Self::create_referrer_list(),
        }
    }
    
    fn create_legitimate_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Accept-Language".to_string(), "en-US,en;q=0.9".to_string());
        headers.insert("Accept-Encoding".to_string(), "gzip, deflate, br".to_string());
        headers.insert("DNT".to_string(), "1".to_string());
        headers.insert("Connection".to_string(), "keep-alive".to_string());
        headers.insert("Upgrade-Insecure-Requests".to_string(), "1".to_string());
        headers
    }
    
    pub async fn make_fronted_request(&self, real_domain: &str, front_domain: &str, data: &[u8]) -> InfraResult<Vec<u8>> {
        let client = reqwest::Client::new();
        
        // Use front domain as URL but real domain in Host header
        let mut request = client
            .post(&format!("https://{}/api/v1/data", front_domain))
            .header("Host", real_domain)
            .body(data.to_vec());
        
        // Add legitimate-looking headers
        for (key, value) in &self.legitimate_headers {
            request = request.header(key, value);
        }
        
        // Random user agent
        let user_agent = self.user_agents.choose(&mut rand::thread_rng()).unwrap();
        request = request.header("User-Agent", user_agent);
        
        // Random referrer
        let referrer = self.referrers.choose(&mut rand::thread_rng()).unwrap();
        request = request.header("Referer", referrer);
        
        let response = request.send().await?;
        let response_data = response.bytes().await?;
        
        Ok(response_data.to_vec())
    }
}
```

## Operational Security

### Configuration Security
```bash
#!/bin/bash
# Secure configuration deployment script

set -euo pipefail

CONFIG_FILE="$1"
TARGET_HOST="$2"

# Validate configuration file
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: Configuration file not found: $CONFIG_FILE"
    exit 1
fi

# Encrypt configuration for transmission
gpg --cipher-algo AES256 --compress-algo 1 --s2k-mode 3 \
    --s2k-digest-algo SHA512 --s2k-count 65536 --symmetric \
    --output "${CONFIG_FILE}.gpg" "$CONFIG_FILE"

# Transfer encrypted config
scp "${CONFIG_FILE}.gpg" "root@${TARGET_HOST}:/tmp/"

# Deploy and decrypt on target
ssh "root@${TARGET_HOST}" << 'EOF'
    # Decrypt configuration
    gpg --decrypt /tmp/config.toml.gpg > /opt/nexus/config/production.toml
    
    # Set proper permissions
    chmod 600 /opt/nexus/config/production.toml
    chown nexus:nexus /opt/nexus/config/production.toml
    
    # Remove temporary files
    rm /tmp/config.toml.gpg
    
    # Restart services if needed
    systemctl reload nexus-server
EOF

# Clean up local encrypted file
rm "${CONFIG_FILE}.gpg"
```

### Log Security
```rust
// Secure logging with data sanitization
use log::{info, warn, error};
use regex::Regex;

pub struct SecureLogger {
    sensitive_patterns: Vec<Regex>,
}

impl SecureLogger {
    pub fn new() -> Self {
        let sensitive_patterns = vec![
            Regex::new(r"(?i)(password|token|key|secret)[:=]\s*\S+").unwrap(),
            Regex::new(r"\b[A-Za-z0-9]{32,}\b").unwrap(), // Potential tokens
            Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap(), // IP addresses
        ];
        
        Self { sensitive_patterns }
    }
    
    pub fn sanitize_message(&self, message: &str) -> String {
        let mut sanitized = message.to_string();
        
        for pattern in &self.sensitive_patterns {
            sanitized = pattern.replace_all(&sanitized, "[REDACTED]").to_string();
        }
        
        sanitized
    }
    
    pub fn secure_info(&self, message: &str) {
        info!("{}", self.sanitize_message(message));
    }
    
    pub fn secure_warn(&self, message: &str) {
        warn!("{}", self.sanitize_message(message));
    }
    
    pub fn secure_error(&self, message: &str) {
        error!("{}", self.sanitize_message(message));
    }
}
```

## Agent Security

### Agent Obfuscation
```rust
// Agent binary obfuscation techniques
pub mod obfuscation {
    use std::collections::HashMap;
    
    // String obfuscation using XOR
    pub fn obfuscate_string(s: &str, key: u8) -> Vec<u8> {
        s.bytes().map(|b| b ^ key).collect()
    }
    
    pub fn deobfuscate_string(data: &[u8], key: u8) -> String {
        let deobfuscated: Vec<u8> = data.iter().map(|&b| b ^ key).collect();
        String::from_utf8_lossy(&deobfuscated).to_string()
    }
    
    // Function name obfuscation
    macro_rules! obfuscated_function {
        ($name:ident, $obfuscated:expr) => {
            #[no_mangle]
            pub extern "C" fn $obfuscated() {
                $name()
            }
            
            fn $name() {
                // Actual function implementation
            }
        };
    }
    
    // Import table obfuscation
    pub fn resolve_obfuscated_api(dll_hash: u32, function_hash: u32) -> Option<usize> {
        // Hash-based API resolution to avoid clear-text imports
        let mut api_map = HashMap::new();
        
        // Populate with pre-computed hashes
        api_map.insert((0x6A4ABC5B, 0x7C0DFCAA), kernel32_virtualalloc_addr());
        api_map.insert((0x6A4ABC5B, 0x300F2F0B), kernel32_virtualfree_addr());
        
        api_map.get(&(dll_hash, function_hash)).copied()
    }
}
```

### Runtime Protection
```rust
// Runtime integrity protection
pub struct RuntimeProtector {
    original_code_hash: [u8; 32],
    anti_hook_checks: Vec<Box<dyn Fn() -> bool + Send + Sync>>,
}

impl RuntimeProtector {
    pub fn new() -> Self {
        Self {
            original_code_hash: Self::calculate_code_hash(),
            anti_hook_checks: vec![
                Box::new(Self::check_inline_hooks),
                Box::new(Self::check_iat_hooks),
                Box::new(Self::check_dll_injections),
            ],
        }
    }
    
    fn calculate_code_hash() -> [u8; 32] {
        // Calculate hash of critical code sections
        // Implementation depends on platform and binary format
        [0u8; 32]
    }
    
    fn check_inline_hooks() -> bool {
        // Check for inline hooks in critical functions
        // Implementation requires disassembly and pattern matching
        false
    }
    
    fn check_iat_hooks() -> bool {
        // Check Import Address Table for modifications
        false
    }
    
    fn check_dll_injections() -> bool {
        // Check for unexpected DLLs in process space
        false
    }
    
    pub async fn continuous_protection(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // Run integrity checks
            for check in &self.anti_hook_checks {
                if check() {
                    // Integrity violation detected
                    warn!("Runtime integrity violation detected");
                    std::process::exit(0);
                }
            }
        }
    }
}
```

## Infrastructure Hardening

### Server Hardening
```bash
#!/bin/bash
# Server hardening script

# System hardening
echo "Applying system hardening..."

# Disable unnecessary services
systemctl disable bluetooth
systemctl disable cups
systemctl disable avahi-daemon

# Kernel parameter hardening
cat >> /etc/sysctl.conf << 'EOF'
# IP forwarding
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# TCP hardening
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_max_syn_backlog = 2048
net.ipv4.tcp_synack_retries = 3

# ICMP hardening
net.ipv4.icmp_echo_ignore_all = 1
net.ipv6.icmp.echo_ignore_all = 1

# Source routing
net.ipv4.conf.all.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
EOF

sysctl -p

# File system hardening
mount -o remount,nodev,nosuid,noexec /tmp
mount -o remount,nodev,nosuid,noexec /var/tmp

# User account hardening
usermod -s /bin/false nexus  # Ensure service account can't login
passwd -l nexus             # Lock service account password

echo "System hardening complete"
```

### Network Hardening
```bash
#!/bin/bash
# Network hardening configuration

# Advanced iptables rules
iptables -F
iptables -X
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT ACCEPT

# Allow loopback
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow gRPC server
iptables -A INPUT -p tcp --dport 443 -m state --state NEW -j ACCEPT

# Rate limiting for new connections
iptables -A INPUT -p tcp --dport 443 -m recent --set --name nexus_clients
iptables -A INPUT -p tcp --dport 443 -m recent --update --seconds 60 --hitcount 10 --name nexus_clients -j DROP

# Allow management SSH from specific IPs only
iptables -A INPUT -p tcp --dport 22 -s MANAGEMENT_IP -j ACCEPT

# Log dropped packets
iptables -A INPUT -j LOG --log-prefix "NEXUS-DROP: "
iptables -A INPUT -j DROP

# Save rules
iptables-save > /etc/iptables/rules.v4

# Configure fail2ban for additional protection
cat > /etc/fail2ban/jail.d/nexus.conf << 'EOF'
[nexus-server]
enabled = true
port = 443
protocol = tcp
filter = nexus-server
logpath = /opt/nexus/logs/nexus-server.log
maxretry = 3
bantime = 3600
findtime = 600
EOF
```

## Access Control

### Role-Based Access Control
```rust
// RBAC implementation for administrative access
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Administrator,  // Full access
    Operator,      // Agent management and task assignment
    Observer,      // Read-only access
    Auditor,       // Audit logs and compliance data
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    AgentManagement,
    TaskAssignment,
    InfrastructureManagement,
    CertificateManagement,
    ConfigurationEdit,
    AuditLogAccess,
    SystemMonitoring,
}

pub struct AccessControl {
    role_permissions: HashMap<Role, Vec<Permission>>,
    user_roles: HashMap<String, Role>,
}

impl AccessControl {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();
        
        // Administrator permissions
        role_permissions.insert(Role::Administrator, vec![
            Permission::AgentManagement,
            Permission::TaskAssignment,
            Permission::InfrastructureManagement,
            Permission::CertificateManagement,
            Permission::ConfigurationEdit,
            Permission::AuditLogAccess,
            Permission::SystemMonitoring,
        ]);
        
        // Operator permissions
        role_permissions.insert(Role::Operator, vec![
            Permission::AgentManagement,
            Permission::TaskAssignment,
            Permission::SystemMonitoring,
        ]);
        
        // Observer permissions
        role_permissions.insert(Role::Observer, vec![
            Permission::SystemMonitoring,
        ]);
        
        // Auditor permissions
        role_permissions.insert(Role::Auditor, vec![
            Permission::AuditLogAccess,
            Permission::SystemMonitoring,
        ]);
        
        Self {
            role_permissions,
            user_roles: HashMap::new(),
        }
    }
    
    pub fn check_permission(&self, user: &str, permission: Permission) -> bool {
        if let Some(role) = self.user_roles.get(user) {
            if let Some(permissions) = self.role_permissions.get(role) {
                return permissions.contains(&permission);
            }
        }
        false
    }
}
```

### Authentication Enhancement
```rust
// Multi-factor authentication for administrative access
use totp_rs::{Algorithm, TOTP};

pub struct MFAValidator {
    totp: TOTP,
    backup_codes: Vec<String>,
}

impl MFAValidator {
    pub fn new(secret: &str) -> InfraResult<Self> {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,        // 6-digit codes
            1,        // 1 step
            30,       // 30-second window
            secret.as_bytes().to_vec(),
        )?;
        
        let backup_codes = Self::generate_backup_codes();
        
        Ok(Self { totp, backup_codes })
    }
    
    pub fn validate_token(&self, token: &str) -> bool {
        self.totp.check_current(token).unwrap_or(false)
    }
    
    pub fn validate_backup_
