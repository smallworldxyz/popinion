use crate::chatgpt_auth::ChatGptAuth;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// OpenAI-compatible chat client. Talks to any `${base_url}/chat/completions`
/// endpoint (OpenAI, Azure, local vLLM, OpenRouter, ...), same as the Python
/// `LLMClient` wrapper around the `openai` SDK.
///
/// When `chatgpt` is set the client instead speaks the ChatGPT subscription
/// backend (`/backend-api/codex/responses`, the Responses API over SSE) using
/// an OAuth access token — a different wire format entirely. See `chatgpt_auth`.
#[derive(Clone)]
pub struct Llm {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    chatgpt: Option<Arc<ChatGptAuth>>,
    /// Attempts per request before giving up. Only tests lower it: the default
    /// backoff takes minutes to exhaust against an unreachable endpoint, which
    /// is right in production and a hang in a unit test.
    max_retries: u32,
}

#[derive(Serialize)]
pub struct Msg {
    pub role: String,
    pub content: String,
}

impl Msg {
    pub fn system(c: impl Into<String>) -> Self {
        Msg { role: "system".into(), content: c.into() }
    }
    pub fn user(c: impl Into<String>) -> Self {
        Msg { role: "user".into(), content: c.into() }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Msg { role: "assistant".into(), content: c.into() }
    }
}

impl Llm {
    pub fn new(api_key: &str, base_url: &str, model: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Llm {
            http,
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            chatgpt: None,
            max_retries: 10,
        }
    }

    /// Same client with a single attempt - for tests that point at a dead
    /// endpoint and want the error, not ten backoff sleeps.
    pub fn without_retries(mut self) -> Self {
        self.max_retries = 1;
        self
    }

    /// A client backed by a ChatGPT subscription (OAuth) instead of an API key.
    pub fn chatgpt(model: &str, auth: Arc<ChatGptAuth>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Llm {
            http,
            api_key: String::new(),
            base_url: crate::chatgpt_auth::BACKEND.to_string(),
            model: model.to_string(),
            chatgpt: Some(auth),
            max_retries: 10,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    async fn call(
        &self,
        messages: &[Msg],
        temperature: f32,
        max_tokens: u32,
        json_mode: bool,
    ) -> Result<String> {
        if self.chatgpt.is_some() {
            return self.chatgpt_call(messages).await;
        }
        // No empty-key rejection: local servers (Ollama, LM Studio) need no key.
        // NOTE: we deliberately do NOT send response_format: json_object. It is
        // not universally supported across OpenAI-compatible servers (LM Studio
        // rejects it for some models, wanting json_schema/text), and the callers
        // already instruct the model to emit JSON. chat_json() extracts the JSON
        // span from the reply, tolerating prose/fence wrapping instead.
        let _ = json_mode;
        let body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let url = format!("{}/chat/completions", self.base_url);
        // Retry on 429 / transient errors with capped exponential backoff + jitter.
        let max_retries = self.max_retries;
        for attempt in 0..max_retries {
            let mut builder = self.http.post(&url).json(&body);
            if !self.api_key.is_empty() {
                builder = builder.bearer_auth(&self.api_key);
            }
            let resp = builder.send().await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let v: Value = r.json().await.context("decode chat response")?;
                    let content = v["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    return Ok(content);
                }
                Ok(r) if r.status().as_u16() == 429 && attempt + 1 < max_retries => {
                    let wait = backoff(attempt);
                    tracing::warn!("LLM 429, waiting {wait:.1}s (attempt {})", attempt + 1);
                    tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                }
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
                    return Err(anyhow!("LLM API error {status}: {text}"));
                }
                Err(e) if attempt + 1 < max_retries => {
                    let wait = backoff(attempt);
                    tracing::warn!("LLM request error: {e}, retrying in {wait:.1}s");
                    tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(anyhow!("LLM retries exhausted"))
    }

    /// The ChatGPT subscription path: the Responses API over SSE against the
    /// private Codex backend. System messages become `instructions`; the rest
    /// become `input` turns. We stream and concatenate the text deltas.
    ///
    /// ponytail: no temperature/max_tokens here — the backend's models are
    /// mostly reasoning models that reject `temperature` and count reasoning
    /// against `max_output_tokens`, so passing them starves or 400s the call.
    /// Add per-model knobs only if a non-reasoning model needs them.
    async fn chatgpt_call(&self, messages: &[Msg]) -> Result<String> {
        let auth = self.chatgpt.as_ref().expect("chatgpt auth set");
        let (access_token, account_id) = auth.valid().await?;

        let instructions: String = messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let input: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                // Responses API: user turns are input_text, assistant output_text.
                let text_type = if m.role == "assistant" { "output_text" } else { "input_text" };
                json!({
                    "type": "message",
                    "role": m.role,
                    "content": [{ "type": text_type, "text": m.content }],
                })
            })
            .collect();
        let body = json!({
            "model": self.model,
            "instructions": if instructions.is_empty() { "You are a helpful assistant.".into() } else { instructions },
            "input": input,
            "store": false,
            "stream": true,
        });

        let url = format!("{}/responses", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&access_token)
            .header("chatgpt-account-id", account_id)
            .header("OpenAI-Beta", "responses=experimental")
            .header("originator", "codex_cli_rs")
            .header("session_id", uuid::Uuid::new_v4().to_string())
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let hint = if status.as_u16() == 401 {
                " (ChatGPT session expired or revoked — sign in again in Settings)"
            } else {
                ""
            };
            return Err(anyhow!("ChatGPT backend error {status}{hint}: {}", text.chars().take(300).collect::<String>()));
        }
        parse_responses_sse(resp).await
    }

