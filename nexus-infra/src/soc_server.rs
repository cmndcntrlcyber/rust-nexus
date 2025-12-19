//! SOC Detection Server Implementation
//!
//! Implements the NexusSOC gRPC service for detection agents,
//! telemetry collection, and detection event management.

use crate::proto::*;
use log::{info, debug};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

/// Detection agent session information
#[derive(Debug, Clone)]
pub struct DetectionAgentSession {
    pub agent_id: String,
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub ip_address: String,
    pub agent_version: String,
    pub mode: i32,
    pub capabilities: Vec<i32>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_health_check: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub config: Option<DetectionConfig>,
    pub events_received: u64,
    pub telemetry_received: u64,
}

/// Detection event storage
#[derive(Debug, Clone)]
pub struct StoredDetectionEvent {
    pub event: DetectionEventProto,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub processed: bool,
}

/// Telemetry event storage
#[derive(Debug, Clone)]
pub struct StoredTelemetry {
    pub event: TelemetryEvent,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

/// Detection task queue
#[derive(Debug, Clone)]
pub struct DetectionTaskQueue {
    pub pending_tasks: HashMap<String, DetectionTask>,
    pub agent_tasks: HashMap<String, Vec<String>>,
    pub completed_tasks: HashMap<String, DetectionStatusUpdate>,
}

impl DetectionTaskQueue {
    pub fn new() -> Self {
        Self {
            pending_tasks: HashMap::new(),
            agent_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
        }
    }

    pub fn queue_task(&mut self, agent_id: &str, task: DetectionTask) {
        let task_id = task.task_id.clone();
        self.pending_tasks.insert(task_id.clone(), task);
        self.agent_tasks
            .entry(agent_id.to_string())
            .or_default()
            .push(task_id);
    }

