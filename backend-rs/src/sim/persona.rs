//! Graph → persona evidence compiler. Turns a knowledge-graph entity into a
//! simulation persona built ONLY from observed evidence — the entity's summary
//! and the facts on its relationships (including SUPPORTS/OPPOSES stance edges).
//! No fabricated attributes; every persona carries provenance back to its entity.
//!
//! This is the link that makes the graph (the "God's-eye" view of the opinion
//! landscape) actually reach the simulation.

use crate::llm::{Llm, Msg};
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
fn stance_polarity(rel: &str) -> Option<bool> {
    let r = rel.to_uppercase();
    if ["SUPPORT", "ENDORS", "BACK", "FAVOR"].iter().any(|k| r.contains(k)) {
        Some(true)
    } else if ["OPPOS", "AGAINST", "CRITIC", "CONDEMN", "REJECT"].iter().any(|k| r.contains(k)) {
        Some(false)
    } else {
        None
    }
}

/// Build one evidence bundle per entity from `db::get_graph_data` output
/// (`{nodes:[{uuid,name,entity_types,summary,attributes}], edges:[{fact,
/// name(relation_type),source_node_uuid,target_node_uuid,source_node_name,
/// target_node_name}]}`).
pub fn build_bundles(graph_data: &Value) -> Vec<EvidenceBundle> {
    let empty = vec![];
    let nodes = graph_data["nodes"].as_array().unwrap_or(&empty);
    let edges = graph_data["edges"].as_array().unwrap_or(&empty);

    let mut bundles: HashMap<String, EvidenceBundle> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for n in nodes {
        let uuid = n["uuid"].as_str().unwrap_or("").to_string();
        if uuid.is_empty() {
            continue;
        }
        let entity_type = n["entity_types"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("Entity")
            .to_string();
        order.push(uuid.clone());
        bundles.insert(
            uuid.clone(),
            EvidenceBundle {
                uuid,
                name: n["name"].as_str().unwrap_or("").to_string(),
                entity_type,
                summary: n["summary"].as_str().unwrap_or("").to_string(),
                facts: Vec::new(),
                stance_facts: Vec::new(),
                sourced_support: false,
                sourced_oppose: false,
            },
        );
    }

    for e in edges {
        let rel = e["name"].as_str().or_else(|| e["fact_type"].as_str()).unwrap_or("RELATED_TO");
        let src_name = e["source_node_name"].as_str().unwrap_or("");
        let tgt_name = e["target_node_name"].as_str().unwrap_or("");
        let fact_text = e["fact"].as_str().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| {
            format!("{src_name} {} {tgt_name}", rel.replace('_', " ").to_lowercase())
        });
        let stance = stance_polarity(rel);
        for uuid_key in [e["source_node_uuid"].as_str(), e["target_node_uuid"].as_str()].into_iter().flatten() {
            if let Some(b) = bundles.get_mut(uuid_key) {
                b.facts.push(fact_text.clone());
                if stance.is_some() {
                    b.stance_facts.push(fact_text.clone());
                }
            }
        }
        // Only the SOURCE authored the stance; record it for faction derivation.
        if let (Some(supports), Some(b)) =
            (stance, e["source_node_uuid"].as_str().and_then(|u| bundles.get_mut(u)))
        {
            if supports {
                b.sourced_support = true;
            } else {
                b.sourced_oppose = true;
            }
        }
    }

    order.into_iter().filter_map(|u| bundles.remove(&u)).collect()
}

/// Does this entity clear the minimum-evidence bar to become a named agent?
pub fn eligible(bundle: &EvidenceBundle, min_evidence: usize) -> bool {
    bundle.evidence_score() >= min_evidence.max(1)
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
pub fn synthesize_audience(graph_data: &Value, per_faction: usize, start_id: i64) -> Vec<Persona> {
    if per_faction == 0 {
        return vec![];
    }
    let empty = vec![];
    let (mut pro, mut con): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for e in graph_data["edges"].as_array().unwrap_or(&empty) {
        let rel = e["name"].as_str().or_else(|| e["fact_type"].as_str()).unwrap_or("");
        let Some(supports) = stance_polarity(rel) else { continue };
        let src = e["source_node_name"].as_str().unwrap_or("");
        let tgt = e["target_node_name"].as_str().unwrap_or("");
        let fact = e["fact"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .unwrap_or_else(|| format!("{src} {} {tgt}", rel.replace('_', " ").to_lowercase()));
        if supports {
            pro.push(fact);
        } else {
            con.push(fact);
        }
    }

    let mut out = Vec::new();
    let mut id = start_id;
    for (label, verb, faction, facts) in
        [("Supporter", "support", "pro", &pro), ("Opponent", "oppose", "con", &con)]
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
                user_name: format!("{verb}er_{id}"),
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
            });
            id += 1;
        }
    }
    out
}

