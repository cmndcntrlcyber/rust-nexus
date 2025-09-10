# Web Interface User Guide

Complete guide to using the Nexus Web UI for interactive management of agents, tasks, and infrastructure through a modern web interface.

## Overview

The Nexus Web UI provides a comprehensive browser-based management interface featuring:
- **Real-time Agent Management** - Live agent status, capabilities, and interaction
- **Task Execution Interface** - Interactive task creation, monitoring, and results
- **Infrastructure Dashboard** - Domain management, certificates, and system health
- **Real-time Updates** - WebSocket-powered live data and notifications
- **File Management** - Upload/download files to/from agents
- **System Monitoring** - Performance metrics and operational insights

## Getting Started

### Accessing the Web Interface

The Web UI runs on a separate port from the gRPC server:

```bash
# Start the web UI server
./target/release/nexus-webui --config nexus.toml

# Default access URL
https://localhost:8080

# Custom configuration
./target/release/nexus-webui --port 3000 --bind 192.168.1.100
```

### Initial Setup

1. **Navigate to Web Interface**
   ```
   https://your-server:8080
   ```

2. **Authentication** (if configured)
   - Enter credentials
   - SSL certificate validation
   - Session establishment

3. **Dashboard Overview**
   - Connected agents summary
   - System health status
   - Recent activity feed

## Dashboard Interface

### Main Dashboard

The main dashboard provides an overview of your C2 infrastructure:

```
┌─────────────────────────────────────────────────────────────────┐
│ 🏠 Nexus C2 Dashboard                                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ 📊 System Status                    📋 Recent Activity          │
│ ┌─────────────────────────────┐    ┌─────────────────────────────┐ │
│ │ Active Agents: 15           │    │ 10:30 Agent WIN-01 connected│ │
│ │ Total Tasks: 42             │    │ 10:25 Task completed: whoami│ │
│ │ Infrastructure: ✅ Healthy   │    │ 10:20 Domain rotation       │ │
│ │ Certificates: ⚠️ Expiring   │    │ 10:15 File upload complete  │ │
│ └─────────────────────────────┘    └─────────────────────────────┘ │
│                                                                 │
│ 🖥️ Agent Overview              🌐 Infrastructure Status         │
│ ┌─────────────────────────────┐    ┌─────────────────────────────┐ │
│ │ [Windows] WIN-DESKTOP-01    │    │ Primary Domain: ✅ Active   │ │
│ │ [Linux] Ubuntu-Server-02    │    │ Backup Domains: 3 Active   │ │
│ │ [MacOS] MacBook-Pro-03      │    │ Certificates: 2 Valid      │ │
│ └─────────────────────────────┘    └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Navigation Menu

- **🏠 Dashboard** - Main overview
- **🖥️ Agents** - Agent management
- **📋 Tasks** - Task execution and monitoring
- **📁 Files** - File management
- **🌐 Infrastructure** - Domain and certificate management
- **📊 Monitoring** - System metrics and logs
- **⚙️ Settings** - Configuration management

## Agent Management Interface

### Agent List View

Browse and manage all connected agents:

```
┌─────────────────────────────────────────────────────────────────┐
│ 🖥️ Agent Management                                   [+] Deploy │
├─────────────────────────────────────────────────────────────────┤
│ Filter: [All] [Active] [Inactive] [Windows] [Linux] [MacOS]     │
├─────────────────────────────────────────────────────────────────┤
│ ID          | Hostname       | Platform | IP Address    | Status│
│ a1b2c3d4    | WIN-DESKTOP-01 | Windows  | 192.168.1.100 | 🟢    │
│ e5f6g7h8    | Ubuntu-Srv-02  | Linux    | 192.168.1.101 | 🟢    │
│ i9j0k1l2    | MacBook-Pro-03 | MacOS    | 192.168.1.102 | 🟡    │
│ m3n4o5p6    | WIN-LAPTOP-04  | Windows  | 10.0.0.50     | 🔴    │
└─────────────────────────────────────────────────────────────────┘
```

**Status Indicators:**
- 🟢 **Active** - Currently connected and responsive
- 🟡 **Inactive** - Connected but not responding to heartbeats
- 🔴 **Disconnected** - No recent communication
- ⚙️ **Executing** - Currently running tasks

### Agent Details View

Click on any agent to view detailed information:

```
┌─────────────────────────────────────────────────────────────────┐
│ 🖥️ Agent: WIN-DESKTOP-01 (a1b2c3d4)                    [Actions]│
├─────────────────────────────────────────────────────────────────┤
│ 📋 System Information                                           │
│ Hostname: WIN-DESKTOP-01        OS: Windows 11 Pro            │
│ IP Address: 192.168.1.100       Architecture: x64             │
│ Username: alice.smith           Process: svchost.exe (1234)    │
│ Connected: 2024-01-15 10:30:15  Last Seen: 2024-01-15 11:45:22│
│                                                                │
│ 🛠️ Capabilities                                                │
│ [✅] Shell Commands    [✅] File Operations  [✅] BOF Execution  │
│ [✅] Fiber Techniques  [❌] Keylogging      [✅] Screenshot     │
│                                                                │
│ 📋 Active Tasks (2)                                            │
│ ┌─────────────────────────────────────────────────────────────┐  │
│ │ Task ID: task-001 | Type: Shell | Command: "whoami"        │  │
│ │ Status: Running   | Started: 11:44:30 | Progress: 80%     │  │
│ │ Task ID: task-002 | Type: BOF   | Function: "go"          │  │
│ │ Status: Queued    | Scheduled: 11:45:00                   │  │
│ └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Action Buttons:**
- **Execute Task** - Send new task to agent
- **Upload File** - Transfer file to agent
- **Download File** - Retrieve file from agent
- **Shell** - Interactive shell session
- **Kill** - Terminate agent connection

