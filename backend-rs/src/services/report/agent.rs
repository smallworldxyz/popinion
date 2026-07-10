//! Report agent: a bounded LLM tool-calling loop over the simulation social
//! store, followed by a boosted-model synthesis pass and reflection rounds.
//!
//! Ports the architecture of the Python ReportAgent (ReACT loop + section
//! writer + reflection) but grounds the tools in `sim::store::Store` instead
//! of Neo4j, and produces a fixed six-section report instead of an LLM-planned
//! outline, so every report has a predictable shape the frontend can render.

use crate::error::{AppError, AppResult};
use crate::llm::Msg;
use crate::sim::store::Store;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::Path;

use super::registry::{self, ReportEntry, ReportSection};
use crate::services::registry::JobStatus;

pub const SECTION_TITLES: [&str; 6] = [
    "Executive Summary",
    "Stance Distribution",
    "Key Arguments by Stance",
    "Sentiment Trend",
    "Notable Personas & Influencers",
    "Risks & Outlook",
];

const MAX_CHAT_TOOL_CALLS: usize = 2;
const OBSERVATION_LIMIT: usize = 4000;
const EVIDENCE_LIMIT: usize = 24000;
const REPORT_CONTEXT_LIMIT: usize = 15000;

// ---- tools over the simulation store ----

fn tools_description() -> &'static str {
    r#"Available tools (query the simulation's social data; every claim in the report must be grounded in tool results):
- search_posts: find posts matching keywords. parameters: {"query": "keywords", "limit": 10}
- statistics: overall numbers - post count, stance distribution, most-liked posts, round range. parameters: {}
- stance_distribution: per-stance post counts and average sentiment. parameters: {}
- web_search: look up current real-world context outside the simulation (news, facts, background). parameters: {"query": "search terms", "max_results": 5}

To call a tool, emit exactly:
<tool_call>
{"name": "tool_name", "parameters": {...}}
</tool_call>
You may emit several tool_call blocks in one reply. When you have enough evidence, reply with the single word DONE."#
}

