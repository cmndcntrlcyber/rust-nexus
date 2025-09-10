# Session Management Guide

Comprehensive guide to managing agent sessions, understanding session lifecycle, and maintaining optimal session health in the Nexus C2 framework.

## Overview

Session management in Nexus provides robust agent lifecycle control with features including:
- **Agent Session Tracking** - Complete session lifecycle from registration to cleanup
- **Heartbeat Management** - Real-time connection health monitoring
- **Task Queue Management** - Per-session task assignment and tracking
- **Session Security** - Authentication, validation, and secure communication
- **Automatic Cleanup** - Resource management and inactive session removal
- **Session Analytics** - Performance metrics and connection insights

## Session Architecture

### Session Components

```rust
pub struct AgentSession {
    // Identity and Registration
    pub agent_id: String,                          // Unique session identifier
    pub registration_info: RegistrationRequest,    // Initial registration data
    
    // Timing Information
    pub connected_at: DateTime<Utc>,              // Session start time
    pub last_heartbeat: DateTime<Utc>,            // Last activity timestamp
    
    // Status Management
    pub is_active: bool,                          // Current session state
    
    // Task Management
    pub task_queue: Vec<Task>,                    // Pending tasks
    pub pending_tasks: HashMap<String, Task>,     // Active tasks
}
```

### Session Manager

The session manager coordinates all session operations:

```rust
pub struct SessionManager {
    // Session Storage
    agents: Arc<RwLock<HashMap<String, AgentSession>>>,
    
    // Task Coordination
    task_manager: Arc<RwLock<TaskManager>>,
    
    // Configuration
    cleanup_interval: Duration,
    session_timeout: Duration,
    max_sessions: usize,
}
```

## Session Lifecycle

### 1. Agent Registration

#### Registration Request
When an agent first connects, it sends registration information:

```rust
pub struct RegistrationRequest {
    pub hostname: String,           // System hostname
    pub os_type: String,           // Operating system
    pub os_version: String,        // OS version details
    pub ip_address: String,        // Agent IP address
    pub username: String,          // Current username
    pub process_id: u32,          // Process ID
    pub process_name: String,     // Process name
    pub architecture: String,     // System architecture
    pub capabilities: Vec<String>, // Available capabilities
    pub public_key: String,       // Encryption key
}
```

#### Registration Process Flow

```
1. Agent → Server: RegistrationRequest
2. Server: Validate registration data
3. Server: Generate unique agent_id
4. Server: Create AgentSession object
5. Server: Store session in session manager
6. Server → Agent: RegistrationResponse with agent_id
```

#### Registration Validation

Server performs validation during registration:

```rust
async fn validate_registration(request: &RegistrationRequest) -> Result<(), ValidationError> {
    // Validate required fields
    if request.hostname.is_empty() {
        return Err(ValidationError::MissingHostname);
    }
    
    // Validate public key format
    if !is_valid_public_key(&request.public_key) {
        return Err(ValidationError::InvalidPublicKey);
    }
    
    // Check for duplicate registrations
    if is_duplicate_registration(request).await? {
        return Err(ValidationError::DuplicateRegistration);
    }
    
    Ok(())
}
```

### 2. Session Establishment

#### Session Creation

Upon successful registration, a session is created:

```rust
let session = AgentSession {
    agent_id: generate_agent_id(),
    registration_info: request.clone(),
    connected_at: Utc::now(),
    last_heartbeat: Utc::now(),
    is_active: true,
    task_queue: Vec::new(),
    pending_tasks: HashMap::new(),
};
```

#### Session Storage

Sessions are stored in a thread-safe collection:

```rust
// Store new session
self.agents.write().await.insert(agent_id.clone(), session);

// Log session creation
info!("New session created: {} from {}", 
      agent_id, request.hostname);
```

### 3. Heartbeat Management

#### Heartbeat Protocol

Agents send periodic heartbeats to maintain session health:

```rust
pub struct HeartbeatRequest {
    pub agent_id: String,                    // Session identifier
    pub status: AgentStatus,                 // Current agent status
    pub task_statuses: Vec<TaskStatus>,      // Task progress updates
    pub system_info: Option<SystemUpdate>,  // Optional system updates
}
```

#### Heartbeat Processing

Server processes heartbeats to update session state:

