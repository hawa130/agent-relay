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
        self.tasks.lock().expect("task manager poisoned").insert(
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
            .expect("task manager poisoned")
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
            .expect("task manager poisoned")
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
