# Infrastructure Monitoring Setup

Comprehensive monitoring and alerting setup for Rust-Nexus infrastructure, covering health monitoring, performance metrics, and operational alerting.

## Monitoring Architecture

```
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│   Metrics Sources   │───►│  Monitoring Stack   │───►│   Alerting & Viz    │
│                     │    │                     │    │                     │
│ • gRPC Server       │    │ • Prometheus        │    │ • Grafana Dashboards│
│ • Domain Manager    │    │ • Node Exporter     │    │ • AlertManager      │
│ • Cert Manager      │    │ • Custom Exporters  │    │ • PagerDuty/Slack   │
│ • Cloudflare API    │    │ • Log Aggregation   │    │ • Email Notifications│
│ • Agent Status      │    │ • Health Checks     │    │ • SMS Alerts        │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
```

## Metrics Collection

### Infrastructure Metrics

#### Domain Health Metrics
```rust
// Implement Prometheus metrics for domain health
use prometheus::{Counter, Gauge, Histogram, Registry, Opts};

pub struct DomainMetrics {
    domain_health_gauge: Gauge,
    dns_resolution_duration: Histogram,
    domain_rotation_counter: Counter,
    active_domains_gauge: Gauge,
}

impl DomainMetrics {
    pub fn new(registry: &Registry) -> InfraResult<Self> {
        let domain_health_gauge = Gauge::with_opts(
            Opts::new("nexus_domain_health_percentage", 
                     "Domain uptime percentage")
        )?;
        
        let dns_resolution_duration = Histogram::with_opts(
            Opts::new("nexus_dns_resolution_duration_seconds",
                     "DNS resolution time in seconds")
        )?;
        
        let domain_rotation_counter = Counter::with_opts(
            Opts::new("nexus_domain_rotations_total",
                     "Total number of domain rotations")
        )?;
        
        let active_domains_gauge = Gauge::with_opts(
            Opts::new("nexus_active_domains",
                     "Number of currently active domains")
        )?;
        
        // Register metrics
        registry.register(Box::new(domain_health_gauge.clone()))?;
        registry.register(Box::new(dns_resolution_duration.clone()))?;
        registry.register(Box::new(domain_rotation_counter.clone()))?;
        registry.register(Box::new(active_domains_gauge.clone()))?;
        
        Ok(Self {
            domain_health_gauge,
            dns_resolution_duration,
            domain_rotation_counter,
            active_domains_gauge,
        })
    }
    
    pub fn update_domain_health(&self, health_percentage: f64) {
        self.domain_health_gauge.set(health_percentage);
    }
    
    pub fn record_dns_resolution(&self, duration: Duration) {
        self.dns_resolution_duration.observe(duration.as_secs_f64());
    }
    
    pub fn increment_rotations(&self) {
        self.domain_rotation_counter.inc();
    }
    
    pub fn update_active_domains(&self, count: usize) {
        self.active_domains_gauge.set(count as f64);
    }
}
```

#### Certificate Metrics
```rust
// Certificate monitoring metrics
pub struct CertificateMetrics {
    cert_expiry_gauge: prometheus::GaugeVec,
    cert_renewal_counter: Counter,
    cert_validation_failures: Counter,
    acme_challenge_duration: Histogram,
}

impl CertificateMetrics {
    pub fn new(registry: &Registry) -> InfraResult<Self> {
        let cert_expiry_gauge = prometheus::GaugeVec::new(
            Opts::new("nexus_certificate_expiry_days", 
                     "Days until certificate expiry"),
            &["domain", "cert_type"]
        )?;
        
        let cert_renewal_counter = Counter::with_opts(
            Opts::new("nexus_certificate_renewals_total",
                     "Total certificate renewals")
        )?;
        
        let cert_validation_failures = Counter::with_opts(
            Opts::new("nexus_certificate_validation_failures_total",
                     "Certificate validation failures")
        )?;
        
        let acme_challenge_duration = Histogram::with_opts(
            Opts::new("nexus_acme_challenge_duration_seconds",
                     "ACME challenge completion time")
        )?;
        
        registry.register(Box::new(cert_expiry_gauge.clone()))?;
        registry.register(Box::new(cert_renewal_counter.clone()))?;
        registry.register(Box::new(cert_validation_failures.clone()))?;
        registry.register(Box::new(acme_challenge_duration.clone()))?;
        
        Ok(Self {
            cert_expiry_gauge,
            cert_renewal_counter,
            cert_validation_failures,
            acme_challenge_duration,
        })
    }
    
    pub fn update_certificate_expiry(&self, domain: &str, cert_type: &str, days: i64) {
        self.cert_expiry_gauge
            .with_label_values(&[domain, cert_type])
            .set(days as f64);
    }
}
```

