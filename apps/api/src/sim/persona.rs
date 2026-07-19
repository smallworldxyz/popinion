//! Graph → persona evidence compiler. Turns a knowledge-graph entity into a
//! simulation persona built ONLY from observed evidence — the entity's summary
//! and the facts on its relationships (including SUPPORTS/OPPOSES stance edges).
//! No fabricated attributes; every persona carries provenance back to its entity.
//!
//! This is the link that makes the graph (the "God's-eye" view of the opinion
//! landscape) actually reach the simulation.

use crate::llm::{Llm, Msg};
use crate::services::graph::db::GraphData;
use crate::sim::agent::Persona;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Everything the graph observed about one entity, gathered for persona compilation.
#[derive(Clone, Debug)]
pub struct EvidenceBundle {
    pub uuid: String,
    pub name: String,
    pub entity_type: String,
    pub summary: String,
    /// Incident relationship facts (this entity as source or target).
    pub facts: Vec<String>,
    /// The subset of `facts` that express a stance (SUPPORTS/OPPOSES/…).
    pub stance_facts: Vec<String>,
    /// Did this entity SOURCE a supportive / an opposing stance edge? An
    /// entity's own camp comes from the stances it authored, not those aimed
    /// at it (the debate subject is the *target* of everyone's stances).
    pub sourced_support: bool,
    pub sourced_oppose: bool,
    /// Can this entity hold an opinion? A law, a project or a budget cannot,
    /// and must never become an agent with a faction.
    pub is_actor: bool,
}

impl EvidenceBundle {
    /// A single number for the minimum-evidence bar: how much the graph actually
    /// knows about this entity. A bare name-only node scores 0.
    pub fn evidence_score(&self) -> usize {
        self.facts.len() + usize::from(!self.summary.trim().is_empty())
    }

    /// The entity's stance camp from its outgoing stance edges: only supports →
    /// "pro", only opposes → "con", both or neither → None (neutral).
    pub fn faction(&self) -> Option<&'static str> {
        match (self.sourced_support, self.sourced_oppose) {
            (true, false) => Some("pro"),
            (false, true) => Some("con"),
            _ => None,
        }
    }
}

/// Polarity of a stance relationship: Some(true)=supportive, Some(false)=opposed,
/// None=not a stance edge.
fn keyword_polarity(rel: &str) -> Option<bool> {
    let r = rel.to_uppercase();
    if ["SUPPORT", "ENDORS", "BACK", "FAVOR"].iter().any(|k| r.contains(k)) {
        Some(true)
    } else if ["OPPOS", "AGAINST", "CRITIC", "CONDEMN", "REJECT"].iter().any(|k| r.contains(k)) {
        Some(false)
    } else {
        None
    }
}

/// Which relation types carry a stance, and in which direction.
///
/// Ontology generation lets the model invent edge type names, so a fixed
/// keyword list cannot recognise them: a graph whose opposition arrived as
/// `QUESTIONS_BENEFITS_FROM` scored as having no opponents at all, which left
/// the synthetic audience one-sided and reported near-unanimous support that
/// the discussion did not contain. The names are classified by the same kind of
/// model that invented them, keywords serving only as a free fast path.
#[derive(Clone, Default, Debug)]
pub struct StanceLexicon {
    learned: HashMap<String, bool>,
}

impl StanceLexicon {
    /// Keyword matching only. For callers with no LLM to hand; a graph using
    /// invented stance verbs will read as stanceless.
    pub fn keywords_only() -> Self {
        Self::default()
    }

    /// A lexicon with classifications already known, bypassing the LLM.
    pub fn from_pairs<I: IntoIterator<Item = (String, bool)>>(pairs: I) -> Self {
        Self { learned: pairs.into_iter().map(|(k, v)| (k.to_uppercase(), v)).collect() }
    }

    pub fn polarity(&self, rel: &str) -> Option<bool> {
        keyword_polarity(rel).or_else(|| self.learned.get(&rel.to_uppercase()).copied())
    }

