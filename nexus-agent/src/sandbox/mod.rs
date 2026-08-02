//! Sandboxed execution context (WS1.5 Phase 1.5b).
//!
//! Trait-based containment and observation abstraction. Platform
//! implementations wrap task execution in OS-level boundaries
//! (Job Objects on Windows, cgroups v2 on Linux) and capture
//! kernel-level telemetry via ETW or `/proc` observers.

use async_trait::async_trait;
use nexus_common::kernel_context::{KernelContext, SandboxConfig};
use std::future::Future;
use std::pin::Pin;

// Platform-specific modules.
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub mod proc_observer;

/// Creates platform-appropriate execution sandboxes.
#[async_trait]
pub trait ExecutionSandbox: Send + Sync + 'static {
    /// Create a containment boundary with the given config.
    async fn create_boundary(
        &self,
        config: &SandboxConfig,
    ) -> Result<Box<dyn ExecutionBoundary>, SandboxError>;
}

/// A live containment boundary that wraps task execution.
#[async_trait]
pub trait ExecutionBoundary: Send + Sync {
    /// Run a closure within the containment boundary, returning the
    /// closure's result alongside the captured kernel context.
    async fn execute(
        &self,
        task_type: &str,
        f: Pin<Box<dyn Future<Output = Result<String, nexus_common::NexusError>> + Send>>,
    ) -> Result<(Result<String, nexus_common::NexusError>, Option<KernelContext>), SandboxError>;

    /// Release OS resources (Job Object handle, cgroup slice, etc.).
    async fn teardown(&mut self) -> Result<(), SandboxError>;
}

/// Errors from the sandbox subsystem.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("containment setup failed: {0}")]
    Setup(String),
    #[error("observation attachment failed: {0}")]
    Observer(String),
    #[error("containment breach detected")]
    Breach,
    #[error("sandbox I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// No-op sandbox: runs the task without containment or observation.
/// Used when the `sandbox` feature is disabled or on unsupported platforms.
pub struct NoopSandbox;

#[async_trait]
impl ExecutionSandbox for NoopSandbox {
    async fn create_boundary(
        &self,
        _config: &SandboxConfig,
    ) -> Result<Box<dyn ExecutionBoundary>, SandboxError> {
        Ok(Box::new(NoopBoundary))
    }
}

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

/// Return the platform-appropriate sandbox implementation.
pub fn create_sandbox() -> Box<dyn ExecutionSandbox> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxSandbox::new())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(NoopSandbox)
    }
}
