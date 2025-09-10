# Real-time Interaction Guide

Comprehensive guide to real-time communication features in the Nexus C2 framework, including WebSocket connections, live updates, and interactive streaming capabilities.

## Overview

Nexus provides extensive real-time interaction capabilities:
- **WebSocket Communication** - Bidirectional real-time messaging
- **Live Agent Updates** - Real-time agent status and activity feeds
- **Interactive Task Monitoring** - Live task execution and progress tracking
- **Event Streaming** - System-wide event broadcasting and subscription
- **Real-time File Transfers** - Live progress monitoring for uploads/downloads
- **Live Dashboard Updates** - Dynamic UI updates without page refresh

## WebSocket Architecture

### Connection Management

The Nexus WebSocket system provides persistent, low-latency communication:

```rust
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    event_broadcaster: broadcast::Sender<WebSocketEvent>,
    connection_pool: ConnectionPool,
}

pub struct WebSocketConnection {
    pub connection_id: String,
    pub client_type: ClientType,
    pub authenticated: bool,
    pub subscriptions: Vec<EventSubscription>,
    pub last_activity: DateTime<Utc>,
    pub sender: UnboundedSender<WebSocketMessage>,
}
```

### Event System

Real-time events flow through a structured event system:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketEvent {
    // Agent Events
    AgentConnected(AgentConnectionEvent),
    AgentDisconnected(AgentDisconnectionEvent),
    AgentStatusUpdate(AgentStatusEvent),
    
    // Task Events
    TaskStarted(TaskStartEvent),
    TaskProgress(TaskProgressEvent),
    TaskCompleted(TaskCompletionEvent),
    TaskFailed(TaskFailureEvent),
    
    // File Transfer Events
    FileUploadStarted(FileTransferEvent),
    FileUploadProgress(FileProgressEvent),
    FileUploadCompleted(FileCompletionEvent),
    
    // System Events
    SystemAlert(SystemAlertEvent),
    DomainRotation(DomainRotationEvent),
    CertificateExpiry(CertificateExpiryEvent),
    
    // Infrastructure Events
    ServerStatus(ServerStatusEvent),
    PerformanceMetrics(PerformanceEvent),
}
```

## WebSocket Connection Setup

### Client Connection

Establishing a WebSocket connection to Nexus:

```javascript
// Basic WebSocket connection
const ws = new WebSocket('wss://your-nexus-server:8080/ws');

// Connection with authentication
const ws = new WebSocket('wss://your-nexus-server:8080/ws', [], {
    headers: {
        'Authorization': 'Bearer your-auth-token'
    }
});

// Connection event handlers
ws.onopen = function(event) {
    console.log('Connected to Nexus WebSocket');
    
    // Subscribe to events after connection
    ws.send(JSON.stringify({
        type: 'subscribe',
        events: ['agent_updates', 'task_progress', 'system_alerts']
    }));
};

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    handleWebSocketMessage(message);
};

ws.onerror = function(error) {
    console.error('WebSocket error:', error);
};

ws.onclose = function(event) {
    console.log('WebSocket connection closed:', event.code, event.reason);
    // Implement reconnection logic
    setTimeout(connectWebSocket, 5000);
};
```

### Server-side WebSocket Handler

```rust
pub async fn handle_websocket(
    ws: warp::ws::Ws,
    state: WebUIState,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |websocket| {
        handle_websocket_connection(websocket, state)
    }))
}

