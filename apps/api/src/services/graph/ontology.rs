//! Ontology generation.
//! One LLM call proposing entity types + relation types for the simulation,
//! then deterministic validation/normalization of the returned JSON.

use crate::llm::{Llm, Msg};
use anyhow::Result;
use serde_json::{json, Value};

const MAX_ENTITY_TYPES: usize = 10;
const MAX_EDGE_TYPES: usize = 10;
const MAX_TEXT_LENGTH_FOR_LLM: usize = 50_000;

const SYSTEM_PROMPT: &str = r#"You are a professional knowledge graph ontology design expert. Your task is to analyze given text content and simulation requirements, and design entity types and relationship types suitable for **social media opinion simulation**.

**IMPORTANT: You must output valid JSON format data only, no other content.**

## Core Task Background

We are building a **social media opinion simulation system**. In this system:
- Each entity represents an "account" or "subject" that can post, interact, and spread information on social media
- Entities will influence each other, repost, comment, and respond
- We need to simulate reactions from various parties to opinion events and information propagation paths

Therefore, **entities must be real-world subjects that can post and interact on social media**:

**CAN BE**:
- Specific people (public figures, parties involved, opinion leaders, experts, scholars, ordinary people)
- Companies, enterprises (including their official accounts)
- Organizations (universities, associations, NGOs, unions, etc.)
- Government departments, regulatory agencies
- Media outlets (newspapers, TV stations, self-media, websites)
- Social media platforms themselves
- Specific group representatives (alumni associations, fan groups, advocacy groups, etc.)

**CANNOT BE**:
- Abstract concepts (like "public opinion", "emotion", "trend")
- Topics/themes (like "academic integrity", "education reform")
- Viewpoints/attitudes (like "supporter", "opponent")

## Output Format

Please output JSON format containing the following structure:

```json
{
    "entity_types": [
        {
            "name": "Entity type name (English, PascalCase)",
            "description": "Brief description (English, no more than 100 characters)",
            "attributes": [
                {
                    "name": "attribute name (English, snake_case)",
                    "type": "text",
                    "description": "attribute description"
                }
            ],
            "examples": ["example entity 1", "example entity 2"]
        }
    ],
    "edge_types": [
        {
            "name": "Relationship type name (English, UPPER_SNAKE_CASE)",
            "description": "Brief description (English, no more than 100 characters)",
            "source_targets": [
                {"source": "source entity type", "target": "target entity type"}
            ],
            "attributes": []
        }
    ],
    "analysis_summary": "Brief analysis of text content (in English)"
}
```

## Design Guidelines (CRITICAL!)

### 1. Entity Type Design - Must Follow Strictly

**Quantity requirement: Must output exactly 10 entity types**

**Hierarchical structure requirement (must include both specific and fallback types)**:

Your 10 entity types must include the following hierarchy:

A. **Fallback types (must include, place at end of list)**:
   - `Person`: Fallback type for any natural person. When a person doesn't belong to any more specific person type, classify under this.
   - `Organization`: Fallback type for any organization. When an organization doesn't belong to any more specific organization type, classify under this.

B. **Specific types (8 types, designed according to text content)**:
   - Design more specific types for main characters appearing in text
   - Example: If text involves academic events, can have `Student`, `Professor`, `University`
   - Example: If text involves business events, can have `Company`, `CEO`, `Employee`

**Specific type design principles**:
- Identify frequently appearing or key role types from text
- Each specific type should have clear boundaries, avoid overlap
- Description must clearly explain how this type differs from fallback type

### 2. Relationship Type Design

- Quantity: 6-10
- Relationships should reflect real connections in social media interactions
- Ensure relationship source_targets cover your defined entity types

### 3. Attribute Design

- Each entity type should have 1-3 key attributes
- **Note**: Attribute names cannot use `name`, `uuid`, `group_id`, `created_at`, `summary` (these are reserved)
- Recommended: `full_name`, `title`, `role`, `position`, `location`, `description`, etc.

## Relationship Type References

- WORKS_FOR, STUDIES_AT, AFFILIATED_WITH, REPRESENTS, REGULATES, REPORTS_ON,
  COMMENTS_ON, RESPONDS_TO, SUPPORTS, OPPOSES, COLLABORATES_WITH, COMPETES_WITH
"#;

pub async fn generate(
    llm: &Llm,
    document_texts: &[String],
    simulation_requirement: &str,
    additional_context: Option<&str>,
) -> Result<Value> {
    let user_message = build_user_message(document_texts, simulation_requirement, additional_context);
    let messages = [Msg::system(SYSTEM_PROMPT), Msg::user(user_message)];
    let raw = llm.chat_json(&messages, 0.3, 4096).await?;
    Ok(validate_and_process(raw))
}

fn build_user_message(
    document_texts: &[String],
    simulation_requirement: &str,
    additional_context: Option<&str>,
) -> String {
    let mut combined = document_texts.join("\n\n---\n\n");
    let original_len = combined.chars().count();
    if original_len > MAX_TEXT_LENGTH_FOR_LLM {
        combined = combined.chars().take(MAX_TEXT_LENGTH_FOR_LLM).collect();
        combined.push_str(&format!(
            "\n\n...(Original text total {original_len} characters, first {MAX_TEXT_LENGTH_FOR_LLM} characters used for ontology analysis)..."
        ));
    }

    let mut msg = format!(
        "## Simulation Requirement\n\n{simulation_requirement}\n\n## Document Content\n\n{combined}\n"
    );
    if let Some(ctx) = additional_context {
        msg.push_str(&format!("\n## Additional Notes\n\n{ctx}\n"));
    }
    msg.push_str(
        "\nPlease design entity types and relationship types suitable for social opinion simulation based on the above content.\n\n\
        **Rules that MUST be followed**:\n\
        1. Must output exactly 10 entity types\n\
        2. Last 2 must be fallback types: Person (person fallback) and Organization (organization fallback)\n\
        3. First 8 are specific types designed according to text content\n\
        4. All entity types must be real-world subjects that can post on social media, cannot be abstract concepts\n\
        5. Attribute names cannot use name, uuid, group_id, etc. reserved words, use full_name, org_name, etc. instead\n",
    );
    msg
}

