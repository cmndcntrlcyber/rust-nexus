# Code Generation Standards - AI Agent Consistency

This document defines the coding standards, patterns, and conventions that ensure consistency across all AI agents working on the Rust-Nexus WASM UI project.

## 🎯 **Consistency Goals**

### **Why Standardization Matters for AI Agents**
- **Seamless Integration**: Code from different agents integrates without conflicts
- **Maintenance Efficiency**: Consistent patterns reduce debugging and refactoring time
- **Knowledge Transfer**: Agents can easily understand and extend each other's work
- **Quality Assurance**: Standardized patterns enable automated quality validation
- **Performance Optimization**: Consistent patterns allow for systematic optimization

## 📋 **Core Coding Standards**

### **1. Rust Code Formatting**

**Mandatory rustfmt Configuration (`.rustfmt.toml`)**:
```toml
# Standard formatting rules for all agents
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
merge_derives = true
use_try_shorthand = true
use_field_init_shorthand = true
force_explicit_abi = true
empty_item_single_line = true
struct_lit_single_line = true
fn_single_line = false
where_single_line = false
imports_layout = "Mixed"
group_imports = "StdExternalCrate"
```

**Clippy Configuration (`.cargo/config.toml`)**:
```toml
[alias]
clippy-strict = "clippy -- -D warnings -D clippy::all -D clippy::pedantic -A clippy::module_name_repetitions"

[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
```

### **2. File Organization Standards**

**Module Structure Template**:
```rust
//! Module documentation with purpose and usage examples
//! 
//! # Examples
//! ```rust
//! use crate::components::AgentStatusDisplay;
//! // Usage example here
//! ```

// Standard import order (enforced by rustfmt)
use std::collections::HashMap;
use std::sync::Arc;

// External crates
use serde::{Deserialize, Serialize};
use yew::prelude::*;

// Local crate imports
use crate::services::AgentService;
use crate::types::{AgentId, AgentStatus};

// Module declarations
pub mod submodule;

// Constants (SCREAMING_SNAKE_CASE)
const MAX_AGENTS_PER_PAGE: usize = 50;
const DEFAULT_REFRESH_INTERVAL: u64 = 30;

// Type aliases for clarity
type AgentMap = HashMap<AgentId, AgentStatus>;
type StatusCallback = Callback<AgentStatusEvent>;

// Main implementation
```

### **3. Component Architecture Standards**

**Yew Component Template**:
```rust
use yew::prelude::*;
use serde::{Deserialize, Serialize};

/// Component for displaying agent status with real-time updates
/// 
/// # Properties
/// - `agent_id`: Unique identifier for the agent
/// - `refresh_interval`: Update frequency in seconds
/// - `on_status_change`: Callback for status change events
/// 
/// # Example
/// ```rust
/// html! {
///     <AgentStatusDisplay
///         agent_id="agent-001"
///         refresh_interval={30}
///         on_status_change={status_callback}
///     />
/// }
/// ```
#[derive(Properties, PartialEq)]
pub struct AgentStatusDisplayProps {
    /// Unique agent identifier
    pub agent_id: AttrValue,
    
    /// Update interval in seconds (default: 30)
    #[prop_or(30)]
    pub refresh_interval: u64,
    
    /// Optional callback for status changes
    #[prop_or_default]
    pub on_status_change: Option<Callback<AgentStatusEvent>>,
    
    /// CSS classes to apply to the component
    #[prop_or_default]
    pub class: Classes,
    
    /// Additional HTML attributes
    #[prop_or_default]
    pub attrs: AttrValue,
}

/// Events emitted by the AgentStatusDisplay component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentStatusEvent {
    /// Agent came online
    Connected { agent_id: String, timestamp: u64 },
    /// Agent went offline
    Disconnected { agent_id: String, timestamp: u64 },
    /// Agent status updated
    StatusChanged { agent_id: String, new_status: AgentStatus },
    /// Error occurred while fetching status
    Error { agent_id: String, error: String },
}

/// Internal component state
#[derive(Debug, Clone, PartialEq)]
struct ComponentState {
    current_status: Option<AgentStatus>,
    last_update: u64,
    is_loading: bool,
    error_message: Option<String>,
}

impl Default for ComponentState {
    fn default() -> Self {
        Self {
            current_status: None,
            last_update: 0,
            is_loading: true,
            error_message: None,
        }
    }
}