async fn handle_websocket_connection(websocket: WebSocket, state: WebUIState) {
    let connection_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", connection_id);
    
    let (ws_sender, mut ws_receiver) = websocket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Store connection
    let connection = WebSocketConnection {
        connection_id: connection_id.clone(),
        client_type: ClientType::WebUI,
        authenticated: false,
        subscriptions: Vec::new(),
        last_activity: Utc::now(),
        sender: tx,
    };
    
    state.websocket_manager
        .connections
        .write()
        .await
        .insert(connection_id.clone(), connection);
    
    // Handle incoming messages
    let incoming_handler = handle_incoming_messages(
        ws_receiver, 
        connection_id.clone(), 
        state.clone()
    );
    
    // Handle outgoing messages
    let outgoing_handler = handle_outgoing_messages(ws_sender, rx);
    
    // Run both handlers concurrently
    tokio::select! {
        _ = incoming_handler => {},
        _ = outgoing_handler => {},
    }
    
    // Cleanup connection
    state.websocket_manager
        .connections
        .write()
        .await
        .remove(&connection_id);
    
    info!("WebSocket connection closed: {}", connection_id);
}
```

## Event Subscription System

### Subscription Management

Clients can subscribe to specific event types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub event_types: Vec<String>,
    pub filters: HashMap<String, String>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionRequest {
    pub action: String, // "subscribe" or "unsubscribe"
    pub events: Vec<String>,
    pub filters: Option<HashMap<String, String>>,
}

async fn handle_subscription_request(
    connection_id: &str,
    request: SubscriptionRequest,
    state: &WebUIState,
) -> Result<()> {
    let mut connections = state.websocket_manager.connections.write().await;
    
    if let Some(connection) = connections.get_mut(connection_id) {
        match request.action.as_str() {
            "subscribe" => {
                let subscription = EventSubscription {
                    event_types: request.events,
                    filters: request.filters.unwrap_or_default(),
                    rate_limit: None,
                };
                
                connection.subscriptions.push(subscription);
                info!("Client {} subscribed to events", connection_id);
            }
            "unsubscribe" => {
                connection.subscriptions.retain(|sub| {
                    !request.events.iter().any(|event| sub.event_types.contains(event))
                });
                info!("Client {} unsubscribed from events", connection_id);
            }
            _ => {
                warn!("Invalid subscription action: {}", request.action);
            }
        }
    }
    
    Ok(())
}
```

### Event Filtering

Advanced event filtering capabilities:

```javascript
// Subscribe with filters
ws.send(JSON.stringify({
    type: 'subscribe',
    events: ['agent_updates'],
    filters: {
        'platform': 'windows',
        'status': 'active'
    }
}));

// Subscribe to specific agent
ws.send(JSON.stringify({
    type: 'subscribe',
    events: ['task_progress'],
    filters: {
        'agent_id': 'a1b2c3d4-5678-9012-3456-789012345678'
    }
}));

// Subscribe with rate limiting
ws.send(JSON.stringify({
    type: 'subscribe',
    events: ['performance_metrics'],
    rate_limit: 5 // Maximum 5 events per second
}));
```

## Real-time Agent Updates

### Agent Connection Events

Real-time notifications when agents connect/disconnect:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConnectionEvent {
    pub agent_id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub connected_at: DateTime<Utc>,
    pub capabilities: Vec<String>,
}

async fn broadcast_agent_connected(&self, agent: &AgentSession) {
    let event = WebSocketEvent::AgentConnected(AgentConnectionEvent {
        agent_id: agent.agent_id.clone(),
        hostname: agent.registration_info.hostname.clone(),
        ip_address: agent.registration_info.ip_address.clone(),
        platform: agent.registration_info.os_type.clone(),
        connected_at: agent.connected_at,
        capabilities: agent.registration_info.capabilities.clone(),
    });
    
    self.broadcast_event(event).await;
}
```

### Agent Status Updates

Live agent status changes:

```javascript
// Handle agent status updates
function handleAgentStatusUpdate(event) {
    const agent = event.data;
    
    // Update agent list in UI
    updateAgentInList(agent.agent_id, {
        status: agent.status,
        last_seen: agent.last_seen,
        current_task: agent.current_task
    });
    
    // Show notification for important status changes
    if (agent.status === 'disconnected') {
        showNotification(`Agent ${agent.hostname} disconnected`, 'warning');
    }
    
    // Update dashboard counters
    updateDashboardStats();
}

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    
    switch(message.type) {
        case 'AgentStatusUpdate':
            handleAgentStatusUpdate(message);
            break;
        case 'AgentConnected':
            handleNewAgent(message.data);
            break;
        case 'AgentDisconnected':
            handleAgentDisconnection(message.data);
            break;
    }
};
```

## Live Task Monitoring

### Task Execution Events

Real-time task execution tracking:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartEvent {
    pub task_id: String,
    pub agent_id: String,
    pub task_type: String,
    pub command: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEvent {
    pub task_id: String,
    pub agent_id: String,
    pub progress: u32,
    pub status_message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionEvent {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub output: String,
    pub completed_at: DateTime<Utc>,
    pub execution_time: Duration,
}
```