### Interactive Task Execution

#### Shell Command Interface

Execute commands directly through the web interface:

```
┌─────────────────────────────────────────────────────────────────┐
│ 💻 Shell - WIN-DESKTOP-01                                [Close] │
├─────────────────────────────────────────────────────────────────┤
│ C:\Users\alice> whoami                                          │
│ DESKTOP-01\alice                                                │
│                                                                 │
│ C:\Users\alice> dir                                             │
│  Volume in drive C has no label.                               │
│  Directory of C:\Users\alice                                    │
│                                                                 │
│ 01/15/2024  11:30 AM    <DIR>          .                       │
│ 01/15/2024  11:30 AM    <DIR>          ..                      │
│ 01/15/2024  10:45 AM    <DIR>          Desktop                 │
│ 01/15/2024  10:45 AM    <DIR>          Documents               │
│                                                                 │
│ C:\Users\alice> █                                               │
│                                                                 │
│ Command: [________________________] [Execute] [Clear] [History] │
└─────────────────────────────────────────────────────────────────┘
```

#### Task Creation Form

Create structured tasks with parameters:

```
┌─────────────────────────────────────────────────────────────────┐
│ ➕ Create Task for WIN-DESKTOP-01                               │
├─────────────────────────────────────────────────────────────────┤
│ Task Type: [Shell Command    ▼]                                │
│                                                                 │
│ Command: [whoami /all                                     ]     │
│                                                                 │
│ ⚙️ Advanced Options                                             │
│ Timeout (seconds): [30      ]  Priority: [Normal ▼]           │
│ Max Retries: [3]               Schedule: [Now     ▼]          │
│                                                                 │
│ 📋 Parameters                                                   │
│ Key: [output_format] Value: [json     ]                        │
│ Key: [               ] Value: [         ] [+]                  │
│                                                                 │
│                               [Cancel] [Execute Task]          │
└─────────────────────────────────────────────────────────────────┘
```

**Available Task Types:**
- **Shell Command** - Execute system commands
- **BOF Execution** - Run Beacon Object Files
- **File Download** - Retrieve files from agent
- **File Upload** - Send files to agent
- **Screenshot** - Capture screen
- **Process List** - Get running processes
- **Registry Query** - Read registry keys
- **Service Control** - Start/stop services

## File Management Interface

### File Browser

Navigate the agent's filesystem:

