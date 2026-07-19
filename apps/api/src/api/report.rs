use crate::error::{AppError, AppResult, Success};
use crate::services::registry::JobStatus;
use crate::services::report::registry;
use crate::services::report::agent;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

/// Endpoints consumed by the frontend (frontend/src/api/report.js + views):
///   POST /generate  /generate/status  /chat
///   GET  /{report_id}  /{report_id}/agent-log  /{report_id}/console-log
///   GET  /list  /by-simulation/{simulation_id}  /check/{simulation_id}
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/generate", post(generate))
        .route("/generate/status", post(generate_status))
        .route("/chat", post(chat))
        .route("/list", get(list_reports))
        .route("/by-simulation/:simulation_id", get(by_simulation))
        .route("/check/:simulation_id", get(check_status))
        .route("/:report_id", get(get_report))
        .route("/:report_id/agent-log", get(agent_log))
        .route("/:report_id/console-log", get(console_log))
}

#[derive(Deserialize)]
struct GenerateReq {
    simulation_id: Option<String>,
    #[serde(alias = "simulation_requirement")]
    topic: Option<String>,
    #[serde(default)]
    force_regenerate: bool,
    /// Sibling runs of the same prepared population, for the noise floor.
    /// Without them the report can only say the wobble is unmeasured.
    #[serde(default)]
    rerun_ids: Vec<String>,
}

async fn generate(
    State(st): State<AppState>,
    Json(req): Json<GenerateReq>,
) -> AppResult<Success<Value>> {
    let simulation_id = req.simulation_id.clone().unwrap_or_default();
    if simulation_id.is_empty() {
        return Err(AppError::BadRequest("Please provide simulation_id".into()));
    }
    // The DB path is resolved from the sim id via the Manager only — never from
    // a caller-supplied path (that let a request open/mutate any SQLite file).
    if simulation_id.contains('/') || simulation_id.contains('\\') || simulation_id.contains("..") {
        return Err(AppError::BadRequest("invalid simulation_id".into()));
    }

    if !req.force_regenerate {
        if let Some(existing) = registry::find_by_simulation(&simulation_id) {
            if existing.status == JobStatus::Completed {
                return Ok(Success(json!({
                    "simulation_id": simulation_id,
                    "report_id": existing.report_id,
                    "status": "completed",
                    "message": "Report already exists",
                    "already_generated": true,
                })));
            }
        }
    }

    let db_path = st.sim_manager().db_path(&simulation_id).to_string_lossy().into_owned();
    if !std::path::Path::new(&db_path).exists() {
        return Err(AppError::BadRequest(format!("Simulation database not found for {simulation_id}")));
    }

    let topic = req.topic.unwrap_or_default();
    let report_id = agent::start(&st, simulation_id.clone(), db_path, topic, req.rerun_ids);

    Ok(Success(json!({
        "simulation_id": simulation_id,
        "report_id": report_id,
        "status": "running",
        "message": "Report generation task started, query progress via /api/report/generate/status",
        "already_generated": false,
    })))
}

#[derive(Deserialize)]
struct StatusReq {
    report_id: Option<String>,
    simulation_id: Option<String>,
}

async fn generate_status(Json(req): Json<StatusReq>) -> AppResult<Success<Value>> {
    let entry = match (&req.report_id, &req.simulation_id) {
        (Some(id), _) => registry::get(id),
        (None, Some(sim)) => registry::find_by_simulation(sim),
        (None, None) => {
            return Err(AppError::BadRequest("Please provide report_id or simulation_id".into()))
        }
    }
    .ok_or_else(|| AppError::NotFound("Report does not exist".into()))?;

    Ok(Success(json!({
        "report_id": entry.report_id,
        "simulation_id": entry.simulation_id,
        "status": entry.status,
        "progress": entry.progress,
        "message": entry.message,
        "error": entry.error,
        "already_completed": entry.status == JobStatus::Completed,
    })))
}

async fn get_report(Path(report_id): Path<String>) -> AppResult<Success<Value>> {
    let entry = registry::get(&report_id)
        .ok_or_else(|| AppError::NotFound(format!("Report does not exist: {report_id}")))?;
    Ok(Success(json!(entry)))
}

#[derive(Deserialize)]
struct ListQuery {
    simulation_id: Option<String>,
    limit: Option<usize>,
}

async fn list_reports(Query(q): Query<ListQuery>) -> AppResult<Success<Value>> {
    let reports = registry::list(q.simulation_id.as_deref(), q.limit.unwrap_or(50));
    Ok(Success(json!(reports)))
}

async fn by_simulation(Path(simulation_id): Path<String>) -> AppResult<Success<Value>> {
    let entry = registry::find_by_simulation(&simulation_id)
        .ok_or_else(|| AppError::NotFound(format!("No report for this simulation: {simulation_id}")))?;
    Ok(Success(json!(entry)))
}

async fn check_status(Path(simulation_id): Path<String>) -> AppResult<Success<Value>> {
    let entry = registry::find_by_simulation(&simulation_id);
    Ok(Success(json!({
        "simulation_id": simulation_id,
        "has_report": entry.is_some(),
        "report_status": entry.as_ref().map(|e| e.status),
        "report_id": entry.as_ref().map(|e| e.report_id.clone()),
        "interview_unlocked": entry.is_some_and(|e| e.status == JobStatus::Completed),
    })))
}

#[derive(Deserialize)]
struct LogQuery {
    #[serde(default)]
    from_line: usize,
}

async fn agent_log(
    Path(report_id): Path<String>,
    Query(q): Query<LogQuery>,
) -> AppResult<Success<Value>> {
    let entry = registry::get(&report_id)
        .ok_or_else(|| AppError::NotFound(format!("Report does not exist: {report_id}")))?;
    Ok(Success(paged_logs(&entry.agent_log, q.from_line)))
}

async fn console_log(
    Path(report_id): Path<String>,
    Query(q): Query<LogQuery>,
) -> AppResult<Success<Value>> {
    let entry = registry::get(&report_id)
        .ok_or_else(|| AppError::NotFound(format!("Report does not exist: {report_id}")))?;
    let lines: Vec<Value> = entry.console_log.iter().map(|l| json!(l)).collect();
    Ok(Success(paged_logs(&lines, q.from_line)))
}

fn paged_logs(logs: &[Value], from_line: usize) -> Value {
    let total = logs.len();
    let slice: Vec<&Value> = logs.iter().skip(from_line).collect();
    json!({
        "logs": slice,
        "total_lines": total,
        "from_line": from_line,
        "has_more": false,
    })
}

#[derive(Deserialize)]
struct ChatReq {
    report_id: Option<String>,
    simulation_id: Option<String>,
    message: String,
    #[serde(default)]
    chat_history: Vec<Value>,
}

async fn chat(
    State(st): State<AppState>,
    Json(req): Json<ChatReq>,
) -> AppResult<Success<Value>> {
    if req.message.trim().is_empty() {
        return Err(AppError::BadRequest("Please provide message".into()));
    }
    let entry = match (&req.report_id, &req.simulation_id) {
        (Some(id), _) => registry::get(id),
        (None, Some(sim)) => registry::find_by_simulation(sim),
        (None, None) => {
            return Err(AppError::BadRequest("Please provide report_id or simulation_id".into()))
        }
    }
    .ok_or_else(|| AppError::NotFound("Report does not exist".into()))?;

    let result = agent::chat(&st, &entry, &req.message, &req.chat_history).await?;
    Ok(Success(result))
}