```rust
async fn process_heartbeat(&self, request: HeartbeatRequest) -> Result<HeartbeatResponse> {
    let mut agents = self.agents.write().await;
    
    if let Some(agent) = agents.get_mut(&request.agent_id) {
        // Update heartbeat timestamp
        agent.last_heartbeat = Utc::now();
        
        // Process task status updates
        for task_status in request.task_statuses {
            self.update_task_status(&mut agent, task_status).await?;
        }
        
        // Update system information if provided
        if let Some(system_update) = request.system_info {
            agent.registration_info.update_from(system_update);
        }
        
        // Prepare response
        let response = HeartbeatResponse {
            success: true,
            heartbeat_interval: self.heartbeat_interval.as_secs() as u32,
            new_domains: self.get_domain_updates(&agent.agent_id).await?,
            config_update: self.get_config_updates(&agent.agent_id).await?,
        };
        
        Ok(response)
    } else {
        Err(SessionError::UnknownAgent(request.agent_id))
    }
}
```

#### Heartbeat Failure Handling

When heartbeats fail or are delayed:

```rust
async fn check_session_health(&self) {
    let now = Utc::now();
    let mut agents = self.agents.write().await;
    
    for (agent_id, agent) in agents.iter_mut() {
        let time_since_heartbeat = now - agent.last_heartbeat;
        
        match time_since_heartbeat {
            t if t > Duration::minutes(30) => {
                warn!("Agent {} has been inactive for {:?}", agent_id, t);
                agent.is_active = false;
            }
            t if t > Duration::minutes(10) => {
                debug!("Agent {} heartbeat delayed: {:?}", agent_id, t);
            }
            _ => {} // Agent is healthy
        }
    }
}
```

### 4. Session Termination

#### Graceful Termination

Sessions can be terminated gracefully:

```rust
async fn terminate_session(&self, agent_id: &str, reason: TerminationReason) -> Result<()> {
    let mut agents = self.agents.write().await;
    
    if let Some(mut session) = agents.remove(agent_id) {
        // Cancel pending tasks
        for (task_id, _) in session.pending_tasks.drain() {
            self.task_manager.write().await.cancel_task(&task_id).await?;
        }
        
        // Clear task queue
        session.task_queue.clear();
        
        // Log termination
        info!("Session terminated: {} (reason: {:?})", agent_id, reason);
        
        // Archive session data if needed
        if self.config.archive_sessions {
            self.archive_session(session).await?;
        }
        
        Ok(())
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

#### Forced Termination

For unresponsive or problematic agents:

```rust
async fn force_terminate_session(&self, agent_id: &str) -> Result<()> {
    warn!("Force terminating session: {}", agent_id);
    
    // Immediate removal without cleanup
    let mut agents = self.agents.write().await;
    if agents.remove(agent_id).is_some() {
        // Log forced termination
        error!("Session forcefully terminated: {}", agent_id);
        Ok(())
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

## Task Queue Management

### Task Assignment

Tasks are assigned to specific agent sessions:

```rust
async fn assign_task(&self, agent_id: &str, task: Task) -> Result<()> {
    let mut agents = self.agents.write().await;
    
    if let Some(agent) = agents.get_mut(agent_id) {
        // Add to task queue
        agent.task_queue.push(task.clone());
        
        // Track in task manager
        self.task_manager.write().await.assign_task(agent_id, task).await?;
        
        info!("Task {} assigned to agent {}", task.task_id, agent_id);
        Ok(())
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

### Task Distribution

When agents request tasks:

```rust
async fn get_tasks_for_agent(&self, agent_id: &str, max_tasks: usize) -> Result<Vec<Task>> {
    let mut agents = self.agents.write().await;
    
    if let Some(agent) = agents.get_mut(agent_id) {
        // Move tasks from queue to pending
        let tasks_to_send: Vec<Task> = agent.task_queue
            .drain(..std::cmp::min(max_tasks, agent.task_queue.len()))
            .collect();
        
        // Mark tasks as pending
        for task in &tasks_to_send {
            agent.pending_tasks.insert(task.task_id.clone(), task.clone());
        }
        
        debug!("Distributed {} tasks to agent {}", tasks_to_send.len(), agent_id);
        Ok(tasks_to_send)
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

### Task Status Updates

Agents report task status through heartbeats:

```rust
async fn update_task_status(&self, agent: &mut AgentSession, status: TaskStatus) -> Result<()> {
    let task_id = &status.task_id;
    
    match status.status {
        TaskState::Completed => {
            // Move from pending to completed
            if let Some(task) = agent.pending_tasks.remove(task_id) {
                self.task_manager.write().await.complete_task(task_id, status.result).await?;
                debug!("Task {} completed by agent {}", task_id, agent.agent_id);
            }
        }
        TaskState::Failed => {
            // Handle task failure
            if let Some(task) = agent.pending_tasks.remove(task_id) {
                self.task_manager.write().await.fail_task(task_id, status.error).await?;
                warn!("Task {} failed on agent {}: {}", task_id, agent.agent_id, status.error.unwrap_or_default());
            }
        }
        TaskState::Running => {
            // Update task progress
            if let Some(task) = agent.pending_tasks.get_mut(task_id) {
                task.progress = status.progress;
                debug!("Task {} progress: {}%", task_id, status.progress.unwrap_or(0));
            }
        }
        _ => {} // Handle other states as needed
    }
    
    Ok(())
}
```

## Session Monitoring

### Real-time Session Status

Monitor active sessions:

```rust
pub struct SessionStatus {
    pub agent_id: String,
    pub hostname: String,
    pub connected_duration: Duration,
    pub last_activity: Duration,
    pub is_active: bool,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub connection_quality: ConnectionQuality,
}

async fn get_session_status(&self) -> Vec<SessionStatus> {
    let agents = self.agents.read().await;
    let now = Utc::now();
    
    agents.values().map(|agent| {
        let connected_duration = now - agent.connected_at;
        let last_activity = now - agent.last_heartbeat;
        
        SessionStatus {
            agent_id: agent.agent_id.clone(),
            hostname: agent.registration_info.hostname.clone(),
            connected_duration,
            last_activity,
            is_active: agent.is_active,
            pending_tasks: agent.pending_tasks.len(),
            completed_tasks: agent.completed_tasks,
            connection_quality: calculate_connection_quality(&agent),
        }
    }).collect()
}
```

### Session Analytics

Gather session performance metrics:

```rust
pub struct SessionAnalytics {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub avg_session_duration: Duration,
    pub task_throughput: f64,
    pub connection_success_rate: f64,
    pub platform_distribution: HashMap<String, usize>,
}

async fn get_session_analytics(&self) -> SessionAnalytics {
    let agents = self.agents.read().await;
    let now = Utc::now();
    
    let mut analytics = SessionAnalytics {
        total_sessions: agents.len(),
        active_sessions: 0,
        avg_session_duration: Duration::zero(),
        task_throughput: 0.0,
        connection_success_rate: 0.0,
        platform_distribution: HashMap::new(),
    };
    
    let mut total_duration = Duration::zero();
    let mut total_tasks = 0;
    
    for agent in agents.values() {
        if agent.is_active {
            analytics.active_sessions += 1;
        }
        
        // Calculate session duration
        let duration = now - agent.connected_at;
        total_duration = total_duration + duration;
        
        // Count tasks
        total_tasks += agent.completed_tasks;
        
        // Platform distribution
        *analytics.platform_distribution
            .entry(agent.registration_info.os_type.clone())
            .or_insert(0) += 1;
    }
    
    // Calculate averages
    if !agents.is_empty() {
        analytics.avg_session_duration = total_duration / agents.len() as i32;
        analytics.task_throughput = total_tasks as f64 / total_duration.num_hours() as f64;
    }
    
    analytics
}
```

## Session Security

### Authentication and Validation

#### Session Authentication
```rust
async fn authenticate_session(&self, agent_id: &str, auth_token: &str) -> Result<bool> {
    let agents = self.agents.read().await;
    
    if let Some(agent) = agents.get(agent_id) {
        // Validate authentication token
        let expected_token = self.generate_session_token(&agent.registration_info)?;
        
        if auth_token == expected_token {
            Ok(true)
        } else {
            warn!("Authentication failed for agent: {}", agent_id);
            Ok(false)
        }
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

#### Session Encryption
```rust
pub struct SessionEncryption {
    pub agent_public_key: PublicKey,
    pub server_private_key: PrivateKey,
    pub session_key: Option<SymmetricKey>,
}

impl SessionEncryption {
    pub fn encrypt_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        if let Some(session_key) = &self.session_key {
            session_key.encrypt(message)
        } else {
            // Fallback to asymmetric encryption
            self.agent_public_key.encrypt(message)
        }
    }
    
    pub fn decrypt_message(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if let Some(session_key) = &self.session_key {
            session_key.decrypt(encrypted)
        } else {
            self.server_private_key.decrypt(encrypted)
        }
    }
}
```

### Session Integrity

#### Tampering Detection
```rust
async fn verify_session_integrity(&self, agent_id: &str) -> Result<bool> {
    let agents = self.agents.read().await;
    
    if let Some(agent) = agents.get(agent_id) {
        // Check session hash
        let expected_hash = self.calculate_session_hash(&agent)?;
        let actual_hash = agent.session_hash.as_ref().ok_or(SessionError::MissingHash)?;
        
        if expected_hash == *actual_hash {
            Ok(true)
        } else {
            error!("Session integrity check failed for agent: {}", agent_id);
            Ok(false)
        }
    } else {
        Err(SessionError::UnknownAgent(agent_id.to_string()))
    }
}
```

## Session Cleanup and Maintenance

### Automatic Cleanup

Periodic cleanup of inactive sessions:

```rust
async fn cleanup_inactive_sessions(&self) -> usize {
    let timeout_duration = self.config.session_timeout;
    let now = Utc::now();
    let mut removed_count = 0;
    
    let mut agents = self.agents.write().await;
    let initial_count = agents.len();
    
    // Remove sessions that have exceeded timeout
    agents.retain(|agent_id, agent| {
        let inactive_duration = now - agent.last_heartbeat;
        
        if inactive_duration > timeout_duration {
            warn!("Cleaning up inactive session: {} (inactive for {:?})", 
                  agent_id, inactive_duration);
            
            // Cleanup associated tasks
            let mut task_manager = self.task_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = task_manager.write().await.cleanup_agent_tasks(agent_id).await {
                    error!("Failed to cleanup tasks for agent {}: {}", agent_id, e);
                }
            });
            
            false // Remove this session
        } else {
            true // Keep this session
        }
    });
    
    removed_count = initial_count - agents.len();
    
    if removed_count > 0 {
        info!("Cleaned up {} inactive sessions", removed_count);
    }
    
    removed_count
}
```

### Session Archival

Archive completed sessions for analysis:

```rust
async fn archive_session(&self, session: AgentSession) -> Result<()> {
    let archive_data = SessionArchive {
        agent_id: session.agent_id,
        hostname: session.registration_info.hostname,
        connected_at: session.connected_at,
        disconnected_at: Utc::now(),
        total_tasks: session.completed_tasks,
        platform: session.registration_info.os_type,
        session_duration: Utc::now() - session.connected_at,
    };
    
    // Store in database or file system
    self.storage.store_session_archive(&archive_data).await?;
    
    debug!("Archived session: {}", archive_data.agent_id);
    Ok(())
}
```

## Session Configuration

### Configuration Options

```toml
[session_management]
# Session timeouts
session_timeout_minutes = 30
heartbeat_interval_seconds = 30
cleanup_interval_minutes = 5

# Session limits
max_concurrent_sessions = 1000
max_tasks_per_session = 50
max_session_duration_hours = 24

# Security settings
require_authentication = true
session_encryption = true
integrity_checking = true

# Archival settings
archive_sessions = true
archive_retention_days = 90
archive_compression = true
```

### Dynamic Configuration Updates

```rust
async fn update_session_config(&mut self, new_config: SessionConfig) -> Result<()> {
    // Validate new configuration
    new_config.validate()?;
    
    // Update timeout values
    self.config.session_timeout = Duration::minutes(new_config.session_timeout_minutes);
    self.config.heartbeat_interval = Duration::seconds(new_config.heartbeat_interval_seconds);
    
    // Apply limits to existing sessions
    if new_config.max_concurrent_sessions < self.agents.read().await.len() {
        warn!("New session limit is lower than current session count");
    }
    
    // Update cleanup schedule
    if new_config.cleanup_interval_minutes != self.config.cleanup_interval_minutes {
        self.reschedule_cleanup(Duration::minutes(new_config.cleanup_interval_minutes)).await?;
    }
    
    self.config = new_config;
    info!("Session configuration updated");
    
    Ok(())
}
```

## Troubleshooting Sessions

### Common Session Issues

#### Registration Failures
```rust
#[derive(Debug)]
pub enum RegistrationError {
    InvalidHostname,
    InvalidPublicKey,
    DuplicateRegistration,
    ServerOverload,
    AuthenticationFailed,
}

fn diagnose_registration_failure(error: &RegistrationError) -> String {
    match error {
        RegistrationError::InvalidHostname => {
            "Agent provided empty or invalid hostname".to_string()
        }
        RegistrationError::InvalidPublicKey => {
            "Agent public key format is invalid or corrupted".to_string()
        }
        RegistrationError::DuplicateRegistration => {
            "Agent is attempting to register multiple times".to_string()
        }
        RegistrationError::ServerOverload => {
            "Server has reached maximum concurrent session limit".to_string()
        }
        RegistrationError::AuthenticationFailed => {
            "Agent failed authentication during registration".to_string()
        }
    }
}
```

#### Heartbeat Problems
```rust
async fn diagnose_heartbeat_issues(&self, agent_id: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let agents = self.agents.read().await;
    
    if let Some(agent) = agents.get(agent_id) {
        let now = Utc::now();
        let time_since_heartbeat = now - agent.last_heartbeat;
        
        // Check heartbeat timing
        if time_since_heartbeat > Duration::minutes(5) {
            issues.push(format!("No heartbeat for {:?}", time_since_heartbeat));
        }
        
        // Check task queue backup
        if agent.pending_tasks.len() > 20 {
            issues.push(format!("Large pending task queue: {}", agent.pending_tasks.len()));
        }
        
        // Check for stuck tasks
        let stuck_tasks = agent.pending_tasks.values()
            .filter(|task| {
                if let Some(started) = task.started_at {
                    now - started > Duration::hours(1)
                } else {
                    false
                }
            })
            .count();
        
        if stuck_tasks > 0 {
            issues.push(format!("{} tasks appear stuck", stuck_tasks));
        }
    } else {
        issues.push("Agent session not found".to_string());
    }
    
    issues
}
```

### Session Recovery

#### Reconnection Handling
```rust
async fn handle_reconnection(&self, agent_id: &str, new_registration: RegistrationRequest) -> Result<()> {
    let mut agents = self.agents.write().await;
    
    if let Some(existing_session) = agents.get_mut(agent_id) {
        // Update registration information
        existing_session.registration_info = new_registration;
        existing_session.last_heartbeat = Utc::now();
        existing_session.is_active = true;
        
        info!("Agent {} reconnected", agent_id);
        
        // Resume pending tasks
        let pending_count = existing_session.pending_tasks.len();
        if pending_count > 0 {
            info!("Resuming {} pending tasks for agent {}", pending_count, agent_id);
        }
        
        Ok(())
    } else {
        // Create new session for reconnecting agent
        warn!("Creating new session for reconnecting agent: {}", agent_id);
        self.register_agent(new_registration).await
    }
}
```

## Session Management CLI

### Command-Line Interface

```bash
# List active sessions
nexus-server sessions list

# Get session details
nexus-server sessions show <agent_id>

# Terminate session
nexus-server sessions kill <agent_id>

# Session statistics
nexus-server sessions stats

# Cleanup inactive sessions
nexus-server sessions cleanup

# Archive old sessions
nexus-server sessions archive --days 30
```

### Interactive Session Management

```bash
# Interactive session browser
nexus-server sessions interactive

# Output example:
# ┌─────────────────────────────────────────────────────────┐
# │ Active Sessions (15)                                    │
# ├─────────────────────────────────────────────────────────┤
# │ ID       │ Hostname      │ Platform │ Connected │ Tasks │
# │ a1b2c3d4 │ WIN-DESK-01   │ Windows  │ 2h 15m    │ 3     │
# │ e5f6g7h8 │ Ubuntu-Srv-02 │ Linux    │ 45m       │ 1     │
# │ i9j0k1l2 │ MacBook-03    │ MacOS    │ 1h 30m    │ 0     │
# └─────────────────────────────────────────────────────────┘
```

This comprehensive session management guide covers all aspects of managing agent sessions in the Nexus C2 framework, from initial registration through cleanup and troubleshooting.