    pub async fn chat(&self, messages: &[Msg], temperature: f32, max_tokens: u32) -> Result<String> {
        self.call(messages, temperature, max_tokens, false).await
    }

    /// Chat returning parsed JSON. Tolerates models that wrap JSON in prose or
    /// markdown fences (or, like reasoning models, leak a token before it) and
    /// repairs truncated output.
    pub async fn chat_json(&self, messages: &[Msg], temperature: f32, max_tokens: u32) -> Result<Value> {
        let raw = self.call(messages, temperature, max_tokens, true).await?;
        let candidate = extract_json(&raw);
        match serde_json::from_str::<Value>(&candidate) {
            Ok(v) => Ok(v),
            Err(_) => serde_json::from_str::<Value>(&repair_truncated_json(&candidate))
                // Truncate by chars, not bytes: multilingual replies (Khmer,
                // Chinese, …) would panic on a mid-codepoint byte slice, and this
                // runs inside decide()'s `?`, taking down the whole sim task.
                .with_context(|| format!("LLM returned invalid JSON: {}", raw.chars().take(500).collect::<String>())),
        }
    }
}

/// Read a Responses API SSE stream and return the assembled assistant text.
/// Events are `data: {json}` lines; we sum `response.output_text.delta` deltas
/// and fall back to the final message in `response.completed` if none arrived.
async fn parse_responses_sse(mut resp: reqwest::Response) -> Result<String> {
    let mut text = String::new();
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.context("read ChatGPT stream")? {
        buf.extend_from_slice(&chunk);
        while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl).collect();
            let line = String::from_utf8_lossy(&line);
            let Some(data) = line.trim().strip_prefix("data:") else { continue };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
            match v["type"].as_str().unwrap_or("") {
                "response.output_text.delta" => {
                    if let Some(d) = v["delta"].as_str() {
                        text.push_str(d);
                    }
                }
                "response.completed" if text.is_empty() => {
                    // No deltas seen (non-streaming-ish backend): dig the final
                    // assistant text out of the completed response object.
                    text = extract_completed_text(&v["response"]);
                }
                "response.failed" | "error" => {
                    let msg = v["response"]["error"]["message"]
                        .as_str()
                        .or_else(|| v["error"]["message"].as_str())
                        .or_else(|| v["message"].as_str())
                        .unwrap_or("stream failed");
                    return Err(anyhow!("ChatGPT backend: {msg}"));
                }
                _ => {}
            }
        }
    }
    if text.is_empty() {
        return Err(anyhow!("ChatGPT backend returned no text"));
    }
    Ok(text)
}

