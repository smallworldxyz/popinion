//! Report agent: a bounded LLM tool-calling loop over the simulation social
//! store, followed by a boosted-model synthesis pass and reflection rounds.
//!
//! Uses a ReACT loop with a section writer and reflection, grounding the tools
//! in `sim::store::Store` instead of Neo4j, and produces a fixed five-section
//! analyst report (Executive Summary, Sentiment Breakdown, Theme Analysis,
//! Trend Alert, Recommendations) instead of an LLM-planned outline, so every
//! report has a predictable shape the frontend can render.

use crate::error::{AppError, AppResult};
use crate::llm::Msg;
use crate::sim::store::Store;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::Path;

use super::registry::{self, ReportEntry, ReportSection};
use crate::services::registry::JobStatus;

pub const SECTION_TITLES: [&str; 5] = [
    "Executive Summary",
    "Sentiment Breakdown",
    "Theme Analysis",
    "Trend Alert",
    "Recommendations",
];

const MAX_CHAT_TOOL_CALLS: usize = 2;
const OBSERVATION_LIMIT: usize = 4000;
const EVIDENCE_LIMIT: usize = 24000;
const REPORT_CONTEXT_LIMIT: usize = 15000;

// ---- tools over the simulation store ----

fn tools_description() -> &'static str {
    r#"Available tools (query the simulation's social data; every claim in the report must be grounded in tool results):
- search_posts: find posts AND comments (replies) matching keywords - the population often voices its opinion in replies, so this covers both. parameters: {"query": "keywords", "limit": 10}
- statistics: overall numbers - post and comment counts, both stance readings (see stance_distribution), most-liked items, round range. parameters: {}
- stance_distribution: THREE readings of the same run. `by_grounded_agent` counts ONCE per agent compiled from a real entity in the source material - this is the ONLY one to quote as the headline distribution. `by_agent` adds the synthetic agents, which were constructed during setup to populate each camp and are therefore not evidence of anyone's opinion. `by_volume` counts every post and comment, so a few prolific agents can dominate it; use it ONLY to say how loud a camp was. parameters: {}
- constituency_map: WHO holds which position - each named actor, the camp it was compiled into, where it ended, and whether it changed. Also reports how many agents changed position at all. This is the report's most defensible content: the graph found these constituencies, and the percentages are only a count of them. parameters: {}
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
            // Posts AND comments: the population usually voices its opinion by
            // replying to the seed, so a posts-only search returns almost nothing.
            let content = store.all_content(2000)?;
            let keywords: Vec<&str> = query.split_whitespace().collect();
            let mut scored: Vec<(usize, &Value)> = content
                .iter()
                .map(|p| {
                    let text = p["content"].as_str().unwrap_or("").to_lowercase();
                    let score = keywords.iter().filter(|k| text.contains(**k)).count();
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
                        "kind": p["kind"],
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
                return Ok(format!("No posts or comments matched query: {query}"));
            }
            Ok(serde_json::to_string_pretty(&hits)?)
        }
        "statistics" => {
            let total_posts = store.count_posts()?;
            let total_comments = store.count_comments()?;
            // One agent, one vote: the population reading, and the same measure
            // /validate and /compare use, so the noise floor bounds the number
            // the report actually quotes.
            let by_grounded = store.grounded_stance_distribution()?;
            let by_agent = store.agent_stance_distribution()?;
            // Volume, kept for "how loud was each camp" - not a share of opinion.
            let by_volume = store.content_stance_distribution()?;
            let content = store.all_content(2000)?;
            let mut by_likes: Vec<&Value> = content.iter().collect();
            by_likes.sort_by_key(|p| -p["num_likes"].as_i64().unwrap_or(0));
            let top: Vec<Value> = by_likes
                .iter()
                .take(5)
                .map(|p| {
                    json!({
                        "kind": p["kind"],
                        "user_name": p["user_name"],
                        "num_likes": p["num_likes"],
                        "stance": p["stance"],
                        "content": trim(p["content"].as_str().unwrap_or(""), 200),
                    })
                })
                .collect();
            let rounds: Vec<i64> = content.iter().filter_map(|p| p["round"].as_i64()).collect();
            // Per-round average sentiment gives the writer a real trend line.
            let mut per_round: std::collections::BTreeMap<i64, (f64, usize)> = Default::default();
            for p in &content {
                if let (Some(r), Some(s)) = (p["round"].as_i64(), p["sentiment"].as_f64()) {
                    let e = per_round.entry(r).or_insert((0.0, 0));
                    e.0 += s;
                    e.1 += 1;
                }
            }
            let sentiment_by_round: Vec<Value> = per_round
                .into_iter()
                .map(|(r, (sum, n))| json!({"round": r, "avg_sentiment": sum / n as f64, "items": n}))
                .collect();
            Ok(serde_json::to_string_pretty(&json!({
                "total_posts": total_posts,
                "total_comments": total_comments,
                "stance_by_grounded_agent": by_grounded,
                "stance_by_agent": by_agent,
                "stance_by_volume": by_volume,
                "round_min": rounds.iter().min(),
                "round_max": rounds.iter().max(),
                "sentiment_by_round": sentiment_by_round,
                "top_by_likes": top,
            }))?)
        }
        "constituency_map" => Ok(serde_json::to_string_pretty(&store.constituency_map()?)?),
        "stance_distribution" => Ok(serde_json::to_string_pretty(&json!({
            "by_grounded_agent": store.grounded_stance_distribution()?,
            "by_agent": store.agent_stance_distribution()?,
            "by_volume": store.content_stance_distribution()?,
        }))?),
        _ => Ok(format!(
            "Unknown tool: {name}. Available: search_posts, statistics, stance_distribution, constituency_map"
        )),
    }
}

