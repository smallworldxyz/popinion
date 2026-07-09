//! Neo4j Cypher layer (port of the subset of neo4j_service.py / neo4j_tools.py
//! that the build + read-back + delete pipeline needs).
//!
//! All nodes carry the `GraphNode` label plus their entity-type labels, and a
//! `graph_id` property scoping them to one built graph.

use anyhow::{anyhow, Result};
use neo4rs::{query, BoltType, Graph};
use serde_json::{json, Map, Value};

/// Labels and relationship types are interpolated into Cypher (they cannot be
/// parameterized), so restrict them to identifier characters.
pub fn sanitize_identifier(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if cleaned.is_empty() || !cleaned.chars().next().unwrap().is_alphabetic() {
        return None;
    }
    Some(cleaned)
}

fn props_param(props: &Map<String, Value>) -> Result<BoltType> {
    Value::Object(props.clone())
        .try_into()
        .map_err(|e| anyhow!("cannot convert properties to bolt: {e:?}"))
}

/// Create the graph metadata node + constraints; returns the new graph_id.
pub async fn create_graph(graph: &Graph, name: &str) -> Result<String> {
    let graph_id = format!("pubop_{}", &uuid::Uuid::new_v4().simple().to_string()[..16]);
    graph
        .run(
            query(
                "CREATE (g:Graph:Metadata {graph_id: $graph_id, name: $name, uuid: $uuid, \
                 description: 'Popinion Social Simulation Graph', created_at: $now})",
            )
            .param("graph_id", graph_id.as_str())
            .param("name", name)
            .param("uuid", uuid::Uuid::new_v4().to_string())
            .param("now", chrono::Utc::now().to_rfc3339()),
        )
        .await?;

    // Best-effort constraints/indexes (global on the GraphNode label; the
    // Python per-graph names created the same schema objects repeatedly).
    for stmt in [
        "CREATE CONSTRAINT graph_node_uuid IF NOT EXISTS FOR (n:GraphNode) REQUIRE n.uuid IS UNIQUE",
        "CREATE INDEX graph_node_graph_id IF NOT EXISTS FOR (n:GraphNode) ON (n.graph_id)",
        "CREATE INDEX graph_node_name IF NOT EXISTS FOR (n:GraphNode) ON (n.name)",
    ] {
        if let Err(e) = graph.run(query(stmt)).await {
            tracing::warn!("constraint/index setup: {e}");
        }
    }

    Ok(graph_id)
}

/// Store the ontology JSON on the metadata node.
pub async fn set_ontology(graph: &Graph, graph_id: &str, ontology: &Value) -> Result<()> {
    graph
        .run(
            query("MATCH (g:Graph:Metadata {graph_id: $graph_id}) SET g.ontology = $ontology")
                .param("graph_id", graph_id)
                .param("ontology", ontology.to_string()),
        )
        .await?;
    Ok(())
}

/// Upsert one entity by (graph_id, name); merges properties and adds labels.
pub async fn upsert_entity(
    graph: &Graph,
    graph_id: &str,
    name: &str,
    labels: &[String],
    properties: &Map<String, Value>,
) -> Result<()> {
    let safe_labels: Vec<String> = labels.iter().filter_map(|l| sanitize_identifier(l)).collect();
    let label_clause = if safe_labels.is_empty() {
        String::new()
    } else {
        format!(
            " SET n:`{}`",
            safe_labels.join("`:`")
        )
    };

    let mut props = properties.clone();
    props.remove("uuid");
    props.insert("graph_id".into(), json!(graph_id));
    props.insert("name".into(), json!(name));

    let q = format!(
        "MERGE (n:GraphNode {{graph_id: $graph_id, name: $name}}) \
         ON CREATE SET n.uuid = $uuid, n.created_at = $now \
         SET n += $props{label_clause}"
    );
    graph
        .run(
            query(&q)
                .param("graph_id", graph_id)
                .param("name", name)
                .param("uuid", uuid::Uuid::new_v4().to_string())
                .param("now", chrono::Utc::now().to_rfc3339())
                .param("props", props_param(&props)?),
        )
        .await?;
    Ok(())
}

