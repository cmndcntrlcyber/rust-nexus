//! Event processing pipeline
//!
//! Unified pipeline for processing detection events from all sources,
//! applying correlation rules, and routing to destinations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, broadcast};

use super::{EventCorrelator, CorrelationRule};
use crate::types::{DetectionEvent, DetectionSource, Severity};
use crate::Result;

/// Event pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Buffer size for event channels
    pub buffer_size: usize,
    /// Enable correlation
    pub correlation_enabled: bool,
    /// Correlation window in seconds
    pub correlation_window_secs: u64,
    /// Enable deduplication
    pub deduplication_enabled: bool,
    /// Deduplication window in seconds
    pub dedup_window_secs: u64,
    /// Minimum severity to process
    pub min_severity: Severity,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            correlation_enabled: true,
            correlation_window_secs: 300,
            deduplication_enabled: true,
            dedup_window_secs: 60,
            min_severity: Severity::Info,
        }
    }
}

/// Event destination handler trait
pub trait EventHandler: Send + Sync {
    /// Handle an event
    fn handle(&self, event: &DetectionEvent) -> Result<()>;
    /// Get handler name
    fn name(&self) -> &str;
}

/// Simple logging handler
pub struct LoggingHandler {
    name: String,
}

impl LoggingHandler {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl EventHandler for LoggingHandler {
    fn handle(&self, event: &DetectionEvent) -> Result<()> {
        log::info!(
            "[{}] Event: {} - {} ({:?})",
            self.name,
            event.rule_id,
            event.description,
            event.severity
        );
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Pipeline statistics
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Total events received
    pub events_received: u64,
    /// Events processed
    pub events_processed: u64,
    /// Events filtered (below min severity)
    pub events_filtered: u64,
    /// Events deduplicated
    pub events_deduplicated: u64,
    /// Correlated events generated
    pub correlation_events: u64,
    /// Events by source
    pub events_by_source: HashMap<String, u64>,
    /// Events by severity
    pub events_by_severity: HashMap<String, u64>,
    /// Handler errors
    pub handler_errors: u64,
}

/// Event pipeline for processing detection events
pub struct EventPipeline {
    /// Configuration
    config: PipelineConfig,
    /// Event correlator
    correlator: Arc<RwLock<EventCorrelator>>,
    /// Event handlers
    handlers: Vec<Box<dyn EventHandler>>,
    /// Pipeline statistics
    stats: Arc<RwLock<PipelineStats>>,
    /// Recent event hashes for deduplication
    recent_events: Arc<RwLock<HashMap<String, std::time::Instant>>>,
    /// Broadcast channel for subscribers
    broadcast_tx: broadcast::Sender<DetectionEvent>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl EventPipeline {
    /// Create a new event pipeline
    pub fn new(config: PipelineConfig) -> Self {
        let correlator = EventCorrelator::with_window(config.correlation_window_secs);
        let (broadcast_tx, _) = broadcast::channel(config.buffer_size);

        Self {
            config,
            correlator: Arc::new(RwLock::new(correlator)),
            handlers: Vec::new(),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
            recent_events: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            running: Arc::new(RwLock::new(true)),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Add an event handler
    pub fn add_handler(&mut self, handler: impl EventHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    /// Add a correlation rule
    pub async fn add_correlation_rule(&self, rule: CorrelationRule) {
        self.correlator.write().await.add_rule(rule);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DetectionEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Process a single event
    pub async fn process(&self, event: DetectionEvent) -> Result<Vec<DetectionEvent>> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.events_received += 1;

            let source_key = format!("{:?}", event.source);
            *stats.events_by_source.entry(source_key).or_insert(0) += 1;

            let severity_key = format!("{:?}", event.severity);
            *stats.events_by_severity.entry(severity_key).or_insert(0) += 1;
        }

        // Check minimum severity
        if event.severity < self.config.min_severity {
            self.stats.write().await.events_filtered += 1;
            return Ok(Vec::new());
        }

        // Deduplication
        if self.config.deduplication_enabled {
            if self.is_duplicate(&event).await {
                self.stats.write().await.events_deduplicated += 1;
                return Ok(Vec::new());
            }
            self.record_event(&event).await;
        }

        // Apply correlation
        let mut output_events = vec![event.clone()];

        if self.config.correlation_enabled {
            let correlated = self.correlator.write().await.add_event(event.clone());
            for corr_event in correlated {
                if corr_event.source == DetectionSource::Correlation {
                    self.stats.write().await.correlation_events += 1;
                    output_events.push(corr_event);
                }
            }
        }

        // Process through handlers
        for handler in &self.handlers {
            for evt in &output_events {
                if let Err(e) = handler.handle(evt) {
                    log::error!("Handler {} failed: {}", handler.name(), e);
                    self.stats.write().await.handler_errors += 1;
                }
            }
        }

        // Broadcast events
        for evt in &output_events {
            let _ = self.broadcast_tx.send(evt.clone());
        }

        self.stats.write().await.events_processed += output_events.len() as u64;

        Ok(output_events)
    }

    /// Process multiple events
    pub async fn process_batch(&self, events: Vec<DetectionEvent>) -> Result<Vec<DetectionEvent>> {
        let mut all_output = Vec::new();

        for event in events {
            match self.process(event).await {
                Ok(output) => all_output.extend(output),
                Err(e) => log::error!("Failed to process event: {}", e),
            }
        }

        Ok(all_output)
    }

    /// Check if event is a duplicate
    async fn is_duplicate(&self, event: &DetectionEvent) -> bool {
        let hash = self.event_hash(event);
        let recent = self.recent_events.read().await;

        if let Some(last_seen) = recent.get(&hash) {
            let dedup_window = std::time::Duration::from_secs(self.config.dedup_window_secs);
            return last_seen.elapsed() < dedup_window;
        }

        false
    }

    /// Record event for deduplication
    async fn record_event(&self, event: &DetectionEvent) {
        let hash = self.event_hash(event);
        self.recent_events
            .write()
            .await
            .insert(hash, std::time::Instant::now());

        // Clean up old entries
        self.cleanup_recent_events().await;
    }

    /// Generate hash for deduplication
    fn event_hash(&self, event: &DetectionEvent) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        event.rule_id.hash(&mut hasher);
        event.asset_id.hash(&mut hasher);
        format!("{:?}", event.source).hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Clean up old dedup entries
    async fn cleanup_recent_events(&self) {
        let dedup_window = std::time::Duration::from_secs(self.config.dedup_window_secs);
        let mut recent = self.recent_events.write().await;

        recent.retain(|_, last_seen| last_seen.elapsed() < dedup_window);
    }

    /// Get pipeline statistics
    pub async fn stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = PipelineStats::default();
    }

    /// Get configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Stop the pipeline
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }
}

/// Async event stream processor
pub struct StreamProcessor {
    pipeline: Arc<EventPipeline>,
    receiver: mpsc::Receiver<DetectionEvent>,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new(pipeline: Arc<EventPipeline>, buffer_size: usize) -> (Self, mpsc::Sender<DetectionEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        (
            Self {
                pipeline,
                receiver: rx,
            },
            tx,
        )
    }