### Interactive Task Progress

Live progress updates for long-running tasks:

```javascript
// Track task progress in real-time
class TaskMonitor {
    constructor() {
        this.activeTasks = new Map();
    }
    
    handleTaskStart(event) {
        const task = event.data;
        this.activeTasks.set(task.task_id, {
            ...task,
            progress: 0,
            startTime: new Date()
        });
        
        this.createTaskProgressBar(task);
    }
    
    handleTaskProgress(event) {
        const progress = event.data;
        const task = this.activeTasks.get(progress.task_id);
        
        if (task) {
            task.progress = progress.progress;
            this.updateProgressBar(progress.task_id, progress.progress);
            
            if (progress.status_message) {
                this.updateTaskStatus(progress.task_id, progress.status_message);
            }
        }
    }
    
    handleTaskCompletion(event) {
        const completion = event.data;
        const task = this.activeTasks.get(completion.task_id);
        
        if (task) {
            this.completeTaskProgress(completion.task_id, completion.success);
            this.displayTaskResult(completion);
            this.activeTasks.delete(completion.task_id);
        }
    }
    
    createTaskProgressBar(task) {
        const progressContainer = document.getElementById('active-tasks');
        const progressElement = document.createElement('div');
        progressElement.id = `task-${task.task_id}`;
        progressElement.innerHTML = `
            <div class="task-progress">
                <div class="task-info">
                    <span class="task-command">${task.command}</span>
                    <span class="task-agent">${task.agent_id}</span>
                </div>
                <div class="progress-bar">
                    <div class="progress-fill" style="width: 0%"></div>
                </div>
                <div class="task-status">Starting...</div>
            </div>
        `;
        progressContainer.appendChild(progressElement);
    }
    
    updateProgressBar(taskId, progress) {
        const element = document.getElementById(`task-${taskId}`);
        const progressFill = element.querySelector('.progress-fill');
        progressFill.style.width = `${progress}%`;
        
        const statusElement = element.querySelector('.task-status');
        statusElement.textContent = `${progress}% complete`;
    }
}

const taskMonitor = new TaskMonitor();

// WebSocket message handlers
ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    
    switch(message.type) {
        case 'TaskStarted':
            taskMonitor.handleTaskStart(message);
            break;
        case 'TaskProgress':
            taskMonitor.handleTaskProgress(message);
            break;
        case 'TaskCompleted':
            taskMonitor.handleTaskCompletion(message);
            break;
    }
};
```

## File Transfer Monitoring

### Real-time File Transfer Events

