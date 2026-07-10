//! Knowledge-graph endpoints (port of backend/app/api/graph.py).

use crate::error::{AppError, AppResult};
use crate::models::Ok as Payload;
use crate::services::graph::{builder, db, file_parser, ontology, projects, tasks};
use crate::state::AppState;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

const DEFAULT_CHUNK_SIZE: usize = 500;
const DEFAULT_CHUNK_OVERLAP: usize = 50;
const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ontology/generate", post(generate_ontology))
        .route("/build", post(build_graph))
        .route("/task/:task_id", get(get_task_status))
        .route("/data/:graph_id", get(get_graph_data))
        .route("/delete/:graph_id", delete(delete_graph))
        .route("/projects", get(list_projects))
        .route("/project/:project_id", get(get_project).delete(delete_project))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

// ============== API 1: Generate Ontology ==============

async fn generate_ontology(
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Payload<Value>> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut simulation_requirement = String::new();
    let mut additional_context = String::new();
    let mut project_name = "Unnamed Project".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid multipart body: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "files" => {
                let filename = field.file_name().unwrap_or_default().to_string();
                if filename.is_empty() {
                    continue;
                }
                if !file_parser::is_allowed(&filename) {
                    return Err(AppError::BadRequest(format!(
                        "File type not supported: {filename}"
                    )));
                }
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("failed to read {filename}: {e}")))?;
                if bytes.is_empty() {
                    return Err(AppError::BadRequest(format!(
                        "File {filename} is empty or corrupted during upload"
                    )));
                }
                files.push((filename, bytes.to_vec()));
            }
            "simulation_requirement" => simulation_requirement = read_text(field).await?,
            "additional_context" => additional_context = read_text(field).await?,
            "project_name" => {
                let v = read_text(field).await?;
                if !v.trim().is_empty() {
                    project_name = v.trim().to_string();
                }
            }
            _ => {}
        }
    }

    let simulation_requirement = simulation_requirement.trim().to_string();
    if files.is_empty() {
        return Err(AppError::BadRequest("At least one file required".into()));
    }
    if simulation_requirement.is_empty() {
        return Err(AppError::BadRequest("Simulation requirement is required".into()));
    }

    // PDF parsing is CPU-bound; keep it off the async runtime.
    let document_texts: Vec<String> = tokio::task::spawn_blocking(move || {
        files
            .iter()
            .map(|(name, bytes)| {
                file_parser::extract_text(name, bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to parse file {name}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| AppError::Other(anyhow::anyhow!("file parsing task panicked: {e}")))??;

    let document_texts: Vec<String> =
        document_texts.into_iter().filter(|t| !t.trim().is_empty()).collect();
    if document_texts.is_empty() {
        return Err(AppError::BadRequest(
            "No valid text extracted from uploaded files".into(),
        ));
    }

    let additional_context = additional_context.trim().to_string();
    let mut project = projects::create(&project_name)?;
    project.simulation_requirement = Some(simulation_requirement.clone());
    project.additional_context =
        (!additional_context.is_empty()).then(|| additional_context.clone());

    let all_text = document_texts.join("\n\n");
    project.total_text_length = Some(all_text.chars().count());
    projects::save_extracted_text(&project.project_id, &all_text)?;

    let generated = ontology::generate(
        &st.llm,
        &document_texts,
        &simulation_requirement,
        (!additional_context.is_empty()).then_some(additional_context.as_str()),
    )
    .await?;

    project.ontology = Some(json!({
        "entity_types": generated["entity_types"],
        "edge_types": generated["edge_types"],
    }));
    project.analysis_summary = generated["analysis_summary"].as_str().map(String::from);
    project.status = projects::status::ONTOLOGY_GENERATED.to_string();
    projects::save(&project)?;

    Ok(Payload(json!({
        "data": {
            "project_id": project.project_id,
            "ontology": project.ontology,
            "analysis_summary": project.analysis_summary,
            "total_text_length": project.total_text_length,
        }
    })))
}

async fn read_text(field: axum::extract::multipart::Field<'_>) -> AppResult<String> {
    field
        .text()
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid form field: {e}")))
}

// ============== API 2: Build Graph ==============

#[derive(Deserialize)]
struct BuildRequest {
    project_id: String,
    #[serde(default)]
    graph_name: Option<String>,
    #[serde(default)]
    chunk_size: Option<usize>,
    #[serde(default)]
    chunk_overlap: Option<usize>,
}

async fn build_graph(
    State(st): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> AppResult<Payload<Value>> {
    let graph = st.graph().clone();

    let project_id = req.project_id.trim().to_string();
    if project_id.is_empty() {
        return Err(AppError::BadRequest("project_id is required".into()));
    }
    let project = projects::get(&project_id)
        .ok_or_else(|| AppError::NotFound(format!("Project does not exist: {project_id}")))?;
    let ontology = project.ontology.clone().ok_or_else(|| {
        AppError::BadRequest(
            "Project ontology not generated, please call /ontology/generate first".into(),
        )
    })?;
    let text = projects::get_extracted_text(&project_id).ok_or_else(|| {
        AppError::NotFound("Project text not found, file may have been deleted".into())
    })?;

    let graph_name = req
        .graph_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Popinion Graph".to_string());

    let task_id = builder::spawn_build(
        graph,
        st.llm.clone(),
        builder::BuildParams {
            project_id,
            text,
            ontology,
            graph_name: format!("{} - {}", project.name, graph_name),
            chunk_size: req.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(1),
            chunk_overlap: req.chunk_overlap.unwrap_or(DEFAULT_CHUNK_OVERLAP),
        },
    );

    Ok(Payload(json!({ "data": { "task_id": task_id } })))
}

// ============== Task Status ==============

async fn get_task_status(Path(task_id): Path<String>) -> AppResult<Payload<Value>> {
    let task = tasks::get(&task_id)
        .ok_or_else(|| AppError::NotFound(format!("Task does not exist: {task_id}")))?;
    Ok(Payload(json!({ "data": task })))
}

// ============== Graph Data ==============

async fn get_graph_data(
    State(st): State<AppState>,
    Path(graph_id): Path<String>,
) -> AppResult<Payload<Value>> {
    let data = db::get_graph_data(st.graph(), &graph_id).await?;
    Ok(Payload(json!({ "data": data })))
}

async fn delete_graph(
    State(st): State<AppState>,
    Path(graph_id): Path<String>,
) -> AppResult<Payload<Value>> {
    db::delete_graph(st.graph(), &graph_id).await?;
    Ok(Payload(json!({ "message": "Graph deleted successfully" })))
}

// ============== Projects ==============

async fn list_projects() -> AppResult<Payload<Value>> {
    let projects = projects::list(50);
    Ok(Payload(json!({
        "data": { "total": projects.len(), "projects": projects }
    })))
}

async fn get_project(Path(project_id): Path<String>) -> AppResult<Payload<Value>> {
    let project = projects::get(&project_id)
        .ok_or_else(|| AppError::NotFound(format!("Project does not exist: {project_id}")))?;
    Ok(Payload(json!({ "data": project })))
}

async fn delete_project(Path(project_id): Path<String>) -> AppResult<Payload<Value>> {
    if !projects::delete(&project_id) {
        return Err(AppError::NotFound(format!("Project does not exist: {project_id}")));
    }
    Ok(Payload(json!({ "message": "Project deleted successfully" })))
}