/// Main component implementation
#[function_component(AgentStatusDisplay)]
pub fn agent_status_display(props: &AgentStatusDisplayProps) -> Html {
    // Hooks for state management
    let state = use_state(ComponentState::default);
    let agent_service = use_context::<AgentService>()
        .expect("AgentService context is required");

    // Effect for periodic status updates
    {
        let agent_id = props.agent_id.clone();
        let refresh_interval = props.refresh_interval;
        let state = state.clone();
        let agent_service = agent_service.clone();
        
        use_effect_with((agent_id.clone(), refresh_interval), move |_| {
            let agent_id = agent_id.clone();
            let state = state.clone();
            let agent_service = agent_service.clone();
            
            // Set up periodic updates
            let interval_handle = gloo::timers::callback::Interval::new(
                (refresh_interval * 1000) as u32,
                move || {
                    let agent_id = agent_id.clone();
                    let state = state.clone();
                    let agent_service = agent_service.clone();
                    
                    wasm_bindgen_futures::spawn_local(async move {
                        match agent_service.get_agent_status(&agent_id).await {
                            Ok(status) => {
                                state.set(ComponentState {
                                    current_status: Some(status),
                                    last_update: js_sys::Date::now() as u64,
                                    is_loading: false,
                                    error_message: None,
                                });
                            },
                            Err(error) => {
                                state.set(ComponentState {
                                    current_status: None,
                                    last_update: js_sys::Date::now() as u64,
                                    is_loading: false,
                                    error_message: Some(error.to_string()),
                                });
                            }
                        }
                    });
                }
            );
            
            // Cleanup function
            move || drop(interval_handle)
        });
    }

    // Event handling
    let on_retry = {
        let state = state.clone();
        Callback::from(move |_| {
            state.set(ComponentState {
                is_loading: true,
                error_message: None,
                ..(*state).clone()
            });
        })
    };

    // Render component
    html! {
        <div class={classes!("agent-status-display", props.class.clone())}>
            { render_status_content(&state, &on_retry) }
        </div>
    }
}

/// Helper function to render status content
fn render_status_content(state: &UseStateHandle<ComponentState>, on_retry: &Callback<()>) -> Html {
    match (state.is_loading, &state.error_message, &state.current_status) {
        (true, None, _) => html! {
            <div class="loading-spinner">
                <span>{"Loading agent status..."}</span>
            </div>
        },
        (_, Some(error), _) => html! {
            <div class="error-display">
                <span class="error-message">{ error }</span>
                <button onclick={on_retry.clone()}>{"Retry"}</button>
            </div>
        },
        (_, None, Some(status)) => html! {
            <div class="status-display">
                <StatusIndicator status={status.clone()} />
                <StatusDetails status={status.clone()} />
            </div>
        },
        (_, None, None) => html! {
            <div class="no-data">
                <span>{"No agent status available"}</span>
            </div>
        },
    }
}

