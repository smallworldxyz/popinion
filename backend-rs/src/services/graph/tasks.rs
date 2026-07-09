//! In-memory registry for long-running graph-build tasks (port of models/task.py).
//! AppState cannot grow a field for this, so the registry is a module-level
//! static (same Arc<Mutex<HashMap>> shape as sim::SimRegistry).

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct Task {
    pub task_id: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub progress: u8,
    pub message: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

fn registry() -> &'static Mutex<HashMap<String, Task>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Task>>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

pub fn create(task_type: &str, metadata: serde_json::Value) -> String {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task = Task {
        task_id: task_id.clone(),
        task_type: task_type.to_string(),
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now().to_rfc3339(),
        progress: 0,
        message: String::new(),
        result: None,
        error: None,
        metadata,
    };
    registry().lock().unwrap().insert(task_id.clone(), task);
    task_id
}

pub fn get(task_id: &str) -> Option<Task> {
    registry().lock().unwrap().get(task_id).cloned()
}

pub fn update(task_id: &str, progress: u8, message: impl Into<String>) {
    with(task_id, |t| {
        t.status = TaskStatus::Processing;
        t.progress = progress;
        t.message = message.into();
    });
}

pub fn complete(task_id: &str, result: serde_json::Value) {
    with(task_id, |t| {
        t.status = TaskStatus::Completed;
        t.progress = 100;
        t.result = Some(result);
    });
}

pub fn fail(task_id: &str, error: impl Into<String>) {
    with(task_id, |t| {
        t.status = TaskStatus::Failed;
        t.error = Some(error.into());
    });
}

fn with(task_id: &str, f: impl FnOnce(&mut Task)) {
    if let Some(t) = registry().lock().unwrap().get_mut(task_id) {
        f(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_lifecycle() {
        let id = create("graph_build", json!({"k": "v"}));
        let t = get(&id).unwrap();
        assert_eq!(t.status, TaskStatus::Pending);
        assert_eq!(t.progress, 0);

        update(&id, 40, "working");
        let t = get(&id).unwrap();
        assert_eq!(t.status, TaskStatus::Processing);
        assert_eq!(t.progress, 40);
        assert_eq!(t.message, "working");

        complete(&id, json!({"graph_id": "g1"}));
        let t = get(&id).unwrap();
        assert_eq!(t.status, TaskStatus::Completed);
        assert_eq!(t.progress, 100);
        assert_eq!(t.result.unwrap()["graph_id"], "g1");
    }

    #[test]
    fn fail_records_error() {
        let id = create("graph_build", json!({}));
        fail(&id, "boom");
        let t = get(&id).unwrap();
        assert_eq!(t.status, TaskStatus::Failed);
        assert_eq!(t.error.as_deref(), Some("boom"));
    }

    #[test]
    fn unknown_task_is_none() {
        assert!(get("nope").is_none());
    }
}
