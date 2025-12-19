//! Event correlation module
//!
//! Correlates events from multiple detection sources to identify
//! complex attack patterns and reduce false positives.

pub mod pipeline;

pub use pipeline::{EventPipeline, PipelineConfig, PipelineStats, EventHandler, LoggingHandler, StreamProcessor};

use std::time::{Duration, Instant};

use crate::types::{DetectionEvent, DetectionSource, Severity};

/// Event correlator
pub struct EventCorrelator {
    /// Event buffer for correlation
    event_buffer: Vec<BufferedEvent>,
    /// Correlation rules
    rules: Vec<CorrelationRule>,
    /// Time window for correlation (default 5 minutes)
    window: Duration,
    /// Enable/disable flag
    enabled: bool,
}

/// Buffered event with timestamp
struct BufferedEvent {
    event: DetectionEvent,
    received_at: Instant,
}

/// Correlation rule
#[derive(Debug, Clone)]
pub struct CorrelationRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Required event patterns
    pub patterns: Vec<EventPattern>,
    /// Time window for correlation
    pub window_secs: u64,
    /// Output severity
    pub severity: Severity,
    /// Output description template
    pub description: String,
}

/// Pattern for matching events
#[derive(Debug, Clone)]
pub struct EventPattern {
    /// Source to match (or None for any)
    pub source: Option<DetectionSource>,
    /// Rule ID prefix to match (or None for any)
    pub rule_prefix: Option<String>,
    /// Minimum severity
    pub min_severity: Option<Severity>,
}

impl EventCorrelator {
    /// Create a new event correlator
    pub fn new() -> Self {
        Self {
            event_buffer: Vec::new(),
            rules: Self::default_rules(),
            window: Duration::from_secs(300), // 5 minutes
            enabled: true,
        }
    }

    /// Create with custom window
    pub fn with_window(window_secs: u64) -> Self {
        let mut correlator = Self::new();
        correlator.window = Duration::from_secs(window_secs);
        correlator
    }

    /// Default correlation rules
    fn default_rules() -> Vec<CorrelationRule> {
        vec![
            CorrelationRule {
                id: "CORR-001".to_string(),
                name: "Multi-stage Attack".to_string(),
                patterns: vec![
                    EventPattern {
                        source: Some(DetectionSource::Process),
                        rule_prefix: None,
                        min_severity: Some(Severity::Medium),
                    },
                    EventPattern {
                        source: Some(DetectionSource::Network),
                        rule_prefix: None,
                        min_severity: Some(Severity::Medium),
                    },
                ],
                window_secs: 300,
                severity: Severity::High,
                description: "Multi-stage attack detected: suspicious process with network activity".to_string(),
            },
            CorrelationRule {
                id: "CORR-002".to_string(),
                name: "Credential Theft Attempt".to_string(),
                patterns: vec![
                    EventPattern {
                        source: Some(DetectionSource::Process),
                        rule_prefix: Some("PROC-001".to_string()),
                        min_severity: None,
                    },
                    EventPattern {
                        source: Some(DetectionSource::Behavioral),
                        rule_prefix: Some("BHV-002".to_string()),
                        min_severity: None,
                    },
                ],
                window_secs: 60,
                severity: Severity::Critical,
                description: "Credential theft attempt: suspicious process accessing credentials".to_string(),
            },
        ]
    }

    /// Add an event for correlation
    pub fn add_event(&mut self, event: DetectionEvent) -> Vec<DetectionEvent> {
        if !self.enabled {
            return vec![event];
        }

        // Clean old events
        self.cleanup_old_events();

        // Buffer the event
        self.event_buffer.push(BufferedEvent {
            event: event.clone(),
            received_at: Instant::now(),
        });

        // Check for correlations
        let mut correlated = self.check_correlations();

        // Always return the original event
        correlated.insert(0, event);

        correlated
    }

    /// Clean up events outside the correlation window
    fn cleanup_old_events(&mut self) {
        let now = Instant::now();
        self.event_buffer
            .retain(|e| now.duration_since(e.received_at) < self.window);
    }

    /// Check for correlation matches
    fn check_correlations(&self) -> Vec<DetectionEvent> {
        let mut correlated_events = Vec::new();

        for rule in &self.rules {
            if self.rule_matches(rule) {
                let event = DetectionEvent::new(
                    DetectionSource::Correlation,
                    rule.severity,
                    &rule.id,
                    &rule.description,
                );
                correlated_events.push(event);
            }
        }

        correlated_events
    }

    /// Check if a rule matches current events
    fn rule_matches(&self, rule: &CorrelationRule) -> bool {
        let window = Duration::from_secs(rule.window_secs);
        let now = Instant::now();

        // All patterns must match
        for pattern in &rule.patterns {
            let has_match = self.event_buffer.iter().any(|buffered| {
                // Check time window
                if now.duration_since(buffered.received_at) > window {
                    return false;
                }

                // Check source
                if let Some(source) = pattern.source {
                    if buffered.event.source != source {
                        return false;
                    }
                }

                // Check rule prefix
                if let Some(ref prefix) = pattern.rule_prefix {
                    if !buffered.event.rule_id.starts_with(prefix) {
                        return false;
                    }
                }

                // Check severity
                if let Some(min_sev) = pattern.min_severity {
                    if buffered.event.severity < min_sev {
                        return false;
                    }
                }

                true
            });

            if !has_match {
                return false;
            }
        }

        true
    }

    /// Add a correlation rule
    pub fn add_rule(&mut self, rule: CorrelationRule) {
        self.rules.push(rule);
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.event_buffer.len()
    }

    /// Clear event buffer
    pub fn clear(&mut self) {
        self.event_buffer.clear();
    }

    /// Check if correlator is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the correlator
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the correlator
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl Default for EventCorrelator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlator_creation() {
        let correlator = EventCorrelator::new();
        assert!(correlator.is_enabled());
        assert!(!correlator.rules.is_empty());
    }

    #[test]
    fn test_add_event() {
        let mut correlator = EventCorrelator::new();
        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::Medium,
            "TEST-001",
            "Test event",
        );

        let results = correlator.add_event(event);
        assert!(!results.is_empty());
        assert_eq!(correlator.buffer_size(), 1);
    }

    #[test]
    fn test_clear_buffer() {
        let mut correlator = EventCorrelator::new();
        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::Medium,
            "TEST-001",
            "Test event",
        );

        correlator.add_event(event);
        assert_eq!(correlator.buffer_size(), 1);

        correlator.clear();
        assert_eq!(correlator.buffer_size(), 0);
    }
}
