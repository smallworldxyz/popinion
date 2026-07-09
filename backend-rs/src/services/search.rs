//! Web search via Tavily (https://tavily.com). One HTTP call returning an
//! LLM-ready digest: a synthesized answer plus the top result snippets.
//! ponytail: single provider; swap the body of `web_search` to change backends.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

const TAVILY_URL: &str = "https://api.tavily.com/search";

/// Run a web search and format the results as text for a tool observation.
/// Returns a clear message (not an error) when no key is configured, so the
/// agent loop degrades gracefully instead of failing the whole report.
pub async fn web_search(api_key: &str, query: &str, max_results: usize) -> Result<String> {
    if api_key.is_empty() {
        return Ok("web_search is not configured (set TAVILY_API_KEY to enable it).".into());
    }
    let body = json!({
        "api_key": api_key,
        "query": query,
        "max_results": max_results.clamp(1, 10),
        "search_depth": "basic",
        "include_answer": true,
    });
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest client");
    let resp = http.post(TAVILY_URL).json(&body).send().await.context("tavily request")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("tavily error {status}: {text}"));
    }
    let v: Value = resp.json().await.context("decode tavily response")?;
    Ok(format_results(&v, query))
}

fn format_results(v: &Value, query: &str) -> String {
    let mut out = String::new();
    if let Some(answer) = v["answer"].as_str() {
        if !answer.is_empty() {
            out.push_str(&format!("Answer: {answer}\n\n"));
        }
    }
    let results = v["results"].as_array().cloned().unwrap_or_default();
    if results.is_empty() {
        return if out.is_empty() {
            format!("No web results for query: {query}")
        } else {
            out
        };
    }
    out.push_str("Sources:\n");
    for (i, r) in results.iter().enumerate() {
        let title = r["title"].as_str().unwrap_or("(untitled)");
        let url = r["url"].as_str().unwrap_or("");
        let content = r["content"].as_str().unwrap_or("");
        let snippet: String = content.chars().take(500).collect();
        out.push_str(&format!("{}. {title} ({url})\n   {snippet}\n", i + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_key_is_a_soft_message_not_an_error() {
        let out = web_search("", "anything", 5).await.unwrap();
        assert!(out.contains("not configured"));
    }

    #[test]
    fn formats_answer_and_sources() {
        let v = json!({
            "answer": "The sky is blue.",
            "results": [
                {"title": "Why", "url": "http://x", "content": "because rayleigh"},
                {"title": "More", "url": "http://y", "content": "scattering"}
            ]
        });
        let out = format_results(&v, "why sky blue");
        assert!(out.contains("Answer: The sky is blue."));
        assert!(out.contains("1. Why (http://x)"));
        assert!(out.contains("2. More (http://y)"));
    }

    #[test]
    fn handles_no_results() {
        let out = format_results(&json!({"results": []}), "obscure query");
        assert!(out.contains("No web results"));
    }
}