    /// Classify the graph's distinct relation types. One call over a handful of
    /// names, not per edge. On any failure the lexicon degrades to keywords.
    pub async fn learn(llm: &Llm, graph_data: &GraphData) -> Self {
        let unknown: Vec<String> = graph_data
            .edges
            .iter()
            .map(|e| e.relation_type.to_uppercase())
            .filter(|r| keyword_polarity(r).is_none())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if unknown.is_empty() {
            return Self::default();
        }

        let prompt = format!(
            "These are relation types from a knowledge graph about a public policy debate. \
             For each, say whether the SOURCE entity is expressing support for, or opposition \
             to, the thing at the centre of the debate.\n\n{}\n\n\
             Reply with JSON only: an object mapping each relation type to \"support\", \
             \"oppose\", or \"neutral\". Use \"neutral\" for relations that describe a factual \
             or structural link (regulating, supplying, employing) rather than a position. \
             Doubt, questioning, warning and demanding conditions all count as \"oppose\".",
            unknown.iter().map(|r| format!("- {r}")).collect::<Vec<_>>().join("\n")
        );

        let Ok(v) = llm.chat_json(&[Msg::user(prompt)], 0.0, 600).await else {
            tracing::warn!("stance lexicon: classification failed, falling back to keywords");
            return Self::default();
        };
        let mut learned = HashMap::new();
        for rel in unknown {
            match v.get(&rel).and_then(|s| s.as_str()) {
                Some("support") => {
                    learned.insert(rel, true);
                }
                Some("oppose") => {
                    learned.insert(rel, false);
                }
                _ => {}
            }
        }
        tracing::info!("stance lexicon: learned {} stance-bearing relation types", learned.len());
        Self { learned }
    }
}

/// How many stance edges must point AT an entity that authors none before it is
/// treated as the thing under debate rather than a participant in it.
const DEBATE_SUBJECT_STANCES: usize = 3;

/// Which entities can hold an opinion, by uuid.
///
/// Extraction marks this per entity, but graphs built before that flag existed
/// carry no mark, and a model can still mislabel one. The structural check is
/// the backstop: an entity that everyone takes a position ON while taking none
/// itself is the subject of the debate, not a party to it. A law does not
/// support itself, and a policy given a voice and a faction votes for its own
/// approval — which is how "Law covering 24 electricity investment projects"
/// came to be counted as a supporter.
fn actor_map(graph_data: &GraphData, lexicon: &StanceLexicon) -> HashMap<String, bool> {
    let (mut targeted, mut authored) = (HashMap::new(), HashMap::new());
    for e in &graph_data.edges {
        if lexicon.polarity(&e.relation_type).is_none() {
            continue;
        }
        *targeted.entry(e.target_node_uuid.clone()).or_insert(0usize) += 1;
        *authored.entry(e.source_node_uuid.clone()).or_insert(0usize) += 1;
    }
    graph_data
        .nodes
        .iter()
        .map(|n| {
            let declared = n.attributes.get("actor").and_then(Value::as_bool);
            let is_subject = targeted.get(&n.uuid).copied().unwrap_or(0) >= DEBATE_SUBJECT_STANCES
                && authored.get(&n.uuid).copied().unwrap_or(0) == 0;
            // Unmarked entities stay agents: absent means unknown, not false.
            (n.uuid.clone(), declared.unwrap_or(true) && !is_subject)
        })
        .collect()
}

/// Build one evidence bundle per entity from `db::get_graph_data`'s typed
/// `GraphData` (one struct shared with the producer — a renamed field breaks
/// the build here instead of silently yielding empty facts).
pub fn build_bundles(graph_data: &GraphData, lexicon: &StanceLexicon) -> Vec<EvidenceBundle> {
    let actors = actor_map(graph_data, lexicon);
    let mut bundles: HashMap<String, EvidenceBundle> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for n in &graph_data.nodes {
        if n.uuid.is_empty() {
            continue;
        }
        let entity_type = n.entity_types.first().cloned().unwrap_or_else(|| "Entity".to_string());
        order.push(n.uuid.clone());
        bundles.insert(
            n.uuid.clone(),
            EvidenceBundle {
                uuid: n.uuid.clone(),
                name: n.name.clone(),
                entity_type,
                summary: n.summary.clone(),
                facts: Vec::new(),
                stance_facts: Vec::new(),
                sourced_support: false,
                sourced_oppose: false,
                is_actor: actors.get(&n.uuid).copied().unwrap_or(true),
            },
        );
    }

    for e in &graph_data.edges {
        let fact_text = edge_fact_text(e);
        let stance = lexicon.polarity(&e.relation_type);
        for uuid_key in [&e.source_node_uuid, &e.target_node_uuid] {
            if let Some(b) = bundles.get_mut(uuid_key) {
                b.facts.push(fact_text.clone());
                if stance.is_some() {
                    b.stance_facts.push(fact_text.clone());
                }
            }
        }
        // Only the SOURCE authored the stance; record it for faction derivation.
        if let (Some(supports), Some(b)) = (stance, bundles.get_mut(&e.source_node_uuid)) {
            if supports {
                b.sourced_support = true;
            } else {
                b.sourced_oppose = true;
            }
        }
    }

    order.into_iter().filter_map(|u| bundles.remove(&u)).collect()
}