// Additional helper components would be defined here...

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_agent_status_display_renders() {
        // Test implementation
        assert!(true); // Placeholder
    }

    #[wasm_bindgen_test]
    async fn test_status_updates() {
        // Test status update functionality
        assert!(true); // Placeholder
    }
}
```

## 🔧 **Service Layer Standards**

### **API Service Template**:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in agent operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum AgentServiceError {
    #[error("Network error: {message}")]
    NetworkError { message: String },
    
    #[error("Authentication failed: {reason}")]
    AuthenticationError { reason: String },
    
    #[error("Agent not found: {agent_id}")]
    AgentNotFound { agent_id: String },
    
    #[error("Invalid response format: {details}")]
    InvalidResponse { details: String },
    
    #[error("Service unavailable: {reason}")]
    ServiceUnavailable { reason: String },
}

/// Result type for agent service operations
pub type AgentServiceResult<T> = Result<T, AgentServiceError>;

/// Configuration for the agent service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentServiceConfig {
    /// Base URL for the API endpoints
    pub base_url: String,
    
    /// Authentication token
    pub auth_token: Option<String>,
    
    /// Request timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u32,
    
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    
    /// Enable request/response logging
    #[serde(default)]
    pub enable_logging: bool,
}

fn default_timeout() -> u32 { 30000 }
fn default_max_retries() -> u32 { 3 }

/// Trait defining agent service operations
#[async_trait(?Send)]
pub trait AgentServiceTrait {
    /// Get status for a specific agent
    async fn get_agent_status(&self, agent_id: &str) -> AgentServiceResult<AgentStatus>;
    
    /// List all available agents
    async fn list_agents(&self) -> AgentServiceResult<Vec<AgentSummary>>;
    
    /// Execute a task on the specified agent
    async fn execute_task(&self, agent_id: &str, task: TaskRequest) -> AgentServiceResult<TaskResult>;
    
    /// Subscribe to real-time agent status updates
    async fn subscribe_status_updates(&self) -> AgentServiceResult<StatusUpdateStream>;
}

/// Implementation of the agent service
#[derive(Debug, Clone)]
pub struct AgentService {
    config: AgentServiceConfig,
    http_client: Arc<HttpClient>,
    auth_manager: Arc<AuthManager>,
}

impl AgentService {
    /// Create a new agent service instance
    pub fn new(config: AgentServiceConfig) -> AgentServiceResult<Self> {
        let http_client = Arc::new(
            HttpClient::new(&config.base_url, config.timeout_ms)
                .map_err(|e| AgentServiceError::NetworkError { 
                    message: e.to_string() 
                })?
        );
        
        let auth_manager = Arc::new(AuthManager::new(config.auth_token.clone()));
        
        Ok(Self {
            config,
            http_client,
            auth_manager,
        })
    }
    
    /// Internal method to make authenticated requests
    async fn make_request<T, R>(&self, endpoint: &str, payload: Option<T>) -> AgentServiceResult<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Implementation details...
        todo!("Implement request logic with retry, authentication, and error handling")
    }
}

#[async_trait(?Send)]
impl AgentServiceTrait for AgentService {
    async fn get_agent_status(&self, agent_id: &str) -> AgentServiceResult<AgentStatus> {
        self.make_request(&format!("/agents/{}/status", agent_id), None::<()>).await
    }
    
    async fn list_agents(&self) -> AgentServiceResult<Vec<AgentSummary>> {
        self.make_request("/agents", None::<()>).await
    }
    
    async fn execute_task(&self, agent_id: &str, task: TaskRequest) -> AgentServiceResult<TaskResult> {
        self.make_request(&format!("/agents/{}/tasks", agent_id), Some(task)).await
    }
    
    async fn subscribe_status_updates(&self) -> AgentServiceResult<StatusUpdateStream> {
        // WebSocket subscription implementation
        todo!("Implement WebSocket status subscription")
    }
}

// Mock implementation for testing
#[cfg(test)]
pub struct MockAgentService {
    agents: HashMap<String, AgentStatus>,
}

#[cfg(test)]
impl MockAgentService {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }
    
    pub fn add_agent(&mut self, agent_id: String, status: AgentStatus) {
        self.agents.insert(agent_id, status);
    }
}

#[cfg(test)]
#[async_trait(?Send)]
impl AgentServiceTrait for MockAgentService {
    async fn get_agent_status(&self, agent_id: &str) -> AgentServiceResult<AgentStatus> {
        self.agents.get(agent_id)
            .cloned()
            .ok_or_else(|| AgentServiceError::AgentNotFound { 
                agent_id: agent_id.to_string() 
            })
    }
    
    async fn list_agents(&self) -> AgentServiceResult<Vec<AgentSummary>> {
        Ok(self.agents.iter().map(|(id, status)| {
            AgentSummary {
                id: id.clone(),
                status: status.health.clone(),
                last_seen: status.last_heartbeat,
            }
        }).collect())
    }
    
    async fn execute_task(&self, _agent_id: &str, _task: TaskRequest) -> AgentServiceResult<TaskResult> {
        // Mock implementation
        Ok(TaskResult::success("mock-task".to_string(), "Task completed".to_string()))
    }
    
    async fn subscribe_status_updates(&self) -> AgentServiceResult<StatusUpdateStream> {
        todo!("Mock WebSocket implementation")
    }
}
```

## 🎨 **Styling and CSS Standards**

### **SCSS Architecture**:
```scss
// Main stylesheet structure (assets/styles/main.scss)

// 1. Configuration and variables
@import 'config/variables';
@import 'config/mixins';
@import 'config/functions';

// 2. Base styles and resets
@import 'base/reset';
@import 'base/typography';
@import 'base/layout';

// 3. Component styles
@import 'components/buttons';
@import 'components/forms';
@import 'components/tables';
@import 'components/modals';
@import 'components/notifications';

// 4. Agent-specific components
@import 'components/agent-status-display';
@import 'components/task-manager';
@import 'components/reporting-dashboard';

// 5. Utility classes
@import 'utilities/spacing';
@import 'utilities/colors';
@import 'utilities/display';
```

