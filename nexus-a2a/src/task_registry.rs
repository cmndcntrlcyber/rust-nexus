//! In-memory task lifecycle registry for the A2A service.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::pb;

const BROADCAST_CAPACITY: usize = 64;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Per-task record stored in the registry.
#[derive(Debug, Clone)]
pub struct TaskRecord {
    /// Protobuf task with id, context, state, timestamps.
    pub task: pb::Task,
    /// Serialized result payload (set on completion/failure).
    pub result: Option<pb::StreamResponse>,
    /// Optional push notification config for this task.
    pub push_config: Option<pb::PushNotificationConfig>,
}

/// Concurrent in-memory task store backing the A2A task lifecycle RPCs.
pub struct TaskRegistry {
    tasks: DashMap<String, TaskRecord>,
    by_context: DashMap<String, Vec<String>>,
    notifiers: DashMap<String, broadcast::Sender<pb::StreamResponse>>,
    next_id: AtomicU64,
}

impl TaskRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            by_context: DashMap::new(),
            notifiers: DashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new task under the given context. Returns the created record.
    pub fn create_task(&self, context_id: &str) -> TaskRecord {
        let task_id = format!("task-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let now = now_unix();
        let task = pb::Task {
            task_id: task_id.clone(),
            context_id: context_id.to_string(),
            state: pb::TaskState::TaskStateSubmitted as i32,
            created_at_unix: now,
            updated_at_unix: now,
        };
        let record = TaskRecord {
            task,
            result: None,
            push_config: None,
        };
        self.tasks.insert(task_id.clone(), record.clone());
        self.by_context
            .entry(context_id.to_string())
            .or_default()
            .push(task_id.clone());
        self.notifiers
            .insert(task_id, broadcast::channel(BROADCAST_CAPACITY).0);
        record
    }

    /// Look up a task by id. Returns a clone of the record.
    pub fn get_task(&self, task_id: &str) -> Option<TaskRecord> {
        self.tasks.get(task_id).map(|r| r.clone())
    }

    /// Transition a task to a new state. Returns the updated task proto or
    /// a not-found error.
    pub fn transition_state(
        &self,
        task_id: &str,
        new_state: i32,
    ) -> Result<pb::Task, tonic::Status> {
        let mut entry = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| tonic::Status::not_found(format!("task {task_id} not found")))?;
        entry.task.state = new_state;
        entry.task.updated_at_unix = now_unix();
        let task = entry.task.clone();
        drop(entry);

        if let Some(tx) = self.notifiers.get(task_id) {
            let notification = pb::StreamResponse {
                payload: Some(pb::stream_response::Payload::Task(pb::TaskStatus {
                    task_id: task_id.to_string(),
                    state: new_state,
                })),
            };
            let _ = tx.send(notification);
        }
        Ok(task)
    }

    /// Set a result payload on a completed/failed task.
    pub fn set_result(&self, task_id: &str, result: pb::StreamResponse) {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.result = Some(result);
        }
    }

    /// Subscribe to state-change notifications for a task.
    pub fn subscribe(
        &self,
        task_id: &str,
    ) -> Result<broadcast::Receiver<pb::StreamResponse>, tonic::Status> {
        let tx = self
            .notifiers
            .get(task_id)
            .ok_or_else(|| tonic::Status::not_found(format!("task {task_id} not found")))?;
        Ok(tx.subscribe())
    }

    /// List tasks with optional filtering and pagination.
    pub fn list_tasks(
        &self,
        context_id: Option<&str>,
        state_filter: Option<i32>,
        page_size: u32,
        page_token: &str,
    ) -> (Vec<pb::Task>, String) {
        let page_size = if page_size == 0 { 50 } else { page_size.min(1000) } as usize;
        let skip: usize = page_token.parse().unwrap_or(0);

        let task_ids: Vec<String> = if let Some(ctx) = context_id {
            self.by_context
                .get(ctx)
                .map(|ids| ids.clone())
                .unwrap_or_default()
        } else {
            self.tasks.iter().map(|e| e.key().clone()).collect()
        };

        let mut tasks: Vec<pb::Task> = task_ids
            .iter()
            .filter_map(|id| self.tasks.get(id).map(|r| r.task.clone()))
            .filter(|t| state_filter.is_none() || Some(t.state) == state_filter)
            .skip(skip)
            .take(page_size + 1)
            .collect();

        let next_page_token = if tasks.len() > page_size {
            tasks.pop();
            (skip + page_size).to_string()
        } else {
            String::new()
        };

        (tasks, next_page_token)
    }

    /// Store a push notification config for a task.
    pub fn set_push_config(
        &self,
        task_id: &str,
        config: pb::PushNotificationConfig,
    ) -> Result<pb::PushNotificationConfig, tonic::Status> {
        let mut entry = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| tonic::Status::not_found(format!("task {task_id} not found")))?;
        entry.push_config = Some(config.clone());
        Ok(config)
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get() {
        let reg = TaskRegistry::new();
        let record = reg.create_task("ctx-1");
        assert_eq!(record.task.context_id, "ctx-1");
        assert_eq!(record.task.state, pb::TaskState::TaskStateSubmitted as i32);

        let fetched = reg.get_task(&record.task.task_id).unwrap();
        assert_eq!(fetched.task.task_id, record.task.task_id);
    }

    #[test]
    fn transition_state_updates_and_notifies() {
        let reg = TaskRegistry::new();
        let record = reg.create_task("ctx-1");
        let mut rx = reg.subscribe(&record.task.task_id).unwrap();

        let updated = reg
            .transition_state(&record.task.task_id, pb::TaskState::TaskStateWorking as i32)
            .unwrap();
        assert_eq!(updated.state, pb::TaskState::TaskStateWorking as i32);

        let notification = rx.try_recv().unwrap();
        if let Some(pb::stream_response::Payload::Task(status)) = notification.payload {
            assert_eq!(status.state, pb::TaskState::TaskStateWorking as i32);
        } else {
            panic!("expected TaskStatus payload");
        }
    }

    #[test]
    fn list_tasks_filters_by_context() {
        let reg = TaskRegistry::new();
        reg.create_task("ctx-a");
        reg.create_task("ctx-a");
        reg.create_task("ctx-b");

        let (tasks, _) = reg.list_tasks(Some("ctx-a"), None, 50, "");
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn list_tasks_filters_by_state() {
        let reg = TaskRegistry::new();
        let r1 = reg.create_task("ctx-1");
        reg.create_task("ctx-1");
        reg.transition_state(&r1.task.task_id, pb::TaskState::TaskStateCompleted as i32)
            .unwrap();

        let (tasks, _) =
            reg.list_tasks(None, Some(pb::TaskState::TaskStateCompleted as i32), 50, "");
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn cancel_sets_state() {
        let reg = TaskRegistry::new();
        let record = reg.create_task("ctx-1");
        let task = reg
            .transition_state(&record.task.task_id, pb::TaskState::TaskStateCanceled as i32)
            .unwrap();
        assert_eq!(task.state, pb::TaskState::TaskStateCanceled as i32);
    }

    #[test]
    fn push_config_roundtrip() {
        let reg = TaskRegistry::new();
        let record = reg.create_task("ctx-1");
        let config = pb::PushNotificationConfig {
            task_id: record.task.task_id.clone(),
            url: "https://example.com/hook".into(),
            token: "secret".into(),
        };
        let stored = reg
            .set_push_config(&record.task.task_id, config.clone())
            .unwrap();
        assert_eq!(stored.url, "https://example.com/hook");

        let fetched = reg.get_task(&record.task.task_id).unwrap();
        assert!(fetched.push_config.is_some());
    }

    #[test]
    fn pagination() {
        let reg = TaskRegistry::new();
        for _ in 0..5 {
            reg.create_task("ctx-page");
        }
        let (page1, token) = reg.list_tasks(Some("ctx-page"), None, 2, "");
        assert_eq!(page1.len(), 2);
        assert!(!token.is_empty());

        let (page2, _) = reg.list_tasks(Some("ctx-page"), None, 2, &token);
        assert_eq!(page2.len(), 2);
    }
}
