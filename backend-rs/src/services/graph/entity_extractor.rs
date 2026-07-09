//! LLM entity/relation extraction (port of llm_entity_extractor.py).
//! One chat_json call per chunk returning {entities, relationships}, then
//! deterministic normalization + dedupe.

use crate::llm::{Llm, Msg};
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// Extract entities and relationships from a text chunk, guided by the ontology.
pub async fn extract(llm: &Llm, text: &str, ontology: &Value) -> Result<Value> {
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
   - labels: The entity type(s) from the schema
   - summary: A 2-3 sentence description of the entity based on the text
   - properties: Extract ALL attributes defined in the schema above!
3. Extract ALL relationships between entities
4. For relationships, include a "fact" property describing the relationship

**Text to Analyze:**
{text}

**Return JSON format:**
{{
  "entities": [
    {{
      "name": "Entity Name",
      "labels": ["EntityType"],
      "summary": "2-3 sentence description",
      "properties": {{"attribute1": "value1"}}
    }}
  ],
  "relationships": [
    {{
      "source_name": "Entity1 Name",
      "target_name": "Entity2 Name",
      "type": "RELATIONSHIP_TYPE",
      "properties": {{"fact": "Descriptive sentence about this relationship"}}
    }}
  ]
}}

Return ONLY valid JSON, no explanations.
"#
    )
}

/// Normalize a raw LLM extraction: drop malformed items, coerce label strings
/// to lists, fold `summary` into properties, dedupe entities by name and
/// relationships by (source, type, target).
pub fn normalize(raw: Value) -> Value {
    let mut entities = Vec::new();
    let mut seen_names = HashSet::new();
    for entity in raw["entities"].as_array().cloned().unwrap_or_default() {
        let Some(obj) = entity.as_object() else { continue };
        let name = obj.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let labels: Vec<String> = match obj.get("labels") {
            Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(String::from).collect(),
            _ => vec![],
        };
        if labels.is_empty() || !seen_names.insert(name.to_lowercase()) {
            continue;
        }
        let mut properties = match obj.get("properties") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        if let Some(summary) = obj.get("summary").and_then(Value::as_str) {
            if !summary.is_empty() && !properties.contains_key("summary") {
                properties.insert("summary".into(), json!(summary));
            }
        }
        entities.push(json!({
            "name": name,
            "labels": labels,
            "properties": sanitize_properties(properties),
        }));
    }

    let mut relationships = Vec::new();
    let mut seen_rels = HashSet::new();
    for rel in raw["relationships"].as_array().cloned().unwrap_or_default() {
        let Some(obj) = rel.as_object() else { continue };
        let source = obj.get("source_name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let target = obj.get("target_name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let rel_type = obj.get("type").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if source.is_empty() || target.is_empty() || rel_type.is_empty() {
            continue;
        }
        let key = format!("{}|{}|{}", source.to_lowercase(), rel_type, target.to_lowercase());
        if !seen_rels.insert(key) {
            continue;
        }
        let properties = match obj.get("properties") {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        relationships.push(json!({
            "source_name": source,
            "target_name": target,
            "type": rel_type,
            "properties": sanitize_properties(properties),
        }));
    }

    json!({ "entities": entities, "relationships": relationships })
}

/// Neo4j property values must be primitives or arrays of primitives; the LLM
/// sometimes emits nested objects. Stringify anything else, drop nulls.
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
    fn normalizes_labels_string_to_list() {
        let out = normalize(json!({
            "entities": [{"name": "Alice", "labels": "Person"}],
            "relationships": []
        }));
        assert_eq!(out["entities"][0]["labels"], json!(["Person"]));
    }

    #[test]
    fn drops_entities_without_name_or_labels() {
        let out = normalize(json!({
            "entities": [
                {"name": "", "labels": ["Person"]},
                {"name": "NoLabels", "labels": []},
                {"name": "Good", "labels": ["Person"]},
                "not an object"
            ],
            "relationships": []
        }));
        assert_eq!(out["entities"].as_array().unwrap().len(), 1);
        assert_eq!(out["entities"][0]["name"], "Good");
    }

    #[test]
    fn dedupes_entities_case_insensitively() {
        let out = normalize(json!({
            "entities": [
                {"name": "Alice", "labels": ["Person"]},
                {"name": "alice", "labels": ["Student"]}
            ],
            "relationships": []
        }));
        assert_eq!(out["entities"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn folds_summary_into_properties() {
        let out = normalize(json!({
            "entities": [{"name": "A", "labels": ["Person"], "summary": "the summary"}],
            "relationships": []
        }));
        assert_eq!(out["entities"][0]["properties"]["summary"], "the summary");
    }

    #[test]
    fn dedupes_and_validates_relationships() {
        let out = normalize(json!({
            "entities": [],
            "relationships": [
                {"source_name": "A", "target_name": "B", "type": "KNOWS"},
                {"source_name": "a", "target_name": "b", "type": "KNOWS"},
                {"source_name": "A", "target_name": "", "type": "KNOWS"},
                {"source_name": "A", "target_name": "B", "type": ""}
            ]
        }));
        assert_eq!(out["relationships"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn sanitizes_nested_property_objects() {
        let out = normalize(json!({
            "entities": [{
                "name": "A",
                "labels": ["Person"],
                "properties": {
                    "ok": "text",
                    "num": 3,
                    "list": ["a", "b"],
                    "nested": {"x": 1},
                    "gone": null
                }
            }],
            "relationships": []
        }));
        let props = &out["entities"][0]["properties"];
        assert_eq!(props["ok"], "text");
        assert_eq!(props["num"], 3);
        assert_eq!(props["list"], json!(["a", "b"]));
        assert!(props["nested"].is_string());
        assert!(props.get("gone").is_none());
    }

    #[test]
    fn handles_missing_arrays() {
        let out = normalize(json!({"unexpected": true}));
        assert_eq!(out["entities"], json!([]));
        assert_eq!(out["relationships"], json!([]));
    }
}