**CSS Variable Standards (`_variables.scss`)**:
```scss
// Color palette following design system
:root {
  // Brand colors
  --color-primary: #2563eb;
  --color-primary-light: #3b82f6;
  --color-primary-dark: #1d4ed8;
  
  // Semantic colors
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-info: #06b6d4;
  
  // Agent status colors
  --color-agent-online: var(--color-success);
  --color-agent-offline: #6b7280;
  --color-agent-error: var(--color-error);
  --color-agent-unknown: var(--color-warning);
  
  // Layout dimensions
  --sidebar-width: 280px;
  --header-height: 64px;
  --border-radius: 8px;
  --border-radius-small: 4px;
  --border-radius-large: 12px;
  
  // Spacing scale (based on 8px grid)
  --spacing-xs: 0.25rem;  /* 4px */
  --spacing-sm: 0.5rem;   /* 8px */
  --spacing-md: 1rem;     /* 16px */
  --spacing-lg: 1.5rem;   /* 24px */
  --spacing-xl: 2rem;     /* 32px */
  --spacing-2xl: 3rem;    /* 48px */
  
  // Typography
  --font-family-sans: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-family-mono: 'JetBrains Mono', 'Cascadia Code', monospace;
  
  --font-size-xs: 0.75rem;   /* 12px */
  --font-size-sm: 0.875rem;  /* 14px */
  --font-size-md: 1rem;      /* 16px */
  --font-size-lg: 1.125rem;  /* 18px */
  --font-size-xl: 1.25rem;   /* 20px */
  --font-size-2xl: 1.5rem;   /* 24px */
  
  // Shadows and effects
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1);
  
  // Transitions
  --transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-normal: 250ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-slow: 350ms cubic-bezier(0.4, 0, 0.2, 1);
}

// Dark theme overrides
[data-theme="dark"] {
  --color-background: #0f172a;
  --color-surface: #1e293b;
  --color-text-primary: #f8fafc;
  --color-text-secondary: #cbd5e1;
  --color-border: #334155;
}
```

**Component Styling Standards**:
```scss
// Component-specific styles (components/_agent-status-display.scss)
.agent-status-display {
  // Use CSS custom properties for themeable values
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--border-radius);
  padding: var(--spacing-md);
  
  // Use semantic class names that describe purpose, not appearance
  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-sm);
  }
  
  &__status-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    
    // Use CSS custom properties for status colors
    &--online { background-color: var(--color-agent-online); }
    &--offline { background-color: var(--color-agent-offline); }
    &--error { background-color: var(--color-agent-error); }
    &--unknown { background-color: var(--color-agent-unknown); }
  }
  
  &__content {
    // Responsive design using CSS Grid
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--spacing-md);
  }
  
  // Loading state
  &--loading {
    opacity: 0.6;
    pointer-events: none;
    
    .agent-status-display__content {
      position: relative;
      
      &::after {
        content: '';
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: var(--color-surface);
        animation: pulse 2s infinite;
      }
    }
  }
  
  // Error state
  &--error {
    border-color: var(--color-error);
    
    .agent-status-display__header {
      color: var(--color-error);
    }
  }
}

// Responsive breakpoints
@media (max-width: 768px) {
  .agent-status-display {
    &__content {
      grid-template-columns: 1fr;
    }
  }
}

// Animation keyframes
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

## 🧪 **Testing Standards**

### **Unit Test Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    // Configure tests to run in browser environment
    wasm_bindgen_test_configure!(run_in_browser);

    // Helper function to create test props
    fn create_test_props() -> AgentStatusDisplayProps {
        AgentStatusDisplayProps {
            agent_id: "test-agent-001".into(),
            refresh_interval: 10,
            on_status_change: None,
            class: Classes::new(),
            attrs: AttrValue::default(),
        }
    }

    // Test basic component rendering
    #[wasm_bindgen_test]
    async fn test_component_renders_with_default_props() {
        let props = create_test_props();
        
        // Create virtual DOM representation
        let component = html! {
            <AgentStatusDisplay ..props />
        };
        
        // Verify component structure
        // Note: Actual testing framework integration would be more sophisticated
        assert!(true); // Placeholder for actual assertion
    }

    // Test component behavior with different props
    #[wasm_bindgen_test]
    async fn test_component_with_custom_refresh_interval() {
        let props = AgentStatusDisplayProps {
            refresh_interval: 5,
            ..create_test_props()
        };
        
        // Test implementation
        assert_eq!(props.refresh_interval, 5);
    }

    // Test error handling
    #[wasm_bindgen_test]
    async fn test_component_handles_service_errors() {
        // Setup mock service that returns errors
        let mock_service = MockAgentService::new();
        
        // Test error handling behavior
        let result = mock_service.get_agent_status("nonexistent-agent").await;
        assert!(matches!(result, Err(AgentServiceError::AgentNotFound { .. })));
    }

    // Integration test with mock data
    #[wasm_bindgen_test]
    async fn test_component_integration_with_mock_service() {
        let mut mock_service = MockAgentService::new();
        mock_service.add_agent("test-agent".to_string(), AgentStatus::default());
        
        let status = mock_service.get_agent_status("test-agent").await.unwrap();
        assert_eq!(status.agent_id, "test-agent");
    }
}
```