### Agent Metrics

```rust
// Agent connectivity and performance metrics
pub struct AgentMetrics {
    connected_agents_gauge: Gauge,
    agent_heartbeat_duration: prometheus::HistogramVec,
    task_execution_duration: prometheus::HistogramVec,
    task_success_counter: prometheus::CounterVec,
    task_failure_counter: prometheus::CounterVec,
}

impl AgentMetrics {
    pub fn record_agent_count(&self, count: usize) {
        self.connected_agents_gauge.set(count as f64);
    }
    
    pub fn record_heartbeat(&self, agent_id: &str, duration: Duration) {
        self.agent_heartbeat_duration
            .with_label_values(&[agent_id])
            .observe(duration.as_secs_f64());
    }
    
    pub fn record_task_execution(&self, task_type: &str, duration: Duration, success: bool) {
        self.task_execution_duration
            .with_label_values(&[task_type])
            .observe(duration.as_secs_f64());
        
        if success {
            self.task_success_counter.with_label_values(&[task_type]).inc();
        } else {
            self.task_failure_counter.with_label_values(&[task_type]).inc();
        }
    }
}
```

## Health Checks

### gRPC Health Check Service

```rust
// Implement gRPC health check service
use tonic_health::{server::HealthReporter, ServingStatus};

pub struct HealthCheckService {
    health_reporter: HealthReporter,
    component_status: Arc<RwLock<HashMap<String, ServingStatus>>>,
}

impl HealthCheckService {
    pub fn new() -> (Self, tonic_health::server::HealthService) {
        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
        
        let service = Self {
            health_reporter,
            component_status: Arc::new(RwLock::new(HashMap::new())),
        };
        
        (service, health_service)
    }
    
    pub async fn update_component_health(&mut self, component: &str, healthy: bool) {
        let status = if healthy {
            ServingStatus::Serving
        } else {
            ServingStatus::NotServing
        };
        
        self.component_status.write().await.insert(component.to_string(), status);
        self.health_reporter.set_service_status(component, status).await;
    }
    
    pub async fn start_health_monitoring(&mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // Check infrastructure components
            self.check_infrastructure_health().await;
        }
    }
    
    async fn check_infrastructure_health(&mut self) {
        // Check domain health
        let domain_health = self.check_domain_health().await;
        self.update_component_health("domain_manager", domain_health).await;
        
        // Check certificate health
        let cert_health = self.check_certificate_health().await;
        self.update_component_health("certificate_manager", cert_health).await;
        
        // Check Cloudflare API health
        let cf_health = self.check_cloudflare_health().await;
        self.update_component_health("cloudflare_api", cf_health).await;
    }
}
```

### HTTP Health Endpoint