    pub fn get_tasks_for_agent(&self, agent_id: &str, max_tasks: usize) -> Vec<DetectionTask> {
        self.agent_tasks
            .get(agent_id)
            .map(|task_ids| {
                task_ids
                    .iter()
                    .filter_map(|id| self.pending_tasks.get(id).cloned())
                    .take(max_tasks)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn complete_task(&mut self, update: DetectionStatusUpdate) {
        let task_id = &update.task_id;
        self.pending_tasks.remove(task_id);
        self.completed_tasks.insert(task_id.clone(), update);
    }
}

/// Sample submission storage
#[derive(Debug, Clone)]
pub struct StoredSample {
    pub sample_id: String,
    pub agent_id: String,
    pub filename: String,
    pub file_hash: String,
    pub file_size: usize,
    pub reason: i32,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub analysis_status: i32,
    pub analysis_result: Option<AnalysisResultResponse>,
}

/// Signature set management
#[derive(Debug, Clone)]
pub struct SignatureManager {
    pub current_version: String,
    pub signature_sets: Vec<SignatureSet>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl SignatureManager {
    pub fn new() -> Self {
        Self {
            current_version: "1.0.0".to_string(),
            signature_sets: Self::default_signature_sets(),
            last_updated: chrono::Utc::now(),
        }
    }

    fn default_signature_sets() -> Vec<SignatureSet> {
        vec![
            SignatureSet {
                set_id: "RS-DEFAULT".to_string(),
                name: "Reverse Shell Detection".to_string(),
                version: "1.0.0".to_string(),
                signatures: vec![
                    SignatureRule {
                        rule_id: "RS-BASH-001".to_string(),
                        name: "Bash Reverse Shell".to_string(),
                        pattern: r"bash\s+-i\s+>&\s*/dev/tcp/".to_string(),
                        pattern_type: PatternType::Regex as i32,
                        severity: SeverityLevel::High as i32,
                        mitre_technique: "T1059.004".to_string(),
                        description: "Bash interactive reverse shell via /dev/tcp".to_string(),
                        enabled: true,
                        tags: vec!["reverse_shell".to_string(), "bash".to_string()],
                    },
                    SignatureRule {
                        rule_id: "RS-NC-001".to_string(),
                        name: "Netcat Reverse Shell".to_string(),
                        pattern: r"nc\s+.*-e\s+(/bin/sh|/bin/bash|cmd\.exe)".to_string(),
                        pattern_type: PatternType::Regex as i32,
                        severity: SeverityLevel::High as i32,
                        mitre_technique: "T1059".to_string(),
                        description: "Netcat reverse shell with execute flag".to_string(),
                        enabled: true,
                        tags: vec!["reverse_shell".to_string(), "netcat".to_string()],
                    },
                ],
                updated_at: Some(prost_types::Timestamp {
                    seconds: chrono::Utc::now().timestamp(),
                    nanos: 0,
                }),
            },
        ]
    }
}

/// Correlation rule management
#[derive(Debug, Clone)]
pub struct CorrelationManager {
    pub version: String,
    pub rules: Vec<CorrelationRuleProto>,
}

impl CorrelationManager {
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            rules: Self::default_rules(),
        }
    }

    fn default_rules() -> Vec<CorrelationRuleProto> {
        vec![
            CorrelationRuleProto {
                rule_id: "CORR-001".to_string(),
                name: "Multi-stage Attack".to_string(),
                patterns: vec![
                    EventPatternProto {
                        source: DetectionSource::Process as i32,
                        rule_prefix: String::new(),
                        min_severity: SeverityLevel::Medium as i32,
                    },
                    EventPatternProto {
                        source: DetectionSource::Network as i32,
                        rule_prefix: String::new(),
                        min_severity: SeverityLevel::Medium as i32,
                    },
                ],
                window_seconds: 300,
                output_severity: SeverityLevel::High as i32,
                description: "Suspicious process with network activity".to_string(),
                enabled: true,
            },
            CorrelationRuleProto {
                rule_id: "CORR-002".to_string(),
                name: "Credential Theft".to_string(),
                patterns: vec![
                    EventPatternProto {
                        source: DetectionSource::Process as i32,
                        rule_prefix: "PROC-LSASS".to_string(),
                        min_severity: SeverityLevel::Low as i32,
                    },
                ],
                window_seconds: 60,
                output_severity: SeverityLevel::Critical as i32,
                description: "Credential theft attempt detected".to_string(),
                enabled: true,
            },
        ]
    }
}

/// SOC Service state
pub struct SocServiceState {
    pub agents: RwLock<HashMap<String, DetectionAgentSession>>,
    pub detection_events: RwLock<Vec<StoredDetectionEvent>>,
    pub telemetry_events: RwLock<Vec<StoredTelemetry>>,
    pub task_queue: RwLock<DetectionTaskQueue>,
    pub samples: RwLock<HashMap<String, StoredSample>>,
    pub signature_manager: RwLock<SignatureManager>,
    pub correlation_manager: RwLock<CorrelationManager>,
    pub telemetry_sequence: RwLock<HashMap<String, u64>>,
}

impl SocServiceState {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            detection_events: RwLock::new(Vec::new()),
            telemetry_events: RwLock::new(Vec::new()),
            task_queue: RwLock::new(DetectionTaskQueue::new()),
            samples: RwLock::new(HashMap::new()),
            signature_manager: RwLock::new(SignatureManager::new()),
            correlation_manager: RwLock::new(CorrelationManager::new()),
            telemetry_sequence: RwLock::new(HashMap::new()),
        }
    }
}

/// Implementation of the NexusSOC service
#[derive(Clone)]
pub struct NexusSocService {
    state: Arc<SocServiceState>,
}

impl NexusSocService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(SocServiceState::new()),
        }
    }

    pub fn with_state(state: Arc<SocServiceState>) -> Self {
        Self { state }
    }

    /// Get default detection configuration
    fn default_detection_config() -> DetectionConfig {
        DetectionConfig {
            telemetry_interval_secs: 30,
            heartbeat_interval_secs: 60,
            enabled_detectors: vec![
                "signature".to_string(),
                "behavioral".to_string(),
                "network".to_string(),
                "process".to_string(),
            ],
            min_severity: SeverityLevel::Low as i32,
            enable_sample_collection: true,
            max_sample_size_bytes: 50 * 1024 * 1024, // 50MB
            settings: HashMap::new(),
        }
    }
}

