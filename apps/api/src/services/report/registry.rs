//! In-process report registry, backed by one `report.json` per simulation.
//! Live progress stays in the map; a report is written to disk once it reaches
//! a terminal state and read back at startup, so a finished report survives a
//! restart instead of costing a fresh LLM run to regenerate.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::services::registry::{JobStatus, Registry};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    pub report_id: String,
    pub simulation_id: String,
    pub topic: String,
    pub status: JobStatus,
    pub progress: i32,
    pub message: String,
    pub sections: Vec<ReportSection>,
    pub markdown_content: String,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    // Not persisted. db_path is recomputed on load from the current data dir;
    // the two logs are live generation telemetry, so a restored report shows
    // its content with empty log panels.
    // ponytail: persist the logs too if replaying a past run's trace matters.
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
            status: JobStatus::Pending,
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

static REPORTS: Registry<ReportEntry> = Registry::new();

pub fn insert(entry: ReportEntry) {
    REPORTS.insert(entry.report_id.clone(), entry);
}

pub fn get(report_id: &str) -> Option<ReportEntry> {
    REPORTS.get(report_id)
}

pub fn find_by_simulation(simulation_id: &str) -> Option<ReportEntry> {
    REPORTS
        .values()
        .into_iter()
        .filter(|e| e.simulation_id == simulation_id)
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
}

pub fn list(simulation_id: Option<&str>, limit: usize) -> Vec<ReportEntry> {
    let mut entries: Vec<ReportEntry> = REPORTS
        .values()
        .into_iter()
        .filter(|e| simulation_id.is_none_or(|s| e.simulation_id == s))
        .collect();
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    entries.truncate(limit);
    entries
}

pub fn update<F: FnOnce(&mut ReportEntry)>(report_id: &str, f: F) -> bool {
    REPORTS.update(report_id, f)
}

/// Write a terminal report to `path`. In-flight reports are skipped: a
/// half-generated draft isn't resumable, so persisting it would only leave a
/// permanently "running" report after a crash.
pub fn persist(report_id: &str, path: &Path) {
    let Some(entry) = get(report_id) else { return };
    if !matches!(entry.status, JobStatus::Completed | JobStatus::Failed) {
        return;
    }
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(&entry)?)
    };
    if let Err(e) = write() {
        tracing::warn!("could not persist report {report_id} to {}: {e}", path.display());
    }
}

/// Read a persisted report back into the registry. `db_path` is supplied by the
/// caller from the live data dir rather than trusted from the file, so moving
/// the data dir doesn't restore reports pointing at paths that no longer exist.
pub fn restore(path: &Path, db_path: String) -> Option<String> {
    let raw = std::fs::read(path).ok()?;
    let mut entry: ReportEntry = serde_json::from_slice(&raw)
        .map_err(|e| tracing::warn!("ignoring unreadable report {}: {e}", path.display()))
        .ok()?;
    entry.db_path = db_path;
    let id = entry.report_id.clone();
    insert(entry);
    Some(id)
}

pub fn set_progress(report_id: &str, status: JobStatus, progress: i32, message: &str) {
    update(report_id, |e| {
        e.status = status;
        e.progress = progress;
        e.message = message.to_string();
    });
    console_log(report_id, "INFO", message);
}

/// Structured agent-log entry (JSONL shape).
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

        set_progress("report_test1", JobStatus::Running, 42, "working");
        agent_log("report_test1", "tool_call", "generating", json!({"tool": "statistics"}));

        let e = get("report_test1").unwrap();
        assert_eq!(e.status, JobStatus::Running);
        assert_eq!(e.progress, 42);
        assert_eq!(e.console_log.len(), 1);
        assert_eq!(e.agent_log.len(), 1);
        assert_eq!(e.agent_log[0]["action"], "tool_call");

        assert!(find_by_simulation("sim_a").is_some());
        assert!(find_by_simulation("sim_missing").is_none());
        assert!(!list(Some("sim_a"), 10).is_empty());
    }

    #[test]
    fn persists_only_terminal_reports_and_restores_them() {
        let dir = std::env::temp_dir().join("popinion_report_persist_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("report.json");

        insert(ReportEntry::new("report_p1".into(), "sim_p".into(), "/old/social.db".into(), "topic".into()));

        // Still running: nothing on disk, so a crash can't strand a "running" report.
        set_progress("report_p1", JobStatus::Running, 50, "half done");
        persist("report_p1", &path);
        assert!(!path.exists());

        update("report_p1", |r| {
            r.status = JobStatus::Completed;
            r.markdown_content = "## Findings\nbody".into();
            r.sections = vec![ReportSection { title: "Findings".into(), content: "body".into() }];
        });
        persist("report_p1", &path);
        assert!(path.exists());

        // db_path comes from the caller, not the file - the stale one is dropped.
        let id = restore(&path, "/new/social.db".into()).unwrap();
        let restored = get(&id).unwrap();
        assert_eq!(restored.status, JobStatus::Completed);
        assert_eq!(restored.markdown_content, "## Findings\nbody");
        assert_eq!(restored.sections.len(), 1);
        assert_eq!(restored.db_path, "/new/social.db");
        assert_eq!(find_by_simulation("sim_p").unwrap().report_id, "report_p1");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
