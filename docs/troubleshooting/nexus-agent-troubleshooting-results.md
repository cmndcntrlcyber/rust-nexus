# Nexus Agent Connection Troubleshooting Results

## Summary of Issues Found

### 1. ✅ **FIXED**: Command Line Argument Parsing
- **Issue**: Agent didn't properly parse `--grpc-endpoint` flag
- **Impact**: Agent was trying to connect to `--grpc-endpoint` instead of the actual URL
- **Resolution**: Fixed argument parsing in `nexus-agent/src/main.rs`
- **Verification**: Debug output now shows "Connecting to server: https://c2.attck-deploy.net:443"

### 2. ✅ **FIXED**: HTTP/2 Support Missing
- **Issue**: Reverse proxy had HTTP/2 support disabled
- **Impact**: gRPC requires HTTP/2 and cannot work over HTTP/1.1  
- **Resolution**: Enabled HTTP/2 support in proxy SSL settings
- **Verification**: curl shows "server accepted to use h2"

### 3. 🔄 **IN PROGRESS**: Reverse Proxy gRPC Configuration
- **Issue**: Standard HTTP proxy configuration doesn't handle gRPC properly
- **Impact**: 502 Bad Gateway errors, gRPC transport failures
- **Resolution**: Need to apply custom nginx gRPC configuration (see grpc-proxy-config.txt)
- **Status**: Configuration created, needs to be applied

### 4. 🔍 **INVESTIGATING**: Backend TLS Configuration
- **Issue**: Backend server may have certificate/TLS configuration issues
- **Evidence**: 
  - Direct curl to backend shows "rsa routines::data too large for modulus"
  - Backend running on port 8443 with mutual TLS enabled
- **Impact**: Reverse proxy can't establish connection to backend

## Current Infrastructure Setup

```
[Internet] → [Cloudflare] → [Reverse Proxy:443] → [Backend Server:8443]
                                ↓ HTTP/2 ✅           ↓ gRPC + TLS
                            c2.attck-deploy.net    192.168.1.124
```

### Backend Server Status
- **Status**: ✅ Running
- **Bind**: 0.0.0.0:8443  
- **Config**: nexus-working.toml
- **TLS**: Mutual TLS enabled
- **Certificate**: Origin SSL cert (./certs/origin.crt)

### Reverse Proxy Status  
- **Frontend**: c2.attck-deploy.net:443
- **Backend**: 192.168.1.124:8443
- **HTTP/2**: ✅ Enabled
- **gRPC Config**: ❌ Missing (standard HTTP proxy)

## Next Steps

### 1. **Apply gRPC Reverse Proxy Configuration**
Copy the configuration from `grpc-proxy-config.txt` to the Advanced tab → Custom Nginx Configuration in your proxy settings.

Key changes needed:
```nginx
location / {
    grpc_pass grpc://192.168.1.124:8443;
    grpc_set_header Host $host;
    # ... (see grpc-proxy-config.txt for full configuration)
}
```

### 2. **Test After Proxy Configuration**
After applying the gRPC configuration:
```bash
./target/debug/nexus-agent --grpc-endpoint https://c2.attck-deploy.net:443
```

### 3. **Alternative: Test Direct Backend Connection** 
If proxy issues persist, test direct connection:
```bash
./target/debug/nexus-agent --grpc-endpoint https://192.168.1.124:8443
```

### 4. **Backend TLS Investigation**
If direct connection fails, investigate backend certificate configuration:
- Check certificate validity
- Verify mutual TLS setup
- Consider temporary non-TLS backend for testing

## Expected Outcome

After applying the gRPC configuration, the agent should:
1. Successfully connect to the reverse proxy
2. Complete gRPC handshake through proxy
3. Register with the backend server
4. Begin heartbeat cycles

## Debug Information

### Agent Command Line Fix
```rust
// OLD (broken)
let server_addr = if args.len() > 1 {
    args[1].clone()  // This took "--grpc-endpoint" as the address
} else {
    "https://c2.attck-deploy.net:8443".to_string()
};

// NEW (fixed)  
for i in 1..args.len() {
    if args[i] == "--grpc-endpoint" && i + 1 < args.len() {
        server_addr = args[i + 1].clone();
        break;
    }
}
```

### Current Error Pattern

### After Mutual TLS Implementation
```
Connecting to server: https://c2.attck-deploy.net:443
Configured mutual TLS with client certificate
Agent cycle error: Network error: gRPC connection failed: transport error
```

### Direct Backend Connection  
```
Connecting to server: https://192.168.1.124:8443
Configured mutual TLS with client certificate
Agent cycle error: Network error: gRPC connection failed: transport error
```

This indicates:
- ✅ Command line parsing works
- ✅ Agent loads and configures client certificates properly
- ✅ Mutual TLS client-side setup is working
- ❌ Transport error persists on both connections
- 🔍 **Remaining Issue**: Either CA certificate validation or gRPC proxy configuration
