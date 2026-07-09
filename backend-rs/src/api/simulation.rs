use crate::error::{AppError, AppResult};
use crate::models::Success;
use crate::services::report::{
    self, AgentInterviewer, PanelChatOptions, Panelist,
};
use crate::sim::agent::AgentProfile;
use crate::sim::config::SimConfig;
use crate::sim::SimHandle;
use crate::state::AppState;
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Bridges the report subsystem's `AgentInterviewer` seam to a live engine task.
struct LiveInterviewer(SimHandle);

#[async_trait]
impl AgentInterviewer for LiveInterviewer {
    async fn interview(&self, agent_id: i64, prompt: &str) -> anyhow::Result<String> {
        self.0.interview_agent(agent_id, prompt.to_string()).await
    }
}

/// Panelists default to the first N personas of the sim when the request omits them.
fn default_panelists(st: &AppState, sim_id: &str, n: usize) -> AppResult<Vec<Panelist>> {
    let path = std::path::Path::new(&st.cfg.sim_data_dir).join(sim_id).join("profiles.json");
    let raw = std::fs::read(path).map_err(|_| AppError::NotFound(format!("profiles for {sim_id}")))?;
    let profiles: Vec<AgentProfile> = serde_json::from_slice(&raw).map_err(|e| AppError::Other(e.into()))?;
    Ok(profiles
        .into_iter()
        .take(n)
        .map(|p| Panelist { agent_id: p.user_id, name: p.name, faction: String::new(), platform: String::new() })
        .collect())
}

/// Simulation lifecycle + read API. The in-process engine (crate::sim::engine)
/// is driven through crate::sim::manager::Manager; reads open the sim's SQLite
/// store directly. panel-chat/survey (report subsystem) are mounted here too.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/prepare", post(prepare))
        .route("/prepare/preview", post(prepare_preview))
        .route("/start", post(start))
        .route("/stop", post(stop))
        .route("/close-env", post(stop))
        .route("/env-status", post(env_status))
        .route("/list", get(list))
        .route("/interview/batch", post(interview_batch))
        .route("/panel-chat", post(panel_chat_h))
        .route("/survey/create", post(survey_create_h))
        .route("/survey/deploy", post(survey_deploy_h))
        .route("/survey/list", get(survey_list_h))
        .route("/survey/:survey_id", get(survey_get_h))
        .route("/:id", get(get_sim))
        .route("/:id/config", get(get_config))
        .route("/:id/profiles", get(get_profiles))
        .route("/:id/run-status", get(run_status))
        .route("/:id/run-status/detail", get(run_status_detail))
        .route("/:id/posts", get(posts))
        .route("/:id/comments", get(comments))
        .route("/:id/timeline", get(timeline))
        .route("/:id/actions", get(actions))
        .route("/:id/agent-stats", get(agent_stats))
        .route("/:id/stance", get(stance))
}

#[derive(Deserialize)]
struct CreateReq {
    #[serde(default = "default_name")]
    name: String,
    #[serde(default)]
    profiles: Vec<AgentProfile>,
    #[serde(default)]
    config: Option<SimConfig>,
    #[serde(default)]
    initial_posts: Vec<crate::sim::config::InitialPost>,
}
fn default_name() -> String {
    "Untitled Simulation".into()
}

async fn create(State(st): State<AppState>, Json(req): Json<CreateReq>) -> AppResult<Success<Value>> {
    if req.profiles.is_empty() {
        return Err(AppError::BadRequest("no agent profiles provided".into()));
    }
    let mut config = req.config.unwrap_or_default();
    if !req.initial_posts.is_empty() {
        config.event_config.initial_posts = req.initial_posts;
    }
    let id = st.sim_manager().create(&req.name, req.profiles, config).map_err(AppError::Other)?;
    Ok(Success(json!({ "simulation_id": id })))
}

// Graph-grounded persona/config generation lands with the persona service.
async fn prepare(State(_st): State<AppState>, Json(_b): Json<Value>) -> AppResult<Success<Value>> {
    Err(AppError::NotImplemented(
        "prepare (graph-grounded persona generation) is wired after the persona service; use /create with profiles for now".into(),
    ))
}
async fn prepare_preview(State(_st): State<AppState>, Json(_b): Json<Value>) -> AppResult<Success<Value>> {
    Err(AppError::NotImplemented("prepare/preview pending persona service".into()))
}

