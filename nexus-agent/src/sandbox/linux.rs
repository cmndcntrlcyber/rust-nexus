//! Linux cgroup v2 sandbox implementation (WS1.5 Phase 1.5c).
//!
//! Creates a cgroup v2 slice under `/sys/fs/cgroup/nexus-{uuid}/` for
//! task containment, setting `memory.max` and `pids.max` from
//! [`SandboxConfig`]. Falls back gracefully to [`NoopBoundary`] when
//! the agent lacks write access to `/sys/fs/cgroup/`.

use async_trait::async_trait;
use nexus_common::kernel_context::{KernelContext, KernelContextBuilder, SandboxConfig, SandboxVerdict};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;

use super::{ExecutionBoundary, ExecutionSandbox, SandboxError};
use crate::sandbox::proc_observer::ProcObserver;

/// Linux cgroup v2 sandbox.
///
/// On construction, tests that `/sys/fs/cgroup/cgroup.controllers` is
/// readable. If the cgroup filesystem is unavailable, every
/// [`create_boundary`] call returns a no-op boundary with a warning.
pub struct LinuxSandbox {
    cgroup_available: bool,
}

impl LinuxSandbox {
    /// Probe the cgroup v2 filesystem and return a new sandbox.
    pub fn new() -> Self {
        let cgroup_available = std::path::Path::new("/sys/fs/cgroup/cgroup.controllers").exists();
        if !cgroup_available {
            tracing::warn!(
                "cgroup v2 filesystem not found at /sys/fs/cgroup/; \
                 sandbox will operate in noop mode"
            );
        }
        Self { cgroup_available }
    }
}

#[async_trait]
impl ExecutionSandbox for LinuxSandbox {
    async fn create_boundary(
        &self,
        config: &SandboxConfig,
    ) -> Result<Box<dyn ExecutionBoundary>, SandboxError> {
        if !self.cgroup_available {
            tracing::warn!("cgroup v2 unavailable; returning NoopBoundary");
            return Ok(Box::new(NoopBoundary));
        }

        match LinuxBoundary::create(config).await {
            Ok(boundary) => Ok(Box::new(boundary)),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "cgroup creation failed (likely permission denied); \
                     falling back to NoopBoundary"
                );
                Ok(Box::new(NoopBoundary))
            }
        }
    }
}

// ─── LinuxBoundary ────────────────────────────────────────────────

/// A live cgroup v2 containment boundary.
///
/// On [`execute`], the current task is "moved" into the cgroup by
/// writing `0` (current PID) to `cgroup.procs`, then the future is
/// polled. After the future resolves, basic stats are captured from
/// `/proc/self/` and the cgroup is cleaned up on [`teardown`].
pub struct LinuxBoundary {
    cgroup_path: PathBuf,
    config: SandboxConfig,
}

impl LinuxBoundary {
    /// Create a new cgroup v2 slice and configure resource limits.
    async fn create(config: &SandboxConfig) -> Result<Self, SandboxError> {
        let id = uuid::Uuid::new_v4();
        let cgroup_path = PathBuf::from(format!("/sys/fs/cgroup/nexus-{}", id));

        // Create the cgroup directory.
        tokio::fs::create_dir_all(&cgroup_path)
            .await
            .map_err(|e| SandboxError::Setup(format!("mkdir {:?}: {}", cgroup_path, e)))?;

        // Set memory.max if configured.
        if let Some(mem) = config.memory_limit_bytes {
            let mem_path = cgroup_path.join("memory.max");
            if let Err(e) = tokio::fs::write(&mem_path, mem.to_string()).await {
                tracing::warn!(error = %e, "failed to write memory.max; ignoring");
            }
        }

        // Set pids.max if configured.
        if let Some(pids) = config.process_limit {
            let pids_path = cgroup_path.join("pids.max");
            if let Err(e) = tokio::fs::write(&pids_path, pids.to_string()).await {
                tracing::warn!(error = %e, "failed to write pids.max; ignoring");
            }
        }

        Ok(Self {
            cgroup_path,
            config: config.clone(),
        })
    }

    /// Attempt to move the current process into this cgroup.
    async fn enter_cgroup(&self) -> Result<(), SandboxError> {
        let procs_path = self.cgroup_path.join("cgroup.procs");
        let pid = std::process::id();
        tokio::fs::write(&procs_path, pid.to_string())
            .await
            .map_err(|e| {
                SandboxError::Setup(format!(
                    "failed to move pid {} into {:?}: {}",
                    pid, procs_path, e
                ))
            })
    }

    /// Remove the cgroup directory (must be empty of processes first).
    async fn remove_cgroup(&self) -> Result<(), SandboxError> {
        // Attempt to remove the cgroup. If it still has processes this
        // will fail — that is expected if the task spawned children that
        // haven't exited yet. We log and move on.
        if let Err(e) = tokio::fs::remove_dir(&self.cgroup_path).await {
            tracing::debug!(
                error = %e,
                path = %self.cgroup_path.display(),
                "cgroup removal failed (may still have processes)"
            );
        }
        Ok(())
    }
}

#[async_trait]
impl ExecutionBoundary for LinuxBoundary {
    async fn execute(
        &self,
        _task_type: &str,
        f: Pin<Box<dyn Future<Output = Result<String, nexus_common::NexusError>> + Send>>,
    ) -> Result<(Result<String, nexus_common::NexusError>, Option<KernelContext>), SandboxError>
    {
        let builder = KernelContextBuilder::new();
        let pid = std::process::id();

        // Attempt to move into the cgroup; non-fatal if it fails.
        if let Err(e) = self.enter_cgroup().await {
            tracing::warn!(error = %e, "could not enter cgroup; running uncontained");
        }

        // Capture pre-execution /proc snapshot.
        let observer = ProcObserver::new();
        let _pre_snapshot = observer.observe(pid);

        let start = Instant::now();
        let result = f.await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        // Capture post-execution /proc snapshot.
        let post_snapshot = observer.observe(pid);

        // Populate the kernel context builder from the post-snapshot.
        builder.add_memory(post_snapshot.memory_allocated_bytes);
        if post_snapshot.network_connections > 0 {
            // Record aggregate count; individual events not tracked via /proc.
            for _ in 0..post_snapshot.network_connections {
                builder.record_network(nexus_common::kernel_context::NetworkEvent {
                    src_addr: String::new(),
                    dst_addr: String::new(),
                    dst_port: 0,
                    protocol: "tcp".into(),
                    direction: "outbound".into(),
                    timestamp_unix: nexus_common::current_timestamp(),
                });
            }
        }

        // Check if memory limit was exceeded.
        if let Some(mem_limit) = self.config.memory_limit_bytes {
            if post_snapshot.memory_allocated_bytes > mem_limit {
                builder.set_verdict(SandboxVerdict::ResourceExceeded);
            }
        }

        let ctx = builder.finalize(elapsed_ns);
        Ok((result, Some(ctx)))
    }

    async fn teardown(&mut self) -> Result<(), SandboxError> {
        self.remove_cgroup().await
    }
}

// ─── NoopBoundary (fallback) ──────────────────────────────────────

/// Fallback boundary used when cgroup creation fails.
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