```
┌─────────────────────────────────────────────────────────────────┐
│ 📁 File Browser - WIN-DESKTOP-01                               │
├─────────────────────────────────────────────────────────────────┤
│ Path: C:\Users\alice\Documents                    [Upload] [⟲]  │
├─────────────────────────────────────────────────────────────────┤
│ Name                     | Size     | Modified     | Actions    │
│ 📁 Projects              | -        | 01/15 10:30  | [Open]     │
│ 📁 Reports               | -        | 01/12 14:20  | [Open]     │
│ 📄 important.docx        | 2.5 MB   | 01/15 11:20  | [↓][✏️][🗑️]│
│ 📄 passwords.txt         | 156 B    | 01/10 09:15  | [↓][✏️][🗑️]│
│ 📄 schedule.pdf          | 890 KB   | 01/14 16:30  | [↓][✏️][🗑️]│
└─────────────────────────────────────────────────────────────────┘
```

**File Actions:**
- **↓ Download** - Transfer file to operator
- **✏️ Edit** - Modify text files
- **🗑️ Delete** - Remove file
- **📋 Copy Path** - Copy full file path

### File Upload Interface

Transfer files to agents:

```
┌─────────────────────────────────────────────────────────────────┐
│ ⬆️ Upload File to WIN-DESKTOP-01                                │
├─────────────────────────────────────────────────────────────────┤
│ Destination Path: [C:\Users\alice\Desktop\               ]     │
│                                                                 │
│ 📂 Select Files                                                 │
│ ┌─────────────────────────────────────────────────────────────┐   │
│ │ 📄 payload.exe          (2.1 MB)                    [Remove]│   │
│ │ 📄 config.json          (456 B)                     [Remove]│   │
│ │                                                             │   │
│ │         [Drag & Drop Files Here] or [Browse...]             │   │
│ └─────────────────────────────────────────────────────────────┘   │
│                                                                 │
│ ⚙️ Upload Options                                               │
│ [ ] Overwrite existing files                                   │
│ [ ] Create directory if not exists                             │
│ [ ] Execute after upload                                       │
│                                                                 │
│                                    [Cancel] [Start Upload]     │
└─────────────────────────────────────────────────────────────────┘
```

### Transfer Progress

Monitor file transfers in real-time:

```
┌─────────────────────────────────────────────────────────────────┐
│ 📊 Active Transfers                                             │
├─────────────────────────────────────────────────────────────────┤
│ ⬆️ Upload: payload.exe → WIN-DESKTOP-01                        │
│ ████████████████████░░░░ 80% (1.68 MB / 2.1 MB)              │
│ Speed: 245 KB/s | ETA: 00:02 | [Pause] [Cancel]              │
│                                                                 │
│ ⬇️ Download: document.pdf ← Ubuntu-Srv-02                      │
│ ██████████████████████ 100% (890 KB / 890 KB)               │
│ Completed in 00:03 | [Open Folder] [View File]               │
└─────────────────────────────────────────────────────────────────┘
```

## Infrastructure Management

### Domain Management Interface

Monitor and control domain infrastructure:

