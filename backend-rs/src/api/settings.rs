//! Model-selector API: read/update the LLM provider settings at runtime, test a
//! provider, and detect locally-running model servers. Keys are never returned.

use crate::error::{AppError, AppResult};
use crate::llm::{Llm, Msg};
use crate::models::Success;
use crate::settings::LlmSlot;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/llm", get(get_llm).put(put_llm))
        .route("/llm/test", post(test_llm))
        .route("/llm/providers", get(providers))
        // LM Studio local model management (list / load / unload / download).
        .route("/lmstudio/models", get(lms_models))
        .route("/lmstudio/load", post(lms_load))
        .route("/lmstudio/unload", post(lms_unload))
        .route("/lmstudio/download", post(lms_download))
        .route("/lmstudio/download/status", post(lms_download_status))
}

/// Slot view with the key masked to a presence flag — never leak the secret.
fn masked(slot: &LlmSlot) -> Value {
    json!({
        "base_url": slot.base_url,
        "model": slot.model,
        "has_key": !slot.api_key.is_empty(),
    })
}

async fn get_llm(State(st): State<AppState>) -> AppResult<Success<Value>> {
    let s = st.llm_settings.read().unwrap();
    Ok(Success(json!({ "bulk": masked(&s.bulk), "boost": masked(&s.boost) })))
}

#[derive(Deserialize)]
struct SlotUpdate {
    base_url: Option<String>,
    model: Option<String>,
    /// Omit to keep the current key; send "" to clear it (local providers).
    api_key: Option<String>,
}

#[derive(Deserialize)]
struct LlmUpdate {
    bulk: Option<SlotUpdate>,
    boost: Option<SlotUpdate>,
}

fn apply(slot: &mut LlmSlot, upd: Option<SlotUpdate>) {
    if let Some(u) = upd {
        if let Some(b) = u.base_url {
            slot.base_url = b;
        }
        if let Some(m) = u.model {
            slot.model = m;
        }
        if let Some(k) = u.api_key {
            slot.api_key = k;
        }
    }
}

async fn put_llm(State(st): State<AppState>, Json(req): Json<LlmUpdate>) -> AppResult<Success<Value>> {
    {
        let mut s = st.llm_settings.write().unwrap();
        apply(&mut s.bulk, req.bulk);
        apply(&mut s.boost, req.boost);
        s.save(std::path::Path::new(&st.cfg.settings_path))
            .map_err(|e| AppError::Other(e.into()))?;
    }
    let s = st.llm_settings.read().unwrap();
    Ok(Success(json!({ "bulk": masked(&s.bulk), "boost": masked(&s.boost) })))
}

#[derive(Deserialize)]
struct TestReq {
    base_url: String,
    model: String,
    #[serde(default)]
    api_key: String,
}