/// An edge's fact sentence, falling back to "<source> <relation> <target>".
fn edge_fact_text(e: &crate::services::graph::db::GraphEdge) -> String {
    if e.fact.is_empty() {
        format!(
            "{} {} {}",
            e.source_node_name,
            e.relation_type.replace('_', " ").to_lowercase(),
            e.target_node_name
        )
    } else {
        e.fact.clone()
    }
}

/// Does this entity clear the minimum-evidence bar to become a named agent?
pub fn eligible(bundle: &EvidenceBundle, min_evidence: usize) -> bool {
    bundle.is_actor && bundle.evidence_score() >= min_evidence.max(1)
}

/// Cap on facts rendered into a persona prompt. A hub entity can have hundreds
/// of incident edges; pasting all of them into every decision prompt is costly
/// and drowns the signal. Stance facts are kept first.
const MAX_EVIDENCE: usize = 12;

/// The facts to ground a persona on: stance facts first, then other facts,
/// deduped and capped.
fn selected_evidence(bundle: &EvidenceBundle) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(MAX_EVIDENCE);
    for f in bundle.stance_facts.iter().chain(bundle.facts.iter()) {
        if out.len() >= MAX_EVIDENCE {
            break;
        }
        if !out.contains(f) {
            out.push(f.clone());
        }
    }
    out
}

/// Deterministic persona from evidence alone (no LLM, no fabrication).
/// `user_id` is the sequential agent id assigned by the caller.
pub fn to_persona(bundle: &EvidenceBundle, user_id: i64) -> Persona {
    let persona = if bundle.summary.trim().is_empty() {
        format!("A real {} observed in the discussion.", bundle.entity_type)
    } else {
        format!("A real {}. {}", bundle.entity_type, bundle.summary.trim())
    };
    Persona {
        user_id,
        user_name: sanitize_username(&bundle.name, user_id),
        name: bundle.name.clone(),
        bio: bundle.summary.clone(),
        persona,
        age: None,
        gender: None,
        mbti: None,
        country: None,
        profession: None,
        interested_topics: vec![],
        source_entity_uuid: Some(bundle.uuid.clone()),
        source_entity_type: Some(bundle.entity_type.clone()),
        evidence: selected_evidence(bundle),
        synthetic: false,
        faction: bundle.faction().map(String::from),
        position: None, // assigned in a batch by assign_positions after prep
    }
}

/// Optional grounded synthesis: ask the model to write a richer first-person
/// persona + initial stance using ONLY the evidence. Falls back to the
/// deterministic persona on any failure. The evidence/provenance fields are set
/// by the caller (via `to_persona`); this only enriches the prose.
pub async fn synthesize_persona(llm: &Llm, bundle: &EvidenceBundle) -> String {
    let facts = if bundle.facts.is_empty() {
        "(no relationship facts recorded)".to_string()
    } else {
        bundle.facts.iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n")
    };
    let sys = "You write concise social-media personas for an opinion simulation. Use ONLY the \
               provided evidence. Do not invent demographics, personality types, or opinions not \
               supported by the facts. 2-4 sentences, third person, describe how this subject \
               behaves and what stance the evidence shows.";
    let user = format!(
        "Entity: {} (type: {})\nSummary: {}\nObserved facts:\n{}",
        bundle.name,
        bundle.entity_type,
        if bundle.summary.is_empty() { "(none)" } else { &bundle.summary },
        facts,
    );
    match llm.chat(&[Msg::system(sys), Msg::user(user)], 0.4, 400).await {
        Ok(text) if !text.trim().is_empty() => text.trim().to_string(),
        _ => {
            if bundle.summary.trim().is_empty() {
                format!("A real {} observed in the discussion.", bundle.entity_type)
            } else {
                format!("A real {}. {}", bundle.entity_type, bundle.summary.trim())
            }
        }
    }
}

