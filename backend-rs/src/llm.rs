use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

/// OpenAI-compatible chat client. Talks to any `${base_url}/chat/completions`
/// endpoint (OpenAI, Azure, local vLLM, OpenRouter, ...), same as the Python
/// `LLMClient` wrapper around the `openai` SDK.
#[derive(Clone)]
pub struct Llm {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
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
        if self.api_key.is_empty() {
            return Err(anyhow!("LLM_API_KEY not configured"));
        }
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
        if json_mode {
            body["response_format"] = json!({ "type": "json_object" });
        }

        let url = format!("{}/chat/completions", self.base_url);
        // Retry on 429 / transient errors with capped exponential backoff + jitter.
        let max_retries = 10u32;
        for attempt in 0..max_retries {
            let resp = self
                .http
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await;

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

    pub async fn chat(&self, messages: &[Msg], temperature: f32, max_tokens: u32) -> Result<String> {
        self.call(messages, temperature, max_tokens, false).await
    }

    /// Chat returning parsed JSON, with a best-effort repair for truncated output.
    pub async fn chat_json(&self, messages: &[Msg], temperature: f32, max_tokens: u32) -> Result<Value> {
        let raw = self.call(messages, temperature, max_tokens, true).await?;
        match serde_json::from_str::<Value>(&raw) {
            Ok(v) => Ok(v),
            Err(_) => serde_json::from_str::<Value>(&repair_truncated_json(&raw))
                .with_context(|| format!("LLM returned invalid JSON: {}", &raw[..raw.len().min(500)])),
        }
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
}
