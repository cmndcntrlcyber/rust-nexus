//! Kernel-level execution context captured during sandboxed task dispatch.
//!
//! These are the Rust-native types mirroring `KernelContextProto` and its
//! sub-messages from `a2a.proto` v1.5. The agent populates these via
//! ETW (Windows) or `/proc` + optional eBPF (Linux) observers during
//! `TaskExecutor::execute_task`, and the ferry serializes them into the
//! proto wire format for `HarnessTaskResult.kernel_context`.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// OS-level containment outcome for a sandboxed task execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxVerdict {
    Nominal,
    ResourceExceeded,
    Timeout,
    ContainmentBreach,
}

impl Default for SandboxVerdict {
    fn default() -> Self {
        Self::Nominal
    }
}

/// Process lifecycle event observed during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub parent_pid: u32,
    pub image_name: String,
    pub timestamp_unix: u64,
    /// create, terminate, thread_create, suspend, resume
    pub event_type: String,
}

/// File I/O event observed during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub path: String,
    /// create, read, write, delete, rename
    pub operation: String,
    pub bytes_affected: u64,
    pub timestamp_unix: u64,
}

/// Network connection event observed during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub src_addr: String,
    pub dst_addr: String,
    pub dst_port: u32,
    /// tcp, udp
    pub protocol: String,
    /// inbound, outbound
    pub direction: String,
    pub timestamp_unix: u64,
}

/// Aggregated kernel-level telemetry for a single task execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KernelContext {
    pub execution_duration_ns: u64,
    pub syscall_count: u64,
    pub memory_allocated_bytes: u64,
    pub processes_spawned: u32,
    pub files_touched: u32,
    pub network_connections: u32,
    pub threads_created: u32,
    pub sandbox_verdict: SandboxVerdict,
    pub process_events: Vec<ProcessEvent>,
    pub file_events: Vec<FileEvent>,
    pub network_events: Vec<NetworkEvent>,
}

/// Per-task containment policy applied by the execution sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub memory_limit_bytes: Option<u64>,
    pub process_limit: Option<u32>,
    pub cpu_timeout_secs: Option<u64>,
    pub observation_level: ObservationLevel,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_bytes: None,
            process_limit: None,
            cpu_timeout_secs: None,
            observation_level: ObservationLevel::None,
        }
    }
}

/// How much kernel-level observation to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationLevel {
    Full,
    ProcessAndNetwork,
    Minimal,
    None,
}

impl Default for ObservationLevel {
    fn default() -> Self {
        Self::None
    }
}

impl SandboxConfig {
    /// Return the appropriate containment profile for a given task type.
    pub fn for_task_type(task_type: &str) -> Self {
        match task_type {
            "shell" | "powershell" => Self {
                memory_limit_bytes: Some(512 * 1024 * 1024), // 512 MB
                process_limit: Some(32),
                cpu_timeout_secs: Some(300),
                observation_level: ObservationLevel::ProcessAndNetwork,
            },
            "fiber_shellcode" => Self {
                memory_limit_bytes: Some(256 * 1024 * 1024),
                process_limit: None,
                cpu_timeout_secs: Some(120),
                observation_level: ObservationLevel::Full,
            },
            "fiber_hollowing" | "early_bird_injection" => Self {
                memory_limit_bytes: Some(256 * 1024 * 1024),
                process_limit: Some(4),
                cpu_timeout_secs: Some(120),
                observation_level: ObservationLevel::Full,
            },
            "file_download" | "file_upload" => Self {
                memory_limit_bytes: Some(1024 * 1024 * 1024),
                process_limit: None,
                cpu_timeout_secs: Some(600),
                observation_level: ObservationLevel::Minimal,
            },
            "keylogger_start" | "keylogger_stop" | "keylogger_status" | "keylogger_flush" => Self {
                memory_limit_bytes: Some(64 * 1024 * 1024),
                process_limit: Some(2),
                cpu_timeout_secs: None, // long-running
                observation_level: ObservationLevel::Minimal,
            },
            // Read-only recon — no containment or observation
            "directory_listing" | "process_list" | "system_info" | "network_info" => Self::default(),
            // Registry/service — quick operations, full OS access needed
            "registry_query" | "registry_set" | "service_control" => Self {
                observation_level: ObservationLevel::Minimal,
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

/// Thread-safe builder that accumulates events during task execution and
/// produces an immutable `KernelContext` on finalization.
pub struct KernelContextBuilder {
    process_events: Mutex<Vec<ProcessEvent>>,
    file_events: Mutex<Vec<FileEvent>>,
    network_events: Mutex<Vec<NetworkEvent>>,
    syscall_count: Mutex<u64>,
    memory_allocated: Mutex<u64>,
    verdict: Mutex<SandboxVerdict>,
}

impl KernelContextBuilder {
    pub fn new() -> Self {
        Self {
            process_events: Mutex::new(Vec::new()),
            file_events: Mutex::new(Vec::new()),
            network_events: Mutex::new(Vec::new()),
            syscall_count: Mutex::new(0),
            memory_allocated: Mutex::new(0),
            verdict: Mutex::new(SandboxVerdict::Nominal),
        }
    }

    pub fn record_process(&self, event: ProcessEvent) {
        if let Ok(mut events) = self.process_events.lock() {
            events.push(event);
        }
    }

    pub fn record_file(&self, event: FileEvent) {
        if let Ok(mut events) = self.file_events.lock() {
            events.push(event);
        }
    }

    pub fn record_network(&self, event: NetworkEvent) {
        if let Ok(mut events) = self.network_events.lock() {
            events.push(event);
        }
    }

    pub fn add_syscalls(&self, count: u64) {
        if let Ok(mut total) = self.syscall_count.lock() {
            *total += count;
        }
    }

    pub fn add_memory(&self, bytes: u64) {
        if let Ok(mut total) = self.memory_allocated.lock() {
            *total += bytes;
        }
    }

    pub fn set_verdict(&self, verdict: SandboxVerdict) {
        if let Ok(mut v) = self.verdict.lock() {
            *v = verdict;
        }
    }

    /// Consume the builder and produce the final `KernelContext`.
    pub fn finalize(self, execution_duration_ns: u64) -> KernelContext {
        let process_events = self.process_events.into_inner().unwrap_or_default();
        let file_events = self.file_events.into_inner().unwrap_or_default();
        let network_events = self.network_events.into_inner().unwrap_or_default();
        let syscall_count = self.syscall_count.into_inner().unwrap_or(0);
        let memory_allocated = self.memory_allocated.into_inner().unwrap_or(0);
        let verdict = self.verdict.into_inner().unwrap_or(SandboxVerdict::Nominal);

        let processes_spawned = process_events
            .iter()
            .filter(|e| e.event_type == "create")
            .count() as u32;
        let files_touched = file_events.len() as u32;
        let network_connections = network_events.len() as u32;
        let threads_created = process_events
            .iter()
            .filter(|e| e.event_type == "thread_create")
            .count() as u32;

        KernelContext {
            execution_duration_ns,
            syscall_count,
            memory_allocated_bytes: memory_allocated,
            processes_spawned,
            files_touched,
            network_connections,
            threads_created,
            sandbox_verdict: verdict,
            process_events,
            file_events,
            network_events,
        }
    }
}

impl Default for KernelContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