/// Dispatch one advertised tool. `web_search` is async (HTTP); the store
/// tools are sync — both the generate loop and chat() route through here so
/// every tool in `tools_description()` is actually servable.
async fn dispatch_tool(st: &AppState, store: &Store, name: &str, params: &Value) -> String {
    if name == "web_search" {
        let query = params["query"].as_str().unwrap_or("");
        let max_results = params["max_results"].as_u64().unwrap_or(5) as usize;
        crate::services::search::web_search(&st.cfg.tavily_api_key, query, max_results)
            .await
            .unwrap_or_else(|e| format!("Tool failed: {e}"))
    } else {
        execute_tool(store, name, params).unwrap_or_else(|e| format!("Tool failed: {e}"))
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

/// Identifies the population a run was prepared with. Two runs are siblings —
/// and so comparable for a noise floor — only when they share one exactly.
fn roster_fingerprint(personas: &[crate::sim::agent::Persona]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    personas.len().hash(&mut h);
    for p in personas {
        p.user_id.hash(&mut h);
        p.name.hash(&mut h);
        p.synthetic.hash(&mut h);
    }
    h.finish()
}

/// What precision has a run earned? Without sibling runs nobody knows how far
/// the number moves on identical inputs, so a decimal place asserts a stability
/// that was never measured. This is the only place that decides it.
fn precision_rule(measured: Option<(f64, usize)>) -> String {
    match measured {
        Some((floor, n)) => {
            let pts = floor * 100.0;
            format!(
                "This run was repeated {n} times. The stance distribution moved by {pts:.0} \
                 percentage points on average between identical runs, so that is the \
                 measurement's wobble. Round every percentage to the nearest whole number, give \
                 it as `X% (+/-{pts:.0} pts, {n} runs)` the first time you state it, and never \
                 call a gap smaller than {:.0} points a difference - it is indistinguishable \
                 from noise.",
                pts * 1.5
            )
        }
        None => "This run has NOT been repeated, so how much the numbers move between identical \
             runs is unmeasured. Give every percentage as a whole number - never a decimal, \
             which would assert a precision nobody has checked - and state once, plainly, that \
             the figures come from a single unrepeated run and their margin is unknown."
            .to_string(),
    }
}

/// Register a new report and spawn its generation task. Returns the report_id.
pub fn start(
    st: &AppState,
    simulation_id: String,
    db_path: String,
    topic: String,
    rerun_ids: Vec<String>,
) -> String {
    let report_id = format!("report_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    let sim_id = simulation_id.clone();
    registry::insert(ReportEntry::new(report_id.clone(), simulation_id, db_path, topic));

    // How far the distribution moves between identical runs. Measured before
    // the writer starts, so it can be told what precision its numbers deserve.
    if !rerun_ids.is_empty() {
        let mgr = st.sim_manager();
        let shares = |id: &str| {
            mgr.store(id)
                .and_then(|s| s.grounded_stance_distribution())
                .map(|d| crate::sim::validate::stance_shares(&d))
                .ok()
        };
        // A floor is how far ONE population wanders between identical runs.
        // Measured across two different populations it is just the distance
        // between two unrelated answers, and would license far more precision
        // than the run has earned. Siblings share a persona roster exactly.
        let roster = |id: &str| mgr.load_personas(id).ok().map(|p| roster_fingerprint(&p));
        let baseline_roster = roster(&sim_id);
        let siblings: Vec<&String> = rerun_ids
            .iter()
            .filter(|id| {
                let same = roster(id) == baseline_roster && baseline_roster.is_some();
                if !same {
                    tracing::warn!("report {report_id}: ignoring rerun {id}, different population");
                }
                same
            })
            .collect();

        let mut runs: Vec<_> = shares(&sim_id).into_iter().collect();
        runs.extend(siblings.iter().filter_map(|id| shares(id)));
        if runs.len() >= 2 {
            let floor = crate::sim::validate::noise_floor(&runs);
            let n = runs.len();
            registry::update(&report_id, |r| {
                r.noise_floor = Some(floor);
                r.validated_runs = n;
            });
            tracing::info!("report {report_id}: noise floor {floor:.3} across {n} runs");
        }
    }

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
        // Persist on success so a restart doesn't cost a regeneration. A failure
        // writes nothing, leaving any previously saved report reachable.
        registry::persist(&id, &st.sim_manager().report_path(&sim_id));
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
            let result = dispatch_tool(st, &store, &name, &params).await;
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
    let precision_rule =
        precision_rule(registry::get(report_id).and_then(|r| r.noise_floor.map(|f| (f, r.validated_runs))));

    let writer_system = format!(
        "You are an expert public-opinion analyst. Analyze the simulated public discussion about \
         \"{topic}\" to determine sentiment, identify key themes, and detect emerging trends.\n\n\
         What this data is, and is not: entities were extracted from source material, compiled into \
         camps from the positions they took there, and then simulated. The population is therefore \
         whoever the source material named - not a sample of any real public - and each agent's \
         starting camp was decided before the run. Never call the result a poll, a survey, or the \
         opinion of a country or its citizens. It is the balance of the constituencies this source \
         described, after they argued with each other.\n\n\
         Data source: a Popinion multi-agent simulation - AI personas grounded in real evidence, each \
         posting or replying with a recorded STANCE toward the topic (support / oppose / neutral) and a \
         SENTIMENT value from -1 to 1. Read stance as the primary opinion signal (support = favorable, \
         oppose = unfavorable, neutral = undecided/mixed) and the sentiment value as emotional intensity. \
         The discussion runs over rounds; treat later rounds as more recent.\n\n\
         PRECISION: {precision_rule}\n\n\
         Write the report in Markdown with EXACTLY these five section headings, in this order:\n{section_list}\n\n\
         Section requirements:\n\
         - Executive Summary: LEAD with WHICH constituencies hold which position and on what stated \
           grounds, taken from constituency_map - that is what this system actually observed. The \
           overall percentage comes second and is a count of those constituencies, not a poll of a \
           population. If `changed_position` is 0 or near it, say so plainly in this section: the \
           agents ended where they were compiled to start, so the run described its own setup and \
           the distribution should be read as a map of the source material rather than a finding.\n\
         - Sentiment Breakdown: the distribution across Favorable, Unfavorable, Neutral (and Mixed if warranted) \
           with PERCENTAGES computed from `by_grounded_agent` ONLY - one vote per agent that was compiled \
           from a real entity in the source material. State that count explicitly. NEVER compute the headline \
           percentages from `by_volume` (a few prolific agents dominate it, so it says who talked most) nor \
           from `by_agent` (it includes synthetic agents built during setup to populate each camp, so their \
           stance was decided before the run began, not by it). You MUST add one sentence disclosing how many \
           synthetic agents took part and that they are excluded from the percentages. You may separately note \
           that a camp was louder or larger than its grounded numbers, citing the other readings by name. \
           For each category name the top 2-3 emotional \
           drivers (anger, fear, hope, satisfaction, distrust, etc.) inferred from the actual arguments.\n\
         - Theme Analysis: extract the top ~5 recurring themes as a Markdown TABLE with columns \
           `Theme | Sentiment Association | Key Keywords`, where sentiment association is mostly favorable, \
           mostly unfavorable, or divided.\n\
         - Trend Alert: describe any significant shift in sentiment/stance across the rounds (use the per-round \
           trend), and flag minority or outlier positions and any emerging narrative that contradicts the majority.\n\
         - Recommendations: exactly 3 actionable insights for stakeholders, each a **bold lead-in** followed by \
           one sentence.\n\n\
         Rules:\n\
         - Every claim must be grounded in the evidence provided; cite the numbers.\n\
         - Quote 1-2 representative posts for the major stances as `>` blockquotes, attributed to their user_name.\n\
         - Bias check: conversation volume is not the same as number of people - a few hyperactive agents can \
           dominate the post count, so do not amplify extreme or repetitive voices disproportionately, and note \
           sampling/representativeness caveats where relevant.\n\
         - Do not invent data. If the evidence is insufficient for a section, say so explicitly.\n\
         - No headings other than the five `##` sections above. Use **bold** for emphasis. Write in English."
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
                         Rewrite the full report fixing every issue. Keep the exact same five section headings."
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
            let result = dispatch_tool(st, &store, &name, &call["parameters"]).await;
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
    use crate::sim::store::NewUser;

    fn seeded_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("popinion-report-{name}-{}", std::process::id()));
        let path = dir.join("s.db");
        std::fs::remove_file(&path).ok();
        let s = Store::open(&path).unwrap();
        s.add_user(NewUser { user_id: 1, name: "Alice", user_name: "alice", bio: "", persona: "activist", synthetic: false, faction: None }).unwrap();
        s.add_user(NewUser { user_id: 2, name: "Bob", user_name: "bob", bio: "", persona: "skeptic", synthetic: false, faction: None }).unwrap();
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
        // The reply is counted now — the opinion often lives in comments.
        assert_eq!(v["total_comments"], 1);
        assert_eq!(v["round_max"], 2);
        assert_eq!(v["sentiment_by_round"].as_array().unwrap().len(), 3);
        assert_eq!(v["top_by_likes"][0]["num_likes"], 1);
        std::fs::remove_dir_all(dir).ok();
    }

    fn count_of(rows: &Value, stance: &str) -> Option<i64> {
        rows.as_array()?.iter().find(|r| r["stance"] == stance).map(|r| r["count"].as_i64().unwrap())
    }

    #[test]
    fn stance_distribution_tool_works() {
        let (store, dir) = seeded_store("stance");
        let out = execute_tool(&store, "stance_distribution", &json!({})).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        // 2 support posts, and oppose = 1 post + 1 comment: the comment is
        // included, so a posts-only distribution (which would show oppose=1) is wrong.
        assert_eq!(count_of(&v["by_volume"], "support"), Some(2));
        assert_eq!(count_of(&v["by_volume"], "oppose"), Some(2));
        // Two agents, one each way, regardless of how much either wrote.
        assert_eq!(count_of(&v["by_agent"], "support"), Some(1));
        assert_eq!(count_of(&v["by_agent"], "oppose"), Some(1));
        std::fs::remove_dir_all(dir).ok();
    }

    /// The map is what the system can defend, and it also exposes the central
    /// risk: factions are assigned at compile time and the distribution is read
    /// back out of the run, so a run where nobody moves has restated its setup.
    #[test]
    fn the_constituency_map_reports_who_moved_and_who_did_not() {
        let dir = std::env::temp_dir().join(format!("popinion-report-map-{}", std::process::id()));
        let path = dir.join("s.db");
        std::fs::remove_file(&path).ok();
        let s = Store::open(&path).unwrap();

        // Compiled as an opponent, argued as one: the setup speaking.
        s.add_user(NewUser { user_id: 1, name: "Residents", user_name: "res", bio: "", persona: "", synthetic: false, faction: Some("con") }).unwrap();
        s.add_post(1, "Still against it", 0, Some("oppose"), Some(-0.5)).unwrap();
        // Compiled as an opponent, ended supporting: the run found something.
        s.add_user(NewUser { user_id: 2, name: "Council", user_name: "council", bio: "", persona: "", synthetic: false, faction: Some("con") }).unwrap();
        s.add_post(2, "Against for now", 0, Some("oppose"), Some(-0.3)).unwrap();
        s.add_post(2, "Persuaded, actually", 3, Some("support"), Some(0.4)).unwrap();
        // Constructed, and with no prior at all.
        s.add_user(NewUser { user_id: 3, name: "Supporter", user_name: "sup", bio: "", persona: "", synthetic: true, faction: Some("pro") }).unwrap();
        s.add_post(3, "For it", 0, Some("support"), Some(0.6)).unwrap();
        s.add_user(NewUser { user_id: 4, name: "Observer", user_name: "obs", bio: "", persona: "", synthetic: false, faction: None }).unwrap();
        s.add_post(4, "Unsure", 0, Some("neutral"), Some(0.0)).unwrap();

        let m: Value = serde_json::from_str(&execute_tool(&s, "constituency_map", &json!({})).unwrap()).unwrap();
        assert_eq!(m["agents_with_a_prior"], 3, "the observer had no camp to depart from");
        assert_eq!(m["changed_position"], 1, "only the council moved");

        let member = |name: &str| {
            m["members"].as_array().unwrap().iter().find(|x| x["name"] == name).unwrap().clone()
        };
        assert_eq!(member("Council")["changed_position"], json!(true));
        assert_eq!(member("Council")["ended_at"], json!("support"));
        assert_eq!(member("Residents")["changed_position"], json!(false));
        assert_eq!(member("Supporter")["grounded"], json!(false), "provenance is visible per member");
        assert_eq!(member("Observer")["compiled_into"], json!(null));
        std::fs::remove_dir_all(dir).ok();
    }

    /// A floor is how far ONE population wanders between identical runs. Taken
    /// across two different populations it is the distance between unrelated
    /// answers, and would license precision the run never earned.
    #[test]
    fn only_an_identical_roster_counts_as_a_sibling_run() {
        use crate::sim::agent::Persona;
        // Built from JSON so the test states only what the fingerprint reads;
        // Persona has no Default and does not need one for this.
        let p = |id: i64, name: &str, synthetic: bool| -> Persona {
            serde_json::from_value(json!({
                "user_id": id, "user_name": name, "name": name, "synthetic": synthetic
            }))
            .unwrap()
        };
        let baseline = vec![p(0, "Ministry", false), p(1, "Residents", false)];

        assert_eq!(
            roster_fingerprint(&baseline),
            roster_fingerprint(&[p(0, "Ministry", false), p(1, "Residents", false)]),
            "a rerun of the same prepared population"
        );
        assert_ne!(
            roster_fingerprint(&baseline),
            roster_fingerprint(&[p(0, "Ministry", false)]),
            "a different roster size"
        );
        assert_ne!(
            roster_fingerprint(&baseline),
            roster_fingerprint(&[p(0, "Ministry", false), p(1, "Retailers", false)]),
            "same size, different members"
        );
        assert_ne!(
            roster_fingerprint(&baseline),
            roster_fingerprint(&[p(0, "Ministry", false), p(1, "Residents", true)]),
            "same names but one is now constructed"
        );
    }

    /// An unrepeated run has no measured wobble, so a decimal place in its
    /// percentages claims a stability nobody checked. "49.1%" was exactly that.
    #[test]
    fn an_unrepeated_run_is_denied_decimal_precision() {
        let rule = precision_rule(None);
        assert!(rule.contains("NOT been repeated"));
        assert!(rule.contains("never a decimal"));
        assert!(rule.contains("margin is unknown"));
    }

    /// With siblings the wobble is a number, so the report may state it — and
    /// must stop calling sub-noise gaps differences.
    #[test]
    fn a_measured_floor_sets_the_rounding_and_the_significance_bar() {
        let rule = precision_rule(Some((0.04, 3)));
        assert!(rule.contains("repeated 3 times"), "states how many runs backed it");
        assert!(rule.contains("4 percentage points"), "states the measured wobble");
        assert!(rule.contains("+/-4 pts, 3 runs"), "gives the format to report it in");
        assert!(rule.contains("6 points"), "significance bar is 1.5x the floor");
    }

    /// Synthetic agents are built during setup to populate each camp, so their
    /// stance was decided before the run started. Counting them can invert the
    /// answer: here the real entities oppose 3 to 1, and the constructed public
    /// turns that into a majority in favour.
    #[test]
    fn synthetic_agents_can_invert_the_reading_and_are_excluded_from_it() {
        let dir = std::env::temp_dir().join(format!("popinion-report-synth-{}", std::process::id()));
        let path = dir.join("s.db");
        std::fs::remove_file(&path).ok();
        let s = Store::open(&path).unwrap();

        s.add_user(NewUser { user_id: 1, name: "Ministry", user_name: "ministry", bio: "", persona: "official", synthetic: false, faction: None }).unwrap();
        s.add_post(1, "We back it", 0, Some("support"), Some(0.6)).unwrap();
        for uid in 2..5 {
            s.add_user(NewUser { user_id: uid, name: "Residents", user_name: &format!("res{uid}"), bio: "", persona: "resident", synthetic: false, faction: None }).unwrap();
            s.add_post(uid, "We are against it", 0, Some("oppose"), Some(-0.5)).unwrap();
        }
        for uid in 10..16 {
            s.add_user(NewUser { user_id: uid, name: "Supporter", user_name: &format!("sup{uid}"), bio: "", persona: "citizen", synthetic: true, faction: None }).unwrap();
            s.add_post(uid, "Sounds good to me", 0, Some("support"), Some(0.5)).unwrap();
        }

        let v: Value =
            serde_json::from_str(&execute_tool(&s, "stance_distribution", &json!({})).unwrap()).unwrap();
        // Everyone: support leads 7 to 3.
        assert_eq!(count_of(&v["by_agent"], "support"), Some(7));
        assert_eq!(count_of(&v["by_agent"], "oppose"), Some(3));
        // Real entities only: opposition leads 3 to 1. Opposite conclusion.
        assert_eq!(count_of(&v["by_grounded_agent"], "support"), Some(1));
        assert_eq!(count_of(&v["by_grounded_agent"], "oppose"), Some(3));

        let stats: Value =
            serde_json::from_str(&execute_tool(&s, "statistics", &json!({})).unwrap()).unwrap();
        assert!(stats["stance_by_grounded_agent"].is_array(), "the writer gets the headline reading");
        std::fs::remove_dir_all(dir).ok();
    }

    /// The reason the report must quote `by_agent`: volume answers "who talked
    /// most", not "what does the population think". One tireless supporter
    /// against three quiet opponents reads as 77% support by volume and 25% by
    /// head count — and only the latter is what /validate and /compare measure.
    #[test]
    fn volume_and_agent_readings_diverge_when_one_agent_dominates() {
        let dir = std::env::temp_dir().join(format!("popinion-report-loud-{}", std::process::id()));
        let path = dir.join("s.db");
        std::fs::remove_file(&path).ok();
        let s = Store::open(&path).unwrap();
        s.add_user(NewUser { user_id: 1, name: "Loud", user_name: "loud", bio: "", persona: "activist", synthetic: false, faction: None }).unwrap();
        for r in 0..10 {
            s.add_post(1, "Backing this all the way", r, Some("support"), Some(0.7)).unwrap();
        }
        for uid in 2..5 {
            s.add_user(NewUser { user_id: uid, name: "Quiet", user_name: &format!("quiet{uid}"), bio: "", persona: "resident", synthetic: false, faction: None }).unwrap();
            s.add_post(uid, "I am against it", 0, Some("oppose"), Some(-0.5)).unwrap();
        }

        let v: Value =
            serde_json::from_str(&execute_tool(&s, "stance_distribution", &json!({})).unwrap()).unwrap();
        assert_eq!(count_of(&v["by_volume"], "support"), Some(10));
        assert_eq!(count_of(&v["by_volume"], "oppose"), Some(3));
        assert_eq!(count_of(&v["by_agent"], "support"), Some(1));
        assert_eq!(count_of(&v["by_agent"], "oppose"), Some(3));

        // Both readings must reach the writer, so it can report the population
        // share and still say one camp was louder.
        let stats: Value =
            serde_json::from_str(&execute_tool(&s, "statistics", &json!({})).unwrap()).unwrap();
        assert!(stats["stance_by_agent"].is_array());
        assert!(stats["stance_by_volume"].is_array());
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
