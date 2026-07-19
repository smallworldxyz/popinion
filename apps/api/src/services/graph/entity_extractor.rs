//! LLM entity/relation extraction.
//! One chat_json call per chunk returning {entities, relationships}, then
//! deterministic normalization + dedupe.

use crate::llm::{Llm, Msg};
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// A normalized extraction — the typed contract between the extractor and
/// `builder::write_extraction`. `normalize` guarantees every entity has a
/// non-empty name and at least one entity type, and every relationship has
/// non-empty source, target and relation_type.
pub struct Extraction {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
}

pub struct ExtractedEntity {
    pub name: String,
    pub entity_types: Vec<String>,
    pub attributes: Map<String, Value>,
}

pub struct ExtractedRelationship {
    pub source_name: String,
    pub target_name: String,
    pub relation_type: String,
    pub attributes: Map<String, Value>,
}

/// Extract entities and relationships from a text chunk, guided by the ontology.
pub async fn extract(llm: &Llm, text: &str, ontology: &Value) -> Result<Extraction> {
    let prompt = build_prompt(text, ontology);
    let messages = [
        Msg::system(
            "You are an expert at extracting structured information from text. \
             Extract entities and relationships according to the provided ontology. \
             Return valid JSON only, no explanations.",
        ),
        Msg::user(prompt),
    ];
    let raw = llm.chat_json(&messages, 0.3, 4096).await?;
    Ok(normalize(raw))
}