/// Generate synthetic background-population agents from the graph's stance
/// edges, so the simulated population is a *public* rather than only the named
/// elites. Supporters and opponents each get `per_faction` ordinary-citizen
/// agents, seeded with the arguments their camp actually made. All are flagged
/// `synthetic=true` (no source entity) so reports can disclose the synthetic share.
// ponytail: factions split by polarity (single-issue assumption) and weighted
// equally. Weight by observed audience size (followers/subscribers) and split by
// target when the graph carries multiple issues.
pub fn synthesize_audience(
    graph_data: &GraphData,
    lexicon: &StanceLexicon,
    per_faction: usize,
    start_id: i64,
) -> Vec<Persona> {
    if per_faction == 0 {
        return vec![];
    }
    let actors = actor_map(graph_data, lexicon);
    let (mut pro, mut con): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for e in &graph_data.edges {
        let Some(supports) = lexicon.polarity(&e.relation_type) else { continue };
        // A camp is founded on positions its members took. A stance authored by
        // a non-actor is an extraction error, and seeding a camp from it hands
        // the synthetic public arguments nobody made.
        if !actors.get(&e.source_node_uuid).copied().unwrap_or(true) {
            continue;
        }
        let fact = edge_fact_text(e);
        if supports {
            pro.push(fact);
        } else {
            con.push(fact);
        }
    }

    let mut out = Vec::new();
    let mut id = start_id;
    for (label, verb, handle, faction, facts) in
        [("Supporter", "support", "supporter", "pro", &pro), ("Opponent", "oppose", "opponent", "con", &con)]
    {
        if facts.is_empty() {
            continue;
        }
        let sample: Vec<String> = facts.iter().take(3).cloned().collect();
        for i in 0..per_faction {
            let persona = format!(
                "You are an ordinary member of the public who tends to {verb} the proposal at the \
                 centre of this discussion. React authentically as a regular citizen — not an \
                 official or an organization. Views common in your camp: {}",
                sample.join(" | ")
            );
            out.push(Persona {
                user_id: id,
                user_name: format!("{handle}_{id}"),
                name: format!("{label} {}", i + 1),
                bio: String::new(),
                persona,
                age: None,
                gender: None,
                mbti: None,
                country: None,
                profession: None,
                interested_topics: vec![],
                source_entity_uuid: None,
                source_entity_type: Some("synthetic_audience".into()),
                evidence: sample.clone(),
                synthetic: true,
                faction: Some(faction.into()),
                position: None,
            });
            id += 1;
        }
    }
    out
}

/// Assign each Persona a fixed [x, y] in [0,1] on the opinion basemap. x is
/// stance (con left, neutral centre, pro right); y separates entity types into
/// evenly spaced bands. A per-persona deterministic jitter spreads marks inside
/// their stance/type cell so they do not stack. This is the deterministic layout
/// the FIELD map reads: the honest floor that always works and the fallback for
/// thin populations, ahead of any learned embedding.
pub fn assign_positions(personas: &mut [Persona]) {
    // Stable band index per entity type, in first-seen order, so the layout is
    // deterministic and identical across runs of the same population.
    let mut types: Vec<String> = Vec::new();
    for p in personas.iter() {
        let t = p.source_entity_type.clone().unwrap_or_else(|| "Entity".to_string());
        if !types.contains(&t) {
            types.push(t);
        }
    }
    let bands = types.len().max(1) as f32;

    for p in personas.iter_mut() {
        let stance_x = match p.faction.as_deref() {
            Some("con") => 0.2,
            Some("pro") => 0.8,
            _ => 0.5,
        };
        let t = p.source_entity_type.clone().unwrap_or_else(|| "Entity".to_string());
        let band = types.iter().position(|x| *x == t).unwrap_or(0) as f32;
        // Centre of this type's horizontal band.
        let band_y = (band + 0.5) / bands;

        // Deterministic jitter in [-0.5, 0.5) from the id, split into two axes,
        // so marks in the same cell fan out without randomness (reproducible).
        let (jx, jy) = jitter(p.user_id);
        let x = (stance_x + jx * 0.12).clamp(0.02, 0.98);
        let y = (band_y + jy * (0.8 / bands)).clamp(0.02, 0.98);
        p.position = Some([x, y]);
    }
}

