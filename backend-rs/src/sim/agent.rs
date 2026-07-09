use serde::{Deserialize, Serialize};

/// A simulated persona. Lenient deserialization so it loads both our own
/// generated profiles and the legacy OASIS `reddit_profiles.json` shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentProfile {
    pub user_id: i64,
    #[serde(alias = "username")]
    pub user_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub persona: String,
    #[serde(default)]
    pub age: Option<i64>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub mbti: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub profession: Option<String>,
    #[serde(default)]
    pub interested_topics: Vec<String>,
}

impl AgentProfile {
    /// A compact persona block for prompting.
    pub fn persona_prompt(&self) -> String {
        let mut s = format!("You are {} (@{}).", self.name, self.user_name);
        if let Some(age) = self.age {
            s.push_str(&format!(" Age: {age}."));
        }
        for (label, v) in [
            ("Profession", &self.profession),
            ("Country", &self.country),
            ("Personality (MBTI)", &self.mbti),
            ("Gender", &self.gender),
        ] {
            if let Some(v) = v {
                if !v.is_empty() {
                    s.push_str(&format!(" {label}: {v}."));
                }
            }
        }
        if !self.interested_topics.is_empty() {
            s.push_str(&format!(" Interests: {}.", self.interested_topics.join(", ")));
        }
        if !self.bio.is_empty() {
            s.push_str(&format!("\nBio: {}", self.bio));
        }
        if !self.persona.is_empty() {
            s.push_str(&format!("\nPersona: {}", self.persona));
        }
        s
    }
}

/// Runtime agent: profile + per-agent activity schedule.
#[derive(Clone, Debug)]
pub struct Agent {
    pub profile: AgentProfile,
    pub activity_level: f64,
    pub active_hours: Vec<u32>,
}

impl Agent {
    pub fn user_id(&self) -> i64 {
        self.profile.user_id
    }
}