/// Upsert one relationship between two named entities in the graph.
pub async fn upsert_relationship(
    graph: &Graph,
    graph_id: &str,
    source_name: &str,
    target_name: &str,
    rel_type: &str,
    properties: &Map<String, Value>,
) -> Result<()> {
    let rel_type = sanitize_identifier(rel_type).unwrap_or_else(|| "RELATED_TO".to_string());

    let now = chrono::Utc::now().to_rfc3339();
    let mut props = properties.clone();
    props.insert("graph_id".into(), json!(graph_id));
    props.insert("created_at".into(), json!(now));
    // valid_at marks the fact as currently valid for downstream memory queries.
    props.insert("valid_at".into(), json!(now));

    let q = format!(
        "MATCH (a:GraphNode {{graph_id: $graph_id, name: $source}}) \
         MATCH (b:GraphNode {{graph_id: $graph_id, name: $target}}) \
         MERGE (a)-[r:`{rel_type}`]->(b) \
         SET r += $props"
    );
    graph
        .run(
            query(&q)
                .param("graph_id", graph_id)
                .param("source", source_name)
                .param("target", target_name)
                .param("props", props_param(&props)?),
        )
        .await?;
    Ok(())
}

pub struct GraphInfo {
    pub node_count: i64,
    pub edge_count: i64,
    pub entity_types: Vec<String>,
}

pub async fn graph_info(graph: &Graph, graph_id: &str) -> Result<GraphInfo> {
    let mut rows = graph
        .execute(
            query(
                "MATCH (n:GraphNode {graph_id: $graph_id}) \
                 OPTIONAL MATCH (n)-[r]->(:GraphNode {graph_id: $graph_id}) \
                 RETURN count(DISTINCT n) as nodes, count(r) as edges",
            )
            .param("graph_id", graph_id),
        )
        .await?;
    let (node_count, edge_count) = match rows.next().await? {
        Some(row) => (row.get::<i64>("nodes")?, row.get::<i64>("edges")?),
        None => (0, 0),
    };

    let mut rows = graph
        .execute(
            query(
                "MATCH (n:GraphNode {graph_id: $graph_id}) \
                 UNWIND labels(n) as label WITH DISTINCT label \
                 WHERE label <> 'GraphNode' RETURN collect(label) as types",
            )
            .param("graph_id", graph_id),
        )
        .await?;
    let entity_types = match rows.next().await? {
        Some(row) => row.get::<Vec<String>>("types")?,
        None => vec![],
    };

    Ok(GraphInfo { node_count, edge_count, entity_types })
}

/// Read the whole graph back as `{nodes, edges}` for the frontend d3 viz
/// (same shape as Python GraphBuilderService.get_graph_data).
pub async fn get_graph_data(graph: &Graph, graph_id: &str) -> Result<Value> {
    let mut nodes = Vec::new();
    let mut rows = graph
        .execute(
            query("MATCH (n:GraphNode {graph_id: $graph_id}) RETURN n, labels(n) as labels")
                .param("graph_id", graph_id),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        let node: neo4rs::Node = row.get("n")?;
        let labels: Vec<String> = row
            .get::<Vec<String>>("labels")?
            .into_iter()
            .filter(|l| l != "GraphNode")
            .collect();

        let mut props = Map::new();
        for key in node.keys() {
            if let Ok(v) = node.get::<Value>(key) {
                props.insert(key.to_string(), v);
            }
        }
        let attributes: Map<String, Value> = props
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "uuid" | "name" | "graph_id" | "created_at"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        nodes.push(json!({
            "uuid": props.get("uuid").cloned().unwrap_or(json!("")),
            "name": props.get("name").cloned().unwrap_or(json!("")),
            "labels": labels,
            "summary": props.get("summary").cloned().unwrap_or(json!("")),
            "attributes": attributes,
            "created_at": props.get("created_at").cloned().unwrap_or(Value::Null),
        }));
    }

    let mut edges = Vec::new();
    let mut rows = graph
        .execute(
            query(
                "MATCH (a:GraphNode {graph_id: $graph_id})-[r]->(b:GraphNode {graph_id: $graph_id}) \
                 RETURN a.uuid as source_uuid, a.name as source_name, \
                        b.uuid as target_uuid, b.name as target_name, \
                        type(r) as rel_type, properties(r) as rel_props",
            )
            .param("graph_id", graph_id),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        let rel_type: String = row.get("rel_type")?;
        let rel_props: Value = row.get("rel_props").unwrap_or(json!({}));
        let props = rel_props.as_object().cloned().unwrap_or_default();
        let attributes: Map<String, Value> = props
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "graph_id" | "created_at"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        edges.push(json!({
            "uuid": props.get("uuid").cloned().unwrap_or(json!("")),
            "name": rel_type,
            "fact": props.get("fact").cloned().unwrap_or(json!("")),
            "fact_type": rel_type,
            "source_node_uuid": row.get::<String>("source_uuid").unwrap_or_default(),
            "target_node_uuid": row.get::<String>("target_uuid").unwrap_or_default(),
            "source_node_name": row.get::<String>("source_name").unwrap_or_default(),
            "target_node_name": row.get::<String>("target_name").unwrap_or_default(),
            "attributes": attributes,
            "created_at": props.get("created_at").cloned().unwrap_or(Value::Null),
        }));
    }

    Ok(json!({
        "graph_id": graph_id,
        "node_count": nodes.len(),
        "edge_count": edges.len(),
        "nodes": nodes,
        "edges": edges,
    }))
}