Live monitoring of file uploads and downloads:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferEvent {
    pub transfer_id: String,
    pub agent_id: String,
    pub filename: String,
    pub file_size: u64,
    pub direction: TransferDirection, // Upload or Download
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProgressEvent {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub transfer_rate: u64, // bytes per second
    pub eta_seconds: Option<u64>,
}

async fn handle_file_upload_progress(
    &self,
    transfer_id: &str,
    bytes_transferred: u64,
    total_bytes: u64,
) {
    let transfer_rate = self.calculate_transfer_rate(transfer_id, bytes_transferred);
    let eta_seconds = if transfer_rate > 0 {
        Some((total_bytes - bytes_transferred) / transfer_rate)
    } else {
        None
    };
    
    let event = WebSocketEvent::FileUploadProgress(FileProgressEvent {
        transfer_id: transfer_id.to_string(),
        bytes_transferred,
        total_bytes,
        transfer_rate,
        eta_seconds,
    });
    
    self.broadcast_event(event).await;
}
```

### Interactive File Transfer UI

```javascript
// Real-time file transfer monitoring
class FileTransferMonitor {
    constructor() {
        this.activeTransfers = new Map();
    }
    
    handleTransferStart(event) {
        const transfer = event.data;
        this.activeTransfers.set(transfer.transfer_id, transfer);
        this.createTransferProgress(transfer);
    }
    
    handleTransferProgress(event) {
        const progress = event.data;
        this.updateTransferProgress(progress);
    }
    
    createTransferProgress(transfer) {
        const container = document.getElementById('file-transfers');
        const element = document.createElement('div');
        element.id = `transfer-${transfer.transfer_id}`;
        element.innerHTML = `
            <div class="file-transfer">
                <div class="transfer-info">
                    <span class="filename">${transfer.filename}</span>
                    <span class="file-size">${formatBytes(transfer.file_size)}</span>
                    <span class="transfer-direction">${transfer.direction}</span>
                </div>
                <div class="progress-bar">
                    <div class="progress-fill"></div>
                </div>
                <div class="transfer-stats">
                    <span class="bytes-transferred">0 B</span>
                    <span class="transfer-rate">0 B/s</span>
                    <span class="eta">Calculating...</span>
                </div>
                <button class="cancel-transfer" onclick="cancelTransfer('${transfer.transfer_id}')">
                    Cancel
                </button>
            </div>
        `;
        container.appendChild(element);
    }
    
    updateTransferProgress(progress) {
        const element = document.getElementById(`transfer-${progress.transfer_id}`);
        const percentage = (progress.bytes_transferred / progress.total_bytes) * 100;
        
        // Update progress bar
        const progressFill = element.querySelector('.progress-fill');
        progressFill.style.width = `${percentage}%`;
        
        // Update statistics
        element.querySelector('.bytes-transferred').textContent = 
            formatBytes(progress.bytes_transferred);
        element.querySelector('.transfer-rate').textContent = 
            formatBytes(progress.transfer_rate) + '/s';
        
        if (progress.eta_seconds) {
            element.querySelector('.eta').textContent = 
                formatDuration(progress.eta_seconds);
        }
    }
}

function formatBytes(bytes) {
    const sizes = ['B', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
}

function formatDuration(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}
```

## System Event Streaming

### Infrastructure Events

Real-time infrastructure status updates:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRotationEvent {
    pub old_domain: String,
    pub new_domain: String,
    pub rotation_reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateExpiryEvent {
    pub domain: String,
    pub certificate_type: String,
    pub expires_at: DateTime<Utc>,
    pub days_remaining: i64,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAlertEvent {
    pub alert_id: String,
    pub severity: AlertSeverity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub auto_resolve: bool,
}
```

### Alert Management

Real-time alert handling:

```javascript
// System alert handler
class AlertManager {
    constructor() {
        this.activeAlerts = new Map();
    }
    
    handleSystemAlert(event) {
        const alert = event.data;
        this.activeAlerts.set(alert.alert_id, alert);
        
        // Show browser notification for critical alerts
        if (alert.severity === 'critical') {
            this.showBrowserNotification(alert);
        }
        
        // Add to alerts list
        this.addToAlertsList(alert);
        
        // Update alert counters
        this.updateAlertCounters();
    }
    
    showBrowserNotification(alert) {
        if (Notification.permission === 'granted') {
            new Notification(`Nexus Alert: ${alert.title}`, {
                body: alert.description,
                icon: '/assets/alert-icon.png',
                tag: alert.alert_id
            });
        }
    }
    
    addToAlertsList(alert) {
        const alertsList = document.getElementById('alerts-list');
        const alertElement = document.createElement('div');
        alertElement.className = `alert alert-${alert.severity}`;
        alertElement.innerHTML = `
            <div class="alert-header">
                <span class="alert-title">${alert.title}</span>
                <span class="alert-time">${formatTimestamp(alert.timestamp)}</span>
                <span class="alert-severity severity-${alert.severity}">${alert.severity}</span>
            </div>
            <div class="alert-description">${alert.description}</div>
            <div class="alert-actions">
                <button onclick="acknowledgeAlert('${alert.alert_id}')">Acknowledge</button>
                ${alert.auto_resolve ? '' : '<button onclick="resolveAlert(\'' + alert.alert_id + '\')">Resolve</button>'}
            </div>
        `;
        alertsList.insertBefore(alertElement, alertsList.firstChild);
    }
}

const alertManager = new AlertManager();

// Request notification permission on page load
if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission();
}
```

## Performance Monitoring

### Real-time Metrics

Live system performance monitoring:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEvent {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_total: u64,
    pub network_in: u64,
    pub network_out: u64,
    pub active_connections: usize,
    pub task_queue_size: usize,
}

async fn broadcast_performance_metrics(&self) {
    let metrics = self.collect_performance_metrics().await;
    
    let event = WebSocketEvent::PerformanceMetrics(PerformanceEvent {
        timestamp: Utc::now(),
        cpu_usage: metrics.cpu_usage,
        memory_usage: metrics.memory_usage,
        memory_total: metrics.memory_total,
        network_in: metrics.network_in,
        network_out: metrics.network_out,
        active_connections: metrics.active_connections,
        task_queue_size: metrics.task_queue_size,
    });
    
    self.broadcast_event(event).await;
}
```

### Live Charts and Graphs

Real-time performance visualization:

```javascript
// Real-time performance charts
class PerformanceMonitor {
    constructor() {
        this.initializeCharts();
        this.dataPoints = {
            cpu: [],
            memory: [],
            network: []
        };
        this.maxDataPoints = 60; // Keep last 60 data points
    }
    
    initializeCharts() {
        // CPU Usage Chart
        this.cpuChart = new Chart(document.getElementById('cpu-chart'), {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'CPU Usage %',
                    data: [],
                    borderColor: 'rgb(75, 192, 192)',
                    backgroundColor: 'rgba(75, 192, 192, 0.2)',
                    tension: 0.1
                }]
            },
            options: {
                responsive: true,
                scales: {
                    y: {
                        beginAtZero: true,
                        max: 100
                    }
                }
            }
        });
        
        // Memory Usage Chart
        this.memoryChart = new Chart(document.getElementById('memory-chart'), {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'Memory Usage GB',
                    data: [],
                    borderColor: 'rgb(255, 99, 132)',
                    backgroundColor: 'rgba(255, 99, 132, 0.2)',
                    tension: 0.1
                }]
            },
            options: {
                responsive: true,
                scales: {
                    y: {
                        beginAtZero: true
                    }
                }
            }
        });
    }
    
    handlePerformanceUpdate(event) {
        const metrics = event.data;
        const timestamp = new Date(metrics.timestamp).toLocaleTimeString();
        
        // Update CPU chart
        this.updateChart(this.cpuChart, timestamp, metrics.cpu_usage);
        
        // Update memory chart  
        const memoryGB = metrics.memory_usage / (1024 * 1024 * 1024);
        this.updateChart(this.memoryChart, timestamp, memoryGB);
        
        // Update dashboard stats
        document.getElementById('cpu-usage').textContent = `${metrics.cpu_usage.toFixed(1)}%`;
        document.getElementById('memory-usage').textContent = 
            `${formatBytes(metrics.memory_usage)} / ${formatBytes(metrics.memory_total)}`;
        document.getElementById('active-connections').textContent = metrics.active_connections;
        document.getElementById('task-queue-size').textContent = metrics.task_queue_size;
    }
    
    updateChart(chart, label, value) {
        chart.data.labels.push(label);
        chart.data.datasets[0].data.push(value);
        
        // Keep only last N data points
        if (chart.data.labels.length > this.maxDataPoints) {
            chart.data.labels.shift();
            chart.data.datasets[0].data.shift();
        }
        
        chart.update('none'); // Update without animation for better performance
    }
}

