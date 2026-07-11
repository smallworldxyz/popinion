//! Knowledge-graph endpoints.

use crate::error::{AppError, AppResult, Success};
use crate::models::CrawlResult;
use crate::services::crawler::{self, telegram};
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
) -> AppResult<Success<Value>> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut simulation_requirement = String::new();
    let mut additional_context = String::new();
    let mut project_name = "Unnamed Project".to_string();
    let mut telegram_channel = String::new();
    let mut telegram_max_posts: usize = 50;

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
            "telegram_channel" => {
                telegram_channel = telegram::normalize_channel(&read_text(field).await?);
            }
            "telegram_max_posts" => {
                if let Ok(n) = read_text(field).await?.trim().parse::<usize>() {
                    telegram_max_posts = n.clamp(1, 200);
                }
            }
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
    if files.is_empty() && telegram_channel.is_empty() {
        return Err(AppError::BadRequest(
            "At least one file or a Telegram channel is required".into(),
        ));
    }
    if simulation_requirement.is_empty() {
        return Err(AppError::BadRequest("Simulation requirement is required".into()));
    }

    let mut sources: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();

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

    let mut document_texts: Vec<String> =
        document_texts.into_iter().filter(|t| !t.trim().is_empty()).collect();

    // Crawl the Telegram channel (if any) with the existing crawler and fold
    // its posts into the same corpus the uploaded files feed. A failed crawl
    // doesn't sink the build as long as other sources yielded text — the
    // outcome is reported honestly in the `crawl` field either way.
    let mut crawl_info: Option<Value> = None;
    if !telegram_channel.is_empty() {
        let result = telegram::crawl_channel(&telegram_channel, telegram_max_posts).await;
        let (doc, info) = crawl_to_source(&result, &telegram_channel);
        if let Some((text, label)) = doc {
            document_texts.push(text);
            sources.push(label);
        }
        crawl_info = Some(info);
    }

    if document_texts.is_empty() {
        let crawl_err = crawl_info
            .as_ref()
            .and_then(|c| c["error"].as_str())
            .unwrap_or("no text posts found");
        return Err(AppError::BadRequest(if telegram_channel.is_empty() {
            "No valid text extracted from uploaded files".to_string()
        } else if sources.is_empty() {
            format!("Telegram crawl of @{telegram_channel} failed: {crawl_err}")
        } else {
            format!(
                "No valid text extracted from uploaded files, and the Telegram \
                 crawl of @{telegram_channel} failed: {crawl_err}"
            )
        }));
    }

    let additional_context = additional_context.trim().to_string();
    let mut project = projects::create(&project_name)?;
    project.simulation_requirement = Some(simulation_requirement.clone());
    project.additional_context =
        (!additional_context.is_empty()).then(|| additional_context.clone());
    project.sources = sources;

    let all_text = document_texts.join("\n\n");
    project.total_text_length = Some(all_text.chars().count());
    projects::save_extracted_text(&project.project_id, &all_text)?;

    let generated = ontology::generate(
        &st.llm(),
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

    Ok(Success(json!({
        "project_id": project.project_id,
        "ontology": project.ontology,
        "analysis_summary": project.analysis_summary,
        "total_text_length": project.total_text_length,
        "sources": project.sources,
        "crawl": crawl_info,
    })))
}

/// Fold a crawl result into the build corpus: the document + source label to
/// append (None when the channel yielded no text posts), plus honest status
/// for the response. Pure, so the merge is testable without a live crawl.
fn crawl_to_source(result: &CrawlResult, channel: &str) -> (Option<(String, String)>, Value) {
    let text_posts = result
        .posts
        .iter()
        .filter(|p| !p.content.trim().is_empty())
        .count();
    let error = result.error.clone().or_else(|| {
        (text_posts == 0).then(|| format!("channel @{channel} yielded no text posts"))
    });
    let info = json!({
        "channel": channel,
        "posts_count": text_posts,
        "error": error,
    });
    let doc = (text_posts > 0).then(|| {
        (
            crawler::corpus_document(result),
            format!("Telegram: @{channel} ({text_posts} posts)"),
        )
    });
    (doc, info)
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
) -> AppResult<Success<Value>> {
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
        st.llm(),
        builder::BuildParams {
            project_id,
            text,
            ontology,
            graph_name: format!("{} - {}", project.name, graph_name),
            chunk_size: req.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(1),
            chunk_overlap: req.chunk_overlap.unwrap_or(DEFAULT_CHUNK_OVERLAP),
        },
    );

    Ok(Success(json!({ "task_id": task_id })))
}

