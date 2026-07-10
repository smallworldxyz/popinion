//! Structured surveys for simulated agents (opinion polls + Likert scales).
//! Ports survey_service.py; templates live in an in-process registry instead
//! of JSON files under uploads/.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::interview::AgentInterviewer;
use super::panel_chat::Persona;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurveyType {
    OpinionPoll,
    Likert,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurveyQuestion {
    pub question_id: String,
    pub question_text: String,
    pub question_type: SurveyType,
    pub options: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurveyTemplate {
    pub survey_id: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<SurveyQuestion>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SurveyResponse {
    pub agent_id: i64,
    pub agent_name: String,
    pub faction: String,
    pub responses: HashMap<String, Option<String>>,
    pub raw_text: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SurveyResult {
    pub survey_id: String,
    pub template: SurveyTemplate,
    pub total_respondents: usize,
    pub responses: Vec<SurveyResponse>,
    pub aggregated: HashMap<String, Value>,
    pub by_faction: HashMap<String, Vec<SurveyResponse>>,
    pub timestamp: String,
}

fn registry() -> &'static Mutex<HashMap<String, SurveyTemplate>> {
    static SURVEYS: OnceLock<Mutex<HashMap<String, SurveyTemplate>>> = OnceLock::new();
    SURVEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn default_options(question_type: SurveyType) -> Vec<String> {
    match question_type {
        SurveyType::OpinionPoll => vec!["Agree".into(), "Disagree".into(), "Neutral".into()],
        SurveyType::Likert => (1..=5).map(|i| i.to_string()).collect(),
    }
}

/// Create and register a survey template. `questions` items:
/// `{"question_text": "...", "question_type": "opinion_poll"|"likert", "options": [...]}`.
pub fn survey_create(title: &str, description: &str, questions: &[Value]) -> anyhow::Result<SurveyTemplate> {
    if title.trim().is_empty() {
        anyhow::bail!("Survey title is required and cannot be empty");
    }
    if questions.is_empty() {
        anyhow::bail!("At least one question is required");
    }

    let mut survey_questions = Vec::with_capacity(questions.len());
    for (i, q) in questions.iter().enumerate() {
        let n = i + 1;
        let text = q["question_text"].as_str().unwrap_or("").trim().to_string();
        if text.is_empty() {
            anyhow::bail!("Question {n}: question_text is required and cannot be empty");
        }
        let type_str = q["question_type"].as_str().unwrap_or("opinion_poll");
        let question_type = match type_str {
            "opinion_poll" => SurveyType::OpinionPoll,
            "likert" => SurveyType::Likert,
            other => anyhow::bail!(
                "Question {n}: Invalid question_type '{other}'. Must be one of: likert, opinion_poll"
            ),
        };
        let options: Vec<String> = q["options"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| default_options(question_type));
        if question_type == SurveyType::OpinionPoll && options.len() < 2 {
            anyhow::bail!("Question {n}: Opinion poll requires at least 2 options");
        }
        survey_questions.push(SurveyQuestion {
            question_id: format!("q{n}"),
            question_text: text,
            question_type,
            options,
        });
    }

    let template = SurveyTemplate {
        survey_id: format!("survey_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
        title: title.trim().to_string(),
        description: description.trim().to_string(),
        questions: survey_questions,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    registry().lock().unwrap().insert(template.survey_id.clone(), template.clone());
    tracing::info!("created survey {} with {} questions", template.survey_id, template.questions.len());
    Ok(template)
}

pub fn survey_get(survey_id: &str) -> Option<SurveyTemplate> {
    registry().lock().unwrap().get(survey_id).cloned()
}

pub fn survey_list() -> Vec<SurveyTemplate> {
    let mut surveys: Vec<SurveyTemplate> = registry().lock().unwrap().values().cloned().collect();
    surveys.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    surveys
}

/// Deploy a survey: interview every persona, parse and aggregate answers.
pub async fn survey_deploy(
    interviewer: &dyn AgentInterviewer,
    survey_id: &str,
    personas: &[Persona],
) -> anyhow::Result<SurveyResult> {
    let template = survey_get(survey_id).ok_or_else(|| anyhow::anyhow!("Survey not found: {survey_id}"))?;
    if personas.is_empty() {
        anyhow::bail!("at least one persona is required");
    }

    let prompt = build_survey_prompt(&template);
    let futures = personas.iter().map(|p| {
        let prompt = prompt.clone();
        async move { (p, interviewer.interview(p.agent_id, &prompt).await) }
    });

    let mut responses = Vec::new();
    for (p, result) in futures::future::join_all(futures).await {
        match result {
            Ok(raw) => responses.push(parse_agent_response(
                &template,
                p.agent_id,
                &p.name,
                if p.faction.is_empty() { "unknown" } else { &p.faction },
                &raw,
            )),
            Err(e) => tracing::warn!("survey: interview of agent {} failed: {e:#}", p.agent_id),
        }
    }
    if responses.is_empty() {
        anyhow::bail!("all survey interviews failed");
    }
    Ok(aggregate_results(&template, responses))
}

pub fn build_survey_prompt(template: &SurveyTemplate) -> String {
    let mut parts = vec![format!("Please respond to this survey: {}", template.title), String::new()];
    if !template.description.is_empty() {
        parts.push(format!("Description: {}", template.description));
        parts.push(String::new());
    }
    for q in &template.questions {
        match q.question_type {
            SurveyType::OpinionPoll => parts.push(format!(
                "{}. {}\n   Options: {}\n   Your answer (choose one option):",
                q.question_id,
                q.question_text,
                q.options.join(", ")
            )),
            SurveyType::Likert => parts.push(format!(
                "{}. {}\n   Rate from 1 (Strongly Disagree) to 5 (Strongly Agree)\n   Your rating (1-5):",
                q.question_id, q.question_text
            )),
        }
        parts.push(String::new());
    }
    parts.push(
        "Please respond with your answers in the format:\nq1: [your answer]\nq2: [your answer]\n...\n\
         Brief explanation (optional): [your reasoning]"
            .to_string(),
    );
    parts.join("\n")
}

pub fn parse_agent_response(
    template: &SurveyTemplate,
    agent_id: i64,
    agent_name: &str,
    faction: &str,
    raw_response: &str,
) -> SurveyResponse {
    let responses = template
        .questions
        .iter()
        .map(|q| (q.question_id.clone(), extract_answer(raw_response, q)))
        .collect();
    SurveyResponse {
        agent_id,
        agent_name: agent_name.to_string(),
        faction: faction.to_string(),
        responses,
        raw_text: raw_response.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Word-boundary containment: avoids the Python service's bug where the
/// option "Agree" matched inside the answer "Disagree".
fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let begin = start + pos;
        let end = begin + needle.len();
        let ok_before = begin == 0
            || !haystack[..begin].chars().next_back().is_some_and(|c| c.is_alphanumeric());
        let ok_after =
            end == haystack.len() || !haystack[end..].chars().next().is_some_and(|c| c.is_alphanumeric());
        if ok_before && ok_after {
            return true;
        }
        start = end;
    }
    false
}

/// Match an answer against poll options, longest option first so
/// "Strongly Agree" wins over "Agree".
fn match_option<'a>(answer: &str, options: &'a [String]) -> Option<&'a String> {
    let lower = answer.to_lowercase();
    let mut by_len: Vec<&String> = options.iter().collect();
    by_len.sort_by_key(|o| std::cmp::Reverse(o.len()));
    by_len
        .into_iter()
        .find(|opt| lower == opt.to_lowercase() || contains_word(&lower, &opt.to_lowercase()))
}

fn extract_answer(raw_response: &str, question: &SurveyQuestion) -> Option<String> {
    // Line-based "q1: answer" / "q1. answer" / "q1) answer" extraction.
    let qid = &question.question_id;
    for line in raw_response.lines() {
        let line = line.trim();
        if line.len() > qid.len() && line[..qid.len()].eq_ignore_ascii_case(qid) {
            let rest = line[qid.len()..].trim_start_matches([':', '.', ')']).trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    // Fallback: look for an option keyword anywhere in the response.
    if question.question_type == SurveyType::OpinionPoll {
        return match_option(raw_response, &question.options).cloned();
    }
    None
}

pub fn aggregate_results(template: &SurveyTemplate, responses: Vec<SurveyResponse>) -> SurveyResult {
    let mut by_faction: HashMap<String, Vec<SurveyResponse>> = HashMap::new();
    for r in &responses {
        by_faction.entry(r.faction.clone()).or_default().push(r.clone());
    }

    let mut aggregated = HashMap::new();
    for q in &template.questions {
        let answers: Vec<&String> = responses
            .iter()
            .filter_map(|r| r.responses.get(&q.question_id).and_then(|a| a.as_ref()))
            .collect();

        let stats = match q.question_type {
            SurveyType::OpinionPoll => {
                let mut counts: HashMap<&str, usize> = q.options.iter().map(|o| (o.as_str(), 0)).collect();
                for answer in &answers {
                    if let Some(opt) = match_option(answer, &q.options) {
                        *counts.get_mut(opt.as_str()).unwrap() += 1;
                    }
                }
                let total: usize = counts.values().sum();
                let distribution: HashMap<&str, f64> = counts
                    .iter()
                    .map(|(opt, c)| {
                        let pct = if total > 0 { (*c as f64 / total as f64 * 1000.0).round() / 10.0 } else { 0.0 };
                        (*opt, pct)
                    })
                    .collect();
                serde_json::json!({
                    "question_text": q.question_text,
                    "question_type": "opinion_poll",
                    "counts": counts,
                    "distribution": distribution,
                    "total_responses": total,
                })
            }
            SurveyType::Likert => {
                let values: Vec<u32> = answers
                    .iter()
                    .filter_map(|a| a.chars().find(char::is_ascii_digit))
                    .filter_map(|c| c.to_digit(10))
                    .filter(|v| (1..=5).contains(v))
                    .collect();
                let avg = if values.is_empty() {
                    0.0
                } else {
                    (values.iter().sum::<u32>() as f64 / values.len() as f64 * 100.0).round() / 100.0
                };
                let distribution: HashMap<String, usize> = (1..=5)
                    .map(|i| (i.to_string(), values.iter().filter(|v| **v == i).count()))
                    .collect();
                serde_json::json!({
                    "question_text": q.question_text,
                    "question_type": "likert",
                    "average": avg,
                    "distribution": distribution,
                    "total_responses": values.len(),
                })
            }
        };
        aggregated.insert(q.question_id.clone(), stats);
    }

    SurveyResult {
        survey_id: template.survey_id.clone(),
        template: template.clone(),
        total_respondents: responses.len(),
        responses,
        aggregated,
        by_faction,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::interview::ScriptedInterviewer;
    use super::*;
    use serde_json::json;

    fn make_survey() -> SurveyTemplate {
        survey_create(
            "Policy Feedback",
            "Opinions on the new policy",
            &[
                json!({"question_text": "Do you support the new policy?", "question_type": "opinion_poll"}),
                json!({"question_text": "The policy is clearly communicated.", "question_type": "likert"}),
            ],
        )
        .unwrap()
    }

    #[test]
    fn create_validates_and_registers() {
        let t = make_survey();
        assert_eq!(t.questions.len(), 2);
        assert_eq!(t.questions[0].question_id, "q1");
        assert_eq!(t.questions[0].options, vec!["Agree", "Disagree", "Neutral"]);
        assert!(survey_get(&t.survey_id).is_some());
        assert!(survey_list().iter().any(|s| s.survey_id == t.survey_id));

        assert!(survey_create("", "", &[json!({"question_text": "x"})]).is_err());
        assert!(survey_create("T", "", &[]).is_err());
        assert!(survey_create("T", "", &[json!({"question_text": ""})]).is_err());
        assert!(survey_create("T", "", &[json!({"question_text": "x", "question_type": "essay"})]).is_err());
        assert!(survey_create("T", "", &[json!({"question_text": "x", "options": ["only-one"]})]).is_err());
    }

    #[test]
    fn option_matching_respects_word_boundaries() {
        let options = vec!["Agree".to_string(), "Disagree".to_string(), "Neutral".to_string()];
        assert_eq!(match_option("I Disagree with this", &options).unwrap(), "Disagree");
        assert_eq!(match_option("agree", &options).unwrap(), "Agree");
        assert_eq!(match_option("strongly disagreeable prose", &options), None);
    }

    #[test]
    fn prompt_lists_questions_and_format() {
        let t = make_survey();
        let prompt = build_survey_prompt(&t);
        assert!(prompt.contains("q1. Do you support the new policy?"));
        assert!(prompt.contains("Options: Agree, Disagree, Neutral"));
        assert!(prompt.contains("Rate from 1 (Strongly Disagree) to 5 (Strongly Agree)"));
        assert!(prompt.contains("q1: [your answer]"));
    }

    #[test]
    fn parses_and_aggregates_responses() {
        let t = make_survey();
        let r1 = parse_agent_response(&t, 1, "Alice", "Student", "q1: Agree\nq2: 5\nBecause it helps.");
        let r2 = parse_agent_response(&t, 2, "Bob", "Official", "q1) Disagree\nq2. 2");
        let r3 = parse_agent_response(&t, 3, "Cara", "Student", "I would say I Agree with it overall.");
        assert_eq!(r1.responses["q1"].as_deref(), Some("Agree"));
        assert_eq!(r3.responses["q1"].as_deref(), Some("Agree")); // keyword fallback

        let result = aggregate_results(&t, vec![r1, r2, r3]);
        assert_eq!(result.total_respondents, 3);
        assert_eq!(result.aggregated["q1"]["counts"]["Agree"], 2);
        assert_eq!(result.aggregated["q1"]["counts"]["Disagree"], 1);
        assert_eq!(result.aggregated["q2"]["average"], 3.5);
        assert_eq!(result.aggregated["q2"]["total_responses"], 2);
        assert_eq!(result.by_faction["Student"].len(), 2);
    }

    #[tokio::test]
    async fn deploys_via_interviewer_seam() {
        let t = make_survey();
        let interviewer = ScriptedInterviewer(std::collections::HashMap::from([
            (1, "q1: Agree\nq2: 4".to_string()),
            (2, "q1: Neutral\nq2: 3".to_string()),
        ]));
        let personas = vec![
            Persona { agent_id: 1, name: "Alice".into(), faction: "Student".into(), platform: String::new() },
            Persona { agent_id: 2, name: "Bob".into(), faction: String::new(), platform: String::new() },
            Persona { agent_id: 3, name: "Ghost".into(), faction: String::new(), platform: String::new() },
        ];
        let result = survey_deploy(&interviewer, &t.survey_id, &personas).await.unwrap();
        assert_eq!(result.total_respondents, 2); // agent 3 failed and was skipped
        assert_eq!(result.aggregated["q1"]["counts"]["Agree"], 1);
        assert_eq!(result.aggregated["q2"]["average"], 3.5);
        assert_eq!(result.by_faction["unknown"].len(), 1);

        assert!(survey_deploy(&interviewer, "survey_missing", &personas).await.is_err());
        assert!(survey_deploy(&interviewer, &t.survey_id, &[]).await.is_err());
    }
}
