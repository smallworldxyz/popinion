//! In-memory registry for long-running graph-build tasks.
//! AppState cannot grow a field for this, so it is a module-level static
//! instance of the generic services::registry::Registry.

use serde::Serialize;

use crate::services::registry::{JobStatus, Registry};

#[derive(Clone, Debug, Serialize)]
pub struct Task {
    pub task_id: String,
    pub task_type: String,
    pub status: JobStatus,
    pub created_at: String,
    pub progress: u8,
    pub message: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

static TASKS: Registry<Task> = Registry::new();

pub fn create(task_type: &str, metadata: serde_json::Value) -> String {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task = Task {
        task_id: task_id.clone(),
        task_type: task_type.to_string(),
        status: JobStatus::Pending,
        created_at: chrono::Utc::now().to_rfc3339(),
        progress: 0,
        message: String::new(),
        result: None,
        error: None,
        metadata,
    };
    TASKS.insert(task_id.clone(), task);
    task_id
}

pub fn get(task_id: &str) -> Option<Task> {
    TASKS.get(task_id)
}

pub fn update(task_id: &str, progress: u8, message: impl Into<String>) {
    TASKS.update(task_id, |t| {
        t.status = JobStatus::Running;
        t.progress = progress;
        t.message = message.into();
    });
}

pub fn complete(task_id: &str, result: serde_json::Value) {
    TASKS.update(task_id, |t| {
        t.status = JobStatus::Completed;
        t.progress = 100;
        t.result = Some(result);
    });
}

pub fn fail(task_id: &str, error: impl Into<String>) {
    TASKS.update(task_id, |t| {
        t.status = JobStatus::Failed;
        t.error = Some(error.into());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_lifecycle() {
        let id = create("graph_build", json!({"k": "v"}));
        let t = get(&id).unwrap();
        assert_eq!(t.status, JobStatus::Pending);
        assert_eq!(t.progress, 0);

        update(&id, 40, "working");
        let t = get(&id).unwrap();
        assert_eq!(t.status, JobStatus::Running);
        assert_eq!(t.progress, 40);
        assert_eq!(t.message, "working");

        complete(&id, json!({"graph_id": "g1"}));
        let t = get(&id).unwrap();
        assert_eq!(t.status, JobStatus::Completed);
        assert_eq!(t.progress, 100);
        assert_eq!(t.result.unwrap()["graph_id"], "g1");
    }

    #[test]
    fn fail_records_error() {
        let id = create("graph_build", json!({}));
        fail(&id, "boom");
        let t = get(&id).unwrap();
        assert_eq!(t.status, JobStatus::Failed);
        assert_eq!(t.error.as_deref(), Some("boom"));
    }

    #[test]
    fn unknown_task_is_none() {
        assert!(get("nope").is_none());
    }

    #[test]
    fn status_serializes_lowercase() {
        assert_eq!(serde_json::to_value(JobStatus::Running).unwrap(), "running");
    }
}