### **Integration Test Standards**:
```rust
// Integration tests (tests/integration_tests.rs)
use wasm_bindgen_test::*;
use nexus_wasm_ui::components::*;
use nexus_wasm_ui::services::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_end_to_end_agent_status_flow() {
    // Test complete user flow
    // 1. Component mounts
    // 2. Service fetches data
    // 3. Component displays data
    // 4. User interactions work
    // 5. Real-time updates function
    
    todo!("Implement full integration test");
}

#[wasm_bindgen_test]
async fn test_cross_component_communication() {
    // Test communication between different components
    todo!("Test component interaction patterns");
}
```

## 📊 **Performance Standards**

### **Performance Monitoring Code**:
```rust
use web_sys::Performance;
use wasm_bindgen::JsCast;

/// Performance monitoring utility for components
pub struct PerformanceMonitor {
    start_times: std::collections::HashMap<String, f64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_times: std::collections::HashMap::new(),
        }
    }
    
    /// Start timing an operation
    pub fn start_timing(&mut self, operation: &str) {
        let performance = web_sys::window()
            .and_then(|w| w.performance())
            .expect("Performance API should be available");
        
        self.start_times.insert(operation.to_string(), performance.now());
    }
    
    /// End timing and log result
    pub fn end_timing(&mut self, operation: &str) {
        let performance = web_sys::window()
            .and_then(|w| w.performance())
            .expect("Performance API should be available");
        
        if let Some(start_time) = self.start_times.remove(operation) {
            let duration = performance.now() - start_time;
            
            // Log performance metrics
            log::info!(
                target: "performance",
                "Operation '{}' took {:.2}ms",
                operation,
                duration
            );
            
            // Warn about slow operations
            if duration > 16.0 {  // >16ms is one frame at 60fps
                log::warn!(
                    target: "performance",
                    "Slow operation detected: '{}' took {:.2}ms (>{:.2}ms threshold)",
                    operation,
                    duration,
                    16.0
                );
            }
        }
    }
}

/// Macro for easy performance monitoring
#[macro_export]
macro_rules! monitor_performance {
    ($monitor:expr, $operation:expr, $code:block) => {{
        $monitor.start_timing($operation);
        let result = $code;
        $monitor.end_timing($operation);
        result
    }};
}
```

## ✅ **Code Review Checklist**

### **Pre-Commit Validation**
Before committing code, each agent must verify:

- [ ] **Formatting**: Code formatted with `cargo fmt`
- [ ] **Linting**: All clippy warnings addressed
- [ ] **Testing**: New tests added for new functionality
- [ ] **Documentation**: Public APIs documented with rustdoc
- [ ] **Performance**: No obvious performance regressions
- [ ] **Compatibility**: Changes don't break existing interfaces
- [ ] **Security**: No security vulnerabilities introduced
- [ ] **Accessibility**: UI components meet WCAG 2.1 standards

### **Architecture Compliance**
- [ ] **Separation of Concerns**: Components, services, and utilities properly separated
- [ ] **Error Handling**: Comprehensive error handling implemented
- [ ] **State Management**: Proper state management patterns used
- [ ] **Type Safety**: Strong typing used throughout
- [ ] **Async Patterns**: Proper async/await usage
- [ ] **Memory Management**: No obvious memory leaks
- [ ] **Browser Compatibility**: Tested in target browsers

### **Communication Standards**
- [ ] **API Contracts**: Changes communicated to dependent agents
- [ ] **Breaking Changes**: Proper migration path provided
- [ ] **Documentation Updates**: Related documentation updated
- [ ] **Integration Points**: Cross-component integration verified

## 🚀 **Automated Enforcement**

### **CI/CD Pipeline Checks**:
```yaml
# .github/workflows/quality-check.yml
name: Code Quality Check

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          target: wasm32-unknown-unknown
          
      - name: Check formatting
        run: cargo fmt -- --check
        
      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
        
      - name: Run tests
        run: |
          cargo test --all-targets --all-features
          wasm-pack test --node
          
      - name: Check documentation
        run: cargo doc --all-features --no-deps
        
      - name: Security audit