/// Two deterministic values in [-0.5, 0.5) derived from an id. A cheap integer
/// hash (splitmix-ish), so the same population always lays out the same way.
fn jitter(id: i64) -> (f32, f32) {
    let mut z = (id as u64).wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x2545F4914F6CDD1D);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27;
    let a = ((z & 0xFFFF) as f32) / 65536.0 - 0.5;
    let b = (((z >> 16) & 0xFFFF) as f32) / 65536.0 - 0.5;
    (a, b)
}

/// Entities grouped by type with evidence scores, for the /prepare/preview picker.
pub fn preview(graph_data: &GraphData, lexicon: &StanceLexicon, min_evidence: usize) -> Value {
    let bundles = build_bundles(graph_data, lexicon);
    let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
    let mut group_order: Vec<String> = Vec::new();
    let (mut eligible_count, mut below_bar) = (0usize, 0usize);

    for b in &bundles {
        let ok = eligible(b, min_evidence);
        if ok {
            eligible_count += 1;
        } else {
            below_bar += 1;
        }
        if !groups.contains_key(&b.entity_type) {
            group_order.push(b.entity_type.clone());
        }
        groups.entry(b.entity_type.clone()).or_default().push(json!({
            "uuid": b.uuid,
            "name": b.name,
            "summary": b.summary,
            "fact_count": b.facts.len(),
            "stance_facts": b.stance_facts.len(),
            "evidence_score": b.evidence_score(),
            "eligible": ok,
            // Distinguishes "too little evidence" from "cannot hold an opinion",
            // so a well-evidenced law doesn't read as an unexplained exclusion.
            "actor": b.is_actor,
        }));
    }

    let grouped: Vec<Value> = group_order
        .into_iter()
        .map(|t| {
            let entities = groups.remove(&t).unwrap_or_default();
            json!({ "entity_type": t, "count": entities.len(), "entities": entities })
        })
        .collect();

    json!({
        "groups": grouped,
        "eligible_count": eligible_count,
        "below_bar_count": below_bar,
        "min_evidence": min_evidence.max(1),
    })
}

