//! Panel chat: a facilitated multi-round discussion among simulated personas.
//! Does stance classification, faction grouping, distribution stats, and
//! summary, plus optional multi-round turns where personas react to each
//! other; the full transcript is returned.

use crate::llm::{Llm, Msg};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::interview::AgentInterviewer;

const CLASSIFY_BATCH_SIZE: usize = 10;
const RESPONSE_PREVIEW_LEN: usize = 200;
const RESPONSE_TRUNCATE_LEN: usize = 300;
const EXCERPTS_PER_ROUND: usize = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persona {
    pub agent_id: i64,
    pub name: String,
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub platform: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PanelResponse {
    pub agent_id: i64,
    pub agent_name: String,
    pub platform: String,
    pub faction: String,
    pub response: String,
    pub stance: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranscriptTurn {
    pub round: u32,
    pub agent_id: i64,
    pub agent_name: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PanelChatResult {
    pub question: String,
    pub timestamp: String,
    pub total_agents: usize,
    pub rounds: u32,
    pub transcript: Vec<TranscriptTurn>,
    pub responses: Vec<PanelResponse>,
    pub by_stance: Value,
    pub by_faction: Value,
    pub stance_counts: HashMap<String, usize>,
    pub stance_distribution: HashMap<String, f64>,
    pub faction_counts: HashMap<String, usize>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PanelChatOptions {
    pub rounds: u32,
    pub classify_stance: bool,
    pub generate_summary: bool,
}

impl Default for PanelChatOptions {
    fn default() -> Self {
        PanelChatOptions { rounds: 1, classify_stance: true, generate_summary: true }
    }
}

fn preview(s: &str, max: usize) -> String {
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

/// Run a panel discussion: interview every persona (multi-round when
/// `opts.rounds > 1`), classify final stances, group and summarize.
pub async fn panel_chat(
    llm: &Llm,
    interviewer: &dyn AgentInterviewer,
    question: &str,
    personas: &[Persona],
    opts: &PanelChatOptions,
) -> anyhow::Result<PanelChatResult> {
    if question.trim().is_empty() {
        anyhow::bail!("question must not be empty");
    }
    if personas.is_empty() {
        anyhow::bail!("at least one persona is required");
    }

    let rounds = opts.rounds.max(1);
    let mut transcript: Vec<TranscriptTurn> = Vec::new();
    let mut latest: HashMap<i64, String> = HashMap::new();

    for round in 1..=rounds {
        let futures = personas.iter().map(|p| {
            let prompt = if round == 1 {
                question.to_string()
            } else {
                let excerpts = transcript
                    .iter()
                    .rev()
                    .filter(|t| t.round == round - 1 && t.agent_id != p.agent_id)
                    .take(EXCERPTS_PER_ROUND)
                    .map(|t| format!("- {}: {}", t.agent_name, preview(&t.content, RESPONSE_PREVIEW_LEN)))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "The panel is discussing: {question}\n\nWhat other panelists said in the last round:\n{excerpts}\n\n\
                     React to the discussion - agree, push back, or refine your position - and restate your view."
                )
            };
            async move { (p.agent_id, interviewer.interview(p.agent_id, &prompt).await) }
        });
        for (agent_id, result) in futures::future::join_all(futures).await {
            match result {
                Ok(answer) if !answer.trim().is_empty() => {
                    let name = personas
                        .iter()
                        .find(|p| p.agent_id == agent_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("Agent_{agent_id}"));
                    transcript.push(TranscriptTurn {
                        round,
                        agent_id,
                        agent_name: name,
                        content: answer.trim().to_string(),
                    });
                    latest.insert(agent_id, answer.trim().to_string());
                }
                Ok(_) => tracing::warn!("panel chat: agent {agent_id} returned an empty answer"),
                Err(e) => tracing::warn!("panel chat: interview of agent {agent_id} failed: {e:#}"),
            }
        }
    }

    let mut responses: Vec<PanelResponse> = personas
        .iter()
        .filter_map(|p| {
            latest.get(&p.agent_id).map(|answer| PanelResponse {
                agent_id: p.agent_id,
                agent_name: p.name.clone(),
                platform: if p.platform.is_empty() { "unknown".into() } else { p.platform.clone() },
                faction: if p.faction.is_empty() { "unknown".into() } else { p.faction.clone() },
                response: answer.clone(),
                stance: "unknown".into(),
            })
        })
        .collect();
    if responses.is_empty() {
        anyhow::bail!("all panel interviews failed");
    }

    if opts.classify_stance {
        classify_stances(llm, question, &mut responses).await;
    }

    // Grouped views + statistics.
    let mut by_stance_map: HashMap<String, Vec<&PanelResponse>> = HashMap::new();
    let mut by_faction_map: HashMap<String, Vec<&PanelResponse>> = HashMap::new();
    for r in &responses {
        by_stance_map.entry(r.stance.clone()).or_default().push(r);
        by_faction_map.entry(r.faction.clone()).or_default().push(r);
    }
    let total = responses.len();
    let stance_counts: HashMap<String, usize> =
        by_stance_map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    let stance_distribution: HashMap<String, f64> = stance_counts
        .iter()
        .map(|(k, c)| (k.clone(), (*c as f64 / total as f64 * 1000.0).round() / 10.0))
        .collect();
    let faction_counts: HashMap<String, usize> =
        by_faction_map.iter().map(|(k, v)| (k.clone(), v.len())).collect();

    let by_stance = Value::Object(
        by_stance_map
            .iter()
            .map(|(stance, rs)| {
                let items: Vec<Value> = rs
                    .iter()
                    .map(|r| {
                        json!({
                            "agent_id": r.agent_id,
                            "agent_name": r.agent_name,
                            "response": preview(&r.response, RESPONSE_PREVIEW_LEN),
                            "faction": r.faction,
                        })
                    })
                    .collect();
                (stance.clone(), Value::Array(items))
            })
            .collect(),
    );
    let by_faction = Value::Object(
        by_faction_map
            .iter()
            .map(|(faction, rs)| {
                let items: Vec<Value> = rs
                    .iter()
                    .map(|r| {
                        json!({
                            "agent_id": r.agent_id,
                            "agent_name": r.agent_name,
                            "response": preview(&r.response, RESPONSE_PREVIEW_LEN),
                            "stance": r.stance,
                        })
                    })
                    .collect();
                (faction.clone(), Value::Array(items))
            })
            .collect(),
    );

    let mut result = PanelChatResult {
        question: question.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_agents: total,
        rounds,
        transcript,
        responses,
        by_stance,
        by_faction,
        stance_counts,
        stance_distribution,
        faction_counts,
        summary: None,
    };

    if opts.generate_summary {
        result.summary = Some(generate_summary(llm, &result).await);
    }
    Ok(result)
}

/// LLM stance classification in batches, with a rule-based fallback per batch.
async fn classify_stances(llm: &Llm, question: &str, responses: &mut [PanelResponse]) {
    for batch in responses.chunks_mut(CLASSIFY_BATCH_SIZE) {
        let listing = batch
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}: {}", i + 1, r.agent_name, preview(&r.response, RESPONSE_TRUNCATE_LEN)))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "Classify each response's stance on the following question.\n\nQuestion: {question}\n\n\
             Responses:\n{listing}\n\nFor each response, classify as \"support\", \"oppose\", or \"neutral\".\n\
             Respond in JSON: {{\"classifications\": [{{\"index\": 1, \"stance\": \"support\"}}, ...]}}"
        );
        let messages = [
            Msg::system("You are a stance classifier. Respond only with valid JSON."),
            Msg::user(prompt),
        ];
        match llm.chat_json(&messages, 0.3, 500).await {
            Ok(data) => {
                for c in data["classifications"].as_array().cloned().unwrap_or_default() {
                    let idx = c["index"].as_i64().unwrap_or(0) - 1;
                    if idx >= 0 && (idx as usize) < batch.len() {
                        batch[idx as usize].stance = match c["stance"].as_str().unwrap_or("neutral") {
                            "support" => "support".into(),
                            "oppose" => "oppose".into(),
                            _ => "neutral".into(),
                        };
                    }
                }
                // Anything the model skipped falls back to rules.
                for r in batch.iter_mut().filter(|r| r.stance == "unknown") {
                    r.stance = rule_based_stance(&r.response);
                }
            }
            Err(e) => {
                tracing::warn!("stance classification failed ({e}); using rule-based fallback");
                for r in batch.iter_mut() {
                    r.stance = rule_based_stance(&r.response);
                }
            }
        }
    }
}

pub fn rule_based_stance(response: &str) -> String {
    const SUPPORT: [&str; 11] = [
        "agree", "support", "favor", "approve", "believe", "should",
        "good idea", "positive", "beneficial", "important", "necessary",
    ];
    const OPPOSE: [&str; 10] = [
        "disagree", "oppose", "against", "reject", "should not",
        "bad idea", "negative", "harmful", "unnecessary", "concerned",
    ];
    let text = response.to_lowercase();
    let support = SUPPORT.iter().filter(|k| text.contains(**k)).count();
    let oppose = OPPOSE.iter().filter(|k| text.contains(**k)).count();
    if support > oppose {
        "support".into()
    } else if oppose > support {
        "oppose".into()
    } else {
        "neutral".into()
    }
}

async fn generate_summary(llm: &Llm, result: &PanelChatResult) -> String {
    let get = |k: &str| result.stance_distribution.get(k).copied().unwrap_or(0.0);
    let quote_for = |stance: &str| {
        result
            .responses
            .iter()
            .find(|r| r.stance == stance)
            .map(|r| preview(&r.response, 100))
            .unwrap_or_else(|| "None".into())
    };
    let prompt = format!(
        "Summarize this panel discussion in 2-3 sentences:\n\nQuestion: {}\nTotal respondents: {}\n\
         Support: {}%\nOppose: {}%\nNeutral: {}%\n\nSample supporting view: {}\nSample opposing view: {}\n\n\
         Provide a balanced summary of the overall sentiment and key tensions.",
        result.question,
        result.total_agents,
        get("support"),
        get("oppose"),
        get("neutral"),
        quote_for("support"),
        quote_for("oppose"),
    );
    match llm
        .chat(&[Msg::system("You are a neutral summarizer."), Msg::user(prompt)], 0.5, 200)
        .await
    {
        Ok(s) => s.trim().to_string(),
        Err(e) => {
            tracing::warn!("panel summary generation failed: {e}");
            format!(
                "Of {} respondents, {}% support, {}% oppose, and {}% are neutral.",
                result.total_agents,
                get("support"),
                get("oppose"),
                get("neutral"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::interview::ScriptedInterviewer;
    use super::*;

    fn offline_llm() -> Llm {
        // A dead port, so every call fails and the rule-based fallback paths run.
        // An empty key does NOT short-circuit the request (local servers need no
        // key), so the retries must be capped or this sleeps through the backoff.
        Llm::new("", "http://127.0.0.1:1", "test").without_retries()
    }

    fn personas() -> Vec<Persona> {
        vec![
            Persona { agent_id: 1, name: "Alice".into(), faction: "Student".into(), platform: "reddit".into() },
            Persona { agent_id: 2, name: "Bob".into(), faction: "Official".into(), platform: "reddit".into() },
            Persona { agent_id: 3, name: "Cara".into(), faction: "Student".into(), platform: String::new() },
        ]
    }

    fn scripted() -> ScriptedInterviewer {
        ScriptedInterviewer(std::collections::HashMap::from([
            (1, "I fully support this, it is a good idea and beneficial.".to_string()),
            (2, "I strongly disagree, this is harmful and I am against it.".to_string()),
            (3, "I can see both sides of the argument here.".to_string()),
        ]))
    }

    #[tokio::test]
    async fn aggregates_stances_and_factions() {
        let result = panel_chat(
            &offline_llm(),
            &scripted(),
            "Do you support the new policy?",
            &personas(),
            &PanelChatOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.total_agents, 3);
        assert_eq!(result.transcript.len(), 3);
        assert_eq!(result.stance_counts["support"], 1);
        assert_eq!(result.stance_counts["oppose"], 1);
        assert_eq!(result.stance_counts["neutral"], 1);
        assert_eq!(result.faction_counts["Student"], 2);
        assert!((result.stance_distribution["support"] - 33.3).abs() < 0.01);
        // Offline LLM -> fallback summary is still produced.
        assert!(result.summary.as_deref().unwrap().contains("3 respondents"));
        assert_eq!(result.by_stance["support"].as_array().unwrap().len(), 1);
        assert_eq!(result.responses.iter().find(|r| r.agent_id == 3).unwrap().platform, "unknown");
    }

    #[tokio::test]
    async fn multi_round_builds_transcript() {
        let opts = PanelChatOptions { rounds: 2, classify_stance: false, generate_summary: false };
        let result = panel_chat(&offline_llm(), &scripted(), "Topic?", &personas(), &opts)
            .await
            .unwrap();
        assert_eq!(result.rounds, 2);
        assert_eq!(result.transcript.len(), 6);
        assert!(result.transcript.iter().any(|t| t.round == 2));
        assert!(result.responses.iter().all(|r| r.stance == "unknown"));
        assert!(result.summary.is_none());
    }

    #[tokio::test]
    async fn partial_interview_failures_are_skipped() {
        let only_one = ScriptedInterviewer(std::collections::HashMap::from([(
            1,
            "I approve, this is necessary.".to_string(),
        )]));
        let opts = PanelChatOptions { rounds: 1, classify_stance: true, generate_summary: false };
        let result = panel_chat(&offline_llm(), &only_one, "Q?", &personas(), &opts).await.unwrap();
        assert_eq!(result.total_agents, 1);
        assert_eq!(result.responses[0].stance, "support");
    }

    #[tokio::test]
    async fn rejects_empty_inputs() {
        assert!(panel_chat(&offline_llm(), &scripted(), "", &personas(), &Default::default()).await.is_err());
        assert!(panel_chat(&offline_llm(), &scripted(), "Q?", &[], &Default::default()).await.is_err());
    }

    #[test]
    fn rule_based_stance_classification() {
        assert_eq!(rule_based_stance("I agree, this is beneficial"), "support");
        assert_eq!(rule_based_stance("I am against it, it is harmful"), "oppose");
        assert_eq!(rule_based_stance("no strong feelings"), "neutral");
    }
}