#[tonic::async_trait]
impl nexus_soc_server::NexusSoc for NexusSocService {
    /// Register a detection agent
    async fn register_detection_agent(
        &self,
        request: Request<DetectionAgentRegistration>,
    ) -> Result<Response<DetectionAgentResponse>, Status> {
        let req = request.into_inner();
        let agent_id = Uuid::new_v4().to_string();

        info!(
            "Registering detection agent: {} from {} (mode: {:?})",
            agent_id, req.hostname, req.mode
        );

        let config = Self::default_detection_config();
        let signature_sets = self.state.signature_manager.read().await.signature_sets.clone();

        let session = DetectionAgentSession {
            agent_id: agent_id.clone(),
            hostname: req.hostname,
            os_type: req.os_type,
            os_version: req.os_version,
            ip_address: req.ip_address,
            agent_version: req.agent_version,
            mode: req.mode,
            capabilities: req.capabilities,
            connected_at: chrono::Utc::now(),
            last_health_check: chrono::Utc::now(),
            is_active: true,
            config: Some(config.clone()),
            events_received: 0,
            telemetry_received: 0,
        };

        self.state.agents.write().await.insert(agent_id.clone(), session);

        let response = DetectionAgentResponse {
            agent_id,
            success: true,
            message: "Agent registered successfully".to_string(),
            config: Some(config),
            initial_signatures: signature_sets,
        };

        Ok(Response::new(response))
    }

    /// Submit telemetry batch
    async fn submit_telemetry(
        &self,
        request: Request<TelemetryBatch>,
    ) -> Result<Response<TelemetryResponse>, Status> {
        let batch = request.into_inner();
        let agent_id = &batch.agent_id;
        let event_count = batch.events.len();

        debug!(
            "Received telemetry batch from {}: {} events (seq: {})",
            agent_id, event_count, batch.sequence_number
        );

        // Update agent stats
        if let Some(agent) = self.state.agents.write().await.get_mut(agent_id) {
            agent.telemetry_received += event_count as u64;
            agent.last_health_check = chrono::Utc::now();
        }

        // Store telemetry events
        let mut telemetry_store = self.state.telemetry_events.write().await;
        for event in batch.events {
            telemetry_store.push(StoredTelemetry {
                event,
                received_at: chrono::Utc::now(),
            });
        }

        // Update sequence
        let next_seq = batch.sequence_number + 1;
        self.state
            .telemetry_sequence
            .write()
            .await
            .insert(agent_id.clone(), next_seq);

        let response = TelemetryResponse {
            success: true,
            events_received: event_count as u64,
            events_processed: event_count as u64,
            errors: vec![],
            next_sequence: next_seq,
        };

        Ok(Response::new(response))
    }

    type StreamTelemetryStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<TelemetryAck, Status>> + Send>>;