// ============== Task Status ==============

async fn get_task_status(Path(task_id): Path<String>) -> AppResult<Success<Value>> {
    let task = tasks::get(&task_id)
        .ok_or_else(|| AppError::NotFound(format!("Task does not exist: {task_id}")))?;
    Ok(Success(json!(task)))
}

// ============== Graph Data ==============

async fn get_graph_data(
    State(st): State<AppState>,
    Path(graph_id): Path<String>,
) -> AppResult<Success<Value>> {
    let data = db::get_graph_data(st.graph(), &graph_id).await?;
    Ok(Success(json!(data)))
}

async fn delete_graph(
    State(st): State<AppState>,
    Path(graph_id): Path<String>,
) -> AppResult<Success<Value>> {
    db::delete_graph(st.graph(), &graph_id).await?;
    Ok(Success(json!({ "message": "Graph deleted successfully" })))
}

// ============== Projects ==============

async fn list_projects() -> AppResult<Success<Value>> {
    let projects = projects::list(50);
    Ok(Success(json!({ "total": projects.len(), "projects": projects })))
}

async fn get_project(Path(project_id): Path<String>) -> AppResult<Success<Value>> {
    let project = projects::get(&project_id)
        .ok_or_else(|| AppError::NotFound(format!("Project does not exist: {project_id}")))?;
    Ok(Success(json!(project)))
}

async fn delete_project(Path(project_id): Path<String>) -> AppResult<Success<Value>> {
    if !projects::delete(&project_id) {
        return Err(AppError::NotFound(format!("Project does not exist: {project_id}")));
    }
    Ok(Success(json!({ "message": "Project deleted successfully" })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ScrapedPost;

    fn fake_result(posts: Vec<&str>, error: Option<&str>) -> CrawlResult {
        CrawlResult {
            platform: "telegram".into(),
            query: Some("chan".into()),
            posts: posts
                .into_iter()
                .enumerate()
                .map(|(i, content)| ScrapedPost {
                    platform: "telegram".into(),
                    post_id: i.to_string(),
                    content: content.into(),
                    author_id: "chan".into(),
                    author_name: "chan".into(),
                    timestamp: chrono::Utc::now(),
                    url: None,
                    likes: 0,
                    shares: 0,
                    comments: 0,
                    views: 0,
                    media_urls: vec![],
                    hashtags: vec![],
                    mentions: vec![],
                })
                .collect(),
            users: vec![],
            crawled_at: chrono::Utc::now(),
            success: error.is_none(),
            error: error.map(String::from),
        }
    }

    #[test]
    fn crawled_posts_become_a_corpus_document_with_source_label() {
        let result = fake_result(vec!["Subsidy ends", "Prices up 12%"], None);
        let (doc, info) = crawl_to_source(&result, "chan");
        let (text, label) = doc.expect("posts should yield a document");
        assert!(text.contains("Subsidy ends"));
        assert!(text.contains("Prices up 12%"));
        assert_eq!(label, "Telegram: @chan (2 posts)");
        assert_eq!(info["posts_count"], 2);
        assert!(info["error"].is_null());
    }

    #[test]
    fn failed_or_empty_crawl_yields_no_document_but_honest_status() {
        let (doc, info) = crawl_to_source(&fake_result(vec![], Some("fetch failed: 404")), "chan");
        assert!(doc.is_none());
        assert_eq!(info["error"], "fetch failed: 404");

        // Media-only posts (no text) must not silently ground a graph.
        let (doc, info) = crawl_to_source(&fake_result(vec!["  ", ""], None), "chan");
        assert!(doc.is_none());
        assert_eq!(info["posts_count"], 0);
        assert!(info["error"].as_str().unwrap().contains("no text posts"));
    }
}
