# Interactive Server Management

Comprehensive guide for managing the Nexus C2 server through its interactive command-line interface and understanding server session management.

## Overview

The Nexus server provides multiple interactive interfaces:
- **Command Line Interface (CLI)** - Direct server management and control
- **gRPC Service Interface** - Programmatic interaction with agents
- **Session Management** - Real-time agent lifecycle and task management
- **Health Monitoring** - Interactive server status and diagnostics

## Command Line Interface

### Server Commands

#### Starting the Server

```bash
# Basic server start
./target/release/nexus-server

# Start with custom configuration
./target/release/nexus-server --config production.toml

# Start with custom bind address and port
./target/release/nexus-server --bind 0.0.0.0 --port 8443

# Start as daemon (background service)
./target/release/nexus-server start --daemon

# Increase verbosity for debugging
./target/release/nexus-server -vvv
```

**Command Options:**
- `-c, --config <FILE>` - Configuration file path (default: nexus.toml)
- `-v, --verbose` - Increase verbosity (use multiple times: -vvv)
- `-b, --bind <ADDRESS>` - Override server bind address
- `-p, --port <PORT>` - Override server port
- `-d, --daemon` - Run as background daemon

#### Configuration Validation

```bash
# Validate configuration and display summary
./target/release/nexus-server validate --config nexus.toml

# Example output:
# ✅ Configuration is valid
# 
# 📋 Configuration Summary:
#   Domain: example.com
#   Primary domains: ["c2.example.com", "backup.example.com"]
#   gRPC Server: 0.0.0.0:8443
#   Mutual TLS: true
#   Max connections: 1000
#   Domain rotation: 24 hours
```

#### Infrastructure Initialization

```bash
# Initialize infrastructure (first-time setup)
./target/release/nexus-server init --config nexus.toml

# Force re-initialization (overwrites existing)
./target/release/nexus-server init --force

# What this does:
# ✅ Creates certificate directories
# ✅ Generates self-signed certificates (if none exist)
# ✅ Initializes domain records
# ✅ Sets up Let's Encrypt integration
```

### Interactive Server Session

Once started, the server provides comprehensive logging and status information:

```
2024-01-15T10:30:00Z [INFO] Nexus C2 Server starting...
2024-01-15T10:30:00Z [INFO] Configuration loaded from: nexus.toml
2024-01-15T10:30:01Z [INFO] ✅ Using existing origin certificates
2024-01-15T10:30:01Z [INFO] Starting gRPC server on 0.0.0.0:8443
2024-01-15T10:30:02Z [INFO] gRPC server started successfully on 0.0.0.0:8443
2024-01-15T10:30:02Z [INFO] ✅ Nexus C2 Server is running
2024-01-15T10:30:02Z [INFO] 🌐 Listening on: https://0.0.0.0:8443
```

### Real-time Server Management

#### Agent Connection Monitoring

When agents connect, you'll see real-time updates:

```
2024-01-15T10:35:15Z [INFO] Registering new agent: a1b2c3d4 from WIN-DESKTOP-01
2024-01-15T10:35:15Z [INFO] Agent a1b2c3d4 registered successfully
2024-01-15T10:35:16Z [DEBUG] Received heartbeat from agent: a1b2c3d4
```

#### Task Execution Monitoring

Track task execution in real-time:

```
2024-01-15T10:40:22Z [INFO] Received task result for task: task-001 from agent: a1b2c3d4
2024-01-15T10:40:22Z [INFO] BOF execution requested for agent: a1b2c3d4 (function: go)
2024-01-15T10:40:23Z [INFO] Shellcode execution requested for agent: a1b2c3d4
```

#### File Transfer Monitoring

Monitor file uploads/downloads:

```
2024-01-15T10:45:30Z [INFO] Received file upload: screenshot.png (2048576 bytes, 16 chunks)
2024-01-15T10:45:35Z [INFO] File download requested: payload.exe for agent: a1b2c3d4
```

#### Session Cleanup

Automatic cleanup of inactive agents:

```
2024-01-15T11:00:00Z [INFO] Cleaned up 2 inactive agents
```

## Server Control and Monitoring

### Graceful Shutdown

The server handles shutdown signals gracefully:

```bash
# Send SIGTERM or SIGINT (Ctrl+C)
kill -TERM <server_pid>

# Server output:
# 🛑 Shutdown signal received, stopping server...
# ✅ Server stopped gracefully
```

### Server Status Checking

#### Process Status

```bash
# Check if server is running
pgrep -f nexus-server

# Check listening ports
ss -tulpn | grep :8443
```

#### Health Check Endpoints

```bash
# gRPC health check
grpcurl -plaintext localhost:8443 grpc.health.v1.Health/Check

# HTTP health endpoint (if configured)
curl -k https://localhost:8443/health
```

## Session Management Interface

### Agent Session Lifecycle

#### Registration Process
1. **Agent Connection** - Agent initiates gRPC connection
2. **Registration Request** - Agent sends system information
3. **Session Creation** - Server creates AgentSession object
4. **ID Assignment** - Server assigns unique agent ID
5. **Response** - Server confirms registration

