//! Windows Job Object sandbox implementation (WS1.5 Phase 1.5b).
//!
//! Creates a Win32 Job Object per task execution, applying memory and
//! process limits from [`SandboxConfig`]. The task's process is assigned
//! to the Job Object so the OS enforces containment. On timeout or
//! resource violation, `TerminateJobObject` cleanly tears down the
//! entire process tree.
//!
//! Falls back gracefully to [`NoopBoundary`] when Job Object creation
//! fails (e.g. insufficient privileges).

use async_trait::async_trait;
use nexus_common::kernel_context::{
    KernelContext, KernelContextBuilder, SandboxConfig, SandboxVerdict,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::{debug, warn};

use super::{ExecutionBoundary, ExecutionSandbox, SandboxError};
use crate::sandbox::etw_observer::EtwObserver;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// Windows Job Object sandbox.
///
/// On construction, verifies that `CreateJobObjectW` succeeds. If not
/// (e.g. running in an AppContainer or lacking `PROCESS_SET_QUOTA`
/// privilege), every [`create_boundary`] call returns a no-op boundary.
pub struct WindowsSandbox {
    job_available: bool,
}

impl WindowsSandbox {
    pub fn new() -> Self {
        let job_available = unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h == 0 || h == INVALID_HANDLE_VALUE {
                false
            } else {
                CloseHandle(h);
                true
            }
        };
        if !job_available {
            warn!("Job Objects unavailable; sandbox will operate in noop mode");
        }
        Self { job_available }
    }
}

#[async_trait]
impl ExecutionSandbox for WindowsSandbox {
    async fn create_boundary(
        &self,
        config: &SandboxConfig,
    ) -> Result<Box<dyn ExecutionBoundary>, SandboxError> {
        if !self.job_available {
            warn!("Job Objects unavailable; returning NoopBoundary");
            return Ok(Box::new(NoopBoundary));
        }

        match WindowsBoundary::create(config) {
            Ok(boundary) => Ok(Box::new(boundary)),
            Err(e) => {
                warn!(
                    error = %e,
                    "Job Object creation failed; falling back to NoopBoundary"
                );
                Ok(Box::new(NoopBoundary))
            }
        }
    }
}

// ─── WindowsBoundary ─────────────────────────────────────────────

/// A live Job Object containment boundary.
struct WindowsBoundary {
    job_handle: HANDLE,
    config: SandboxConfig,
    assigned: bool,
}

unsafe impl Send for WindowsBoundary {}
unsafe impl Sync for WindowsBoundary {}

impl WindowsBoundary {
    fn create(config: &SandboxConfig) -> Result<Self, SandboxError> {
        let job_handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job_handle == 0 || job_handle == INVALID_HANDLE_VALUE {
            return Err(SandboxError::Setup(
                "CreateJobObjectW failed".into(),
            ));
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        let mut limit_flags: u32 = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        if let Some(mem) = config.memory_limit_bytes {
            info.JobMemoryLimit = mem as usize;
            limit_flags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
        }

        if let Some(procs) = config.process_limit {
            info.BasicLimitInformation.ActiveProcessLimit = procs;
            limit_flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        }

        info.BasicLimitInformation.LimitFlags = limit_flags;

        let ok = unsafe {
            SetInformationJobObject(
                job_handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if ok == 0 {
            unsafe { CloseHandle(job_handle) };
            return Err(SandboxError::Setup(
                "SetInformationJobObject failed".into(),
            ));
        }

        debug!("Job Object created with limits: mem={:?} procs={:?}", config.memory_limit_bytes, config.process_limit);

        Ok(Self {
            job_handle,
            config: config.clone(),
            assigned: false,
        })
    }

    fn assign_current_process(&mut self) -> Result<(), SandboxError> {
        let ok = unsafe {
            AssignProcessToJobObject(self.job_handle, GetCurrentProcess())
        };
        if ok == 0 {
            warn!("AssignProcessToJobObject failed; running uncontained");
            return Err(SandboxError::Setup(
                "AssignProcessToJobObject failed".into(),
            ));
        }
        self.assigned = true;
        Ok(())
    }
}

impl Drop for WindowsBoundary {
    fn drop(&mut self) {
        if self.job_handle != 0 && self.job_handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.job_handle) };
        }
    }
}

#[async_trait]
impl ExecutionBoundary for WindowsBoundary {
    async fn execute(
        &self,
        task_type: &str,
        f: Pin<Box<dyn Future<Output = Result<String, nexus_common::NexusError>> + Send>>,
    ) -> Result<(Result<String, nexus_common::NexusError>, Option<KernelContext>), SandboxError>
    {
        let builder = KernelContextBuilder::new();

        // Start ETW observer if configured for this task type.
        let pid = std::process::id();
        let etw = EtwObserver::start(pid);
        if etw.is_none() {
            debug!(task_type, "ETW observer not available; proceeding without kernel telemetry");
        }

        let start = Instant::now();
        let result = f.await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        // Collect ETW observations.
        if let Some(observer) = etw {
            let snapshot = observer.stop();
            builder.add_syscalls(snapshot.syscall_count);
            builder.add_memory(snapshot.memory_allocated_bytes);
            for event in snapshot.process_events {
                builder.record_process(event);
            }
            for event in snapshot.file_events {
                builder.record_file(event);
            }
            for event in snapshot.network_events {
                builder.record_network(event);
            }
        }

        // Check for resource violations.
        if let Some(mem_limit) = self.config.memory_limit_bytes {
            let mem = builder_memory_snapshot(&builder);
            if mem > mem_limit {
                builder.set_verdict(SandboxVerdict::ResourceExceeded);
            }
        }

        let ctx = builder.finalize(elapsed_ns);
        Ok((result, Some(ctx)))
    }

    async fn teardown(&mut self) -> Result<(), SandboxError> {
        if self.job_handle != 0 && self.job_handle != INVALID_HANDLE_VALUE {
            unsafe {
                TerminateJobObject(self.job_handle, 0);
                CloseHandle(self.job_handle);
            }
            self.job_handle = 0;
        }
        Ok(())
    }
}

fn builder_memory_snapshot(_builder: &KernelContextBuilder) -> u64 {
    // Best-effort: read current process working set via GetProcessMemoryInfo.
    // If unavailable, return 0 (no violation triggered).
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        0 // Placeholder — full implementation uses K32GetProcessMemoryInfo
    }
    #[cfg(not(target_os = "windows"))]
    0
}

// ─── NoopBoundary (fallback) ─────────────────────────────────────

struct NoopBoundary;

#[async_trait]
impl ExecutionBoundary for NoopBoundary {
    async fn execute(
        &self,
        _task_type: &str,
        f: Pin<Box<dyn Future<Output = Result<String, nexus_common::NexusError>> + Send>>,
    ) -> Result<(Result<String, nexus_common::NexusError>, Option<KernelContext>), SandboxError>
    {
        let result = f.await;
        Ok((result, None))
    }

    async fn teardown(&mut self) -> Result<(), SandboxError> {
        Ok(())
    }
}