/// One tiny chat to confirm a provider/model works before saving it.
async fn test_llm(Json(req): Json<TestReq>) -> AppResult<Success<Value>> {
    let llm = Llm::new(&req.api_key, &req.base_url, &req.model);
    match llm.chat(&[Msg::user("Reply with the single word: ok")], 0.0, 8).await {
        Ok(reply) => Ok(Success(json!({ "ok": true, "reply": reply.chars().take(80).collect::<String>() }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": format!("{e}") }))),
    }
}

/// Detected local servers (with their loaded models) + remote presets, so the UI
/// can offer a one-click pick. Probing happens here (not the browser) to dodge CORS.
async fn providers() -> AppResult<Success<Value>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(700))
        .build()
        .map_err(|e| AppError::Other(e.into()))?;

    let mut detected = Vec::new();

    // Ollama (native /api/tags → models[].name; OpenAI-compat lives at /v1).
    if let Ok(r) = http.get("http://127.0.0.1:11434/api/tags").send().await {
        if let Ok(v) = r.json::<Value>().await {
            let models: Vec<String> = v["models"]
                .as_array()
                .map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
                .unwrap_or_default();
            detected.push(json!({
                "id": "ollama", "label": "Ollama (local)",
                "base_url": "http://127.0.0.1:11434/v1", "needs_key": false, "models": models,
            }));
        }
    }
    // LM Studio (/v1/models → data[].id).
    if let Ok(r) = http.get("http://127.0.0.1:1234/v1/models").send().await {
        if let Ok(v) = r.json::<Value>().await {
            let models: Vec<String> = v["data"]
                .as_array()
                .map(|a| a.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
                .unwrap_or_default();
            detected.push(json!({
                "id": "lmstudio", "label": "LM Studio (local)",
                "base_url": "http://127.0.0.1:1234/v1", "needs_key": false, "models": models,
            }));
        }
    }

    let presets = json!([
        {"id": "openai", "label": "OpenAI", "base_url": "https://api.openai.com/v1", "needs_key": true},
        {"id": "openrouter", "label": "OpenRouter", "base_url": "https://openrouter.ai/api/v1", "needs_key": true},
        {"id": "anthropic", "label": "Anthropic", "base_url": "https://api.anthropic.com/v1", "needs_key": true},
    ]);

    Ok(Success(json!({ "detected": detected, "presets": presets })))
}

// ---- LM Studio local model management ----

/// The `lms` CLI binary: on PATH, else the default install location.
fn lms_bin() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let default = format!("{home}/.lmstudio/bin/lms");
    if std::path::Path::new(&default).exists() {
        default
    } else {
        "lms".to_string()
    }
}

/// Run an `lms` subcommand off the async runtime; Ok(stdout) / Err(stderr).
async fn run_lms(args: Vec<String>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new(lms_bin())
            .args(&args)
            .output()
            .map_err(|e| format!("cannot run lms (is LM Studio's CLI installed?): {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(format!("lms {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
        }
    })
    .await
    .unwrap_or_else(|e| Err(format!("task join error: {e}")))
}

/// All downloaded models with their loaded/not-loaded state (LM Studio REST).
/// Embedding models are excluded — they can't drive chat.
async fn lms_models() -> AppResult<Success<Value>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| AppError::Other(e.into()))?;
    let v: Value = http
        .get("http://127.0.0.1:1234/api/v0/models")
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("LM Studio not reachable on :1234 — is it running? ({e})")))?
        .json()
        .await
        .map_err(|e| AppError::Other(e.into()))?;
    let models: Vec<Value> = v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| m["type"].as_str() != Some("embeddings"))
                .map(|m| json!({ "id": m["id"], "state": m["state"], "type": m["type"] }))
                .collect()
        })
        .unwrap_or_default();
    Ok(Success(json!({ "models": models })))
}

#[derive(Deserialize)]
struct ModelReq {
    model: String,
}

async fn lms_load(Json(req): Json<ModelReq>) -> AppResult<Success<Value>> {
    match run_lms(vec!["load".into(), req.model.clone(), "-y".into()]).await {
        Ok(_) => Ok(Success(json!({ "ok": true, "model": req.model, "state": "loaded" }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": e }))),
    }
}

async fn lms_unload(Json(req): Json<ModelReq>) -> AppResult<Success<Value>> {
    match run_lms(vec!["unload".into(), req.model.clone()]).await {
        Ok(_) => Ok(Success(json!({ "ok": true, "model": req.model, "state": "not-loaded" }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": e }))),
    }
}

/// Download a model by hub id (e.g. "openai/gpt-oss-20b"). It's a multi-GB
/// download, so it runs as a background task; poll /lmstudio/download/status.
async fn lms_download(Json(req): Json<ModelReq>) -> AppResult<Success<Value>> {
    let model = req.model.trim().to_string();
    if model.is_empty() {
        return Err(AppError::BadRequest("model id required".into()));
    }
    let task_id = crate::services::graph::tasks::create("lms_download", json!({ "model": model }));
    let tid = task_id.clone();
    tokio::spawn(async move {
        crate::services::graph::tasks::update(&tid, 10, format!("Downloading {model}…"));
        match run_lms(vec!["get".into(), model.clone(), "--yes".into()]).await {
            Ok(_) => crate::services::graph::tasks::complete(&tid, json!({ "model": model, "downloaded": true })),
            Err(e) => crate::services::graph::tasks::fail(&tid, e),
        }
    });
    Ok(Success(json!({ "task_id": task_id, "model": req.model })))
}

#[derive(Deserialize)]
struct TaskIdReq {
    task_id: String,
}

async fn lms_download_status(Json(req): Json<TaskIdReq>) -> AppResult<Success<Value>> {
    let task = crate::services::graph::tasks::get(&req.task_id)
        .ok_or_else(|| AppError::NotFound(format!("task {}", req.task_id)))?;
    Ok(Success(serde_json::to_value(task).map_err(|e| AppError::Other(e.into()))?))
}