```rust
// HTTP health check endpoint for external monitoring
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;

pub struct HealthHttpServer {
    health_status: Arc<RwLock<HealthStatus>>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall_status: String,
    pub components: HashMap<String, ComponentHealth>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub status: String,
    pub last_check: DateTime<Utc>,
    pub details: HashMap<String, String>,
}

impl HealthHttpServer {
    pub async fn serve(self, addr: SocketAddr) -> InfraResult<()> {
        let make_service = hyper::service::make_service_fn(move |_conn| {
            let health_status = self.health_status.clone();
            async move {
                Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                    Self::handle_request(req, health_status.clone())
                }))
            }
        });
        
        let server = Server::bind(&addr).serve(make_service);
        info!("Health check server listening on {}", addr);
        
        server.await.map_err(|e| InfraError::NetworkError(e.into()))?;
        Ok(())
    }
    
    async fn handle_request(
        req: Request<Body>,
        health_status: Arc<RwLock<HealthStatus>>,
    ) -> Result<Response<Body>, Infallible> {
        match req.uri().path() {
            "/health" => Self::health_check(health_status).await,
            "/health/live" => Self::liveness_check().await,
            "/health/ready" => Self::readiness_check(health_status).await,
            _ => Self::not_found().await,
        }
    }
    
    async fn health_check(health_status: Arc<RwLock<HealthStatus>>) -> Result<Response<Body>, Infallible> {
        let status = health_status.read().await;
        let json_response = serde_json::json!({
            "status": status.overall_status,
            "timestamp": status.last_updated.to_rfc3339(),
            "components": status.components,
        });
        
        let response = Response::builder()
            .status(if status.overall_status == "healthy" { 200 } else { 503 })
            .header("Content-Type", "application/json")
            .body(Body::from(json_response.to_string()))
            .unwrap();
        
        Ok(response)
    }
}
```

## Monitoring Stack Setup

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "nexus-alerts.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  # Nexus gRPC servers
  - job_name: 'nexus-server'
    static_configs:
      - targets: ['server1.yourdomain.com:9090', 'server2.yourdomain.com:9090']
    scheme: https
    tls_config:
      ca_file: /etc/ssl/certs/nexus-ca.pem
      cert_file: /etc/ssl/certs/prometheus.pem
      key_file: /etc/ssl/private/prometheus.key
    scrape_interval: 30s
    metrics_path: /metrics

  # Infrastructure health checks
  - job_name: 'nexus-infrastructure'
    static_configs:
      - targets: ['infrastructure.yourdomain.com:8080']
    metrics_path: /health/metrics
    scrape_interval: 60s

  # Node exporters for system metrics
  - job_name: 'nexus-nodes'
    static_configs:
      - targets: ['server1.yourdomain.com:9100', 'server2.yourdomain.com:9100']
```

### Alert Rules

```yaml
# nexus-alerts.yml
groups:
  - name: nexus-infrastructure
    rules:
      - alert: NexusCertificateExpiry
        expr: nexus_certificate_expiry_days < 7
        for: 1h
        labels:
          severity: critical
        annotations:
          summary: "Nexus certificate expiring soon"
          description: "Certificate for {{ $labels.domain }} expires in {{ $value }} days"

      - alert: NexusDomainHealthLow
        expr: nexus_domain_health_percentage < 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Nexus domain health degraded"
          description: "Domain {{ $labels.domain }} health is {{ $value }}%"

      - alert: NexusAgentDisconnected
        expr: nexus_connected_agents == 0
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "No active Nexus agents"
          description: "All agents have disconnected from C2 servers"

      - alert: NexusCloudflareAPIFailure
        expr: increase(nexus_cloudflare_api_errors_total[5m]) > 5
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Cloudflare API errors detected"
          description: "{{ $value }} Cloudflare API errors in the last 5 minutes"

  - name: nexus-performance
    rules:
      - alert: NexusHighMemoryUsage
        expr: process_resident_memory_bytes{job="nexus-server"} > 1073741824  # 1GB
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage detected"
          description: "Nexus server using {{ $value | humanize }}B memory"

      - alert: NexusHighCPUUsage
        expr: rate(process_cpu_seconds_total{job="nexus-server"}[5m]) * 100 > 80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage detected"
          description: "Nexus server CPU usage is {{ $value }}%"
```

### Custom Metrics Exporter

```rust
// Custom metrics exporter for Nexus infrastructure
use prometheus::{Encoder, TextEncoder, Registry};
use hyper::{Body, Request, Response, Server, StatusCode};