/// Entities grouped by type with evidence scores, for the /prepare/preview picker.
pub fn preview(graph_data: &Value, min_evidence: usize) -> Value {
    let bundles = build_bundles(graph_data);
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

    fn fixture() -> Value {
        json!({
            "nodes": [
                {"uuid": "u1", "name": "Labor Union", "entity_types": ["Organization"], "summary": "Represents factory workers."},
                {"uuid": "u2", "name": "Minister X", "entity_types": ["Politician"], "summary": "Proposed the policy."},
                {"uuid": "u3", "name": "Bare Node", "entity_types": ["Person"], "summary": ""}
            ],
            "edges": [
                {"name": "OPPOSES", "fact": "The union opposes the policy on wage grounds.",
                 "source_node_uuid": "u1", "target_node_uuid": "u2",
                 "source_node_name": "Labor Union", "target_node_name": "Minister X"},
                {"name": "PROPOSED", "fact": "Minister X proposed the policy.",
                 "source_node_uuid": "u2", "target_node_uuid": "u2",
                 "source_node_name": "Minister X", "target_node_name": "Policy"}
            ]
        })
    }

    #[test]
    fn bundles_gather_summary_and_incident_facts() {
        let bundles = build_bundles(&fixture());
        let u1 = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert_eq!(u1.facts.len(), 1);
        assert_eq!(u1.stance_facts.len(), 1, "OPPOSES is a stance edge");
        assert!(u1.stance_facts[0].contains("opposes"));
    }

    #[test]
    fn bar_excludes_bare_nodes() {
        let bundles = build_bundles(&fixture());
        let bare = bundles.iter().find(|b| b.uuid == "u3").unwrap();
        assert_eq!(bare.evidence_score(), 0);
        assert!(!eligible(bare, 2), "no summary, no facts -> background population");
        let union = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert!(eligible(union, 2), "summary + 1 fact clears the bar");
    }

    #[test]
    fn profile_is_grounded_and_not_synthetic() {
        let bundles = build_bundles(&fixture());
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
        let bundles = build_bundles(&fixture());
        let union = bundles.iter().find(|b| b.uuid == "u1").unwrap();
        assert_eq!(union.faction(), Some("con"), "sources an OPPOSES edge");
        assert_eq!(to_persona(union, 0).faction.as_deref(), Some("con"));
        let minister = bundles.iter().find(|b| b.uuid == "u2").unwrap();
        assert_eq!(minister.faction(), None, "target of stances, source of none -> neutral");
        assert!(to_persona(minister, 1).faction.is_none());
    }

    #[test]
    fn faction_neutral_when_stances_mixed() {
        let g = json!({
            "nodes": [{"uuid": "u1", "name": "Fence Sitter", "entity_types": ["Person"], "summary": "s"}],
            "edges": [
                {"name": "SUPPORTS", "fact": "backs part A", "source_node_uuid": "u1", "target_node_uuid": "x",
                 "source_node_name": "Fence Sitter", "target_node_name": "A"},
                {"name": "OPPOSES", "fact": "rejects part B", "source_node_uuid": "u1", "target_node_uuid": "y",
                 "source_node_name": "Fence Sitter", "target_node_name": "B"}
            ]
        });
        let bundles = build_bundles(&g);
        assert_eq!(bundles[0].faction(), None, "both camps -> no single faction");
    }

    #[test]
    fn audience_synthesizes_both_factions_marked_synthetic() {
        let g = json!({"nodes": [], "edges": [
            {"name": "SUPPORTS", "fact": "A backs the plan on cost grounds.", "source_node_name": "A", "target_node_name": "P"},
            {"name": "OPPOSES", "fact": "B rejects the plan over job losses.", "source_node_name": "B", "target_node_name": "P"},
            {"name": "AFFILIATED_WITH", "fact": "C works for P.", "source_node_name": "C", "target_node_name": "P"}
        ]});
        let a = synthesize_audience(&g, 2, 100);
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
        let g = json!({"nodes": [], "edges": [{"name": "AFFILIATED_WITH", "source_node_name": "A", "target_node_name": "B"}]});
        assert!(synthesize_audience(&g, 5, 0).is_empty());
        assert!(synthesize_audience(&fixture(), 0, 0).is_empty());
    }

    #[test]
    fn preview_groups_and_counts() {
        let out = preview(&fixture(), 2);
        assert_eq!(out["eligible_count"], 2); // union + minister
        assert_eq!(out["below_bar_count"], 1); // bare node
        let groups = out["groups"].as_array().unwrap();
        assert!(groups.iter().any(|g| g["entity_type"] == "Organization"));
    }
}