    /// Stream telemetry with acknowledgments
    async fn stream_telemetry(
        &self,
        request: Request<Streaming<TelemetryEvent>>,
    ) -> Result<Response<Self::StreamTelemetryStream>, Status> {
        let mut stream = request.into_inner();
        let state = self.state.clone();

        let output_stream = async_stream::stream! {
            while let Some(event_result) = stream.message().await.transpose() {
                match event_result {
                    Ok(event) => {
                        let event_id = event.event_id.clone();

                        // Store the event
                        state.telemetry_events.write().await.push(StoredTelemetry {
                            event,
                            received_at: chrono::Utc::now(),
                        });

                        yield Ok(TelemetryAck {
                            event_id,
                            acknowledged: true,
                            error: String::new(),
                        });
                    }
                    Err(e) => {
                        yield Ok(TelemetryAck {
                            event_id: String::new(),
                            acknowledged: false,
                            error: e.to_string(),
                        });
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }

    /// Submit detection event
    async fn submit_detection_event(
        &self,
        request: Request<DetectionEventRequest>,
    ) -> Result<Response<DetectionEventResponse>, Status> {
        let req = request.into_inner();
        let agent_id = req.agent_id;

        if let Some(event) = req.event {
            let event_id = event.event_id.clone();

            info!(
                "Detection event from {}: {} - {} ({:?})",
                agent_id, event.rule_id, event.description, event.severity
            );

            // Update agent stats
            if let Some(agent) = self.state.agents.write().await.get_mut(&agent_id) {
                agent.events_received += 1;
            }

            // Store event
            self.state.detection_events.write().await.push(StoredDetectionEvent {
                event,
                received_at: chrono::Utc::now(),
                processed: false,
            });

            // Generate response actions based on severity
            let recommended_actions = vec![ResponseAction {
                action_type: ResponseActionType::Alert as i32,
                description: "Alert SOC team".to_string(),
                parameters: HashMap::new(),
                auto_execute: true,
            }];

            let response = DetectionEventResponse {
                success: true,
                event_id,
                message: "Detection event received".to_string(),
                recommended_actions,
            };

            Ok(Response::new(response))
        } else {
            Err(Status::invalid_argument("Missing detection event"))
        }
    }

    type GetDetectionEventsStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DetectionEventProto, Status>> + Send>>;

    /// Query detection events
    async fn get_detection_events(
        &self,
        request: Request<DetectionEventQuery>,
    ) -> Result<Response<Self::GetDetectionEventsStream>, Status> {
        let query = request.into_inner();
        let events = self.state.detection_events.read().await.clone();

        let filtered_events: Vec<DetectionEventProto> = events
            .into_iter()
            .filter(|stored| {
                // Filter by agent if specified
                if !query.agent_id.is_empty() && stored.event.agent_id != query.agent_id {
                    return false;
                }

                // Filter by minimum severity
                if stored.event.severity < query.min_severity {
                    return false;
                }

                // Filter by source if specified
                if !query.sources.is_empty()
                    && !query.sources.contains(&stored.event.source)
                {
                    return false;
                }

                true
            })
            .take(query.limit.max(100) as usize)
            .map(|stored| stored.event)
            .collect();

        let output_stream = async_stream::stream! {
            for event in filtered_events {
                yield Ok(event);
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }

    type GetDetectionTasksStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DetectionTask, Status>> + Send>>;

    /// Get detection tasks for agent
    async fn get_detection_tasks(
        &self,
        request: Request<DetectionTaskRequest>,
    ) -> Result<Response<Self::GetDetectionTasksStream>, Status> {
        let req = request.into_inner();
        let tasks = self
            .state
            .task_queue
            .read()
            .await
            .get_tasks_for_agent(&req.agent_id, 10);

        let output_stream = async_stream::stream! {
            for task in tasks {
                yield Ok(task);
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }

    /// Update detection task status
    async fn update_detection_status(
        &self,
        request: Request<DetectionStatusUpdate>,
    ) -> Result<Response<DetectionStatusResponse>, Status> {
        let update = request.into_inner();

        info!(
            "Task {} status update from {}: {:?}",
            update.task_id, update.agent_id, update.status
        );

        self.state.task_queue.write().await.complete_task(update);

        let response = DetectionStatusResponse {
            success: true,
            message: "Status updated".to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get signature updates
    async fn get_signature_updates(
        &self,
        request: Request<SignatureUpdateRequest>,
    ) -> Result<Response<SignatureUpdateResponse>, Status> {
        let req = request.into_inner();
        let sig_manager = self.state.signature_manager.read().await;

        let update_available = req.current_version != sig_manager.current_version;

        let response = SignatureUpdateResponse {
            update_available,
            new_version: sig_manager.current_version.clone(),
            signature_sets: if update_available {
                sig_manager.signature_sets.clone()
            } else {
                vec![]
            },
        };

        Ok(Response::new(response))
    }

    /// Get correlation rules
    async fn get_correlation_rules(
        &self,
        request: Request<CorrelationRuleRequest>,
    ) -> Result<Response<CorrelationRuleResponse>, Status> {
        let req = request.into_inner();
        let corr_manager = self.state.correlation_manager.read().await;

        let needs_update = req.current_version != corr_manager.version;

        let response = CorrelationRuleResponse {
            version: corr_manager.version.clone(),
            rules: if needs_update {
                corr_manager.rules.clone()
            } else {
                vec![]
            },
        };

        Ok(Response::new(response))
    }

    /// Submit sample for analysis
    async fn submit_sample(
        &self,
        request: Request<SampleSubmission>,
    ) -> Result<Response<SampleSubmissionResponse>, Status> {
        let req = request.into_inner();
        let sample_id = Uuid::new_v4().to_string();

        info!(
            "Sample submission from {}: {} ({} bytes)",
            req.agent_id,
            req.filename,
            req.file_content.len()
        );

        let stored_sample = StoredSample {
            sample_id: sample_id.clone(),
            agent_id: req.agent_id,
            filename: req.filename,
            file_hash: req.file_hash_sha256,
            file_size: req.file_content.len(),
            reason: req.reason,
            submitted_at: chrono::Utc::now(),
            analysis_status: AnalysisStatus::Pending as i32,
            analysis_result: None,
        };

        self.state
            .samples
            .write()
            .await
            .insert(sample_id.clone(), stored_sample);

        let response = SampleSubmissionResponse {
            success: true,
            sample_id,
            message: "Sample received".to_string(),
            analysis_queued: true,
        };

        Ok(Response::new(response))
    }

    /// Get analysis result
    async fn get_analysis_result(
        &self,
        request: Request<AnalysisResultRequest>,
    ) -> Result<Response<AnalysisResultResponse>, Status> {
        let req = request.into_inner();

        if let Some(sample) = self.state.samples.read().await.get(&req.sample_id) {
            if let Some(result) = &sample.analysis_result {
                Ok(Response::new(result.clone()))
            } else {
                Ok(Response::new(AnalysisResultResponse {
                    sample_id: req.sample_id,
                    status: sample.analysis_status,
                    threat_score: 0,
                    malware_family: String::new(),
                    signatures_matched: vec![],
                    iocs: vec![],
                    mitre_techniques: vec![],
                    detailed_report: String::new(),
                }))
            }
        } else {
            Err(Status::not_found("Sample not found"))
        }
    }

    /// Agent health check
    async fn agent_health_check(
        &self,
        request: Request<AgentHealthRequest>,
    ) -> Result<Response<AgentHealthResponse>, Status> {
        let req = request.into_inner();

        debug!(
            "Health check from {}: {} events generated, {} submitted",
            req.agent_id, req.events_generated, req.events_submitted
        );

        // Update agent health
        if let Some(agent) = self.state.agents.write().await.get_mut(&req.agent_id) {
            agent.last_health_check = chrono::Utc::now();
            agent.is_active = true;
        }

        let response = AgentHealthResponse {
            success: true,
            next_checkin_secs: 60,
            config_update: None,
            messages: vec![],
        };

        Ok(Response::new(response))
    }
}

/// SOC Server that wraps the service and manages the server lifecycle
pub struct SocServer {
    state: Arc<SocServiceState>,
}

impl SocServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(SocServiceState::new()),
        }
    }

    /// Get the SOC service for adding to a tonic server
    pub fn service(&self) -> NexusSocService {
        NexusSocService::with_state(self.state.clone())
    }

    /// Get connected agents
    pub async fn get_agents(&self) -> Vec<DetectionAgentSession> {
        self.state.agents.read().await.values().cloned().collect()
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> Option<DetectionAgentSession> {
        self.state.agents.read().await.get(agent_id).cloned()
    }

    /// Queue a detection task for an agent
    pub async fn queue_task(&self, agent_id: &str, task: DetectionTask) {
        self.state.task_queue.write().await.queue_task(agent_id, task);
    }

    /// Get detection events
    pub async fn get_detection_events(&self) -> Vec<StoredDetectionEvent> {
        self.state.detection_events.read().await.clone()
    }

    /// Get telemetry events
    pub async fn get_telemetry_events(&self) -> Vec<StoredTelemetry> {
        self.state.telemetry_events.read().await.clone()
    }

    /// Get samples
    pub async fn get_samples(&self) -> Vec<StoredSample> {
        self.state.samples.read().await.values().cloned().collect()
    }

    /// Cleanup inactive agents
    pub async fn cleanup_inactive_agents(&self, timeout_minutes: u64) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes as i64);
        let mut agents = self.state.agents.write().await;

        let initial = agents.len();
        agents.retain(|_, agent| agent.last_health_check > cutoff);
        let removed = initial - agents.len();

        if removed > 0 {
            info!("Cleaned up {} inactive detection agents", removed);
        }

        removed
    }
}

impl Default for SocServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_task_queue() {
        let mut queue = DetectionTaskQueue::new();

        let task = DetectionTask {
            task_id: "task-1".to_string(),
            task_type: DetectionTaskType::ScanMemory as i32,
            parameters: HashMap::new(),
            priority: 100,
            expires_at: None,
        };

        queue.queue_task("agent-1", task);

        let tasks = queue.get_tasks_for_agent("agent-1", 10);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "task-1");
    }

    #[test]
    fn test_signature_manager() {
        let manager = SignatureManager::new();
        assert!(!manager.signature_sets.is_empty());
        assert!(!manager.signature_sets[0].signatures.is_empty());
    }

    #[test]
    fn test_correlation_manager() {
        let manager = CorrelationManager::new();
        assert!(!manager.rules.is_empty());
    }

    #[tokio::test]
    async fn test_soc_service_creation() {
        let server = SocServer::new();
        let agents = server.get_agents().await;
        assert!(agents.is_empty());
    }
}