pub struct MetricsExporter {
    registry: Registry,
    domain_metrics: DomainMetrics,
    certificate_metrics: CertificateMetrics,
    agent_metrics: AgentMetrics,
}

impl MetricsExporter {
    pub fn new() -> InfraResult<Self> {
        let registry = Registry::new();
        
        Ok(Self {
            domain_metrics: DomainMetrics::new(&registry)?,
            certificate_metrics: CertificateMetrics::new(&registry)?,
            agent_metrics: AgentMetrics::new(&registry)?,
            registry,
        })
    }
    
    pub async fn start_metrics_server(&self, addr: SocketAddr) -> InfraResult<()> {
        let registry = self.registry.clone();
        
        let make_service = hyper::service::make_service_fn(move |_conn| {
            let registry = registry.clone();
            async move {
                Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                    Self::handle_metrics_request(req, registry.clone())
                }))
            }
        });
        
        let server = Server::bind(&addr).serve(make_service);
        info!("Metrics server listening on {}", addr);
        
        server.await.map_err(|e| InfraError::NetworkError(e.into()))?;
        Ok(())
    }
    
    async fn handle_metrics_request(
        req: Request<Body>,
        registry: Registry,
    ) -> Result<Response<Body>, Infallible> {
        if req.uri().path() == "/metrics" {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            
            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                eprintln!("Failed to encode metrics: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to encode metrics"))
                    .unwrap());
            }
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", encoder.format_type())
                .body(Body::from(buffer))
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap())
        }
    }
}
```

## Log Aggregation

### Structured Logging

```rust
// Structured logging implementation
use serde_json::json;
use tracing::{info, warn, error, debug, instrument};

pub struct StructuredLogger {
    service_name: String,
    version: String,
    environment: String,
}

impl StructuredLogger {
    pub fn new(service_name: String, version: String, environment: String) -> Self {
        Self { service_name, version, environment }
    }
    
    #[instrument(skip(self))]
    pub fn log_infrastructure_event(&self, event_type: &str, details: serde_json::Value) {
        info!(
            target: "infrastructure",
            event_type = event_type,
            service = %self.service_name,
            version = %self.version,
            environment = %self.environment,
            timestamp = %chrono::Utc::now().to_rfc3339(),
            details = %details,
            "Infrastructure event"
        );
    }
    
    #[instrument(skip(self))]
    pub fn log_security_event(&self, event_type: &str, severity: &str, details: serde_json::Value) {
        let log_level = match severity {
            "critical" | "high" => tracing::Level::ERROR,
            "medium" => tracing::Level::WARN,
            "low" => tracing::Level::INFO,
            _ => tracing::Level::DEBUG,
        };
        
        tracing::event!(
            log_level,
            target: "security",
            event_type = event_type,
            severity = severity,
            service = %self.service_name,
            timestamp = %chrono::Utc::now().to_rfc3339(),
            details = %details,
            "Security event"
        );
    }
}
```

### ELK Stack Integration

```yaml
# logstash.conf
input {
  beats {
    port => 5044
  }
}

