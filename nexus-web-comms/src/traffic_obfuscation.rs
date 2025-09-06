//! Traffic obfuscation implementation

use crate::*;

pub struct DataObfuscator {
    config: ObfuscationConfig,
}

impl DataObfuscator {
    pub fn new(config: &ObfuscationConfig) -> Self {
        Self { config: config.clone() }
    }
    
    pub fn obfuscate(&self, data: &str) -> Result<String> {
        if self.config.base64_encode {
            Ok(general_purpose::STANDARD.encode(data))
        } else {
            Ok(data.to_string())
        }
    }
    
    pub fn deobfuscate(&self, data: &str) -> Result<String> {
        if self.config.base64_encode {
            let decoded = general_purpose::STANDARD
                .decode(data)
                .map_err(|e| NexusError::ConfigurationError(format!("Base64 decode error: {}", e)))?;
            Ok(String::from_utf8_lossy(&decoded).to_string())
        } else {
            Ok(data.to_string())
        }
    }
}