const performanceMonitor = new PerformanceMonitor();
```

## Error Handling and Recovery

### Connection Recovery

Automatic WebSocket reconnection:

```javascript
class WebSocketManager {
    constructor(url, options = {}) {
        this.url = url;
        this.options = options;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = options.maxReconnectAttempts || 5;
        this.reconnectInterval = options.reconnectInterval || 5000;
        this.subscriptions = [];
        this.messageQueue = [];
        
        this.connect();
    }
    
    connect() {
        try {
            this.ws = new WebSocket(this.url);
            this.setupEventHandlers();
        } catch (error) {
            console.error('WebSocket connection failed:', error);
            this.handleConnectionFailure();
        }
    }
    
    setupEventHandlers() {
        this.ws.onopen = (event) => {
            console.log('WebSocket connected');
            this.reconnectAttempts = 0;
            
            // Resubscribe to events
            this.resubscribe();
            
            // Send queued messages
            this.flushMessageQueue();
            
            if (this.onConnected) this.onConnected(event);
        };
        
        this.ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                if (this.onMessage) this.onMessage(message);
            } catch (error) {
                console.error('Failed to parse WebSocket message:', error);
            }
        };
        
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            if (this.onError) this.onError(error);
        };
        
        this.ws.onclose = (event) => {
            console.log('WebSocket connection closed:', event.code, event.reason);
            this.handleConnectionClose(event);
        };
    }
    
    handleConnectionClose(event) {
        // Don't reconnect if closed intentionally
        if (event.code === 1000) return;
        
        // Attempt to reconnect
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);
            
            setTimeout(() => {
                this.connect();
            }, this.reconnectInterval * this.reconnectAttempts);
        } else {
            console.error('Max reconnection attempts reached');
            if (this.onReconnectFailed) this.onReconnectFailed();
        }
    }
    
    send(message) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        } else {
            // Queue message for later
            this.messageQueue.push(message);
        }
    }
    
    subscribe(events, filters = {}) {
        const subscription = { events, filters };
        this.subscriptions.push(subscription);
        
        this.send({
            type: 'subscribe',
            events: events,
            filters: filters
        });
    }
    
    resubscribe() {
        for (const subscription of this.subscriptions) {
            this.send({
                type: 'subscribe',
                events: subscription.events,
                filters: subscription.filters
            });
        }
    }
    
    flushMessageQueue() {
        while (this.messageQueue.length > 0) {
            const message = this.messageQueue.shift();
            this.send(message);
        }
    }
}

