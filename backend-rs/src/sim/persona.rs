//! Graph → persona evidence compiler. Turns a knowledge-graph entity into a
//! simulation persona built ONLY from observed evidence — the entity's summary
//! and the facts on its relationships (including SUPPORTS/OPPOSES stance edges).
//! No fabricated attributes; every persona carries provenance back to its entity.
//!
//! This is the link that makes the graph (the "God's-eye" view of the opinion
//! landscape) actually reach the simulation.

use crate::llm::{Llm, Msg};
use crate::sim::agent::AgentProfile;
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
}

impl EvidenceBundle {
    /// A single number for the minimum-evidence bar: how much the graph actually
    /// knows about this entity. A bare name-only node scores 0.
    pub fn evidence_score(&self) -> usize {
        self.facts.len() + usize::from(!self.summary.trim().is_empty())
    }
}

fn is_stance_rel(rel: &str) -> bool {
    let r = rel.to_uppercase();
    ["SUPPORT", "OPPOS", "ENDORS", "CRITIC", "AGAINST", "BACK", "CONDEMN"]
        .iter()
        .any(|k| r.contains(k))
}

/// Build one evidence bundle per entity from `neo4j::get_graph_data` output
/// (`{nodes:[{uuid,name,labels,summary,attributes}], edges:[{fact,name(rel),
/// source_node_uuid,target_node_uuid,source_node_name,target_node_name}]}`).
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
        let entity_type = n["labels"]
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
        let stance = is_stance_rel(rel);
        for uuid_key in [e["source_node_uuid"].as_str(), e["target_node_uuid"].as_str()].into_iter().flatten() {
            if let Some(b) = bundles.get_mut(uuid_key) {
                b.facts.push(fact_text.clone());
                if stance {
                    b.stance_facts.push(fact_text.clone());
                }
            }
        }
    }

    order.into_iter().filter_map(|u| bundles.remove(&u)).collect()
}

/// Does this entity clear the minimum-evidence bar to become a named agent?
pub fn eligible(bundle: &EvidenceBundle, min_evidence: usize) -> bool {
    bundle.evidence_score() >= min_evidence.max(1)
}

/// Deterministic persona from evidence alone (no LLM, no fabrication).
/// `user_id` is the sequential agent id assigned by the caller.
pub fn to_profile(bundle: &EvidenceBundle, user_id: i64) -> AgentProfile {
    let persona = if bundle.summary.trim().is_empty() {
        format!("A real {} observed in the discussion.", bundle.entity_type)
    } else {
        format!("A real {}. {}", bundle.entity_type, bundle.summary.trim())
    };
    AgentProfile {
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
        evidence: bundle.facts.clone(),
        synthetic: false,
    }
}

/// Optional grounded synthesis: ask the model to write a richer first-person
/// persona + initial stance using ONLY the evidence. Falls back to the
/// deterministic persona on any failure. The evidence/provenance fields are set
/// by the caller (via `to_profile`); this only enriches the prose.
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
                {"uuid": "u1", "name": "Labor Union", "labels": ["Organization"], "summary": "Represents factory workers."},
                {"uuid": "u2", "name": "Minister X", "labels": ["Politician"], "summary": "Proposed the policy."},
                {"uuid": "u3", "name": "Bare Node", "labels": ["Person"], "summary": ""}
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
        let p = to_profile(union, 0);
        assert!(!p.synthetic);
        assert_eq!(p.source_entity_uuid.as_deref(), Some("u1"));
        assert_eq!(p.source_entity_type.as_deref(), Some("Organization"));
        assert!(p.mbti.is_none(), "no fabricated personality");
        assert!(!p.evidence.is_empty(), "carries the facts it rests on");
        // The evidence shows up in the prompt block.
        assert!(p.persona_prompt().contains("opposes the policy"));
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