fn build_prompt(text: &str, ontology: &Value) -> String {
    let empty = vec![];
    let entity_types = ontology["entity_types"].as_array().unwrap_or(&empty);
    let edge_types = ontology["edge_types"].as_array().unwrap_or(&empty);

    let entity_descriptions: String = entity_types
        .iter()
        .map(|et| {
            let attrs: Vec<String> = et["attributes"]
                .as_array()
                .unwrap_or(&empty)
                .iter()
                .filter_map(|a| a["name"].as_str().map(|n| format!("\"{n}\"")))
                .collect();
            let attr_list = if attrs.is_empty() { "none specified".to_string() } else { attrs.join(", ") };
            format!(
                "  - **{}**: {}\n    Required attributes: [{}]",
                et["name"].as_str().unwrap_or("?"),
                et["description"].as_str().unwrap_or("No description"),
                attr_list
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let edge_descriptions: String = edge_types
        .iter()
        .map(|et| {
            format!(
                "  - {}: {}",
                et["name"].as_str().unwrap_or("?"),
                et["description"].as_str().unwrap_or("No description")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut attribute_schema = Map::new();
    for et in entity_types {
        let Some(name) = et["name"].as_str() else { continue };
        let mut attrs = Map::new();
        for a in et["attributes"].as_array().unwrap_or(&empty) {
            if let Some(an) = a["name"].as_str() {
                attrs.insert(an.into(), a["description"].clone());
            }
        }
        attribute_schema.insert(name.into(), Value::Object(attrs));
    }
    let schema_json = serde_json::to_string_pretty(&attribute_schema).unwrap_or_default();

    format!(
        r#"You are an expert entity extractor for knowledge graph construction.
Extract ALL entities and relationships from the text below.

**IMPORTANT: Be thorough and extract RICH information for each entity!**

**Entity Types to Extract:**
{entity_descriptions}

**Relationship Types to Extract:**
{edge_descriptions}

**Attribute Schema (extract ALL these attributes for each entity type):**
{schema_json}

**Extraction Instructions:**
1. Find ALL entities matching the defined types in the text
2. For EACH entity, extract:
   - name: The entity's proper name
   - entity_types: The entity type(s) from the schema
   - summary: A 2-3 sentence description of the entity based on the text
   - attributes: Extract ALL attributes defined in the schema above!
3. Extract ALL relationships between entities
4. For relationships, include a "fact" attribute describing the relationship
5. STANCE IS MANDATORY. This graph drives an opinion simulation, so a position
   taken in the text must become an edge, not just prose in a summary. Whenever
   an entity backs, welcomes or defends the matter at hand, emit a `SUPPORTS`
   edge from that entity. Whenever an entity criticises, doubts, questions,
   warns about, or demands conditions on it, emit an `OPPOSES` edge from that
   entity. Use these exact relation names for positions; do not soften them
   into structural verbs such as COMMENTS_ON or QUESTIONS_BENEFITS_FROM. If the
   text contains disagreement and you emit no `OPPOSES` edge, the extraction is
   wrong.
6. Set `"actor"` on every entity. `true` means the entity can HOLD AN OPINION
   and speak: a person, an organisation, a company, an institution, or a
   COLLECTIVE of people. Collectives are actors - "rooftop solar owners",
   "residents of X province", a union, "small business owners" all count, and
   they matter most, because they are the public.
   `false` means the entity is a thing being talked ABOUT and cannot hold a
   view: a law, a policy, a project, a budget, an investment package, a place
   name, an event, a document, an abstract topic.
   Still extract non-actors - they are what the actors take positions on, so
   they must exist as the TARGET of `SUPPORTS` and `OPPOSES` edges. A law never
   supports itself; if you are about to emit a stance edge whose SOURCE is a
   non-actor, the source is wrong.

**Text to Analyze:**
{text}

**Return JSON format:**
{{
  "entities": [
    {{
      "name": "Entity Name",
      "entity_types": ["EntityType"],
      "actor": true,
      "summary": "2-3 sentence description",
      "attributes": {{"attribute1": "value1"}}
    }}
  ],
  "relationships": [
    {{
      "source_name": "Entity1 Name",
      "target_name": "Entity2 Name",
      "relation_type": "RELATIONSHIP_TYPE",
      "attributes": {{"fact": "Descriptive sentence about this relationship"}}
    }}
  ]
}}

Return ONLY valid JSON, no explanations.
"#
    )
}

/// Normalize a raw LLM extraction into a typed `Extraction`: drop malformed
/// items, coerce entity-type strings to lists, fold `summary` into attributes,
/// dedupe entities by name and relationships by (source, relation_type, target).
pub fn normalize(raw: Value) -> Extraction {
    let mut entities = Vec::new();
    let mut seen_names = HashSet::new();
    for entity in raw["entities"].as_array().cloned().unwrap_or_default() {
        let Some(obj) = entity.as_object() else { continue };
        let name = obj.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let entity_types: Vec<String> = match obj.get("entity_types") {
            Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(String::from).collect(),
            _ => vec![],
        };
        if entity_types.is_empty() || !seen_names.insert(name.to_lowercase()) {
            continue;
        }
        let mut attributes = match obj.get("attributes") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        if let Some(summary) = obj.get("summary").and_then(Value::as_str) {
            if !summary.is_empty() && !attributes.contains_key("summary") {
                attributes.insert("summary".into(), json!(summary));
            }
        }
        // Can this entity hold an opinion? Absent means unknown, not false: a
        // graph built before the flag existed must keep its agents, so the
        // decision falls to the structural check in persona compilation.
        if let Some(actor) = obj.get("actor").and_then(Value::as_bool) {
            attributes.insert("actor".into(), json!(actor));
        }
        entities.push(ExtractedEntity { name, entity_types, attributes: sanitize_properties(attributes) });
    }

    let mut relationships = Vec::new();
    let mut seen_rels = HashSet::new();
    for rel in raw["relationships"].as_array().cloned().unwrap_or_default() {
        let Some(obj) = rel.as_object() else { continue };
        let source = obj.get("source_name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let target = obj.get("target_name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let relation_type = obj.get("relation_type").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if source.is_empty() || target.is_empty() || relation_type.is_empty() {
            continue;
        }
        let key = format!("{}|{}|{}", source.to_lowercase(), relation_type, target.to_lowercase());
        if !seen_rels.insert(key) {
            continue;
        }
        let attributes = match obj.get("attributes") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        relationships.push(ExtractedRelationship {
            source_name: source,
            target_name: target,
            relation_type,
            attributes: sanitize_properties(attributes),
        });
    }

    Extraction { entities, relationships }
}

/// Attribute values must be primitives or arrays of primitives (a rule kept
/// from the Neo4j layer so stored shapes stay stable); the LLM sometimes emits
/// nested objects. Stringify anything else, drop nulls.
fn sanitize_properties(props: Map<String, Value>) -> Map<String, Value> {
    props
        .into_iter()
        .filter_map(|(k, v)| match v {
            Value::Null => None,
            Value::String(_) | Value::Number(_) | Value::Bool(_) => Some((k, v)),
            Value::Array(a) if a.iter().all(|x| !x.is_object() && !x.is_array()) => {
                Some((k, Value::Array(a)))
            }
            other => Some((k, json!(other.to_string()))),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_entity_type_string_to_list() {
        let out = normalize(json!({
            "entities": [{"name": "Alice", "entity_types": "Person"}],
            "relationships": []
        }));
        assert_eq!(out.entities[0].entity_types, vec!["Person".to_string()]);
    }

    #[test]
    fn drops_entities_without_name_or_entity_types() {
        let out = normalize(json!({
            "entities": [
                {"name": "", "entity_types": ["Person"]},
                {"name": "NoTypes", "entity_types": []},
                {"name": "Good", "entity_types": ["Person"]},
                "not an object"
            ],
            "relationships": []
        }));
        assert_eq!(out.entities.len(), 1);
        assert_eq!(out.entities[0].name, "Good");
    }

    #[test]
    fn dedupes_entities_case_insensitively() {
        let out = normalize(json!({
            "entities": [
                {"name": "Alice", "entity_types": ["Person"]},
                {"name": "alice", "entity_types": ["Student"]}
            ],
            "relationships": []
        }));
        assert_eq!(out.entities.len(), 1);
    }

    #[test]
    fn folds_summary_into_attributes() {
        let out = normalize(json!({
            "entities": [{"name": "A", "entity_types": ["Person"], "summary": "the summary"}],
            "relationships": []
        }));
        assert_eq!(out.entities[0].attributes["summary"], "the summary");
    }

    #[test]
    fn dedupes_and_validates_relationships() {
        let out = normalize(json!({
            "entities": [],
            "relationships": [
                {"source_name": "A", "target_name": "B", "relation_type": "KNOWS"},
                {"source_name": "a", "target_name": "b", "relation_type": "KNOWS"},
                {"source_name": "A", "target_name": "", "relation_type": "KNOWS"},
                {"source_name": "A", "target_name": "B", "relation_type": ""}
            ]
        }));
        assert_eq!(out.relationships.len(), 1);
    }

    #[test]
    fn sanitizes_nested_attribute_objects() {
        let out = normalize(json!({
            "entities": [{
                "name": "A",
                "entity_types": ["Person"],
                "attributes": {
                    "ok": "text",
                    "num": 3,
                    "list": ["a", "b"],
                    "nested": {"x": 1},
                    "gone": null
                }
            }],
            "relationships": []
        }));
        let props = &out.entities[0].attributes;
        assert_eq!(props["ok"], "text");
        assert_eq!(props["num"], 3);
        assert_eq!(props["list"], json!(["a", "b"]));
        assert!(props["nested"].is_string());
        assert!(props.get("gone").is_none());
    }

    #[test]
    fn handles_missing_arrays() {
        let out = normalize(json!({"unexpected": true}));
        assert!(out.entities.is_empty());
        assert!(out.relationships.is_empty());
    }
}