/// Deterministic post-processing of the LLM's ontology JSON: ensure required
/// fields exist, truncate long descriptions, guarantee Person/Organization
/// fallback types, and cap type counts.
pub fn validate_and_process(mut result: Value) -> Value {
    if !result.is_object() {
        result = json!({});
    }
    let obj = result.as_object_mut().unwrap();
    obj.entry("entity_types").or_insert(json!([]));
    obj.entry("edge_types").or_insert(json!([]));
    obj.entry("analysis_summary").or_insert(json!(""));

    for key in ["entity_types", "edge_types"] {
        if !obj[key].is_array() {
            obj[key] = json!([]);
        }
        for item in obj[key].as_array_mut().unwrap() {
            let Some(m) = item.as_object_mut() else { continue };
            if key == "entity_types" {
                m.entry("attributes").or_insert(json!([]));
                m.entry("examples").or_insert(json!([]));
            } else {
                m.entry("source_targets").or_insert(json!([]));
                m.entry("attributes").or_insert(json!([]));
            }
            if let Some(desc) = m.get("description").and_then(Value::as_str) {
                if desc.chars().count() > 100 {
                    let truncated: String = desc.chars().take(97).collect();
                    m.insert("description".into(), json!(format!("{truncated}...")));
                }
            }
        }
    }

    let entity_types = obj["entity_types"].as_array_mut().unwrap();
    let names: Vec<String> = entity_types
        .iter()
        .filter_map(|e| e["name"].as_str().map(String::from))
        .collect();

    let mut fallbacks = Vec::new();
    if !names.iter().any(|n| n == "Person") {
        fallbacks.push(json!({
            "name": "Person",
            "description": "Any individual person not fitting other specific person types.",
            "attributes": [
                {"name": "full_name", "type": "text", "description": "Full name of the person"},
                {"name": "role", "type": "text", "description": "Role or occupation"}
            ],
            "examples": ["ordinary citizen", "anonymous netizen"]
        }));
    }
    if !names.iter().any(|n| n == "Organization") {
        fallbacks.push(json!({
            "name": "Organization",
            "description": "Any organization not fitting other specific organization types.",
            "attributes": [
                {"name": "org_name", "type": "text", "description": "Name of the organization"},
                {"name": "org_type", "type": "text", "description": "Type of organization"}
            ],
            "examples": ["small business", "community group"]
        }));
    }
    if !fallbacks.is_empty() {
        let keep = MAX_ENTITY_TYPES.saturating_sub(fallbacks.len());
        entity_types.truncate(keep);
        entity_types.extend(fallbacks);
    }
    entity_types.truncate(MAX_ENTITY_TYPES);

    obj["edge_types"].as_array_mut().unwrap().truncate(MAX_EDGE_TYPES);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_missing_fields_and_fallbacks() {
        let out = validate_and_process(json!({
            "entity_types": [{"name": "Student", "description": "A student"}],
            "edge_types": [{"name": "STUDIES_AT"}]
        }));
        let entities = out["entity_types"].as_array().unwrap();
        let names: Vec<&str> = entities.iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Person"));
        assert!(names.contains(&"Organization"));
        assert!(entities[0]["attributes"].is_array());
        assert!(entities[0]["examples"].is_array());
        assert!(out["edge_types"][0]["source_targets"].is_array());
        assert_eq!(out["analysis_summary"], "");
    }

    #[test]
    fn caps_entity_types_at_ten_keeping_fallbacks() {
        let many: Vec<Value> = (0..12)
            .map(|i| json!({"name": format!("Type{i}"), "description": "d"}))
            .collect();
        let out = validate_and_process(json!({"entity_types": many, "edge_types": []}));
        let entities = out["entity_types"].as_array().unwrap();
        assert_eq!(entities.len(), 10);
        assert_eq!(entities[8]["name"], "Person");
        assert_eq!(entities[9]["name"], "Organization");
    }

    #[test]
    fn keeps_existing_fallbacks_without_duplication() {
        let out = validate_and_process(json!({
            "entity_types": [
                {"name": "Person", "description": "p"},
                {"name": "Organization", "description": "o"}
            ],
            "edge_types": []
        }));
        assert_eq!(out["entity_types"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn truncates_long_descriptions() {
        let long = "x".repeat(150);
        let out = validate_and_process(json!({
            "entity_types": [{"name": "A", "description": long}],
            "edge_types": []
        }));
        let desc = out["entity_types"][0]["description"].as_str().unwrap();
        assert_eq!(desc.chars().count(), 100);
        assert!(desc.ends_with("..."));
    }

    #[test]
    fn handles_garbage_input() {
        let out = validate_and_process(json!("not an object"));
        assert!(out["entity_types"].as_array().unwrap().len() >= 2);
    }
}