// Usage
const wsManager = new WebSocketManager('wss://your-nexus-server:8080/ws', {
    maxReconnectAttempts: 10,
    reconnectInterval: 5000
});

wsManager.onMessage = function(message) {
    // Handle incoming messages
    handleWebSocketMessage(message);
};

wsManager.onConnected = function() {
    // Subscribe to events after connection
    wsManager.subscribe(['agent_updates', 'task_progress', 'system_alerts']);
};
```

## Rate Limiting and Throttling

### Server-side Rate Limiting

```rust
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_capacity: u32,
    refill_rate: u32,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, connection_id: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets.entry(connection_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_capacity, self.refill_rate));
        
        bucket.consume(1)
    }
}

async fn broadcast_event_with_rate_limiting(&self, event: WebSocketEvent) {
    let connections = self.connections.read().await;
    
    for (connection_id, connection) in connections.iter() {
        // Check if client is subscribed to this event type
        if self.is_subscribed(connection, &event) {
            // Apply rate limiting
            if self.rate_limiter.check_rate_limit(connection_id).await {
                if let Err(e) = connection.sender.send(WebSocketMessage::Event(event.clone())) {
                    warn!("Failed to send event to connection {}: {}", connection_id, e);
                }
            } else {
                debug!("Rate limit exceeded for connection: {}", connection_id);
            }
        }
    }
}
```

### Client-side Throttling

```javascript
// Client-side event throttling
class EventThrottler {
    constructor(maxEventsPerSecond = 10) {
        this.maxEventsPerSecond = maxEventsPerSecond;
        this.eventQueue = [];
        this.lastProcessTime = Date.now();
        this.processing = false;
    }
    
    addEvent(event) {
        this.eventQueue.push(event);
        if (!this.processing) {
            this.processQueue();
        }
    }
    