filter {
  if [fields][service] == "nexus" {
    json {
      source => "message"
    }
    
    mutate {
      add_field => { "[@metadata][index_prefix]" => "nexus" }
    }
    
    # Parse infrastructure events
    if [event_type] == "domain_rotation" {
      mutate {
        add_tag => [ "infrastructure", "domain_management" ]
      }
    }
    
    # Parse security events
    if [target] == "security" {
      mutate {
        add_tag => [ "security", "alert" ]
      }
      
      if [severity] in ["critical", "high"] {
        mutate {
          add_tag => [ "urgent" ]
        }
      }
    }
  }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "%{[@metadata][index_prefix]}-%{+YYYY.MM.dd}"
  }
}
```

### Grafana Dashboards

```json
{
  "dashboard": {
    "title": "Nexus Infrastructure Overview",
    "panels": [
      {
        "title": "Active Domains",
        "type": "stat",
        "targets": [
          {
            "expr": "nexus_active_domains",
            "legendFormat": "Active Domains"
          }
        ]
      },
      {
        "title": "Domain Health",
        "type": "gauge", 
        "targets": [
          {
            "expr": "nexus_domain_health_percentage",
            "legendFormat": "Health %"
          }
        ]
      },
      {
        "title": "Certificate Expiry",
        "type": "table",
        "targets": [
          {
            "expr": "nexus_certificate_expiry_days",
            "legendFormat": "{{ domain }} - {{ cert_type }}"
          }
        ]
      },
      {
        "title": "Connected Agents",
        "type": "graph",
        "targets": [
          {
            "expr": "nexus_connected_agents",
            "legendFormat": "Agents"
          }
        ]
      }
    ]
  }
}
```

## Alerting Configuration

### AlertManager Setup

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'mail.company.com:587'
  smtp_from: 'nexus-alerts@yourdomain.com'

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'web.hook'
  routes:
    # Critical infrastructure alerts
    - match:
        severity: critical
        service: nexus
      receiver: 'nexus-critical'
      group_wait: 0s
      repeat_interval: 5m
    
    # Security alerts
    - match_re:
        target: security
      receiver: 'nexus-security'
      group_wait: 30s

receivers:
  - name: 'nexus-critical'
    email_configs:
      - to: 'security-team@company.com'
        subject: 'CRITICAL: Nexus Infrastructure Alert'
        body: |
          {{ range .Alerts }}
          Alert: {{ .Annotations.summary }}
          Description: {{ .Annotations.description }}
          Severity: {{ .Labels.severity }}
          Time: {{ .StartsAt.Format "2006-01-02 15:04:05" }}
          {{ end }}
    
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#nexus-alerts'
        title: 'Nexus Critical Alert'
        text: '{{ .CommonAnnotations.summary }}'
    
    pagerduty_configs:
      - routing_key: 'your-pagerduty-integration-key'
        description: '{{ .CommonAnnotations.summary }}'

  - name: 'nexus-security'
    email_configs:
      - to: 'security-incidents@company.com'
        subject: 'Nexus Security Event'
```

### Notification Integrations

```rust
// Multi-channel notification system
pub enum NotificationChannel {
    Email { recipients: Vec<String> },
    Slack { webhook_url: String, channel: String },
    PagerDuty { integration_key: String },
    Teams { webhook_url: String },
    SMS { phone_numbers: Vec<String> },
}

pub struct NotificationManager {
    channels: Vec<NotificationChannel>,
    client: reqwest::Client,
}

impl NotificationManager {
    pub async fn send_alert(&self, alert: &Alert) -> InfraResult<()> {
        let futures = self.channels.iter().map(|channel| {
            self.send_to_channel(channel, alert)
        });
        
        // Send to all channels concurrently
        let results = futures_util::future::join_all(futures).await;
        
        // Log any failures
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                warn!("Failed to send alert via channel {}: {}", i, e);
            }
        }
        
        Ok(())
    }
    
    async fn send_to_channel(&self, channel: &NotificationChannel, alert: &Alert) -> InfraResult<()> {
        match channel {
            NotificationChannel::Slack { webhook_url, channel } => {
                self.send_slack_message(webhook_url, channel, alert).await
            }
            NotificationChannel::Email { recipients } => {
                self.send_email_alert(recipients, alert).await
            }
            NotificationChannel::PagerDuty { integration_key } => {
                self.send_pagerduty_alert(integration_key, alert).await
            }
            _ => Ok(()), // Implement other channels as needed
        }
    }
    
    async fn send_slack_message(&self, webhook_url: &str, channel: &str, alert: &Alert) -> InfraResult<()> {
        let payload = json!({
            "channel": channel,
            "username": "Nexus Monitor",
            "icon_emoji": ":warning:",
            "attachments": [{
                "color": match alert.severity.as_str() {
                    "critical" => "danger",
                    "warning" => "warning", 
                    _ => "good"
                },
                "title": format!("Nexus Alert: {}", alert.title),
                "text": alert.description,
                "fields": [
                    {
                        "title": "Severity",
                        "value": alert.severity,
                        "short": true
                    },
                    {
                        "title": "Component", 
                        "value": alert.component,
                        "short": true
                    },
                    {
                        "title": "Timestamp",
                        "value": alert.timestamp.to_rfc3339(),
                        "short": false
                    }
                ]
            }]
        });
        
        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(InfraError::NetworkError(
                reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Slack webhook failed: {}", response.status())
                ))
            ));
        }
        
        Ok(())
    }
}
```

