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
        .route("/prepare/status", post(prepare_status))
        .route("/validate", post(validate))
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
    /// Wizard flow: create an empty sim tied to a graph, then /prepare it.
    #[serde(default)]
    graph_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
}
fn default_name() -> String {
    "Untitled Simulation".into()
}

async fn create(State(st): State<AppState>, Json(req): Json<CreateReq>) -> AppResult<Success<Value>> {
    // Personas may be supplied directly, or omitted when the sim is graph-linked
    // and will be populated by /prepare. Reject only the truly empty case.
    if req.profiles.is_empty() && req.graph_id.is_none() && req.project_id.is_none() {
        return Err(AppError::BadRequest(
            "provide agent profiles, or a graph_id/project_id to prepare personas from".into(),
        ));
    }
    let mut config = req.config.unwrap_or_default();
    if !req.initial_posts.is_empty() {
        config.event_config.initial_posts = req.initial_posts;
    }
    let id = st
        .sim_manager()
        .create(&req.name, req.profiles, config, req.graph_id, req.project_id)
        .map_err(AppError::Other)?;
    Ok(Success(json!({ "simulation_id": id })))
}

// ---- graph-grounded persona generation (the God's-eye → simulation link) ----

use crate::services::graph::{neo4j, projects, tasks};
use crate::sim::persona;

/// Which knowledge graph does this sim draw its population from?
fn resolve_graph_id(st: &AppState, sim_id: &str) -> AppResult<String> {
    let meta = st
        .sim_manager()
        .meta(sim_id)
        .map_err(|_| AppError::NotFound(format!("simulation {sim_id}")))?;
    if let Some(g) = meta.graph_id {
        return Ok(g);
    }
    if let Some(pid) = meta.project_id {
        if let Some(g) = projects::get(&pid).and_then(|p| p.graph_id) {
            return Ok(g);
        }
    }
    Err(AppError::BadRequest(format!("simulation {sim_id} has no graph to prepare from")))
}

#[derive(Deserialize)]
struct PreparePreviewReq {
    simulation_id: String,
    #[serde(default)]
    min_evidence: Option<usize>,
}

/// Entities grouped by type with evidence scores, so the user picks who becomes
/// a named agent. Synchronous — one graph read.
async fn prepare_preview(State(st): State<AppState>, Json(req): Json<PreparePreviewReq>) -> AppResult<Success<Value>> {
    let graph_id = resolve_graph_id(&st, &req.simulation_id)?;
    let data = neo4j::get_graph_data(st.graph()?, &graph_id).await.map_err(AppError::Other)?;
    Ok(Success(persona::preview(&data, req.min_evidence.unwrap_or(2))))
}

#[derive(Deserialize)]
struct PrepareReq {
    simulation_id: String,
    /// Explicit entity uuids to instantiate. If empty, every eligible entity
    /// (optionally filtered by `entity_types`) is used.
    #[serde(default)]
    selected_entity_ids: Vec<String>,
    #[serde(default)]
    entity_types: Vec<String>,
    #[serde(default = "default_true")]
    use_llm_for_profiles: bool,
    #[serde(default)]
    min_evidence: Option<usize>,
    /// Optional scenario/event to seed the discussion (e.g. the policy under test).
    #[serde(default)]
    event: Option<String>,
}
fn default_true() -> bool {
    true
}

/// Compile graph entities into grounded personas and attach them to the sim.
/// Async: returns a task_id to poll via /prepare/status.
async fn prepare(State(st): State<AppState>, Json(req): Json<PrepareReq>) -> AppResult<Success<Value>> {
    let graph_id = resolve_graph_id(&st, &req.simulation_id)?;
    let graph = st.graph()?.clone();
    // Persona synthesis is low-volume + quality-sensitive → the boost slot.
    let llm = st.llm_boost.clone();
    let manager = st.sim_manager();
    let sim_id = req.simulation_id;
    let selected = req.selected_entity_ids;
    let types = req.entity_types;
    let use_llm = req.use_llm_for_profiles;
    let min_evidence = req.min_evidence.unwrap_or(2);
    let event = req.event;

    let task_id = tasks::create("persona_prepare", json!({"simulation_id": sim_id, "graph_id": graph_id}));
    let tid = task_id.clone();
    let sim_for_task = sim_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_prepare(
            &graph, &llm, &manager, &tid, &sim_for_task, &graph_id, selected, types, use_llm, min_evidence, event,
        )
        .await
        {
            tracing::error!("persona prepare {tid} failed: {e:#}");
            tasks::fail(&tid, format!("{e:#}"));
        }
    });
    Ok(Success(json!({ "task_id": task_id, "simulation_id": sim_id })))
}

