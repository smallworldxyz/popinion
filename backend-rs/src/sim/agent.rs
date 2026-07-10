use serde::{Deserialize, Serialize};

/// A grounded persona record. Lenient deserialization so it loads both our own
/// generated profiles and the legacy OASIS `reddit_profiles.json` shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persona {
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

    // ---- provenance (graph-grounded personas) ----
    /// The knowledge-graph entity this persona was compiled from.
    #[serde(default)]
    pub source_entity_uuid: Option<String>,
    #[serde(default)]
    pub source_entity_type: Option<String>,
    /// Real facts (entity summary + relationship facts) this persona rests on.
    /// Empty for fabricated/legacy profiles.
    #[serde(default)]
    pub evidence: Vec<String>,
    /// True when attributes were invented rather than observed. Reports must
    /// disclose synthetic personas; graph-grounded ones set this false.
    #[serde(default)]
    pub synthetic: bool,
    /// Stance camp: "pro" or "con". None = neutral (the debate subject, media)
    /// or unknown; neutral agents are followed by both camps at seeding.
    #[serde(default)]
    pub faction: Option<String>,
}

impl Persona {
    /// A compact persona block for prompting.
    pub fn persona_prompt(&self) -> String {
        let mut s = format!("You are {} (@{}).", self.name, self.user_name);
        if let Some(age) = self.age {
            s.push_str(&format!(" Age: {age}."));
        }
        // MBTI is deliberately not rendered: it was fabricated (round-robin by
        // id) and only injects the model's stereotype priors as false texture.
        for (label, v) in [
            ("Profession", &self.profession),
            ("Country", &self.country),
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
        // Ground the persona in observed evidence when we have it.
        if !self.evidence.is_empty() {
            s.push_str("\nWhat is known about you (from real observed data):");
            for fact in &self.evidence {
                s.push_str(&format!("\n- {fact}"));
            }
        }
        s
    }
}

/// Runtime agent: a Persona plus per-agent activity schedule.
#[derive(Clone, Debug)]
pub struct Agent {
    pub profile: Persona,
    pub activity_level: f64,
    pub active_hours: Vec<u32>,
}
