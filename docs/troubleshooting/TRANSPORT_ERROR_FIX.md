# Nexus C2 Transport Error Fix

## Problem Summary
The Nexus C2 server was encountering a "transport error" during startup. Root cause analysis revealed this was due to insufficient permissions to bind to privileged port 443.

## Root Cause
- **Issue**: Permission denied when binding to port 443
- **Cause**: Non-root user attempting to bind to privileged port (<1024)
- **Evidence**: Server successfully loaded certificates and configured TLS, but failed at network binding stage

## Solution Implemented
✅ **Immediate Fix**: Modified configuration to use port 8443 (non-privileged)
✅ **Result**: Server now runs successfully without transport errors
✅ **Verification**: TLS handshake working, mutual TLS configured, server listening properly

## Production Deployment Options

### Option 1: setcap (Recommended for Development)
```bash
sudo setcap 'cap_net_bind_service=+ep' ./target/release/nexus-server
./target/release/nexus-server --config nexus.toml
```

### Option 2: Run as Root (Quick but less secure)
```bash
sudo ./target/release/nexus-server --config nexus.toml
```

### Option 3: Use Non-Privileged Port (Current Working Solution)
```bash
# Edit nexus.toml: change port from 443 to 8443
./target/release/nexus-server --config nexus.toml
```

### Option 4: systemd Service (Production Recommended)
Create `/etc/systemd/system/nexus-c2.service`:
```ini
[Unit]
Description=Nexus C2 Server
After=network.target

[Service]
Type=simple
User=nexus
Group=nexus
WorkingDirectory=/opt/nexus-c2
ExecStart=/opt/nexus-c2/target/release/nexus-server --config nexus.toml
Restart=always
RestartSec=10
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

### Option 5: Reverse Proxy (Production Alternative)
Use nginx/Apache to proxy from port 443 to 8443:
```nginx
server {
    listen 443 ssl http2;
    server_name c2.attck-deploy.net;
    
    location / {
        proxy_pass https://127.0.0.1:8443;
        proxy_http_version 2.0;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Current Status
- ✅ Server running on port 8443
- ✅ TLS working with Cloudflare Origin SSL certificates
- ✅ Mutual TLS configured and functional
- ✅ No transport errors
- ✅ Ready for agent connections

## Next Steps
1. **For Development**: Continue using port 8443 or apply setcap
2. **For Production**: Implement systemd service with proper user/permissions
3. **For High Security**: Use reverse proxy with additional security headers
4. **Update Agent Configurations**: Ensure agents connect to correct port

## Security Notes
- Origin SSL certificates work correctly for C2 infrastructure
- Mutual TLS is properly configured for agent authentication
- Consider firewall rules for production deployment
- Monitor for certificate expiration (current cert expires 2040-09-03)