#[allow(clippy::too_many_arguments)]
async fn run_prepare(
    graph: &neo4rs::Graph,
    llm: &crate::llm::Llm,
    manager: &crate::sim::manager::Manager,
    task_id: &str,
    sim_id: &str,
    graph_id: &str,
    selected: Vec<String>,
    types: Vec<String>,
    use_llm: bool,
    min_evidence: usize,
    event: Option<String>,
) -> anyhow::Result<()> {
    tasks::update(task_id, 10, "Loading graph...");
    let data = neo4j::get_graph_data(graph, graph_id).await?;
    let bundles = persona::build_bundles(&data);
    tasks::update(task_id, 40, format!("{} entities in graph", bundles.len()));

    let id_set: std::collections::HashSet<String> = selected.into_iter().collect();
    let type_set: std::collections::HashSet<String> = types.into_iter().collect();
    let chosen: Vec<&persona::EvidenceBundle> = bundles
        .iter()
        .filter(|b| {
            if !id_set.is_empty() {
                return id_set.contains(&b.uuid);
            }
            persona::eligible(b, min_evidence) && (type_set.is_empty() || type_set.contains(&b.entity_type))
        })
        .collect();
    if chosen.is_empty() {
        anyhow::bail!("no entities matched (min_evidence={min_evidence}); loosen selection or lower the bar");
    }

    let total = chosen.len();
    let mut profiles = Vec::with_capacity(total);
    for (i, b) in chosen.into_iter().enumerate() {
        let mut p = persona::to_profile(b, i as i64);
        if use_llm {
            p.persona = persona::synthesize_persona(llm, b).await;
        }
        profiles.push(p);
        let prog = 40 + (50 * (i + 1) / total) as u8;
        tasks::update(task_id, prog.min(90), format!("Compiled {}/{total} personas", i + 1));
    }

    manager.attach_profiles(sim_id, profiles, event)?;
    tasks::complete(
        task_id,
        json!({
            "simulation_id": sim_id,
            "personas_created": total,
            "min_evidence": min_evidence,
            "grounded": true,
        }),
    );
    Ok(())
}

#[derive(Deserialize)]
struct PrepareStatusReq {
    #[serde(default)]
    task_id: Option<String>,
}

async fn prepare_status(Json(req): Json<PrepareStatusReq>) -> AppResult<Success<Value>> {
    let tid = req.task_id.ok_or_else(|| AppError::BadRequest("task_id required".into()))?;
    let task = tasks::get(&tid).ok_or_else(|| AppError::NotFound(format!("task {tid}")))?;
    Ok(Success(serde_json::to_value(task).map_err(|e| AppError::Other(e.into()))?))
}

// ---- honesty tests: seed-variance noise floor + persona-permutation ablation ----

#[derive(Deserialize)]
struct ValidateReq {
    /// Sibling runs (same prepared sim, different seeds) → noise floor.
    #[serde(default)]
    simulation_ids: Vec<String>,
    /// Optional ablation pair: a normal run vs a persona-permuted run.
    #[serde(default)]
    baseline_id: Option<String>,
    #[serde(default)]
    permuted_id: Option<String>,
}

/// Compare the stance distributions of completed runs. Callers produce the runs
/// (start the same sim with different `seed`s, and one with `permute_personas`),
/// then pass the ids here to see whether any effect exceeds the noise floor.
async fn validate(State(st): State<AppState>, Json(req): Json<ValidateReq>) -> AppResult<Success<Value>> {
    let manager = st.sim_manager();
    // Agent-weighted (one vote per agent) — the honest population metric.
    let shares_of = |id: &str| -> AppResult<std::collections::BTreeMap<String, f64>> {
        let store = manager.store(id).map_err(|_| AppError::NotFound(format!("simulation {id}")))?;
        let dist = store.agent_stance_distribution().map_err(AppError::Other)?;
        Ok(crate::sim::validate::stance_shares(&dist))
    };

    let mut runs = Vec::new();
    let mut run_shares = Vec::new();
    for id in &req.simulation_ids {
        let s = shares_of(id)?;
        runs.push(json!({ "simulation_id": id, "shares": s }));
        run_shares.push(s);
    }
    let floor = crate::sim::validate::noise_floor(&run_shares);

    let mut out = json!({
        "runs": runs,
        "noise_floor": floor,
        "note": "a claimed effect smaller than noise_floor is indistinguishable from seed noise",
    });
    if let (Some(b), Some(p)) = (&req.baseline_id, &req.permuted_id) {
        let dist = crate::sim::validate::total_variation(&shares_of(b)?, &shares_of(p)?);
        let threshold = if floor > 0.0 { floor * 1.5 } else { 0.05 };
        out["persona_ablation"] = json!({
            "baseline_id": b,
            "permuted_id": p,
            "distance": dist,
            "personas_matter": dist > threshold,
            "note": "distance at or below the noise floor => personas are inert; the output is the model prior",
        });
    }
    Ok(Success(out))
}

#[derive(Deserialize)]
struct StartReq {
    simulation_id: String,
    #[serde(default)]
    max_rounds: Option<u32>,
    /// Honesty-test overrides: fix the seed (seed-variance) and/or permute
    /// personas (ablation) for this run.
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    permute_personas: bool,
}
async fn start(State(st): State<AppState>, Json(req): Json<StartReq>) -> AppResult<Success<Value>> {
    st.sim_manager()
        .start(&req.simulation_id, req.max_rounds, req.seed, req.permute_personas)
        .map_err(AppError::Other)?;
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