fn trim(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

pub fn execute_tool(store: &Store, name: &str, params: &Value) -> anyhow::Result<String> {
    match name {
        "search_posts" => {
            let query = params["query"].as_str().unwrap_or("").to_lowercase();
            let limit = params["limit"].as_i64().unwrap_or(10).clamp(1, 50) as usize;
            let posts = store.list_posts(1000, 0)?;
            let keywords: Vec<&str> = query.split_whitespace().collect();
            let mut scored: Vec<(usize, &Value)> = posts
                .iter()
                .map(|p| {
                    let content = p["content"].as_str().unwrap_or("").to_lowercase();
                    let score = keywords.iter().filter(|k| content.contains(**k)).count();
                    (score, p)
                })
                .filter(|(score, _)| keywords.is_empty() || *score > 0)
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            let hits: Vec<Value> = scored
                .into_iter()
                .take(limit)
                .map(|(_, p)| {
                    json!({
                        "post_id": p["post_id"],
                        "user_name": p["user_name"],
                        "content": trim(p["content"].as_str().unwrap_or(""), 500),
                        "round": p["round"],
                        "num_likes": p["num_likes"],
                        "stance": p["stance"],
                        "sentiment": p["sentiment"],
                    })
                })
                .collect();
            if hits.is_empty() {
                return Ok(format!("No posts matched query: {query}"));
            }
            Ok(serde_json::to_string_pretty(&hits)?)
        }
        "statistics" => {
            let total = store.count_posts()?;
            let dist = store.stance_distribution()?;
            let posts = store.list_posts(1000, 0)?;
            let mut by_likes: Vec<&Value> = posts.iter().collect();
            by_likes.sort_by_key(|p| -p["num_likes"].as_i64().unwrap_or(0));
            let top: Vec<Value> = by_likes
                .iter()
                .take(5)
                .map(|p| {
                    json!({
                        "post_id": p["post_id"],
                        "user_name": p["user_name"],
                        "num_likes": p["num_likes"],
                        "stance": p["stance"],
                        "content": trim(p["content"].as_str().unwrap_or(""), 200),
                    })
                })
                .collect();
            let rounds: Vec<i64> = posts.iter().filter_map(|p| p["round"].as_i64()).collect();
            // Per-round average sentiment gives the writer a real trend line.
            let mut per_round: std::collections::BTreeMap<i64, (f64, usize)> = Default::default();
            for p in &posts {
                if let (Some(r), Some(s)) = (p["round"].as_i64(), p["sentiment"].as_f64()) {
                    let e = per_round.entry(r).or_insert((0.0, 0));
                    e.0 += s;
                    e.1 += 1;
                }
            }
            let sentiment_by_round: Vec<Value> = per_round
                .into_iter()
                .map(|(r, (sum, n))| json!({"round": r, "avg_sentiment": sum / n as f64, "posts": n}))
                .collect();
            Ok(serde_json::to_string_pretty(&json!({
                "total_posts": total,
                "stance_distribution": dist,
                "round_min": rounds.iter().min(),
                "round_max": rounds.iter().max(),
                "sentiment_by_round": sentiment_by_round,
                "top_posts_by_likes": top,
            }))?)
        }
        "stance_distribution" => Ok(serde_json::to_string_pretty(&store.stance_distribution()?)?),
        _ => Ok(format!(
            "Unknown tool: {name}. Available: search_posts, statistics, stance_distribution"
        )),
    }
}

/// Parse `<tool_call>{...}</tool_call>` blocks from an LLM reply.
pub fn parse_tool_calls(response: &str) -> Vec<Value> {
    let mut calls = Vec::new();
    let mut rest = response;
    while let Some(start) = rest.find("<tool_call>") {
        let after = &rest[start + "<tool_call>".len()..];
        let Some(end) = after.find("</tool_call>") else { break };
        if let Ok(v) = serde_json::from_str::<Value>(after[..end].trim()) {
            if v["name"].is_string() {
                calls.push(v);
            }
        }
        rest = &after[end + "</tool_call>".len()..];
    }
    calls
}

fn strip_tool_calls(response: &str) -> String {
    let mut out = String::new();
    let mut rest = response;
    while let Some(start) = rest.find("<tool_call>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</tool_call>") {
            Some(end) => rest = &rest[start + end + "</tool_call>".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// Split a markdown draft into sections on `## ` headings.
pub fn split_sections(markdown: &str) -> Vec<ReportSection> {
    let mut sections: Vec<ReportSection> = Vec::new();
    let mut current: Option<ReportSection> = None;
    for line in markdown.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            current = Some(ReportSection { title: title.trim().to_string(), content: String::new() });
        } else if let Some(sec) = current.as_mut() {
            sec.content.push_str(line);
            sec.content.push('\n');
        }
    }
    if let Some(sec) = current.take() {
        sections.push(sec);
    }
    for sec in &mut sections {
        sec.content = sec.content.trim().to_string();
    }
    if sections.is_empty() && !markdown.trim().is_empty() {
        sections.push(ReportSection { title: "Report".into(), content: markdown.trim().to_string() });
    }
    sections
}

// ---- generation ----

/// Register a new report and spawn its generation task. Returns the report_id.
pub fn start(st: &AppState, simulation_id: String, db_path: String, topic: String) -> String {
    let report_id = format!("report_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    registry::insert(ReportEntry::new(report_id.clone(), simulation_id, db_path, topic));

    let st = st.clone();
    let id = report_id.clone();
    tokio::spawn(async move {
        registry::agent_log(&id, "report_start", "pending", json!({}));
        if let Err(e) = generate(&st, &id).await {
            tracing::error!("report {id} generation failed: {e:#}");
            registry::update(&id, |r| {
                r.status = JobStatus::Failed;
                r.progress = -1;
                r.error = Some(format!("{e:#}"));
                r.message = format!("Report generation failed: {e:#}");
            });
            registry::agent_log(&id, "report_failed", "failed", json!({"error": format!("{e:#}")}));
            registry::console_log(&id, "ERROR", &format!("Report generation failed: {e:#}"));
        }
    });
    report_id
}

async fn generate(st: &AppState, report_id: &str) -> anyhow::Result<()> {
    let entry = registry::get(report_id).ok_or_else(|| anyhow::anyhow!("report vanished"))?;
    let store = Store::open(Path::new(&entry.db_path))?;
    let topic = entry.topic.clone();

    registry::set_progress(report_id, JobStatus::Running, 5, "Gathering baseline statistics...");

    // Deterministic grounding: the model starts from real numbers, not guesses.
    let baseline = execute_tool(&store, "statistics", &json!({}))?;
    registry::agent_log(report_id, "tool_result", "gathering", json!({"tool": "statistics", "auto": true}));

    // Phase 1: bounded evidence-gathering tool loop.
    let max_calls = st.cfg.report_max_tool_calls as usize;
    let system = format!(
        "You are an opinion-analysis researcher preparing to write a report about a social \
         simulation. Simulation topic: {topic}\n\nYour job right now is ONLY to gather evidence \
         from the simulation data using tools. Look for: dominant stances and their arguments, \
         sentiment shifts across rounds, influential users, and notable or risky posts.\n\n{tools}",
        tools = tools_description()
    );
    let mut messages = vec![
        Msg::system(system),
        Msg::user(format!(
            "Baseline statistics from the simulation:\n{baseline}\n\nGather any further evidence \
             you need (you have {max_calls} tool calls), then reply DONE."
        )),
    ];
    let mut evidence = vec![format!("═══ statistics ═══\n{baseline}")];
    let mut calls_used = 0usize;

    while calls_used < max_calls {
        registry::set_progress(
            report_id,
            JobStatus::Running,
            10 + (40 * calls_used / max_calls.max(1)) as i32,
            &format!("Gathering evidence ({calls_used}/{max_calls} tool calls)"),
        );
        let response = st.llm().chat(&messages, 0.3, 2048).await?;
        registry::agent_log(
            report_id,
            "llm_turn",
            "gathering",
            json!({"response": trim(&response, 1000), "tool_calls_used": calls_used}),
        );
        let tool_calls = parse_tool_calls(&response);
        if tool_calls.is_empty() {
            registry::console_log(report_id, "INFO", "Evidence gathering complete");
            break;
        }
        let mut observations = String::new();
        for call in tool_calls {
            if calls_used >= max_calls {
                break;
            }
            let name = call["name"].as_str().unwrap_or("").to_string();
            let params = call["parameters"].clone();
            registry::agent_log(report_id, "tool_call", "gathering", json!({"tool": name, "parameters": params}));
            registry::console_log(report_id, "INFO", &format!("tool call: {name} {params}"));
            // web_search is async (HTTP); the store tools are sync. Dispatch here.
            let result = if name == "web_search" {
                let query = params["query"].as_str().unwrap_or("");
                let max_results = params["max_results"].as_u64().unwrap_or(5) as usize;
                crate::services::search::web_search(&st.cfg.tavily_api_key, query, max_results)
                    .await
                    .unwrap_or_else(|e| format!("Tool failed: {e}"))
            } else {
                execute_tool(&store, &name, &params).unwrap_or_else(|e| format!("Tool failed: {e}"))
            };
            let result = trim(&result, OBSERVATION_LIMIT);
            registry::agent_log(
                report_id,
                "tool_result",
                "gathering",
                json!({"tool": name, "result_preview": trim(&result, 500)}),
            );
            observations.push_str(&format!("═══ {name} ═══\n{result}\n\n"));
            calls_used += 1;
        }
        evidence.push(observations.clone());
        messages.push(Msg::assistant(response));
        messages.push(Msg::user(format!(
            "Observation:\n{observations}\nTool calls used: {calls_used}/{max_calls}. \
             Emit more <tool_call> blocks if needed, or reply DONE."
        )));
    }

    // Phase 2: synthesis with the boost model.
    registry::set_progress(report_id, JobStatus::Running, 60, "Writing report sections...");
    let evidence_text = trim(&evidence.join("\n\n"), EVIDENCE_LIMIT);
    let section_list = SECTION_TITLES
        .iter()
        .map(|t| format!("## {t}"))
        .collect::<Vec<_>>()
        .join("\n");
    let writer_system = format!(
        "You are an expert public-opinion analyst writing a simulation analysis report.\n\
         Simulation topic: {topic}\n\n\
         Write the report in Markdown with EXACTLY these section headings, in this order:\n{section_list}\n\n\
         Rules:\n\
         - Every claim must be grounded in the evidence provided; cite numbers from it.\n\
         - Quote representative posts as standalone `>` blockquotes, attributed to their user_name.\n\
         - Do not invent data. If the evidence is insufficient for a section, say so explicitly.\n\
         - No headings other than the six `##` sections above. Use **bold** for emphasis.\n\
         - Write in English."
    );
    let mut draft = st
        .llm_boost()
        .chat(
            &[
                Msg::system(writer_system.clone()),
                Msg::user(format!("Evidence gathered from the simulation:\n\n{evidence_text}\n\nWrite the full report now.")),
            ],
            st.cfg.report_temperature,
            8192,
        )
        .await?;
    registry::agent_log(report_id, "draft_complete", "writing", json!({"length": draft.len()}));

    // Phase 3: bounded reflection passes.
    for round in 0..st.cfg.report_max_reflection_rounds {
        registry::set_progress(
            report_id,
            JobStatus::Running,
            75 + (15 * round / st.cfg.report_max_reflection_rounds.max(1)) as i32,
            &format!("Reflection pass {}/{}", round + 1, st.cfg.report_max_reflection_rounds),
        );
        let critique = st
            .llm()
            .chat(
                &[
                    Msg::system(
                        "You are a strict reviewer of data-analysis reports. Check the draft against \
                         the evidence: flag ungrounded claims, numbers that contradict the evidence, \
                         missing required sections, and non-English text. If the draft is acceptable, \
                         reply with exactly APPROVED. Otherwise list the concrete issues."
                            .to_string(),
                    ),
                    Msg::user(format!("Evidence:\n{evidence_text}\n\nDraft report:\n{draft}")),
                ],
                0.2,
                1024,
            )
            .await?;
        registry::agent_log(
            report_id,
            "reflection",
            "reflecting",
            json!({"round": round + 1, "critique": trim(&critique, 1000)}),
        );
        if critique.trim().to_uppercase().starts_with("APPROVED") {
            registry::console_log(report_id, "INFO", "Reflection: draft approved");
            break;
        }
        draft = st
            .llm_boost()
            .chat(
                &[
                    Msg::system(writer_system.clone()),
                    Msg::user(format!(
                        "Evidence gathered from the simulation:\n\n{evidence_text}\n\n\
                         Previous draft:\n{draft}\n\nReviewer feedback:\n{critique}\n\n\
                         Rewrite the full report fixing every issue. Keep the exact same six section headings."
                    )),
                ],
                st.cfg.report_temperature,
                8192,
            )
            .await?;
        registry::agent_log(report_id, "revision_complete", "reflecting", json!({"round": round + 1, "length": draft.len()}));
    }

    let sections = split_sections(&draft);
    registry::update(report_id, |r| {
        r.sections = sections;
        r.markdown_content = draft.clone();
        r.status = JobStatus::Completed;
        r.progress = 100;
        r.message = "Report generation completed".into();
        r.completed_at = Some(chrono::Utc::now().to_rfc3339());
    });
    registry::agent_log(report_id, "report_complete", "completed", json!({"sections": SECTION_TITLES.len()}));
    registry::console_log(report_id, "INFO", "Report generation completed");
    Ok(())
}

// ---- follow-up chat grounded in a generated report ----

pub async fn chat(
    st: &AppState,
    entry: &ReportEntry,
    message: &str,
    history: &[Value],
) -> AppResult<Value> {
    let report_content = if entry.markdown_content.is_empty() {
        "(No report content available yet)".to_string()
    } else {
        trim(&entry.markdown_content, REPORT_CONTEXT_LIMIT)
    };
    let system = format!(
        "You are a concise simulation-analysis assistant.\n\n\
         Simulation topic: {topic}\n\n[Generated analysis report]\n{report_content}\n\n\
         Rules:\n\
         - Answer from the report above first; call tools only if it is insufficient (max {MAX_CHAT_TOOL_CALLS} calls).\n\
         - Be direct: conclusion first, then brief support. Quote the report with `>`.\n\
         - Answer in English.\n\n{tools}",
        topic = entry.topic,
        tools = tools_description()
    );

    let mut messages = vec![Msg::system(system)];
    for h in history.iter().rev().take(10).rev() {
        let role = h["role"].as_str().unwrap_or("user");
        let content = h["content"].as_str().unwrap_or("").to_string();
        messages.push(match role {
            "assistant" => Msg::assistant(content),
            _ => Msg::user(content),
        });
    }
    messages.push(Msg::user(message.to_string()));

    let store = Store::open(Path::new(&entry.db_path)).map_err(AppError::Other)?;
    let mut tool_calls_made: Vec<Value> = Vec::new();

    for _ in 0..MAX_CHAT_TOOL_CALLS {
        let response = st.llm().chat(&messages, 0.5, 2048).await.map_err(AppError::Other)?;
        let tool_calls = parse_tool_calls(&response);
        if tool_calls.is_empty() || tool_calls_made.len() >= MAX_CHAT_TOOL_CALLS {
            return Ok(chat_result(&response, &tool_calls_made));
        }
        let mut observations = String::new();
        for call in tool_calls.into_iter().take(1) {
            let name = call["name"].as_str().unwrap_or("").to_string();
            let result = execute_tool(&store, &name, &call["parameters"])
                .unwrap_or_else(|e| format!("Tool failed: {e}"));
            observations.push_str(&format!("[{name} result]\n{}\n", trim(&result, 1500)));
            tool_calls_made.push(call);
        }
        messages.push(Msg::assistant(response));
        messages.push(Msg::user(format!("{observations}\nAnswer the question concisely in English.")));
    }

    let final_response = st.llm().chat(&messages, 0.5, 2048).await.map_err(AppError::Other)?;
    Ok(chat_result(&final_response, &tool_calls_made))
}

fn chat_result(response: &str, tool_calls_made: &[Value]) -> Value {
    json!({
        "response": strip_tool_calls(response),
        "tool_calls": tool_calls_made,
        "sources": tool_calls_made
            .iter()
            .map(|tc| tc["parameters"]["query"].as_str().unwrap_or("").to_string())
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("popinion-report-{name}-{}", std::process::id()));
        let path = dir.join("s.db");
        std::fs::remove_file(&path).ok();
        let s = Store::open(&path).unwrap();
        s.add_user(1, "Alice", "alice", "", "activist").unwrap();
        s.add_user(2, "Bob", "bob", "", "skeptic").unwrap();
        let p1 = s.add_post(1, "I fully support the new climate policy", 0, Some("support"), Some(0.8)).unwrap();
        s.add_post(2, "This climate policy will ruin the economy", 1, Some("oppose"), Some(-0.6)).unwrap();
        s.add_post(1, "Everyone should read the climate policy details", 2, Some("support"), Some(0.5)).unwrap();
        s.add_comment(p1, 2, "I disagree strongly", 0, Some("oppose"), Some(-0.4)).unwrap();
        s.like_post(p1, 2, false).unwrap();
        (s, dir)
    }

    #[test]
    fn search_posts_finds_keyword_matches() {
        let (store, dir) = seeded_store("search");
        let out = execute_tool(&store, "search_posts", &json!({"query": "climate policy", "limit": 5})).unwrap();
        assert!(out.contains("climate"));
        let hits: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(hits.as_array().unwrap().len(), 3);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn statistics_reports_counts_and_trend() {
        let (store, dir) = seeded_store("stats");
        let out = execute_tool(&store, "statistics", &json!({})).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["total_posts"], 3);
        assert_eq!(v["round_max"], 2);
        assert_eq!(v["sentiment_by_round"].as_array().unwrap().len(), 3);
        assert_eq!(v["top_posts_by_likes"][0]["num_likes"], 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn stance_distribution_tool_works() {
        let (store, dir) = seeded_store("stance");
        let out = execute_tool(&store, "stance_distribution", &json!({})).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v[0]["stance"], "support");
        assert_eq!(v[0]["count"], 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unknown_tool_is_reported_not_fatal() {
        let (store, dir) = seeded_store("unknown");
        let out = execute_tool(&store, "insight_forge", &json!({})).unwrap();
        assert!(out.contains("Unknown tool"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn parses_tool_call_blocks() {
        let response = r#"Thought: need data.
<tool_call>
{"name": "search_posts", "parameters": {"query": "policy", "limit": 5}}
</tool_call>
and also
<tool_call>
{"name": "statistics", "parameters": {}}
</tool_call>"#;
        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["name"], "search_posts");
        assert_eq!(calls[0]["parameters"]["limit"], 5);
        assert_eq!(calls[1]["name"], "statistics");
        assert!(parse_tool_calls("no calls here").is_empty());
    }

    #[test]
    fn strips_tool_calls_from_reply() {
        let cleaned = strip_tool_calls("Answer part one <tool_call>{\"name\":\"x\"}</tool_call> part two");
        assert_eq!(cleaned, "Answer part one  part two");
    }

    #[test]
    fn splits_markdown_into_sections() {
        let md = "# Title\n\n## Executive Summary\n\nSummary text.\n\n## Risks & Outlook\n\nRisk text.\n";
        let sections = split_sections(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "Executive Summary");
        assert_eq!(sections[0].content, "Summary text.");
        assert_eq!(sections[1].title, "Risks & Outlook");

        let fallback = split_sections("just prose, no headings");
        assert_eq!(fallback.len(), 1);
        assert_eq!(fallback[0].title, "Report");
    }
}
