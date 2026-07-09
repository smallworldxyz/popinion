//! In-process report registry. Replaces the Python ReportManager's file-based
//! persistence (report.json / progress.json / agent_log.jsonl / console.log)
//! with a module-level map; reports live for the process lifetime.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportState {
    Pending,
    Generating,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
}

#[derive(Clone, Serialize)]
pub struct ReportEntry {
    pub report_id: String,
    pub simulation_id: String,
    pub topic: String,
    pub status: ReportState,
    pub progress: i32,
    pub message: String,
    pub sections: Vec<ReportSection>,
    pub markdown_content: String,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    #[serde(skip)]
    pub db_path: String,
    #[serde(skip)]
    pub agent_log: Vec<Value>,
    #[serde(skip)]
    pub console_log: Vec<String>,
    #[serde(skip)]
    pub started_at_ms: i64,
}

impl ReportEntry {
    pub fn new(report_id: String, simulation_id: String, db_path: String, topic: String) -> Self {
        ReportEntry {
            report_id,
            simulation_id,
            topic,
            status: ReportState::Pending,
            progress: 0,
            message: "Initializing report...".into(),
            sections: Vec::new(),
            markdown_content: String::new(),
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            db_path,
            agent_log: Vec::new(),
            console_log: Vec::new(),
            started_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

fn registry() -> &'static Mutex<HashMap<String, ReportEntry>> {
    static REPORTS: OnceLock<Mutex<HashMap<String, ReportEntry>>> = OnceLock::new();
    REPORTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn insert(entry: ReportEntry) {
    registry().lock().unwrap().insert(entry.report_id.clone(), entry);
}

pub fn get(report_id: &str) -> Option<ReportEntry> {
    registry().lock().unwrap().get(report_id).cloned()
}

pub fn find_by_simulation(simulation_id: &str) -> Option<ReportEntry> {
    registry()
        .lock()
        .unwrap()
        .values()
        .filter(|e| e.simulation_id == simulation_id)
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
        .cloned()
}

pub fn list(simulation_id: Option<&str>, limit: usize) -> Vec<ReportEntry> {
    let map = registry().lock().unwrap();
    let mut entries: Vec<ReportEntry> = map
        .values()
        .filter(|e| simulation_id.is_none_or(|s| e.simulation_id == s))
        .cloned()
        .collect();
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    entries.truncate(limit);
    entries
}

pub fn update<F: FnOnce(&mut ReportEntry)>(report_id: &str, f: F) -> bool {
    let mut map = registry().lock().unwrap();
    match map.get_mut(report_id) {
        Some(entry) => {
            f(entry);
            true
        }
        None => false,
    }
}

pub fn set_progress(report_id: &str, status: ReportState, progress: i32, message: &str) {
    update(report_id, |e| {
        e.status = status;
        e.progress = progress;
        e.message = message.to_string();
    });
    console_log(report_id, "INFO", message);
}

/// Structured agent-log entry (mirrors the Python ReportLogger JSONL shape).
pub fn agent_log(report_id: &str, action: &str, stage: &str, details: Value) {
    let now = chrono::Utc::now();
    update(report_id, |e| {
        let elapsed = (now.timestamp_millis() - e.started_at_ms) as f64 / 1000.0;
        e.agent_log.push(json!({
            "timestamp": now.to_rfc3339(),
            "elapsed_seconds": elapsed,
            "report_id": report_id,
            "action": action,
            "stage": stage,
            "details": details,
        }));
    });
}

/// Plain-text console-style log line.
pub fn console_log(report_id: &str, level: &str, msg: &str) {
    let line = format!("[{}] {level}: {msg}", chrono::Utc::now().format("%H:%M:%S"));
    update(report_id, |e| e.console_log.push(line));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_roundtrip() {
        let entry = ReportEntry::new("report_test1".into(), "sim_a".into(), "/tmp/x.db".into(), "topic".into());
        insert(entry);

        set_progress("report_test1", ReportState::Generating, 42, "working");
        agent_log("report_test1", "tool_call", "generating", json!({"tool": "statistics"}));

        let e = get("report_test1").unwrap();
        assert_eq!(e.status, ReportState::Generating);
        assert_eq!(e.progress, 42);
        assert_eq!(e.console_log.len(), 1);
        assert_eq!(e.agent_log.len(), 1);
        assert_eq!(e.agent_log[0]["action"], "tool_call");

        assert!(find_by_simulation("sim_a").is_some());
        assert!(find_by_simulation("sim_missing").is_none());
        assert!(!list(Some("sim_a"), 10).is_empty());
    }
}
