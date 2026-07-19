//! Model-selector API: read/update the LLM provider settings at runtime, test a
//! provider, and detect locally-running model servers. Keys are never returned.

use crate::error::{AppError, AppResult, Success};
use crate::llm::Msg;
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
        .route("/llm/status", get(llm_status))
        .route("/llm/providers", get(providers))
        // "Sign in with ChatGPT" — use a ChatGPT subscription as the provider.
        .route("/llm/chatgpt/login", post(chatgpt_login))
        .route("/llm/chatgpt/status", get(chatgpt_status))
        .route("/llm/chatgpt/logout", post(chatgpt_logout))
        // LM Studio local model management (list / load / unload / download).
        .route("/lmstudio/models", get(lms_models))
        .route("/lmstudio/load", post(lms_load))
        .route("/lmstudio/unload", post(lms_unload))
        .route("/lmstudio/download", post(lms_download))
        .route("/lmstudio/download/status", post(lms_download_status))
        // Ollama local model management (native /api/pull — no CLI needed).
        .route("/ollama/pull", post(ollama_pull))
        .route("/ollama/pull/status", post(lms_download_status))
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
    let s = st.llm_settings.read().unwrap_or_else(|e| e.into_inner());
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
        let mut s = st.llm_settings.write().unwrap_or_else(|e| e.into_inner());
        apply(&mut s.bulk, req.bulk);
        apply(&mut s.boost, req.boost);
        s.save(std::path::Path::new(&st.cfg.settings_path))
            .map_err(|e| AppError::Other(e.into()))?;
    }
    let s = st.llm_settings.read().unwrap_or_else(|e| e.into_inner());
    Ok(Success(json!({ "bulk": masked(&s.bulk), "boost": masked(&s.boost) })))
}

#[derive(Deserialize)]
struct TestReq {
    base_url: String,
    model: String,
    #[serde(default)]
    api_key: String,
}

