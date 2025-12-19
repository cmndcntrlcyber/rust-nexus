# 🐾 Baby Step 1.4: Event Pipeline

> Build unified event correlation pipeline.

**STATUS: ✅ COMPLETE**

## 📋 Objective

Create the event processing pipeline that correlates detection events from multiple sources.

## ✅ Prerequisites

- [x] Baby Step 1.2 complete (signatures)
- [x] Baby Step 1.3 complete (LitterBox)
- [x] Understand async event processing

## 🔧 Implementation (Completed)

### Event Correlator (correlation/mod.rs)

```rust
pub struct EventCorrelator {
    event_buffer: Vec<BufferedEvent>,
    rules: Vec<CorrelationRule>,
    window: Duration,            // Correlation time window
    enabled: bool,
}

pub struct CorrelationRule {
    pub id: String,              // e.g., "CORR-001"
    pub name: String,
    pub patterns: Vec<EventPattern>,
    pub window_secs: u64,
    pub severity: Severity,
    pub description: String,
}

impl EventCorrelator {
    pub fn add_event(&mut self, event: DetectionEvent) -> Vec<DetectionEvent>
    pub fn add_rule(&mut self, rule: CorrelationRule)
    pub fn buffer_size(&self) -> usize
    pub fn clear(&mut self)
}
```

### Default Correlation Rules

1. **CORR-001: Multi-stage Attack**
   - Process + Network events within 5 minutes
   - Minimum Medium severity
   - Output: High severity alert

2. **CORR-002: Credential Theft Attempt**
   - PROC-001 + BHV-002 within 60 seconds
   - Output: Critical severity alert

### Event Pipeline (correlation/pipeline.rs)

```rust
pub struct EventPipeline {
    config: PipelineConfig,
    correlator: Arc<RwLock<EventCorrelator>>,
    handlers: Vec<Box<dyn EventHandler>>,
    stats: Arc<RwLock<PipelineStats>>,
    recent_events: Arc<RwLock<HashMap<String, Instant>>>,
    broadcast_tx: broadcast::Sender<DetectionEvent>,
    running: Arc<RwLock<bool>>,
}

pub struct PipelineConfig {
    pub buffer_size: usize,           // Event channel buffer
    pub correlation_enabled: bool,
    pub correlation_window_secs: u64,
    pub deduplication_enabled: bool,
    pub dedup_window_secs: u64,
    pub min_severity: Severity,       // Filter threshold
}

impl EventPipeline {
    pub async fn process(&self, event: DetectionEvent) -> Result<Vec<DetectionEvent>>
    pub async fn process_batch(&self, events: Vec<DetectionEvent>) -> Result<Vec<DetectionEvent>>
    pub fn subscribe(&self) -> broadcast::Receiver<DetectionEvent>
    pub fn add_handler(&mut self, handler: impl EventHandler)
    pub async fn stats(&self) -> PipelineStats
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &DetectionEvent) -> Result<()>;
    fn name(&self) -> &str;
}
```

### Pipeline Features

- **Deduplication**: Hash-based duplicate filtering within configurable window
- **Severity Filtering**: Minimum severity threshold
- **Correlation**: Multi-source event pattern matching
- **Broadcasting**: Pub/sub for event subscribers
- **Handler Chain**: Extensible event processing
- **Statistics**: events_received, events_processed, events_filtered, correlation_events, handler_errors

### Stream Processor

```rust
pub struct StreamProcessor {
    pipeline: Arc<EventPipeline>,
    receiver: mpsc::Receiver<DetectionEvent>,
}

impl StreamProcessor {
    pub fn new(pipeline: Arc<EventPipeline>, buffer_size: usize) -> (Self, mpsc::Sender<DetectionEvent>)
    pub async fn run(mut self)
}
```

## ✅ Verification Checklist

- [x] Events from all sources processed
- [x] Deduplication working correctly
- [x] Correlation rules applied
- [x] Severity filtering working
- [x] Event broadcasting to subscribers
- [x] Handler chain extensible
- [x] Unit tests pass (7 pipeline tests)

## 📤 Output

- `correlation/mod.rs` - EventCorrelator with correlation rules
- `correlation/pipeline.rs` - Full EventPipeline
- Async processing with tokio channels
- Statistics tracking

## ➡️ Next Step

[completion-checklist.md](completion-checklist.md)

---
**Completed**: 2024-12-19
**Assigned To**: Detection Engine Agent