## Infrastructure Monitoring

### Automated Monitoring Service

```rust
// Comprehensive infrastructure monitoring service
pub struct InfrastructureMonitor {
    domain_manager: Arc<DomainManager>,
    cert_manager: Arc<CertificateManager>,
    cf_manager: Arc<CloudflareManager>,
    metrics: Arc<MetricsExporter>,
    alerts: Arc<NotificationManager>,
}

impl InfrastructureMonitor {
    pub async fn start_monitoring(&self) -> InfraResult<()> {
        info!("Starting infrastructure monitoring service");
        
        // Start different monitoring tasks
        let domain_monitor = self.start_domain_monitoring();
        let cert_monitor = self.start_certificate_monitoring();
        let api_monitor = self.start_api_monitoring();
        let health_monitor = self.start_health_monitoring();
        
        // Run all monitoring tasks concurrently
        tokio::try_join!(
            domain_monitor,
            cert_monitor, 
            api_monitor,
            health_monitor
        )?;
        
        Ok(())
    }
    
    async fn start_domain_monitoring(&self) -> InfraResult<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
        
        loop {
            interval.tick().await;
            
            // Check domain health
            match self.domain_manager.check_domain_health().await {
                Ok(health_results) => {
                    for health in health_results {
                        // Update metrics
                        self.metrics.certificate_metrics.update_certificate_expiry(
                            &cert_info.domain,
                            "lets_encrypt",
                            days_until_expiry,
                        );
                        
                        // Check for expiration alerts
                        if days_until_expiry <= 7 {
                            let alert = Alert {
                                title: "Certificate Expiring Soon".to_string(),
                                description: format!("Certificate {} expires in {} days", 
                                                   cert_info.domain, days_until_expiry),
                                severity: if days_until_expiry <= 3 { "critical" } else { "warning" }.to_string(),
                                component: "certificate_manager".to_string(),
                                timestamp: chrono::Utc::now(),
                            };
                            
                            self.alerts.send_alert(&alert).await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Certificate monitoring failed: {}", e);
                }
            }
        }
    }
    
    async fn start_api_monitoring(&self) -> InfraResult<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // 1 minute
        
        loop {
            interval.tick().await;
            
            // Test Cloudflare API connectivity
            match self.cf_manager.verify_access().await {
                Ok(_) => {
                    debug!("Cloudflare API health check passed");
                }
                Err(e) => {
                    warn!("Cloudflare API health check failed: {}", e);
                    
                    let alert = Alert {
                        title: "Cloudflare API Failure".to_string(),
                        description: format!("Cloudflare API error: {}", e),
                        severity: "warning".to_string(),
                        component: "cloudflare_api".to_string(),
                        timestamp: chrono::Utc::now(),
                    };
                    
                    self.alerts.send_alert(&alert).await?;
                }
            }
        }
    }
    
    async fn start_health_monitoring(&self) -> InfraResult<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(30)); // 30 seconds
        
        loop {
            interval.tick().await;
            
            // Aggregate overall health status
            let overall_health = self.calculate_overall_health().await;
            
            if !overall_health.is_healthy {
                let alert = Alert {
                    title: "Infrastructure Health Degraded".to_string(),
                    description: overall_health.description,
                    severity: overall_health.severity,
                    component: "infrastructure".to_string(),
                    timestamp: chrono::Utc::now(),
                };
                
                self.alerts.send_alert(&alert).await?;
            }
        }
    }
    
    async fn calculate_overall_health(&self) -> OverallHealth {
        let mut issues = Vec::new();
        let mut severity = "info";
        
        // Check domain health
        if let Ok(domain_health) = self.domain_manager.get_domain_health().await {
            let unhealthy_domains: Vec<_> = domain_health.iter()
                .filter(|h| h.uptime_percentage < 95.0)
                .collect();
            
            if !unhealthy_domains.is_empty() {
                issues.push(format!("{} domains with poor health", unhealthy_domains.len()));
                severity = "warning";
            }
        }
        
        // Check certificate status
        if let Ok(certificates) = self.cert_manager.list_certificates() {
            let expiring_soon: Vec<_> = certificates.iter()
                .filter(|c| (c.expires_at - chrono::Utc::now()).num_days() <= 30)
                .collect();
            
            if !expiring_soon.is_empty() {
                issues.push(format!("{} certificates expiring within 30 days", expiring_soon.len()));
                if expiring_soon.iter().any(|c| (c.expires_at - chrono::Utc::now()).num_days() <= 7) {
                    severity = "critical";
                } else if severity != "critical" {
                    severity = "warning";
                }
            }
        }
        
        OverallHealth {
            is_healthy: issues.is_empty(),
            description: if issues.is_empty() {
                "All systems operational".to_string()
            } else {
                issues.join("; ")
            },
            severity: severity.to_string(),
        }
    }
}

#[derive(Debug)]
struct OverallHealth {
    is_healthy: bool,
    description: String,
    severity: String,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub title: String,
    pub description: String,
    pub severity: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

## Performance Monitoring

### System Resource Monitoring

```bash
#!/bin/bash
# System resource monitoring script