#[derive(Deserialize)]
struct StartReq {
    simulation_id: String,
    #[serde(default)]
    max_rounds: Option<u32>,
}
async fn start(State(st): State<AppState>, Json(req): Json<StartReq>) -> AppResult<Success<Value>> {
    st.sim_manager().start(&req.simulation_id, req.max_rounds).map_err(AppError::Other)?;
    Ok(Success(json!({ "simulation_id": req.simulation_id, "status": "running" })))
}

#[derive(Deserialize)]
struct StopReq {
    simulation_id: String,
}
async fn stop(State(st): State<AppState>, Json(req): Json<StopReq>) -> AppResult<Success<Value>> {
    st.sim_manager().stop(&req.simulation_id).await.map_err(AppError::Other)?;
    Ok(Success(json!({ "simulation_id": req.simulation_id, "status": "stopped" })))
}

async fn env_status(State(st): State<AppState>, Json(req): Json<StopReq>) -> AppResult<Success<Value>> {
    let status = st.sims.get(&req.simulation_id).map(|h| h.status()).unwrap_or_else(|| "not_running".into());
    Ok(Success(json!({ "simulation_id": req.simulation_id, "status": status })))
}

async fn list(State(st): State<AppState>) -> AppResult<Success<Value>> {
    Ok(Success(json!({ "simulations": st.sim_manager().list() })))
}

async fn get_sim(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let m = st.sim_manager().meta(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(serde_json::to_value(m).unwrap()))
}

async fn get_config(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let path = std::path::Path::new(&st.cfg.sim_data_dir).join(&id).join("config.json");
    let raw = std::fs::read(path).map_err(|_| AppError::NotFound(format!("config for {id}")))?;
    let cfg: Value = serde_json::from_slice(&raw).map_err(|e| AppError::Other(e.into()))?;
    Ok(Success(json!({ "config": cfg })))
}

async fn get_profiles(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let path = std::path::Path::new(&st.cfg.sim_data_dir).join(&id).join("profiles.json");
    let raw = std::fs::read(path).map_err(|_| AppError::NotFound(format!("profiles for {id}")))?;
    let profiles: Value = serde_json::from_slice(&raw).map_err(|e| AppError::Other(e.into()))?;
    Ok(Success(json!({ "profiles": profiles })))
}

async fn run_status(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let status = st.sims.get(&id).map(|h| h.status()).unwrap_or_else(|| "not_running".into());
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    let posts = store.count_posts().unwrap_or(0);
    Ok(Success(json!({ "simulation_id": id, "status": status, "post_count": posts })))
}

async fn run_status_detail(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let mgr = st.sim_manager();
    let status = st.sims.get(&id).map(|h| h.status()).unwrap_or_else(|| "not_running".into());
    let store = mgr.store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({
        "simulation_id": id,
        "status": status,
        "post_count": store.count_posts().unwrap_or(0),
        "timeline": store.timeline().unwrap_or_default(),
        "stance": store.stance_distribution().unwrap_or(Value::Null),
    })))
}

#[derive(Deserialize)]
struct Page {
    #[serde(default = "d_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}
fn d_limit() -> i64 {
    50
}

async fn posts(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(p): Query<Page>,
) -> AppResult<Success<Value>> {
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({
        "posts": store.list_posts(p.limit, p.offset)?,
        "total": store.count_posts()?,
    })))
}

async fn comments(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Success<Value>> {
    let post_id: i64 = q
        .get("post_id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::BadRequest("post_id query param required".into()))?;
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({ "comments": store.list_comments(post_id)? })))
}

async fn timeline(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({ "timeline": store.timeline()? })))
}

async fn actions(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(p): Query<Page>,
) -> AppResult<Success<Value>> {
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({ "actions": store.list_actions(p.limit)? })))
}

async fn agent_stats(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({ "agents": store.agent_stats()? })))
}

