//! Project persistence.
//! One directory per project under ./uploads/projects/<project_id>/ with
//! metadata.json + extracted_text.txt — a layout the simulation/report
//! subsystems can share.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROJECTS_DIR: &str = "./uploads/projects";

pub mod status {
    pub const CREATED: &str = "created";
    pub const ONTOLOGY_GENERATED: &str = "ontology_generated";
    pub const GRAPH_BUILT: &str = "graph_built";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub project_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub ontology: Option<serde_json::Value>,
    #[serde(default)]
    pub analysis_summary: Option<String>,
    #[serde(default)]
    pub total_text_length: Option<usize>,
    #[serde(default)]
    pub graph_id: Option<String>,
    #[serde(default)]
    pub node_count: Option<i64>,
    #[serde(default)]
    pub edge_count: Option<i64>,
    #[serde(default)]
    pub simulation_requirement: Option<String>,
    #[serde(default)]
    pub additional_context: Option<String>,
    #[serde(default)]
    pub simulation_id: Option<String>,
    #[serde(default)]
    pub simulation_config: Option<serde_json::Value>,
    #[serde(default)]
    pub files: Vec<serde_json::Value>,
    /// Human-readable reality-seed labels, e.g. "report.pdf" or
    /// "Telegram: @channel (143 posts)".
    #[serde(default)]
    pub sources: Vec<String>,
}

fn project_dir(project_id: &str) -> PathBuf {
    PathBuf::from(PROJECTS_DIR).join(project_id)
}

fn metadata_path(project_id: &str) -> PathBuf {
    project_dir(project_id).join("metadata.json")
}

fn text_path(project_id: &str) -> PathBuf {
    project_dir(project_id).join("extracted_text.txt")
}

pub fn create(name: &str) -> Result<Project> {
    let now = chrono::Utc::now().to_rfc3339();
    let project = Project {
        project_id: format!("proj_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]),
        name: name.to_string(),
        status: status::CREATED.to_string(),
        created_at: now.clone(),
        updated_at: now,
        ontology: None,
        analysis_summary: None,
        total_text_length: None,
        graph_id: None,
        node_count: None,
        edge_count: None,
        simulation_requirement: None,
        additional_context: None,
        simulation_id: None,
        simulation_config: None,
        files: vec![],
        sources: vec![],
    };
    save(&project)?;
    Ok(project)
}

pub fn save(project: &Project) -> Result<()> {
    let mut project = project.clone();
    project.updated_at = chrono::Utc::now().to_rfc3339();
    let path = metadata_path(&project.project_id);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(&project)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn get(project_id: &str) -> Option<Project> {
    let raw = std::fs::read_to_string(metadata_path(project_id)).ok()?;
    match serde_json::from_str(&raw) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!("corrupt project metadata for {project_id}: {e}");
            None
        }
    }
}

pub fn list(limit: usize) -> Vec<Project> {
    let Ok(entries) = std::fs::read_dir(PROJECTS_DIR) else {
        return vec![];
    };
    let mut projects: Vec<Project> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| get(&e.file_name().to_string_lossy()))
        .collect();
    projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    projects.truncate(limit);
    projects
}

pub fn delete(project_id: &str) -> bool {
    let dir = project_dir(project_id);
    if !dir.exists() {
        return false;
    }
    std::fs::remove_dir_all(dir).is_ok()
}

pub fn save_extracted_text(project_id: &str, text: &str) -> Result<()> {
    let path = text_path(project_id);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn get_extracted_text(project_id: &str) -> Option<String> {
    std::fs::read_to_string(text_path(project_id)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests write under ./uploads/projects relative to the crate dir
    // (same location the server uses) and clean up after themselves.
    #[test]
    fn create_save_get_delete_roundtrip() {
        let mut p = create("Test Project").unwrap();
        let id = p.project_id.clone();
        assert!(id.starts_with("proj_"));

        p.status = status::ONTOLOGY_GENERATED.to_string();
        p.analysis_summary = Some("summary".into());
        save(&p).unwrap();

        let loaded = get(&id).unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.status, status::ONTOLOGY_GENERATED);
        assert_eq!(loaded.analysis_summary.as_deref(), Some("summary"));

        save_extracted_text(&id, "some text").unwrap();
        assert_eq!(get_extracted_text(&id).as_deref(), Some("some text"));

        assert!(list(50).iter().any(|p| p.project_id == id));

        assert!(delete(&id));
        assert!(get(&id).is_none());
        assert!(!delete(&id));
    }
}