    processQueue() {
        this.processing = true;
        const now = Date.now();
        const timeSinceLastProcess = now - this.lastProcessTime;
        const maxEvents = Math.floor((timeSinceLastProcess / 1000) * this.maxEventsPerSecond);
        
        // Process up to maxEvents from the queue
        const eventsToProcess = this.eventQueue.splice(0, maxEvents);
        
        for (const event of eventsToProcess) {
            this.handleEvent(event);
        }
        
        this.lastProcessTime = now;
        
        // Schedule next processing if queue is not empty
        if (this.eventQueue.length > 0) {
            setTimeout(() => this.processQueue(), 100);
        } else {
            this.processing = false;
        }
    }
    
    handleEvent(event) {
        // Process the event
        switch(event.type) {
            case 'AgentStatusUpdate':
                updateAgentStatus(event.data);
                break;
            case 'TaskProgress':
                updateTaskProgress(event.data);
                break;
            case 'PerformanceMetrics':
                updatePerformanceCharts(event.data);
                break;
        }
    }
}

const throttler = new EventThrottler(20); // Max 20 events per second

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    throttler.addEvent(message);
};
```

## Integration Examples

### Complete WebSocket Integration

```javascript
// Complete Nexus WebSocket client implementation
class NexusWebSocketClient {
    constructor(serverUrl, options = {}) {
        this.serverUrl = serverUrl;
        this.options = {
            autoReconnect: true,
            maxReconnectAttempts: 10,
            reconnectInterval: 5000,
            heartbeatInterval: 30000,
            ...options
        };
        
        this.eventHandlers = new Map();
        this.subscriptions = new Set();
        this.wsManager = null;
        this.heartbeatTimer = null;
        
        this.init();
    }
    
    init() {
        this.wsManager = new WebSocketManager(this.serverUrl + '/ws', {
            maxReconnectAttempts: this.options.maxReconnectAttempts,
            reconnectInterval: this.options.reconnectInterval
        });
        
        this.wsManager.onMessage = (message) => this.handleMessage(message);
        this.wsManager.onConnected = () => this.handleConnected();
        this.wsManager.onError = (error) => this.handleError(error);
        
        this.startHeartbeat();
    }
    
    handleMessage(message) {
        const handlers = this.eventHandlers.get(message.type);
        if (handlers) {
            handlers.forEach(handler => {
                try {
                    handler(message.data);
                } catch (error) {
                    console.error(`Error in event handler for ${message.type}:`, error);
                }
            });
        }
    }
    
    handleConnected() {
        console.log('Connected to Nexus server');
        
        // Resubscribe to events
        if (this.subscriptions.size > 0) {
            this.wsManager.send({
                type: 'subscribe',
                events: Array.from(this.subscriptions)
            });
        }
        
        // Trigger connected event
        this.emit('connected');
    }
    
    handleError(error) {
        console.error('WebSocket error:', error);
        this.emit('error', error);
    }
    
    // Event subscription methods
    subscribe(eventType, handler) {
        if (!this.eventHandlers.has(eventType)) {
            this.eventHandlers.set(eventType, new Set());
        }
        this.eventHandlers.get(eventType).add(handler);
        this.subscriptions.add(eventType);
        
        // Send subscription to server if connected
        if (this.wsManager && this.wsManager.ws && this.wsManager.ws.readyState === WebSocket.OPEN) {
            this.wsManager.send({
                type: 'subscribe',
                events: [eventType]
            });
        }
    }
    
    unsubscribe(eventType, handler) {
        const handlers = this.eventHandlers.get(eventType);
        if (handlers) {
            handlers.delete(handler);
            if (handlers.size === 0) {
                this.eventHandlers.delete(eventType);
                this.subscriptions.delete(eventType);
                
                // Send unsubscription to server
                this.wsManager.send({
                    type: 'unsubscribe',
                    events: [eventType]
                });
            }
        }
    }
    
    // Utility methods
    emit(eventType, data) {
        const handlers = this.eventHandlers.get(eventType);
        if (handlers) {
            handlers.forEach(handler => handler(data));
        }
    }
    
