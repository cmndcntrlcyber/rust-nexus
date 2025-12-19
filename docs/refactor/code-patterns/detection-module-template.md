# 🔍 Detection Module Template

> Standard template for implementing detection modules.

## 📋 Module Structure

```rust
//! Detection module for [DETECTION_TYPE]
//!
//! # Overview
//! [Brief description of what this module detects]

use crate::types::{DetectionResult, Confidence, ThreatLevel};
use crate::config::DetectorConfig;

/// Configuration for this detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct [ModuleName]Config {
    pub enabled: bool,
    pub sensitivity: f32,
    // TODO: Add module-specific config
}

/// Main detector implementation
pub struct [ModuleName]Detector {
    config: [ModuleName]Config,
    // TODO: Add internal state
}

impl [ModuleName]Detector {
    pub fn new(config: [ModuleName]Config) -> Self {
        Self { config }
    }

    pub async fn analyze(&self, data: &[DataType]) -> Result<Vec<DetectionResult>> {
        // TODO: Implement detection logic
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detection_positive() {
        // TODO: Test known-bad samples
    }

    #[tokio::test]
    async fn test_detection_negative() {
        // TODO: Test benign samples
    }
}
```

## ✅ Required Implementations

- [ ] Detector struct with config
- [ ] `new()` constructor
- [ ] `analyze()` method returning DetectionResult
- [ ] Unit tests for positive cases
- [ ] Unit tests for negative cases (false positive check)

## 📝 Integration

<!-- TODO: Add integration steps -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Detection Engine Agent