/// Delete a graph's nodes, relationships and metadata node (the Python version
/// leaked the metadata node; we clean it up too).
pub async fn delete_graph(graph: &Graph, graph_id: &str) -> Result<()> {
    graph
        .run(
            query("MATCH (n:GraphNode {graph_id: $graph_id}) DETACH DELETE n")
                .param("graph_id", graph_id),
        )
        .await?;
    graph
        .run(
            query("MATCH (g:Graph:Metadata {graph_id: $graph_id}) DETACH DELETE g")
                .param("graph_id", graph_id),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_identifiers() {
        assert_eq!(sanitize_identifier("Person"), Some("Person".into()));
        assert_eq!(sanitize_identifier("WORKS_FOR"), Some("WORKS_FOR".into()));
        assert_eq!(sanitize_identifier("has space"), Some("has_space".into()));
        assert_eq!(sanitize_identifier("evil`) DETACH DELETE (n"), Some("evil___DETACH_DELETE__n".into()));
        assert_eq!(sanitize_identifier("1starts_with_digit"), None);
        assert_eq!(sanitize_identifier(""), None);
        assert_eq!(sanitize_identifier("  "), None);
    }

    #[test]
    fn props_convert_to_bolt() {
        let mut m = Map::new();
        m.insert("a".into(), json!("text"));
        m.insert("b".into(), json!(42));
        m.insert("c".into(), json!([1, 2, 3]));
        assert!(props_param(&m).is_ok());
    }

    // Live-DB smoke test; requires a running Neo4j with default env credentials.
    #[tokio::test]
    #[ignore]
    async fn roundtrip_against_live_neo4j() {
        let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into());
        let user = std::env::var("NEO4J_USERNAME").unwrap_or_else(|_| "neo4j".into());
        let pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "pubop123".into());
        let graph = Graph::new(&uri, &user, &pass).await.unwrap();

        let gid = create_graph(&graph, "test graph").await.unwrap();
        let mut props = Map::new();
        props.insert("role".into(), json!("tester"));
        upsert_entity(&graph, &gid, "Alice", &["Person".into()], &props).await.unwrap();
        upsert_entity(&graph, &gid, "Acme", &["Organization".into()], &Map::new()).await.unwrap();
        upsert_relationship(&graph, &gid, "Alice", "Acme", "WORKS_FOR", &Map::new()).await.unwrap();

        let info = graph_info(&graph, &gid).await.unwrap();
        assert_eq!(info.node_count, 2);
        assert_eq!(info.edge_count, 1);

        let data = get_graph_data(&graph, &gid).await.unwrap();
        assert_eq!(data["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(data["edges"].as_array().unwrap().len(), 1);

        delete_graph(&graph, &gid).await.unwrap();
        let info = graph_info(&graph, &gid).await.unwrap();
        assert_eq!(info.node_count, 0);
    }
}