fn sanitize_username(name: &str, user_id: i64) -> String {
    let mut clean = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            clean.push(c.to_ascii_lowercase());
        } else if !clean.ends_with('_') {
            clean.push('_');
        }
    }
    let clean: String = clean.trim_matches('_').chars().take(30).collect();
    if clean.is_empty() {
        format!("entity_{user_id}")
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::graph::db::{GraphEdge, GraphNode};
    use crate::sim::agent::Persona;

    fn persona_with(id: i64, faction: Option<&str>, etype: &str) -> Persona {
        Persona {
            user_id: id,
            user_name: format!("u{id}"),
            name: format!("p{id}"),
            bio: String::new(),
            persona: String::new(),
            age: None,
            gender: None,
            mbti: None,
            country: None,
            profession: None,
            interested_topics: vec![],
            source_entity_uuid: None,
            source_entity_type: Some(etype.into()),
            evidence: vec![],
            synthetic: false,
            faction: faction.map(String::from),
            position: None,
        }
    }

    #[test]
    fn positions_encode_stance_on_x_and_are_deterministic() {
        let mut a = vec![
            persona_with(1, Some("con"), "Citizen"),
            persona_with(2, Some("pro"), "Citizen"),
            persona_with(3, None, "Ministry"),
        ];
        assign_positions(&mut a);

        let x = |p: &Persona| p.position.unwrap()[0];
        // Oppose sits left of neutral sits left of support.
        assert!(x(&a[0]) < x(&a[2]), "con should be left of neutral");
        assert!(x(&a[2]) < x(&a[1]), "neutral should be left of pro");
        // Different entity types land in different y bands.
        assert_ne!(a[0].position.unwrap()[1], a[2].position.unwrap()[1]);
        // Everything is inside the map.
        for p in &a {
            let [px, py] = p.position.unwrap();
            assert!((0.0..=1.0).contains(&px) && (0.0..=1.0).contains(&py));
        }

        // Same population lays out identically (no randomness).
        let mut b = vec![
            persona_with(1, Some("con"), "Citizen"),
            persona_with(2, Some("pro"), "Citizen"),
            persona_with(3, None, "Ministry"),
        ];
        assign_positions(&mut b);
        assert_eq!(a[0].position, b[0].position);
    }

    fn node(uuid: &str, name: &str, entity_type: &str, summary: &str) -> GraphNode {
        GraphNode {
            uuid: uuid.into(),
            name: name.into(),
            entity_types: vec![entity_type.into()],
            summary: summary.into(),
            ..Default::default()
        }
    }

    fn non_actor(uuid: &str, name: &str, entity_type: &str, summary: &str) -> GraphNode {
        let mut n = node(uuid, name, entity_type, summary);
        n.attributes.insert("actor".into(), json!(false));
        n
    }

    fn edge(rel: &str, fact: &str, src: (&str, &str), tgt: (&str, &str)) -> GraphEdge {
        GraphEdge {
            relation_type: rel.into(),
            fact: fact.into(),
            source_node_uuid: src.0.into(),
            source_node_name: src.1.into(),
            target_node_uuid: tgt.0.into(),
            target_node_name: tgt.1.into(),
            ..Default::default()
        }
    }

    fn graph(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> GraphData {
        GraphData { nodes, edges, ..Default::default() }
    }

    fn fixture() -> GraphData {
        graph(
            vec![
                node("u1", "Labor Union", "Organization", "Represents factory workers."),
                node("u2", "Minister X", "Politician", "Proposed the policy."),
                node("u3", "Bare Node", "Person", ""),
            ],
            vec![
                edge(
                    "OPPOSES",
                    "The union opposes the policy on wage grounds.",
                    ("u1", "Labor Union"),
                    ("u2", "Minister X"),
                ),
                edge("PROPOSED", "Minister X proposed the policy.", ("u2", "Minister X"), ("u2", "Policy")),
            ],
        )
    }

    /// A graph whose opposition arrived as an invented verb. Keywords alone see
    /// no opponents, so the synthetic public came out entirely pro and the run
    /// reported near-unanimous support that nobody had argued for.
    fn invented_verb_graph() -> GraphData {
        graph(
            vec![
                node("r1", "Residents", "Group", "Live near the site."),
                node("p1", "Project", "Policy", "The approval under debate."),
                node("m1", "Ministry", "Government", "Approved it."),
            ],
            vec![
                edge(
                    "QUESTIONS_BENEFITS_FROM",
                    "Residents question who benefits from the project.",
                    ("r1", "Residents"),
                    ("p1", "Project"),
                ),
                edge(
                    "APPROVES_INVESTMENT_FOR",
                    "The ministry approved investment for the project.",
                    ("m1", "Ministry"),
                    ("p1", "Project"),
                ),
            ],
        )
    }

    /// A law that "supports" its own approval became an agent with faction pro,
    /// and its self-authored stance seeded the synthetic pro camp's arguments.
    #[test]
    fn a_non_actor_never_becomes_an_agent_or_founds_a_camp() {
        let data = graph(
            vec![
                non_actor("l1", "Law covering 24 projects", "Policy", "The approval under debate."),
                node("u1", "Residents", "Group", "Live near the site."),
            ],
            vec![
                edge("SUPPORTS", "The law supports the approval.", ("l1", "Law"), ("l1", "Law")),
                edge("OPPOSES", "Residents oppose it.", ("u1", "Residents"), ("l1", "Law")),
            ],
        );
        let lex = StanceLexicon::keywords_only();

        let bundles = build_bundles(&data, &lex);
        let law = bundles.iter().find(|b| b.uuid == "l1").unwrap();
        assert!(!law.is_actor, "a law cannot hold an opinion");
        assert!(!eligible(law, 1), "and so cannot become an agent, however much evidence it has");

        let residents = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert!(eligible(residents, 1), "a collective of people still can");

        // The law's self-supporting edge must not found the pro camp.
        let audience = synthesize_audience(&data, &lex, 3, 0);
        assert_eq!(audience.iter().filter(|p| p.faction.as_deref() == Some("pro")).count(), 0);
        assert_eq!(audience.iter().filter(|p| p.faction.as_deref() == Some("con")).count(), 3);
    }

    /// Graphs built before the flag existed carry no mark, so actorhood falls to
    /// the structural read: everyone takes a position on it, it takes none.
    #[test]
    fn an_unmarked_debate_subject_is_recognised_structurally() {
        let mut nodes = vec![node("p1", "The proposal", "Policy", "Under debate.")];
        let mut edges = Vec::new();
        for i in 0..DEBATE_SUBJECT_STANCES {
            let id = format!("a{i}");
            nodes.push(node(&id, &format!("Group {i}"), "Group", "A constituency."));
            edges.push(edge("OPPOSES", "Opposes it.", (&id, "Group"), ("p1", "The proposal")));
        }
        let data = graph(nodes, edges);
        let bundles = build_bundles(&data, &StanceLexicon::keywords_only());

        let subject = bundles.iter().find(|b| b.uuid == "p1").unwrap();
        assert!(!subject.is_actor, "targeted by every stance, author of none");
        assert!(bundles.iter().filter(|b| b.uuid != "p1").all(|b| b.is_actor));
    }

    /// An unmarked entity that simply has no stance edges is still an agent —
    /// absent means unknown, not disqualified.
    #[test]
    fn an_unmarked_entity_without_stances_stays_an_agent() {
        let bundles = build_bundles(&fixture(), &StanceLexicon::keywords_only());
        assert!(bundles.iter().find(|b| b.uuid == "u3").unwrap().is_actor);
    }

    #[test]
    fn invented_stance_verbs_are_invisible_to_keywords_alone() {
        let data = invented_verb_graph();
        let bare = StanceLexicon::keywords_only();
        assert_eq!(bare.polarity("QUESTIONS_BENEFITS_FROM"), None);

        let audience = synthesize_audience(&data, &bare, 5, 0);
        assert!(audience.is_empty(), "no camp is recognised, so no public is synthesized");

        let residents = build_bundles(&data, &bare).into_iter().find(|b| b.uuid == "r1").unwrap();
        assert_eq!(residents.faction(), None, "objectors read as neutral");
    }

    #[test]
    fn a_learned_lexicon_recovers_both_camps() {
        let data = invented_verb_graph();
        let learned = StanceLexicon::from_pairs([
            ("QUESTIONS_BENEFITS_FROM".to_string(), false),
            ("APPROVES_INVESTMENT_FOR".to_string(), true),
        ]);
        assert_eq!(learned.polarity("questions_benefits_from"), Some(false), "case-insensitive");
        // Keywords still win where they apply, so learning cannot invert them.
        assert_eq!(learned.polarity("OPPOSES"), Some(false));

        let audience = synthesize_audience(&data, &learned, 5, 0);
        let pro = audience.iter().filter(|p| p.faction.as_deref() == Some("pro")).count();
        let con = audience.iter().filter(|p| p.faction.as_deref() == Some("con")).count();
        assert_eq!((pro, con), (5, 5), "both camps populated, so the public is not one-sided");

        let residents = build_bundles(&data, &learned).into_iter().find(|b| b.uuid == "r1").unwrap();
        assert_eq!(residents.faction(), Some("con"), "objectors are counted as opposed");
    }

    #[test]
    fn bundles_gather_summary_and_incident_facts() {
        let bundles = build_bundles(&fixture(), &StanceLexicon::keywords_only());
        let u1 = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert_eq!(u1.facts.len(), 1);
        assert_eq!(u1.stance_facts.len(), 1, "OPPOSES is a stance edge");
        assert!(u1.stance_facts[0].contains("opposes"));
    }

    #[test]
    fn bar_excludes_bare_nodes() {
        let bundles = build_bundles(&fixture(), &StanceLexicon::keywords_only());
        let bare = bundles.iter().find(|b| b.uuid == "u3").unwrap();
        assert_eq!(bare.evidence_score(), 0);
        assert!(!eligible(bare, 2), "no summary, no facts -> background population");
        let union = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert!(eligible(union, 2), "summary + 1 fact clears the bar");
    }

    #[test]
    fn profile_is_grounded_and_not_synthetic() {
        let bundles = build_bundles(&fixture(), &StanceLexicon::keywords_only());
        let union = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        let p = to_persona(union, 0);
        assert!(!p.synthetic);
        assert_eq!(p.source_entity_uuid.as_deref(), Some("u1"));
        assert_eq!(p.source_entity_type.as_deref(), Some("Organization"));
        assert!(p.mbti.is_none(), "no fabricated personality");
        assert!(!p.evidence.is_empty(), "carries the facts it rests on");
        // The evidence shows up in the prompt block.
        assert!(p.persona_prompt().contains("opposes the policy"));
    }

    #[test]
    fn faction_from_outgoing_stance_edges() {
        let bundles = build_bundles(&fixture(), &StanceLexicon::keywords_only());
        let union = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert_eq!(union.faction(), Some("con"), "sources an OPPOSES edge");
        assert_eq!(to_persona(union, 0).faction.as_deref(), Some("con"));
        let minister = bundles.iter().find(|b| b.uuid == "u2").unwrap();
        assert_eq!(minister.faction(), None, "target of stances, source of none -> neutral");
        assert!(to_persona(minister, 1).faction.is_none());
    }

    #[test]
    fn faction_neutral_when_stances_mixed() {
        let g = graph(
            vec![node("u1", "Fence Sitter", "Person", "s")],
            vec![
                edge("SUPPORTS", "backs part A", ("u1", "Fence Sitter"), ("x", "A")),
                edge("OPPOSES", "rejects part B", ("u1", "Fence Sitter"), ("y", "B")),
            ],
        );
        let bundles = build_bundles(&g, &StanceLexicon::keywords_only());
        assert_eq!(bundles[0].faction(), None, "both camps -> no single faction");
    }

    #[test]
    fn audience_synthesizes_both_factions_marked_synthetic() {
        let g = graph(
            vec![],
            vec![
                edge("SUPPORTS", "A backs the plan on cost grounds.", ("", "A"), ("", "P")),
                edge("OPPOSES", "B rejects the plan over job losses.", ("", "B"), ("", "P")),
                edge("AFFILIATED_WITH", "C works for P.", ("", "C"), ("", "P")),
            ],
        );
        let a = synthesize_audience(&g, &StanceLexicon::keywords_only(), 2, 100);
        assert_eq!(a.len(), 4, "2 per faction × 2 factions; non-stance edge ignored");
        assert!(a.iter().all(|p| p.synthetic && p.source_entity_uuid.is_none()));
        assert!(a.iter().any(|p| p.name.starts_with("Supporter")));
        assert!(a.iter().any(|p| p.name.starts_with("Opponent")));
        assert_eq!(a[0].user_id, 100, "ids continue from start_id");
        // The camp's real arguments seed the persona; the camp is its faction.
        let sup = a.iter().find(|p| p.name.starts_with("Supporter")).unwrap();
        assert!(sup.persona.contains("backs the plan"));
        assert_eq!(sup.faction.as_deref(), Some("pro"));
        let opp = a.iter().find(|p| p.name.starts_with("Opponent")).unwrap();
        assert_eq!(opp.faction.as_deref(), Some("con"));
    }

    #[test]
    fn audience_empty_when_no_stance_edges_or_zero_count() {
        let g = graph(vec![], vec![edge("AFFILIATED_WITH", "", ("", "A"), ("", "B"))]);
        assert!(synthesize_audience(&g, &StanceLexicon::keywords_only(), 5, 0).is_empty());
        assert!(synthesize_audience(&fixture(), &StanceLexicon::keywords_only(), 0, 0).is_empty());
    }

    #[test]
    fn preview_groups_and_counts() {
        let out = preview(&fixture(), &StanceLexicon::keywords_only(), 2);
        assert_eq!(out["eligible_count"], 2); // union + minister
        assert_eq!(out["below_bar_count"], 1); // bare node
        let groups = out["groups"].as_array().unwrap();
        assert!(groups.iter().any(|g| g["entity_type"] == "Organization"));
    }
}
