//! Independent stance measurement. The agents self-report their stance inside
//! the same LLM call that produces the action — so the acting model grades its
//! own homework, and correlated errors are invisible to every downstream check.
//!
//! This module re-labels each post/comment with a SEPARATE classifier pass,
//! ideally on a different model (the boost slot), so the two labels can be
//! compared. Low agreement means the self-reports (and thus the headline stance
//! numbers) should not be trusted. Unlike the old English-keyword fallback, the
//! classifier is a model call, so it works on any language the model handles.

use crate::llm::{Llm, Msg};

/// Ask the model to independently label one statement's stance toward the topic.
pub async fn classify_stance(llm: &Llm, topic: &str, text: &str) -> String {
    let sys = "You are a neutral stance classifier. Given a statement and a topic, decide whether \
               the statement SUPPORTS, OPPOSES, or is NEUTRAL toward the topic. Consider only the \
               statement's own position. Reply with exactly one word: support, oppose, or neutral.";
    let user = format!("Topic: {topic}\nStatement: {text}\nStance (one word):");
    match llm.chat(&[Msg::system(sys), Msg::user(user)], 0.0, 8).await {
        Ok(reply) => parse_stance_label(&reply),
        Err(_) => "unknown".to_string(),
    }
}

/// Map a free-form model reply to a canonical stance label.
pub fn parse_stance_label(reply: &str) -> String {
    let r = reply.to_lowercase();
    // Order matters: check oppose/support before neutral, and prefer the first
    // clear signal so "not support, oppose" resolves to oppose.
    let sup = r.find("support").or_else(|| r.find("favor")).or_else(|| r.find("for the"));
    let opp = r.find("oppose").or_else(|| r.find("against")).or_else(|| r.find("reject"));
    match (sup, opp) {
        (Some(s), Some(o)) => if s < o { "support".into() } else { "oppose".into() },
        (Some(_), None) => "support".into(),
        (None, Some(_)) => "oppose".into(),
        (None, None) => {
            if r.contains("neutral") || r.contains("mixed") {
                "neutral".into()
            } else {
                "unknown".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_labels() {
        assert_eq!(parse_stance_label("support"), "support");
        assert_eq!(parse_stance_label("Oppose."), "oppose");
        assert_eq!(parse_stance_label("Neutral"), "neutral");
        assert_eq!(parse_stance_label("The statement is AGAINST the policy"), "oppose");
        assert_eq!(parse_stance_label("banana"), "unknown");
    }

    #[test]
    fn first_signal_wins_on_conflict() {
        assert_eq!(parse_stance_label("oppose, definitely not support"), "oppose");
        assert_eq!(parse_stance_label("support rather than oppose"), "support");
    }
}