# Monitor disk usage
CERT_DIR_USAGE=$(df /opt/nexus/certs | tail -1 | awk '{print $5}' | sed 's/%//')
if [ "$CERT_DIR_USAGE" -gt 80 ]; then
    echo "ALERT: Certificate directory usage high: ${CERT_DIR_USAGE}%"
fi

# Monitor memory usage
MEMORY_USAGE=$(ps aux | grep nexus-server | awk '{sum+=$4} END {print sum}')
if (( $(echo "$MEMORY_USAGE > 80" | bc -l) )); then
    echo "ALERT: High memory usage: ${MEMORY_USAGE}%"
fi

# Monitor file descriptor usage
FD_COUNT=$(lsof -p $(pgrep nexus-server) 2>/dev/null | wc -l)
FD_LIMIT=$(ulimit -n)
FD_USAGE=$((FD_COUNT * 100 / FD_LIMIT))

if [ "$FD_USAGE" -gt 80 ]; then
    echo "ALERT: High file descriptor usage: ${FD_USAGE}%"
fi

# Monitor connection count
CONN_COUNT=$(netstat -an | grep :443 | grep ESTABLISHED | wc -l)
echo "Active connections: $CONN_COUNT"
```

### Performance Profiling

```rust
// Performance profiling and monitoring
use std::time::{Duration, Instant};

pub struct PerformanceProfiler {
    operation_timings: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    memory_usage: Arc<AtomicUsize>,
}

impl PerformanceProfiler {
    pub async fn profile_operation<F, R, Fut>(&self, operation_name: &str, operation: F) -> InfraResult<R>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = InfraResult<R>>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        
        // Record timing
        {
            let mut timings = self.operation_timings.write().await;
            timings.entry(operation_name.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
            
            // Keep only last 100 measurements
            let measurements = timings.get_mut(operation_name).unwrap();
            if measurements.len() > 100 {
                measurements.drain(0..measurements.len() - 100);
            }
        }
        
        // Log slow operations
        if duration > Duration::from_secs(10) {
            warn!("Slow operation detected: {} took {:?}", operation_name, duration);
        }
        
        result
    }
    