#### Session Data Structure

Each agent session contains:

```rust
pub struct AgentSession {
    pub agent_id: String,                    // Unique identifier
    pub registration_info: RegistrationRequest, // System details
    pub connected_at: DateTime<Utc>,         // Connection timestamp
    pub last_heartbeat: DateTime<Utc>,       // Last activity
    pub is_active: bool,                     // Current status
    pub task_queue: Vec<Task>,               // Pending tasks
    pub pending_tasks: HashMap<String, Task>, // In-progress tasks
}
```

#### Heartbeat Management

Agents send periodic heartbeats:

```rust
// Heartbeat every 30 seconds (configurable)
HeartbeatRequest {
    agent_id: "a1b2c3d4",
    status: AgentStatus::Active,
    task_statuses: [
        TaskStatus { task_id: "task-001", status: Completed },
        TaskStatus { task_id: "task-002", status: Running }
    ]
}
```

#### Session Cleanup

Automatic cleanup removes inactive agents:
- **Timeout**: 30 minutes of inactivity (configurable)
- **Frequency**: Every 5 minutes
- **Logging**: Reports cleanup actions

### Task Queue Management

#### Task Assignment Flow

1. **Task Creation** - Operator creates task
2. **Queue Assignment** - Task added to agent's queue
3. **Task Retrieval** - Agent requests pending tasks
4. **Execution** - Agent executes and reports status
5. **Completion** - Task moved to completed list

#### Task State Tracking

Tasks progress through states:
- `Pending` - Waiting in queue
- `Running` - Currently executing
- `Completed` - Successfully finished
- `Failed` - Execution failed
- `Timeout` - Exceeded time limit
- `Cancelled` - Manually cancelled

## Interactive Configuration

### Runtime Configuration Updates

Some settings can be updated without restart:

#### Logging Level
```bash
# Increase verbosity
export RUST_LOG=debug

# Target specific modules
export RUST_LOG=nexus_server=trace,nexus_infra=debug
```

#### Connection Limits
Configuration changes require restart, but current connections are displayed:

```
Connected Agents: 15/1000
Active Tasks: 42
Pending Tasks: 8
```

### Development Mode

For development and testing:

```bash
# Start with self-signed certificates
./target/release/nexus-server --config dev.toml

# Enable debug logging
RUST_LOG=debug ./target/release/nexus-server

# Test configuration
./target/release/nexus-server validate --config test.toml
```

## Troubleshooting Interactive Sessions

### Common Connection Issues

#### Certificate Problems
```bash
# Verify certificate validity
openssl x509 -in certs/origin.crt -text -noout

# Check certificate dates
openssl x509 -in certs/origin.crt -dates -noout
```

#### Port Binding Issues
```bash
# Check port availability
sudo netstat -tlnp | grep :8443

# Check firewall rules
sudo iptables -L -n | grep 8443
```

#### Agent Registration Problems

Check logs for registration failures:
```
2024-01-15T10:35:15Z [WARN] Heartbeat from unknown agent: unknown-id
2024-01-15T10:35:16Z [ERROR] Agent registration failed: Invalid public key
```

### Performance Monitoring

#### Memory Usage
```bash
# Monitor server memory
ps aux | grep nexus-server

# Check for memory leaks
valgrind ./target/release/nexus-server --config test.toml
```

#### Connection Monitoring
```bash
# Active connections
ss -tn | grep :8443 | wc -l

# Connection states
ss -tn state established '( dport = :8443 or sport = :8443 )'
```

## Best Practices

### Production Deployment

1. **Use Systemd Service**
```bash
# /etc/systemd/system/nexus-server.service
[Unit]
Description=Nexus C2 Server
After=network.target

[Service]
Type=simple
User=nexus
WorkingDirectory=/opt/nexus
ExecStart=/opt/nexus/bin/nexus-server --config /opt/nexus/config/production.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

2. **Log Rotation**
```bash
# /etc/logrotate.d/nexus-server
/var/log/nexus-server/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 nexus nexus
    postrotate
        systemctl reload nexus-server
    endscript
}
```

3. **Monitoring Integration**
```bash
# Health check script for monitoring systems
#!/bin/bash
if ! pgrep -f nexus-server > /dev/null; then
    echo "CRITICAL: Nexus server not running"
    exit 2
fi

if ! grpcurl -plaintext localhost:8443 grpc.health.v1.Health/Check > /dev/null 2>&1; then
    echo "WARNING: Nexus server not responding"
    exit 1
fi

echo "OK: Nexus server healthy"
exit 0
```

### Security Considerations

1. **Certificate Management**
   - Regularly rotate certificates
   - Monitor expiration dates
   - Use proper CA validation

2. **Access Control**
   - Limit server bind address in production
   - Use firewalls to restrict access
   - Monitor failed authentication attempts

3. **Logging Security**
   - Avoid logging sensitive data
   - Secure log files appropriately
   - Implement log monitoring

This completes the interactive server management guide. The server provides comprehensive real-time monitoring, session management, and administrative capabilities through its CLI and service interfaces.