/// Concatenate `output_text` parts from a completed Responses object.
fn extract_completed_text(response: &Value) -> String {
    response["output"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .flat_map(|item| item["content"].as_array().cloned().unwrap_or_default())
                .filter_map(|c| c["text"].as_str().map(String::from))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

/// Pull the JSON document out of an LLM reply: strip markdown fences and any
/// prose before the first `{`/`[` or after the last `}`/`]`. Non-strict models
/// (and reasoning models that leak a stray token) otherwise fail to parse.
pub fn extract_json(s: &str) -> String {
    let mut t = s.trim();
    if let Some(rest) = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")) {
        t = rest.trim();
    }
    if let Some(rest) = t.strip_suffix("```") {
        t = rest.trim();
    }
    // Every caller returns a JSON object, so prefer the object braces: prose
    // before the payload that happens to contain a '[' (e.g. "[note] {...}")
    // otherwise derails extraction. Fall back to array brackets for array docs.
    // ponytail: object-first; a top-level array of objects would need the min of
    // the two positions instead — no caller returns one today.
    let start = t.find('{').or_else(|| t.find('['));
    let end = t.rfind('}').or_else(|| t.rfind(']'));
    match (start, end) {
        (Some(a), Some(b)) if b >= a => t[a..=b].to_string(),
        (Some(a), _) => t[a..].to_string(),
        _ => t.to_string(),
    }
}

fn backoff(attempt: u32) -> f64 {
    let base = 10.0_f64;
    // deterministic pseudo-jitter from attempt count; avoids an rng dependency
    let jitter = 0.5 + ((attempt as f64 * 1.3).fract());
    (base * 2f64.powi(attempt as i32) + jitter).min(120.0)
}

/// Close unbalanced brackets/quotes on a truncated JSON document.
pub fn repair_truncated_json(content: &str) -> String {
    let mut s = content.trim().to_string();
    let open_braces = s.matches('{').count() as i64 - s.matches('}').count() as i64;
    let open_brackets = s.matches('[').count() as i64 - s.matches(']').count() as i64;
    if let Some(last) = s.chars().last() {
        if !matches!(last, '"' | ',' | '}' | ']') {
            // crude: if we appear to be mid-string value, close the quote
            if s.rsplit(':').next().map(|t| t.trim_start().starts_with('"')).unwrap_or(false) {
                s.push('"');
            }
        }
    }
    for _ in 0..open_brackets.max(0) {
        s.push(']');
    }
    for _ in 0..open_braces.max(0) {
        s.push('}');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repairs_truncated_object() {
        let broken = r#"{"a": 1, "b": [1, 2"#;
        let fixed = repair_truncated_json(broken);
        assert!(serde_json::from_str::<Value>(&fixed).is_ok(), "got: {fixed}");
    }
    #[test]
    fn repairs_truncated_string() {
        let broken = r#"{"a": "hello wor"#;
        let fixed = repair_truncated_json(broken);
        assert!(serde_json::from_str::<Value>(&fixed).is_ok(), "got: {fixed}");
    }

    #[test]
    fn extract_json_strips_prose_fences_and_leaks() {
        assert_eq!(extract_json(r#" Festival{"a":1}"#), r#"{"a":1}"#);
        assert_eq!(extract_json("```json\n{\"a\":1}\n```"), r#"{"a":1}"#);
        assert_eq!(extract_json("Here is the answer: [1,2,3] hope it helps"), "[1,2,3]");
        assert_eq!(extract_json(r#"{"a":1}"#), r#"{"a":1}"#);
        // Prose before the object containing a stray bracket must not derail it.
        assert_eq!(extract_json(r#"[note] {"action":"like_post"}"#), r#"{"action":"like_post"}"#);
    }
}