```
┌─────────────────────────────────────────────────────────────────┐
│ 🌐 Infrastructure Management                                    │
├─────────────────────────────────────────────────────────────────┤
│ 📋 Domains                                       [Rotate Now]   │
│ ┌─────────────────────────────────────────────────────────────┐   │
│ │ Primary: c2.example.com          Status: ✅ Active         │   │
│ │ Backup:  backup.example.com      Status: ✅ Active         │   │
│ │ Backup:  secondary.example.com   Status: ⚠️ Degraded      │   │
│ │ Next Rotation: 2024-01-16 10:30:15 (22h 45m)             │   │
│ └─────────────────────────────────────────────────────────────┘   │
│                                                                 │
│ 🔐 Certificates                                [Renew All]      │
│ ┌─────────────────────────────────────────────────────────────┐   │
│ │ c2.example.com           Expires: 2024-03-15 (58 days)    │   │
│ │ *.example.com            Expires: 2024-02-20 (35 days)    │   │
│ │ backup.example.com       Expires: 2024-04-01 (76 days)    │   │
│ └─────────────────────────────────────────────────────────────┘   │
│                                                                 │
│ 📊 Health Metrics                                              │
│ ┌─────────────────────────────────────────────────────────────┐   │
│ │ Domain Uptime: 99.8%     Response Time: 45ms avg          │   │
│ │ Certificate Valid: Yes   TLS Handshake: 12ms avg          │   │
│ │ DNS Resolution: 8ms      CDN Cache Hit: 94%               │   │
│ └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Cloudflare Integration

Manage Cloudflare DNS settings:

```
┌─────────────────────────────────────────────────────────────────┐
│ ☁️ Cloudflare Management                                        │
├─────────────────────────────────────────────────────────────────┤
│ Zone: example.com                        API Status: ✅ Connected│
│                                                                 │
│ 📋 DNS Records                                      [Add Record] │
│ ┌─────────────────────────────────────────────────────────────┐   │
│ │ Type | Name        | Content        | TTL  | Proxy | Actions│   │
│ │ A    | c2          | 203.0.113.10   | 300  |  ✅   | [✏️][🗑️]│   │
│ │ A    | backup      | 203.0.113.11   | 300  |  ✅   | [✏️][🗑️]│   │
│ │ TXT  | _acme-chall | validation...  | 120  |  ❌   | [✏️][🗑️]│   │
│ └─────────────────────────────────────────────────────────────┘   │
│                                                                 │
│ ⚡ Quick Actions                                                │
│ [Create Subdomain] [Rotate IPs] [Update Proxy Settings]       │
└─────────────────────────────────────────────────────────────────┘
```

## Real-time Monitoring

### Live Activity Feed

Monitor all system activity in real-time:

```
┌─────────────────────────────────────────────────────────────────┐
│ 📊 Live Activity Monitor                          [Pause] [Clear]│
├─────────────────────────────────────────────────────────────────┤
│ 11:45:30 📱 Agent a1b2c3d4 connected from 192.168.1.100       │
│ 11:45:25 ⚡ Task completed: whoami (agent: a1b2c3d4)           │
│ 11:45:20 🌐 Domain health check: c2.example.com ✅            │
│ 11:45:15 📄 File upload started: payload.exe → a1b2c3d4       │
│ 11:45:10 ⚠️ Certificate expiring: *.example.com (35 days)     │
│ 11:45:05 🔄 Agent heartbeat: e5f6g7h8                         │
│ 11:45:00 🗑️ Cleaned up inactive agent: old-agent-123          │
│ 11:44:55 🚀 BOF execution queued: agent e5f6g7h8              │
└─────────────────────────────────────────────────────────────────┘
```

### System Metrics Dashboard

View performance metrics and graphs:

```
┌─────────────────────────────────────────────────────────────────┐
│ 📈 System Metrics                                    [Refresh]  │
├─────────────────────────────────────────────────────────────────┤
│ 🖥️ Server Performance         📊 Agent Statistics              │
│ ┌─────────────────────────┐    ┌─────────────────────────────────┐ │
│ │ CPU Usage:  15%         │    │ Total Agents: 15               │ │
│ │ Memory: 2.4GB / 8GB     │    │ Active: 12 | Inactive: 3       │ │
│ │ Network: ↑ 45KB/s       │    │ Tasks/min: 23                   │ │
│ │         ↓ 128KB/s       │    │ Success Rate: 96.2%             │ │
│ └─────────────────────────┘    └─────────────────────────────────┘ │
│                                                                 │
│ 🌐 Infrastructure Health      📋 Task Queue Status              │
│ ┌─────────────────────────┐    ┌─────────────────────────────────┐ │
│ │ Domains: 3/3 Active     │    │ Pending: 8                     │ │
│ │ Certificates: Valid     │    │ Running: 5                     │ │
│ │ DNS Response: 8ms       │    │ Completed: 157                 │ │
│ │ CDN Health: 99.9%       │    │ Failed: 3                      │ │
│ └─────────────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## WebSocket Real-time Updates

### Event Stream Configuration

Configure real-time event subscriptions:

```javascript
// WebSocket connection example
const ws = new WebSocket('wss://your-server:8080/ws');

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    switch(data.type) {
        case 'AgentConnected':
            updateAgentList(data.data);
            break;
        case 'TaskResult':
            displayTaskResult(data.data);
            break;
        case 'SystemAlert':
            showAlert(data.data);
            break;
    }
};
```

### Live Notifications

Real-time browser notifications for important events:

```
🔔 Notifications

[New] Agent WIN-DESKTOP-05 connected        [11:46:15]
[Task] Screenshot completed from agent e5f6g7h8  [11:46:10]
[Alert] Certificate expires in 7 days       [11:45:00]
[File] Upload completed: payload.exe         [11:44:30]
```

## Settings and Configuration

### Web UI Configuration

Customize the web interface:

```
┌─────────────────────────────────────────────────────────────────┐
│ ⚙️ Settings                                                      │
├─────────────────────────────────────────────────────────────────┤
│ 🎨 Appearance                                                   │
│ Theme: [Dark ▼]                 Refresh Rate: [5 seconds ▼]    │
│ Language: [English ▼]           Time Zone: [UTC-5 ▼]          │
│                                                                 │
│ 🔔 Notifications                                               │
│ [ ✅ ] Browser notifications    [ ✅ ] Sound alerts           │
│ [ ✅ ] Agent connections        [ ❌ ] Task completions       │
│ [ ✅ ] System alerts           [ ✅ ] File transfers         │
│                                                                 │
│ 🛡️ Security                                                    │
│ Session timeout: [30 minutes ▼]                               │
│ [ ✅ ] Require re-authentication for sensitive actions        │
│ [ ✅ ] Log all user actions                                   │
│                                                                 │
│ 🔧 Advanced                                                    │
│ gRPC Endpoint: [https://localhost:8443        ]               │
│ Connection timeout: [30 seconds]                              │
│ Max file size: [100 MB]                                       │
│                                                                 │
│                                      [Reset] [Save Changes]    │
└─────────────────────────────────────────────────────────────────┘
```

## Keyboard Shortcuts

Enhance productivity with keyboard shortcuts:

- **Ctrl+/** - Show help overlay
- **Ctrl+K** - Quick command search
- **Ctrl+R** - Refresh current view
- **Ctrl+1-9** - Switch between tabs/views
- **Esc** - Close dialogs/cancel actions
- **Ctrl+Enter** - Execute commands in shell
- **Ctrl+Shift+T** - New task creation
- **Ctrl+Shift+F** - Global search

## Mobile Responsive Interface

The web UI adapts to mobile devices for on-the-go management:

- **Responsive Design** - Optimized for tablets and phones
- **Touch-Friendly** - Large buttons and gesture support
- **Simplified Views** - Essential information prioritized
- **Offline Notifications** - Service worker for offline alerts

## API Integration

### REST API Endpoints

The web UI exposes REST APIs for integration:

```bash
# Get agent list
GET /api/agents

# Get agent details
GET /api/agents/{agent_id}

# Execute task
POST /api/agents/{agent_id}/tasks

# Get system information
GET /api/system

# Domain management
GET /api/domains
POST /api/domains/rotate
```

### WebSocket API

Real-time events via WebSocket:

```javascript
// Event types
{
    "type": "AgentConnected",
    "data": { "agent_id": "...", "hostname": "..." }
}

{
    "type": "TaskResult", 
    "data": { "agent_id": "...", "task_id": "...", "result": "..." }
}

{
    "type": "SystemAlert",
    "data": { "level": "warning", "message": "Certificate expiring" }
}
```

## Best Practices

### Security Considerations

1. **HTTPS Only** - Always use TLS encryption
2. **Authentication** - Implement proper user authentication
3. **Session Management** - Use secure session handling
4. **Input Validation** - Sanitize all user inputs
5. **Rate Limiting** - Prevent abuse of API endpoints

### Performance Optimization

1. **Lazy Loading** - Load data as needed
2. **Caching** - Cache static resources
3. **Compression** - Enable gzip/brotli compression
4. **WebSocket Efficiency** - Minimize message frequency
5. **Browser Compatibility** - Test on multiple browsers

### User Experience

1. **Progressive Enhancement** - Ensure basic functionality without JS
2. **Error Handling** - Provide clear error messages
3. **Loading States** - Show progress indicators
4. **Accessibility** - Support keyboard navigation and screen readers
5. **Mobile First** - Design for mobile devices first

This completes the comprehensive Web UI user guide. The interface provides powerful, intuitive management of the Nexus C2 infrastructure through a modern browser-based interface with real-time capabilities.
