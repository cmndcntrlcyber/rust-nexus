//! System profiling implementation

use crate::*;

#[allow(dead_code)] // config read in upcoming profiler wiring
pub struct SystemProfiler {
    config: ReconConfig,
}

impl SystemProfiler {
    pub fn new(config: ReconConfig) -> Self {
        Self { config }
    }

    pub async fn profile_systems(&self, _targets: &[String]) -> Result<Vec<SystemProfile>> {
        // Stub implementation
        Ok(vec![])
    }
}