async fn stance(State(st): State<AppState>, Path(id): Path<String>) -> AppResult<Success<Value>> {
    let store = st.sim_manager().store(&id).map_err(|e| AppError::NotFound(e.to_string()))?;
    Ok(Success(json!({ "stance_distribution": store.stance_distribution()? })))
}

#[derive(Deserialize)]
struct InterviewBatchReq {
    simulation_id: String,
    interviews: Vec<InterviewItem>,
}
#[derive(Deserialize)]
struct InterviewItem {
    agent_id: i64,
    prompt: String,
}

async fn interview_batch(
    State(st): State<AppState>,
    Json(req): Json<InterviewBatchReq>,
) -> AppResult<Success<Value>> {
    let handle = st
        .sims
        .get(&req.simulation_id)
        .ok_or_else(|| AppError::BadRequest("simulation not running (interview needs a live env)".into()))?;
    let mut results = Vec::new();
    for item in req.interviews {
        let resp = handle.interview_agent(item.agent_id, item.prompt.clone()).await;
        results.push(json!({
            "agent_id": item.agent_id,
            "prompt": item.prompt,
            "response": resp.as_ref().ok(),
            "error": resp.as_ref().err().map(|e| e.to_string()),
        }));
    }
    Ok(Success(json!({ "results": results })))
}

// ---- deliberation (report subsystem, mounted here since the frontend calls
//      them under /api/simulation) ----

#[derive(Deserialize)]
struct PanelChatReq {
    simulation_id: String,
    question: String,
    #[serde(default)]
    panelists: Vec<Panelist>,
    #[serde(default)]
    rounds: Option<u32>,
}

async fn panel_chat_h(State(st): State<AppState>, Json(req): Json<PanelChatReq>) -> AppResult<Success<Value>> {
    let handle = st
        .sims
        .get(&req.simulation_id)
        .ok_or_else(|| AppError::BadRequest("simulation not running (panel chat needs a live env)".into()))?;
    let panelists = if req.panelists.is_empty() {
        default_panelists(&st, &req.simulation_id, 12)?
    } else {
        req.panelists
    };
    let opts = PanelChatOptions { rounds: req.rounds.unwrap_or(1), ..Default::default() };
    let interviewer = LiveInterviewer(handle);
    let result = report::panel_chat(&st.llm, &interviewer, &req.question, &panelists, &opts)
        .await
        .map_err(AppError::Other)?;
    Ok(Success(json!({ "result": result })))
}

#[derive(Deserialize)]
struct SurveyCreateReq {
    title: String,
    #[serde(default)]
    description: String,
    questions: Vec<Value>,
}

async fn survey_create_h(Json(req): Json<SurveyCreateReq>) -> AppResult<Success<Value>> {
    let t = report::survey_create(&req.title, &req.description, &req.questions).map_err(AppError::Other)?;
    Ok(Success(json!({ "survey": t })))
}

#[derive(Deserialize)]
struct SurveyDeployReq {
    simulation_id: String,
    survey_id: String,
    #[serde(default)]
    respondents: Vec<Panelist>,
}

async fn survey_deploy_h(State(st): State<AppState>, Json(req): Json<SurveyDeployReq>) -> AppResult<Success<Value>> {
    let handle = st
        .sims
        .get(&req.simulation_id)
        .ok_or_else(|| AppError::BadRequest("simulation not running (survey needs a live env)".into()))?;
    let respondents = if req.respondents.is_empty() {
        default_panelists(&st, &req.simulation_id, 50)?
    } else {
        req.respondents
    };
    let interviewer = LiveInterviewer(handle);
    let result = report::survey_deploy(&interviewer, &req.survey_id, &respondents)
        .await
        .map_err(AppError::Other)?;
    Ok(Success(json!({ "result": result })))
}

async fn survey_list_h() -> AppResult<Success<Value>> {
    Ok(Success(json!({ "surveys": report::survey_list() })))
}

async fn survey_get_h(Path(survey_id): Path<String>) -> AppResult<Success<Value>> {
    report::survey_get(&survey_id)
        .map(|s| Success(json!({ "survey": s })))
        .ok_or_else(|| AppError::NotFound(format!("survey {survey_id}")))
}
