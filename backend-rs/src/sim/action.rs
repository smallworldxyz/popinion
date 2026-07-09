use serde::{Deserialize, Serialize};

/// The action space for opinion-dynamics simulation. Deliberately tighter than
/// OASIS's 13 Reddit actions — these are the ones that actually move opinion.
/// ponytail: add repost/quote/mute later only if analysis needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    CreatePost,
    CreateComment,
    LikePost,
    DislikePost,
    Follow,
    DoNothing,
    Interview,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::CreatePost => "create_post",
            ActionType::CreateComment => "create_comment",
            ActionType::LikePost => "like_post",
            ActionType::DislikePost => "dislike_post",
            ActionType::Follow => "follow",
            ActionType::DoNothing => "do_nothing",
            ActionType::Interview => "interview",
        }
    }
}

/// A parsed agent decision. `stance`/`sentiment` are captured at creation time
/// so opinion analysis is a DB query, not a later LLM pass.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Decision {
    pub action: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub target_post_id: Option<i64>,
    #[serde(default)]
    pub target_user_id: Option<i64>,
    #[serde(default)]
    pub stance: Option<String>,
    #[serde(default)]
    pub sentiment: Option<f64>,
    #[serde(default)]
    pub reasoning: Option<String>,
}

impl Decision {
    pub fn action_type(&self) -> ActionType {
        match self.action.trim().to_lowercase().as_str() {
            "create_post" | "post" => ActionType::CreatePost,
            "create_comment" | "comment" | "reply" => ActionType::CreateComment,
            "like_post" | "like" => ActionType::LikePost,
            "dislike_post" | "dislike" => ActionType::DislikePost,
            "follow" => ActionType::Follow,
            "interview" => ActionType::Interview,
            _ => ActionType::DoNothing,
        }
    }

    /// Clamp sentiment into [-1, 1]; normalize an empty stance to None.
    pub fn normalized(mut self) -> Self {
        if let Some(s) = self.sentiment {
            self.sentiment = Some(s.clamp(-1.0, 1.0));
        }
        if let Some(st) = &self.stance {
            if st.trim().is_empty() {
                self.stance = None;
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_action_variants() {
        assert_eq!(
            serde_json::from_str::<Decision>(r#"{"action":"post","content":"hi","stance":"support","sentiment":1.5}"#)
                .unwrap()
                .normalized()
                .action_type(),
            ActionType::CreatePost
        );
        let d: Decision = serde_json::from_str(r#"{"action":"like","target_post_id":3}"#).unwrap();
        assert_eq!(d.action_type(), ActionType::LikePost);
        assert_eq!(d.target_post_id, Some(3));
        // unknown -> do_nothing
        let d: Decision = serde_json::from_str(r#"{"action":"wat"}"#).unwrap();
        assert_eq!(d.action_type(), ActionType::DoNothing);
    }

    #[test]
    fn clamps_sentiment() {
        let d = Decision { sentiment: Some(1.5), ..Default::default() }.normalized();
        assert_eq!(d.sentiment, Some(1.0));
        let d = Decision { sentiment: Some(-9.0), ..Default::default() }.normalized();
        assert_eq!(d.sentiment, Some(-1.0));
    }
}