    /// Run the processor loop
    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            if !self.pipeline.is_running().await {
                break;
            }

            if let Err(e) = self.pipeline.process(event).await {
                log::error!("Pipeline processing error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let pipeline = EventPipeline::with_defaults();
        assert!(pipeline.is_running().await);
    }

    #[tokio::test]
    async fn test_process_event() {
        let pipeline = EventPipeline::with_defaults();

        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "TEST-001",
            "Test detection",
        );

        let results = pipeline.process(event).await.unwrap();
        assert!(!results.is_empty());

        let stats = pipeline.stats().await;
        assert_eq!(stats.events_received, 1);
        assert_eq!(stats.events_processed, 1);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let config = PipelineConfig {
            deduplication_enabled: true,
            dedup_window_secs: 60,
            ..Default::default()
        };
        let pipeline = EventPipeline::new(config);

        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "TEST-001",
            "Test detection",
        );

        // First event should pass
        let results1 = pipeline.process(event.clone()).await.unwrap();
        assert!(!results1.is_empty());

        // Duplicate should be filtered
        let results2 = pipeline.process(event).await.unwrap();
        assert!(results2.is_empty());

        let stats = pipeline.stats().await;
        assert_eq!(stats.events_received, 2);
        assert_eq!(stats.events_deduplicated, 1);
    }

    #[tokio::test]
    async fn test_severity_filter() {
        let config = PipelineConfig {
            min_severity: Severity::High,
            ..Default::default()
        };
        let pipeline = EventPipeline::new(config);

        // Low severity should be filtered
        let low_event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::Low,
            "TEST-001",
            "Low severity",
        );
        let results = pipeline.process(low_event).await.unwrap();
        assert!(results.is_empty());

        // High severity should pass
        let high_event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "TEST-002",
            "High severity",
        );
        let results = pipeline.process(high_event).await.unwrap();
        assert!(!results.is_empty());

        let stats = pipeline.stats().await;
        assert_eq!(stats.events_filtered, 1);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let pipeline = EventPipeline::with_defaults();

        let events: Vec<_> = (0..5)
            .map(|i| {
                DetectionEvent::new(
                    DetectionSource::Signature,
                    Severity::Medium,
                    format!("TEST-{:03}", i),
                    format!("Test event {}", i),
                )
            })
            .collect();

        let results = pipeline.process_batch(events).await.unwrap();
        assert_eq!(results.len(), 5);

        let stats = pipeline.stats().await;
        assert_eq!(stats.events_received, 5);
    }

    #[tokio::test]
    async fn test_subscription() {
        let pipeline = EventPipeline::with_defaults();
        let mut subscriber = pipeline.subscribe();

        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "TEST-001",
            "Test event",
        );

        // Process in a task
        let pipeline_clone = Arc::new(pipeline);
        let event_clone = event.clone();

        tokio::spawn(async move {
            pipeline_clone.process(event_clone).await.unwrap();
        });

        // Receive the broadcast
        let received = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            subscriber.recv(),
        )
        .await;

        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_handler() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingHandler {
            count: Arc<AtomicUsize>,
        }

        impl EventHandler for CountingHandler {
            fn handle(&self, _event: &DetectionEvent) -> Result<()> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn name(&self) -> &str {
                "counter"
            }
        }

        let count = Arc::new(AtomicUsize::new(0));
        let mut pipeline = EventPipeline::with_defaults();
        pipeline.add_handler(CountingHandler { count: count.clone() });

        let event = DetectionEvent::new(
            DetectionSource::Signature,
            Severity::High,
            "TEST-001",
            "Test event",
        );

        pipeline.process(event).await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