    startHeartbeat() {
        if (this.options.heartbeatInterval > 0) {
            this.heartbeatTimer = setInterval(() => {
                this.wsManager.send({
                    type: 'ping',
                    timestamp: Date.now()
                });
            }, this.options.heartbeatInterval);
        }
    }
    
    disconnect() {
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
        }
        
        if (this.wsManager && this.wsManager.ws) {
            this.wsManager.ws.close(1000, 'Client disconnect');
        }
    }
}

// Usage example
const nexusClient = new NexusWebSocketClient('wss://your-nexus-server:8080');

// Subscribe to various events
nexusClient.subscribe('AgentConnected', (agent) => {
    console.log('New agent connected:', agent.hostname);
    addAgentToUI(agent);
});

nexusClient.subscribe('TaskCompleted', (task) => {
    console.log('Task completed:', task.task_id);
    updateTaskStatus(task);
});

nexusClient.subscribe('SystemAlert', (alert) => {
    if (alert.severity === 'critical') {
        showCriticalAlert(alert);
    }
});

nexusClient.subscribe('connected', () => {
    console.log('Successfully connected to Nexus server');
    updateConnectionStatus(true);
});
```

## Best Practices

### Performance Optimization

1. **Event Batching**
```javascript
// Batch similar events to reduce UI updates
class EventBatcher {
    constructor(batchSize = 10, flushInterval = 100) {
        this.batches = new Map();
        this.batchSize = batchSize;
        this.flushInterval = flushInterval;
        this.flushTimer = setInterval(() => this.flush(), flushInterval);
    }
    
    addEvent(eventType, event) {
        if (!this.batches.has(eventType)) {
            this.batches.set(eventType, []);
        }
        
        const batch = this.batches.get(eventType);
        batch.push(event);
        
        if (batch.length >= this.batchSize) {
            this.flushBatch(eventType);
        }
    }
    
    flush() {
        for (const [eventType, batch] of this.batches) {
            if (batch.length > 0) {
                this.flushBatch(eventType);
            }
        }
    }
    
    flushBatch(eventType) {
        const batch = this.batches.get(eventType);
        if (batch && batch.length > 0) {
            this.processBatch(eventType, batch);
            this.batches.set(eventType, []);
        }
    }
}
```

2. **Memory Management**
```javascript
// Clean up old event data to prevent memory leaks
class EventHistory {
    constructor(maxSize = 1000) {
        this.events = [];
        this.maxSize = maxSize;
    }
    
    addEvent(event) {
        this.events.push({
            ...event,
            timestamp: Date.now()
        });
        
        // Remove old events
        if (this.events.length > this.maxSize) {
            this.events.splice(0, this.events.length - this.maxSize);
        }
    }
    
    getRecentEvents(count = 100) {
        return this.events.slice(-count);
    }
    
    clearOldEvents(maxAge = 3600000) { // 1 hour
        const cutoff = Date.now() - maxAge;
        this.events = this.events.filter(event => event.timestamp > cutoff);
    }
}
```

### Security Considerations

1. **Message Validation**
```javascript
function validateWebSocketMessage(message) {
    // Validate message structure
    if (!message || typeof message !== 'object') {
        throw new Error('Invalid message format');
    }
    
    // Validate required fields
    if (!message.type || typeof message.type !== 'string') {
        throw new Error('Missing or invalid message type');
    }
    
    // Validate data based on message type
    switch (message.type) {
        case 'AgentConnected':
            validateAgentData(message.data);
            break;
        case 'TaskProgress':
            validateTaskProgress(message.data);
            break;
        // Add other validations
    }
}
```

2. **Authentication**
```javascript
// Include authentication token in WebSocket connection
const ws = new WebSocket('wss://server/ws', [], {
    headers: {
        'Authorization': `Bearer ${authToken}`
    }
});

// Handle authentication failures
ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    
    if (message.type === 'AuthenticationError') {
        console.error('WebSocket authentication failed');
        // Redirect to login or refresh token
        handleAuthenticationFailure();
    }
};
```

This comprehensive guide covers all aspects of real-time interaction with the Nexus C2 framework, providing the foundation for building responsive, interactive applications on top of the Nexus infrastructure.
