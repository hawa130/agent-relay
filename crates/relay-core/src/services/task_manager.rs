use crate::models::{RelayTaskKind, RelayTaskStatus, TaskUpdate};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TaskCancellationHandle(Arc<dyn Fn() + Send + Sync>);

impl TaskCancellationHandle {
    pub fn new<F>(cancel: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self(Arc::new(cancel))
    }

    pub fn cancel(&self) {
        (self.0)();
    }
}

struct RunningTask {
    kind: RelayTaskKind,
    started_at: DateTime<Utc>,
    cancel: TaskCancellationHandle,
}

#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<Mutex<HashMap<String, RunningTask>>>,
    next_id: Arc<AtomicU64>,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start(&self, kind: RelayTaskKind, cancel: TaskCancellationHandle) -> TaskUpdate {
        let started_at = Utc::now();
        let task_id = format!("task-{}", self.next_id.fetch_add(1, Ordering::Relaxed) + 1);
        self.tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .insert(
                task_id.clone(),
                RunningTask {
                    kind,
                    started_at,
                    cancel,
                },
            );
        TaskUpdate {
            task_id,
            kind,
            status: RelayTaskStatus::Pending,
            started_at,
            finished_at: None,
            message: None,
            error_code: None,
            result: None,
        }
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        let cancel = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(task_id)
            .map(|task| task.cancel.clone());
        if let Some(cancel) = cancel {
            cancel.cancel();
            true
        } else {
            false
        }
    }

    pub fn finish_succeeded(
        &self,
        task_id: &str,
        result: Value,
        message: Option<String>,
    ) -> Option<TaskUpdate> {
        self.finish(
            task_id,
            RelayTaskStatus::Succeeded,
            message,
            None,
            Some(result),
        )
    }

    pub fn finish_failed(
        &self,
        task_id: &str,
        error_code: String,
        message: String,
    ) -> Option<TaskUpdate> {
        self.finish(
            task_id,
            RelayTaskStatus::Failed,
            Some(message),
            Some(error_code),
            None,
        )
    }

    pub fn finish_cancelled(&self, task_id: &str, message: Option<String>) -> Option<TaskUpdate> {
        self.finish(task_id, RelayTaskStatus::Cancelled, message, None, None)
    }

    fn finish(
        &self,
        task_id: &str,
        status: RelayTaskStatus,
        message: Option<String>,
        error_code: Option<String>,
        result: Option<Value>,
    ) -> Option<TaskUpdate> {
        let task = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(task_id)?;
        Some(TaskUpdate {
            task_id: task_id.to_string(),
            kind: task.kind,
            status,
            started_at: task.started_at,
            finished_at: Some(Utc::now()),
            message,
            error_code,
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    fn noop_cancel() -> TaskCancellationHandle {
        TaskCancellationHandle::new(|| {})
    }

    #[test]
    fn start_creates_task_with_pending_status() {
        let manager = TaskManager::new();
        let update = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());

        assert_eq!(update.task_id, "task-1");
        assert_eq!(update.kind, RelayTaskKind::ProfileLogin);
        assert_eq!(update.status, RelayTaskStatus::Pending);
        assert!(update.finished_at.is_none());
        assert!(update.message.is_none());
        assert!(update.error_code.is_none());
        assert!(update.result.is_none());
    }

    #[test]
    fn cancel_invokes_cancellation_handle() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let cancel = TaskCancellationHandle::new(move || {
            flag.store(true, Ordering::Relaxed);
        });
        let manager = TaskManager::new();
        let update = manager.start(RelayTaskKind::ProfileLogin, cancel);

        let result = manager.cancel(&update.task_id);

        assert!(result);
        assert!(cancelled.load(Ordering::Relaxed));
    }

    #[test]
    fn cancel_returns_false_for_unknown_task() {
        let manager = TaskManager::new();
        assert!(!manager.cancel("nonexistent"));
    }

    #[test]
    fn finish_succeeded_removes_task_and_returns_update() {
        let manager = TaskManager::new();
        let update = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());

        let finished = manager
            .finish_succeeded(&update.task_id, json!({"ok": true}), Some("done".into()))
            .expect("should return update");

        assert_eq!(finished.status, RelayTaskStatus::Succeeded);
        assert_eq!(finished.result, Some(json!({"ok": true})));
        assert_eq!(finished.message.as_deref(), Some("done"));
        assert!(finished.finished_at.is_some());

        // Task is removed, so finishing again returns None
        assert!(
            manager
                .finish_succeeded(&update.task_id, json!(null), None)
                .is_none()
        );
    }

    #[test]
    fn finish_failed_records_error() {
        let manager = TaskManager::new();
        let update = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());

        let finished = manager
            .finish_failed(&update.task_id, "ERR_AUTH".into(), "auth failed".into())
            .expect("should return update");

        assert_eq!(finished.status, RelayTaskStatus::Failed);
        assert_eq!(finished.error_code.as_deref(), Some("ERR_AUTH"));
        assert_eq!(finished.message.as_deref(), Some("auth failed"));
        assert!(finished.result.is_none());
    }

    #[test]
    fn finish_cancelled_returns_cancelled_status() {
        let manager = TaskManager::new();
        let update = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());

        let finished = manager
            .finish_cancelled(&update.task_id, Some("user cancelled".into()))
            .expect("should return update");

        assert_eq!(finished.status, RelayTaskStatus::Cancelled);
        assert_eq!(finished.message.as_deref(), Some("user cancelled"));
    }

    #[test]
    fn two_tasks_get_different_ids() {
        let manager = TaskManager::new();
        let first = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());
        let second = manager.start(RelayTaskKind::ProfileLogin, noop_cancel());

        assert_ne!(first.task_id, second.task_id);
        assert_eq!(first.task_id, "task-1");
        assert_eq!(second.task_id, "task-2");
    }

    #[test]
    fn finishing_nonexistent_task_returns_none() {
        let manager = TaskManager::new();
        assert!(
            manager
                .finish_succeeded("no-such-task", json!(null), None)
                .is_none()
        );
        assert!(
            manager
                .finish_failed("no-such-task", "E".into(), "m".into())
                .is_none()
        );
        assert!(manager.finish_cancelled("no-such-task", None).is_none());
    }
}