    pub async fn get_operation_stats(&self, operation_name: &str) -> Option<OperationStats> {
        let timings = self.operation_timings.read().await;
        let measurements = timings.get(operation_name)?;
        
        if measurements.is_empty() {
            return None;
        }
        
        let total_duration: Duration = measurements.iter().sum();
        let avg_duration = total_duration / measurements.len() as u32;
        let min_duration = *measurements.iter().min()?;
        let max_duration = *measurements.iter().max()?;
        
        Some(OperationStats {
            operation_name: operation_name.to_string(),
            count: measurements.len(),
            avg_duration,
            min_duration,
            max_duration,
            total_duration,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OperationStats {
    pub operation_name: String,
    pub count: usize,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub total_duration: Duration,
}
```

## Deployment Monitoring

### Deployment Health Checks

```bash
#!/bin/bash
# Comprehensive deployment health check

echo "=== Nexus Infrastructure Health Check ==="

# Check service status
if systemctl is-active --quiet nexus-server; then
    echo "✓ Nexus server service running"
else
    echo "✗ Nexus server service not running"
    exit 1
fi

# Check gRPC connectivity
if grpcurl -plaintext localhost:443 grpc.health.v1.Health/Check > /dev/null 2>&1; then
    echo "✓ gRPC health check passed"
else
    echo "✗ gRPC health check failed"
fi

# Check certificate validity
for cert_file in /opt/nexus/certs/*.crt; do
    if [ -f "$cert_file" ]; then
        EXPIRY_DATE=$(openssl x509 -in "$cert_file" -noout -enddate | cut -d= -f2)
        EXPIRY_EPOCH=$(date -d "$EXPIRY_DATE" +%s)
        NOW_EPOCH=$(date +%s)
        DAYS_LEFT=$(( (EXPIRY_EPOCH - NOW_EPOCH) / 86400 ))
        
        if [ "$DAYS_LEFT" -lt 30 ]; then
            echo "⚠ Certificate expiring soon: $(basename $cert_file) ($DAYS_LEFT days)"
        else
            echo "✓ Certificate valid: $(basename $cert_file) ($DAYS_LEFT days)"
        fi
    fi
done

# Check domain health
./target/release/nexus-infra domains health --config /opt/nexus/config/production.toml

# Check Cloudflare API connectivity
./target/release/nexus-infra cloudflare verify --config /opt/nexus/config/production.toml

echo "=== Health Check Complete ==="
```

This completes the comprehensive monitoring documentation for the Rust-Nexus infrastructure.
                        self.metrics.domain_metrics.update_domain_health(health.uptime_percentage);
                        
                        if let Some(response_time) = health.response_time_ms {
                            self.metrics.domain_metrics.record_dns_resolution(
                                Duration::from_millis(response_time)
                            );
                        }
                        
                        // Check for alerts
                        if health.uptime_percentage < 90.0 {
                            let alert = Alert {
                                title: "Domain Health Degraded".to_string(),
                                description: format!("Domain {} health: {:.1}%", 
                                                   health.domain, health.uptime_percentage),
                                severity: if health.uptime_percentage < 50.0 { "critical" } else { "warning" }.to_string(),
                                component: "domain_manager".to_string(),
                                timestamp: chrono::Utc::now(),
                            };
                            
                            self.alerts.send_alert(&alert).await?;
                        }
                    }
                    
                    // Update active domain count
                    let active_domains = self.domain_manager.get_active_domains().await;
                    self.metrics.domain_metrics.update_active_domains(active_domains.len());
                }
                Err(e) => {
                    warn!("Domain health check failed: {}", e);
                }
            }
        }
    }
    
    async fn start_certificate_monitoring(&self) -> InfraResult<()> {
        let mut interval = tokio::time::interval(Duration::from_hours(6)); // Check every 6 hours
        
        loop {
            interval.tick().await;
            
            match self.cert_manager.list_certificates() {
                Ok(certificates) => {
                    for cert_info in certificates {
                        let days_until_expiry = (cert_info.expires_at - chrono::Utc::now()).num_days();
                        
                        // Update metrics