/// One tiny chat to confirm a provider/model works before saving it. Routes
/// through the same `client_for` the engine uses, so the ChatGPT subscription
/// provider is tested over its OAuth backend rather than as a keyed API.
async fn test_llm(State(st): State<AppState>, Json(req): Json<TestReq>) -> AppResult<Success<Value>> {
    if crate::chatgpt_auth::is_chatgpt_backend(&req.base_url) && !st.chatgpt.logged_in() {
        return Ok(Success(
            json!({ "ok": false, "error": "Not signed in to ChatGPT — sign in from Settings." }),
        ));
    }
    let llm = st.client_for(&LlmSlot { base_url: req.base_url, model: req.model, api_key: req.api_key });
    match llm.chat(&[Msg::user("Reply with the single word: ok")], 0.0, 8).await {
        Ok(reply) => Ok(Success(json!({ "ok": true, "reply": reply.chars().take(80).collect::<String>() }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": format!("{e}") }))),
    }
}

/// Readiness of the CURRENTLY-CONFIGURED bulk slot — the one Start Engine uses.
/// A deliberate fast probe: one 1-token chat with a short timeout and NO retries,
/// unlike `Llm::chat` (300s timeout, 10 retries with backoff) which would hang
/// the landing page on a dead endpoint.
async fn llm_status(State(st): State<AppState>) -> AppResult<Success<Value>> {
    let slot = st.llm_settings.read().unwrap_or_else(|e| e.into_inner()).bulk.clone();
    // The ChatGPT subscription slot has no /chat/completions to probe — its
    // readiness is simply whether we hold OAuth credentials.
    let reason = if crate::chatgpt_auth::is_chatgpt_backend(&slot.base_url) {
        (!st.chatgpt.logged_in()).then(|| "Not signed in to ChatGPT — sign in from Settings.".to_string())
    } else {
        probe_slot(&slot).await
    };
    Ok(Success(json!({
        "ready": reason.is_none(),
        "model": slot.model,
        "base_url": slot.base_url,
        "reason": reason.unwrap_or_default(),
    })))
}

/// Start the browser OAuth flow and hand the frontend the URL to open.
async fn chatgpt_login(State(st): State<AppState>) -> AppResult<Success<Value>> {
    let url = st
        .chatgpt
        .clone()
        .begin_login()
        .await
        .map_err(|e| AppError::BadRequest(format!("{e}")))?;
    Ok(Success(json!({ "auth_url": url })))
}

/// { logged_in, email, plan, status } — the UI polls this after opening the URL.
async fn chatgpt_status(State(st): State<AppState>) -> AppResult<Success<Value>> {
    Ok(Success(st.chatgpt.status_json()))
}

async fn chatgpt_logout(State(st): State<AppState>) -> AppResult<Success<Value>> {
    st.chatgpt.logout();
    Ok(Success(json!({ "logged_in": false })))
}

/// None = ready; Some(reason) = human-readable why-not. Fails within ~5s.
async fn probe_slot(slot: &LlmSlot) -> Option<String> {
    if let Some(r) = unconfigured_reason(&slot.base_url, &slot.model) {
        return Some(r);
    }
    // Dead endpoints fail at connect (≤3s) or instantly on refusal; the wider
    // total timeout is headroom for a slow local model producing its 1 token.
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .ok()?; // client build never fails in practice; treat as ready-unknown
    let url = format!("{}/chat/completions", slot.base_url.trim_end_matches('/'));
    let body = json!({
        "model": slot.model,
        "messages": [{ "role": "user", "content": "ok" }],
        "temperature": 0.0,
        "max_tokens": 1,
    });
    let mut req = http.post(&url).json(&body);
    if !slot.api_key.is_empty() {
        req = req.bearer_auth(&slot.api_key);
    }
    match req.send().await {
        Ok(r) if r.status().is_success() => None,
        Ok(r) => {
            let status = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            Some(http_error_reason(status, &text, &slot.model, !slot.api_key.is_empty()))
        }
        Err(e) => Some(unreachable_reason(&slot.base_url, e.is_timeout())),
    }
}

fn unconfigured_reason(base_url: &str, model: &str) -> Option<String> {
    if base_url.trim().is_empty() {
        Some("No provider configured — pick one in Settings.".into())
    } else if model.trim().is_empty() {
        Some("No model configured — pick one in Settings.".into())
    } else {
        None
    }
}

/// Friendly name for a known local server, keyed off its default port.
fn local_server_name(base_url: &str) -> Option<&'static str> {
    if base_url.contains(":11434") {
        Some("Ollama")
    } else if base_url.contains(":1234") {
        Some("LM Studio")
    } else {
        None
    }
}

fn unreachable_reason(base_url: &str, timed_out: bool) -> String {
    match (local_server_name(base_url), timed_out) {
        (Some(name), false) => format!("{name} is not reachable — is it running?"),
        (Some(name), true) => format!("{name} did not answer in time — is the model loaded?"),
        (None, false) => format!("{base_url} is not reachable."),
        (None, true) => format!("{base_url} did not respond in time."),
    }
}

fn http_error_reason(status: u16, body: &str, model: &str, has_key: bool) -> String {
    let lower = body.to_lowercase();
    match status {
        401 | 403 if !has_key => "This provider requires an API key — none is set.".into(),
        401 | 403 => "API key rejected by the provider.".into(),
        404 | 400 if lower.contains("load") || lower.contains("not found") || lower.contains("no such") => {
            format!("Model \"{model}\" is not available on the server — is it loaded?")
        }
        429 => "Provider is rate-limiting requests right now.".into(),
        _ => {
            let snippet: String = body.chars().take(120).collect();
            format!("Provider returned error {status}: {snippet}")
        }
    }
}

/// Detected local servers (with their loaded models) + remote presets, so the UI
/// can offer a one-click pick. Probing happens here (not the browser) to dodge CORS.
/// Every provider Popinion can talk to, tagged `local` or `remote` so the UI can
/// split them into tabs. Local ones are probed live: `running` says whether the
/// server answered, and `models` is what it actually has installed.
async fn providers() -> AppResult<Success<Value>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(700))
        .build()
        .map_err(|e| AppError::Other(e.into()))?;

    // Ollama (native /api/tags → models[].name; OpenAI-compat lives at /v1).
    let mut ollama_models: Vec<String> = Vec::new();
    let mut ollama_running = false;
    if let Ok(r) = http.get("http://127.0.0.1:11434/api/tags").send().await {
        if let Ok(v) = r.json::<Value>().await {
            ollama_running = true;
            ollama_models = v["models"]
                .as_array()
                .map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
                .unwrap_or_default();
        }
    }
    // LM Studio (/v1/models → data[].id).
    let mut lms_models: Vec<String> = Vec::new();
    let mut lms_running = false;
    if let Ok(r) = http.get("http://127.0.0.1:1234/v1/models").send().await {
        if let Ok(v) = r.json::<Value>().await {
            lms_running = true;
            lms_models = v["data"]
                .as_array()
                .map(|a| a.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
                .unwrap_or_default();
        }
    }

    // Remote presets are OpenAI-compatible `/chat/completions` endpoints, so the
    // one client works for all of them. ChatGPT is the odd one out: OAuth against
    // a subscription instead of a metered key.
    let providers = json!([
        {"id": "ollama", "label": "Ollama", "kind": "local", "base_url": "http://127.0.0.1:11434/v1",
         "needs_key": false, "running": ollama_running, "models": ollama_models,
         "hint": "Free, private, no key. Needs a GPU to be usable."},
        {"id": "lmstudio", "label": "LM Studio", "kind": "local", "base_url": "http://127.0.0.1:1234/v1",
         "needs_key": false, "running": lms_running, "models": lms_models,
         "hint": "Free, private, no key. Needs a GPU to be usable."},

        // Codex rejects any model not entitled to ChatGPT-account auth (gpt-5,
        // gpt-5-codex, gpt-5.2, gpt-5.3-codex are all retired on this path).
        // Terra first: the everyday workhorse, and this is bulk work.
        {"id": "chatgpt", "label": "ChatGPT (subscription)", "kind": "remote", "base_url": crate::chatgpt_auth::BACKEND,
         "needs_key": false, "oauth": true,
         "models": ["gpt-5.6-terra", "gpt-5.6-sol", "gpt-5.6-luna", "gpt-5.5"],
         "hint": "Sign in with your ChatGPT plan — no API key, no per-token bill."},
        {"id": "openai", "label": "OpenAI (API key)", "kind": "remote", "base_url": "https://api.openai.com/v1",
         "needs_key": true, "models": ["gpt-4o", "gpt-4o-mini", "o3-mini"],
         "hint": "Metered API key from platform.openai.com — billed per token."},
        // Z.ai bills a Coding Plan key against subscription quota ONLY on the
        // /coding/ endpoint. The same key also authenticates on the general
        // endpoint below, where it silently draws down wallet balance instead —
        // hence two entries rather than one guessable base_url.
        {"id": "glm-coding", "label": "Z.ai GLM (coding plan)", "kind": "remote",
         "base_url": "https://api.z.ai/api/coding/paas/v4",
         "needs_key": true, "models": ["glm-5.2", "glm-5-turbo", "glm-4.7", "glm-4.5-air"],
         "hint": "Coding Plan subscription quota. Key from the Coding Plan console — not a general Z.ai key."},
        {"id": "glm", "label": "Z.ai GLM (pay per token)", "kind": "remote",
         "base_url": "https://api.z.ai/api/paas/v4",
         "needs_key": true, "models": ["glm-5.2", "glm-5-turbo", "glm-4.7", "glm-4.5-air"],
         "hint": "Metered wallet balance from z.ai. A Coding Plan key here bills your wallet, not the plan."},
        {"id": "kimi", "label": "Moonshot (Kimi)", "kind": "remote", "base_url": "https://api.moonshot.ai/v1",
         "needs_key": true, "models": ["kimi-k3", "kimi-k2.7-code", "kimi-k2.6", "moonshot-v1-128k"],
         "hint": "Metered key from platform.moonshot.ai — long context."},
        {"id": "deepseek", "label": "DeepSeek", "kind": "remote", "base_url": "https://api.deepseek.com/v1",
         "needs_key": true, "models": ["deepseek-chat", "deepseek-reasoner"],
         "hint": "Key from platform.deepseek.com — low cost."},
        {"id": "anthropic", "label": "Anthropic", "kind": "remote", "base_url": "https://api.anthropic.com/v1",
         "needs_key": true, "models": ["claude-opus-4-8", "claude-sonnet-5", "claude-haiku-4-5"],
         "hint": "Key from console.anthropic.com — uses the OpenAI-compatible endpoint."},
        {"id": "openrouter", "label": "OpenRouter", "kind": "remote", "base_url": "https://openrouter.ai/api/v1",
         "needs_key": true, "models": [],
         "hint": "One key, many models — type any OpenRouter model id."},
    ]);

    Ok(Success(json!({ "providers": providers })))
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

/// A model id is a hub path like "openai/gpt-oss-20b". Reject empty, a leading
/// '-' (which the `lms` CLI would parse as a flag, not a model), and anything
/// outside the hub-id charset — guards the subprocess against argument injection.
fn valid_model_id(model: &str) -> bool {
    let m = model.trim();
    !m.is_empty()
        && !m.starts_with('-')
        && m.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-' | ':'))
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
    if !valid_model_id(&req.model) {
        return Ok(Success(json!({ "ok": false, "error": "invalid model id" })));
    }
    match run_lms(vec!["load".into(), req.model.clone(), "-y".into()]).await {
        Ok(_) => Ok(Success(json!({ "ok": true, "model": req.model, "state": "loaded" }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": e }))),
    }
}

async fn lms_unload(Json(req): Json<ModelReq>) -> AppResult<Success<Value>> {
    if !valid_model_id(&req.model) {
        return Ok(Success(json!({ "ok": false, "error": "invalid model id" })));
    }
    match run_lms(vec!["unload".into(), req.model.clone()]).await {
        Ok(_) => Ok(Success(json!({ "ok": true, "model": req.model, "state": "not-loaded" }))),
        Err(e) => Ok(Success(json!({ "ok": false, "error": e }))),
    }
}

/// Download a model by hub id (e.g. "openai/gpt-oss-20b"). It's a multi-GB
/// download, so it runs as a background task; poll /lmstudio/download/status.
async fn lms_download(Json(req): Json<ModelReq>) -> AppResult<Success<Value>> {
    let model = req.model.trim().to_string();
    if !valid_model_id(&model) {
        return Err(AppError::BadRequest("invalid model id".into()));
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

// ---- Ollama local model management ----

#[derive(Deserialize)]
struct OllamaPullReq {
    model: String,
    /// The slot's base_url (e.g. http://localhost:11434/v1); the native pull API
    /// lives at the host root. Defaults to the standard local Ollama port.
    #[serde(default)]
    base_url: Option<String>,
}

/// This endpoint only ever manages the LOCAL Ollama, so the caller-supplied
/// host must resolve to loopback. Without this the caller could aim the pull at
/// any internal address and read the reply back off the task status — a
/// request-and-read SSRF primitive (e.g. the cloud metadata service).
fn local_host(host: &str) -> AppResult<String> {
    let url = reqwest::Url::parse(host).map_err(|e| AppError::BadRequest(format!("invalid base_url: {e}")))?;
    let loopback = matches!(url.scheme(), "http" | "https")
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        && url.username().is_empty();
    if !loopback {
        return Err(AppError::BadRequest(
            "base_url must point at a local Ollama (127.0.0.1, ::1 or localhost)".into(),
        ));
    }
    Ok(host.to_string())
}

/// Pull an Ollama model (e.g. "gemma4:e4b") via its native /api/pull, which
/// streams progress. Runs as a background task; poll /ollama/pull/status.
async fn ollama_pull(Json(req): Json<OllamaPullReq>) -> AppResult<Success<Value>> {
    let model = req.model.trim().to_string();
    if !valid_model_id(&model) {
        return Err(AppError::BadRequest("invalid model id".into()));
    }
    // The native API lives at the host root; strip a trailing /v1 so an
    // OpenAI-compat base_url (http://host:11434/v1) resolves to http://host:11434.
    let base = req.base_url.unwrap_or_else(|| "http://127.0.0.1:11434".into());
    let host = base.trim().trim_end_matches('/').trim_end_matches("/v1").trim_end_matches('/');
    let host = local_host(host)?;
    let url = format!("{host}/api/pull");

    let task_id = crate::services::graph::tasks::create("ollama_pull", json!({ "model": model }));
    let tid = task_id.clone();
    let model_c = model.clone();
    tokio::spawn(async move {
        match run_ollama_pull(&url, &model_c, &tid).await {
            Ok(_) => crate::services::graph::tasks::complete(&tid, json!({ "model": model_c, "downloaded": true })),
            Err(e) => crate::services::graph::tasks::fail(&tid, e),
        }
    });
    Ok(Success(json!({ "task_id": task_id, "model": model })))
}

/// Stream Ollama's /api/pull NDJSON, updating task progress from total/completed.
async fn run_ollama_pull(url: &str, model: &str, tid: &str) -> Result<(), String> {
    // No redirects: following one would let a local service bounce the request
    // back out to a non-loopback host, undoing the `local_host` check.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .post(url)
        .json(&json!({ "name": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("Ollama not reachable — is it running? ({e})"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama returned {}", resp.status()));
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut last_pct: u8 = 0;
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        buf.extend_from_slice(&chunk);
        // Ollama emits one JSON object per line; parse each complete line.
        while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl).collect();
            if line.iter().all(|b| b.is_ascii_whitespace()) {
                continue;
            }
            let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
            if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                return Err(err.to_string());
            }
            let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
            let pct = match (
                v.get("total").and_then(|t| t.as_u64()),
                v.get("completed").and_then(|c| c.as_u64()),
            ) {
                (Some(t), Some(c)) if t > 0 => ((c as f64 / t as f64) * 100.0) as u8,
                _ => last_pct,
            };
            last_pct = pct;
            crate::services::graph::tasks::update(tid, pct.min(99), format!("{status} ({pct}%)"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_slot_reasons() {
        assert!(unconfigured_reason("", "gpt-4o").unwrap().contains("No provider"));
        assert!(unconfigured_reason("https://api.openai.com/v1", " ").unwrap().contains("No model"));
        assert!(unconfigured_reason("https://api.openai.com/v1", "gpt-4o").is_none());
    }

    #[test]
    fn unreachable_names_known_local_servers() {
        assert_eq!(unreachable_reason("http://127.0.0.1:11434/v1", false), "Ollama is not reachable — is it running?");
        assert_eq!(unreachable_reason("http://127.0.0.1:1234/v1", true), "LM Studio did not answer in time — is the model loaded?");
        assert!(unreachable_reason("https://api.example.com/v1", true).contains("did not respond"));
        assert!(unreachable_reason("https://api.example.com/v1", false).contains("not reachable"));
    }

    #[test]
    fn ollama_pull_host_must_be_loopback() {
        assert!(local_host("http://127.0.0.1:11434").is_ok());
        assert!(local_host("http://localhost:11434").is_ok());
        assert!(local_host("http://[::1]:11434").is_ok());
        // The SSRF cases: cloud metadata, an internal host, a non-HTTP scheme.
        assert!(local_host("http://169.254.169.254").is_err());
        assert!(local_host("http://internal.corp:8080").is_err());
        assert!(local_host("file:///etc/passwd").is_err());
        assert!(local_host("http://127.0.0.1@evil.com").is_err());
    }

    #[test]
    fn http_errors_classify_auth_and_missing_model() {
        assert_eq!(http_error_reason(401, "", "m", false), "This provider requires an API key — none is set.");
        assert_eq!(http_error_reason(401, "", "m", true), "API key rejected by the provider.");
        // LM Studio with nothing loaded / Ollama with an unknown model.
        assert!(http_error_reason(404, r#"{"error":"No models loaded"}"#, "qwen2.5:7b", false).contains("is it loaded?"));
        assert!(http_error_reason(404, r#"{"error":"model 'x' not found"}"#, "x", false).contains("not available"));
        assert!(http_error_reason(429, "slow down", "m", true).contains("rate-limiting"));
        assert!(http_error_reason(500, "boom", "m", true).contains("error 500"));
    }
}
