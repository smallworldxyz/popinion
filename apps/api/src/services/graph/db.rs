//! Embedded graph store (SQLite). Replaces the Neo4j layer: the codebase only
//! ever used Neo4j as a keyed document store — upsert entities/relationships by
//! name, read the whole graph back as JSON for the frontend viz and the persona
//! compiler. There are zero traversals, so a graph database bought nothing over
//! two tables. This lets the whole app run with no external services.
//!
//! Function signatures mirror the old `neo4j` module (async, same params) so
//! callers are unchanged; the bodies are synchronous SQLite. Graph ops are
//! low-frequency (chunked builds + occasional reads), so one shared connection
//! behind a Mutex is fine.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The graph wire shape. THE one definition shared by the producer
/// (`get_graph_data`) and every consumer (the API handlers that serialize it
/// for the frontend viz, and `sim::persona`'s bundle compiler) — a renamed
/// field is a compile error, not silent data loss.
#[derive(Default, Serialize, Deserialize)]
pub struct GraphData {
    pub graph_id: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct GraphNode {
    pub uuid: String,
    pub name: String,
    pub entity_types: Vec<String>,
    /// Convenience copy of `attributes["summary"]` (empty when absent).
    pub summary: String,
    pub attributes: Map<String, Value>,
    pub created_at: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct GraphEdge {
    pub uuid: String,
    pub relation_type: String,
    /// Convenience copy of `attributes["fact"]` (empty when absent).
    pub fact: String,
    pub source_node_uuid: String,
    pub target_node_uuid: String,
    pub source_node_name: String,
    pub target_node_name: String,
    pub attributes: Map<String, Value>,
    pub created_at: Option<String>,
}

/// Cheap-to-clone handle to the graph database (the spawned build task holds a clone).
#[derive(Clone)]
pub struct GraphDb {
    conn: Arc<Mutex<Connection>>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS graph_meta (
    graph_id    TEXT PRIMARY KEY,
    name        TEXT,
    uuid        TEXT,
    description TEXT,
    ontology    TEXT,
    created_at  TEXT
);
CREATE TABLE IF NOT EXISTS graph_node (
    graph_id   TEXT NOT NULL,
    name       TEXT NOT NULL,
    uuid       TEXT,
    entity_types TEXT,      -- JSON array of entity types
    properties   TEXT,      -- JSON object: summary + extracted attributes
    created_at TEXT,
    PRIMARY KEY (graph_id, name)
);
CREATE TABLE IF NOT EXISTS graph_edge (
    graph_id    TEXT NOT NULL,
    source_name TEXT NOT NULL,
    target_name TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    uuid          TEXT,
    properties    TEXT,     -- JSON object: fact + attributes
    created_at    TEXT,
    PRIMARY KEY (graph_id, source_name, target_name, relation_type)
);
CREATE INDEX IF NOT EXISTS idx_node_graph ON graph_node(graph_id);
CREATE INDEX IF NOT EXISTS idx_edge_graph ON graph_edge(graph_id);
"#;

impl GraphDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")?;
        conn.execute_batch(SCHEMA)?;
        // Vocabulary migration for databases created before the rename:
        // labels -> entity_types, rel_type -> relation_type. No-ops (ignored
        // errors) on fresh databases where the new names already exist.
        conn.execute("ALTER TABLE graph_node RENAME COLUMN labels TO entity_types", []).ok();
        conn.execute("ALTER TABLE graph_edge RENAME COLUMN rel_type TO relation_type", []).ok();
        Ok(GraphDb { conn: Arc::new(Mutex::new(conn)) })
    }
}

/// Entity types and relation types come from the LLM; normalize them to
/// identifier characters (rule kept from the Neo4j layer so stored values stay
/// stable across the migration).
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

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Create the graph metadata row; returns the new graph_id.
pub async fn create_graph(db: &GraphDb, name: &str) -> Result<String> {
    let graph_id = format!("popinion_{}", &uuid::Uuid::new_v4().simple().to_string()[..16]);
    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    c.execute(
        "INSERT INTO graph_meta (graph_id, name, uuid, description, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            graph_id,
            name,
            uuid::Uuid::new_v4().to_string(),
            "Popinion Social Simulation Graph",
            now()
        ],
    )?;
    Ok(graph_id)
}

/// Store the ontology JSON on the metadata row.
pub async fn set_ontology(db: &GraphDb, graph_id: &str, ontology: &Value) -> Result<()> {
    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    c.execute(
        "UPDATE graph_meta SET ontology = ?1 WHERE graph_id = ?2",
        params![ontology.to_string(), graph_id],
    )?;
    Ok(())
}

/// Upsert one entity by (graph_id, name); merges attributes and unions entity
/// types across chunks, matching the old Neo4j MERGE + `n += props` semantics.
pub async fn upsert_entity(
    db: &GraphDb,
    graph_id: &str,
    name: &str,
    entity_types: &[String],
    attributes: &Map<String, Value>,
) -> Result<()> {
    let safe_types: Vec<String> = entity_types.iter().filter_map(|l| sanitize_identifier(l)).collect();
    let mut props = attributes.clone();
    props.remove("uuid");

    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    let existing: Option<(String, String)> = c
        .query_row(
            "SELECT entity_types, properties FROM graph_node WHERE graph_id = ?1 AND name = ?2",
            params![graph_id, name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    match existing {
        Some((old_types, old_attrs)) => {
            let mut merged_attrs: Map<String, Value> =
                serde_json::from_str(&old_attrs).unwrap_or_default();
            merged_attrs.extend(props);
            let mut merged_types: Vec<String> =
                serde_json::from_str(&old_types).unwrap_or_default();
            for l in safe_types {
                if !merged_types.contains(&l) {
                    merged_types.push(l);
                }
            }
            c.execute(
                "UPDATE graph_node SET entity_types = ?1, properties = ?2 WHERE graph_id = ?3 AND name = ?4",
                params![json!(merged_types).to_string(), Value::Object(merged_attrs).to_string(), graph_id, name],
            )?;
        }
        None => {
            c.execute(
                "INSERT INTO graph_node (graph_id, name, uuid, entity_types, properties, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    graph_id,
                    name,
                    uuid::Uuid::new_v4().to_string(),
                    json!(safe_types).to_string(),
                    Value::Object(props).to_string(),
                    now()
                ],
            )?;
        }
    }
    Ok(())
}

/// Upsert one relationship between two named entities in the graph.
pub async fn upsert_relationship(
    db: &GraphDb,
    graph_id: &str,
    source_name: &str,
    target_name: &str,
    relation_type: &str,
    attributes: &Map<String, Value>,
) -> Result<()> {
    let relation_type = sanitize_identifier(relation_type).unwrap_or_else(|| "RELATED_TO".to_string());
    let mut props = attributes.clone();
    // valid_at marks the fact as currently valid for downstream memory queries.
    props.insert("valid_at".into(), json!(now()));

    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    let existing: Option<String> = c
        .query_row(
            "SELECT properties FROM graph_edge
             WHERE graph_id = ?1 AND source_name = ?2 AND target_name = ?3 AND relation_type = ?4",
            params![graph_id, source_name, target_name, relation_type],
            |r| r.get(0),
        )
        .optional()?;

    match existing {
        Some(old) => {
            let mut merged: Map<String, Value> = serde_json::from_str(&old).unwrap_or_default();
            merged.extend(props);
            c.execute(
                "UPDATE graph_edge SET properties = ?1
                 WHERE graph_id = ?2 AND source_name = ?3 AND target_name = ?4 AND relation_type = ?5",
                params![Value::Object(merged).to_string(), graph_id, source_name, target_name, relation_type],
            )?;
        }
        None => {
            c.execute(
                "INSERT INTO graph_edge (graph_id, source_name, target_name, relation_type, uuid, properties, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    graph_id,
                    source_name,
                    target_name,
                    relation_type,
                    uuid::Uuid::new_v4().to_string(),
                    Value::Object(props).to_string(),
                    now()
                ],
            )?;
        }
    }
    Ok(())
}

pub struct GraphInfo {
    pub node_count: i64,
    pub edge_count: i64,
    pub entity_types: Vec<String>,
}

pub async fn graph_info(db: &GraphDb, graph_id: &str) -> Result<GraphInfo> {
    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    let node_count: i64 =
        c.query_row("SELECT COUNT(*) FROM graph_node WHERE graph_id = ?1", params![graph_id], |r| r.get(0))?;
    let edge_count: i64 =
        c.query_row("SELECT COUNT(*) FROM graph_edge WHERE graph_id = ?1", params![graph_id], |r| r.get(0))?;

    let mut stmt = c.prepare(
        "SELECT DISTINCT je.value FROM graph_node n, json_each(n.entity_types) je WHERE n.graph_id = ?1",
    )?;
    let entity_types = stmt
        .query_map(params![graph_id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(GraphInfo { node_count, edge_count, entity_types })
}

/// Read the whole graph back as typed `GraphData` — the shape the frontend
/// d3 viz and `sim::persona::build_bundles` consume.
pub async fn get_graph_data(db: &GraphDb, graph_id: &str) -> Result<GraphData> {
    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());

    let mut nodes = Vec::new();
    let mut stmt = c.prepare(
        "SELECT uuid, name, entity_types, properties, created_at FROM graph_node WHERE graph_id = ?1",
    )?;
    let mut rows = stmt.query(params![graph_id])?;
    while let Some(row) = rows.next()? {
        let props: Map<String, Value> =
            serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default();
        nodes.push(GraphNode {
            uuid: row.get(0)?,
            name: row.get(1)?,
            entity_types: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
            summary: props.get("summary").and_then(Value::as_str).unwrap_or_default().to_string(),
            attributes: props,
            created_at: row.get(4)?,
        });
    }

    let mut edges = Vec::new();
    let mut stmt = c.prepare(
        "SELECT e.uuid, e.relation_type, e.properties, e.created_at,
                sn.uuid AS source_uuid, tn.uuid AS target_uuid, e.source_name, e.target_name
         FROM graph_edge e
         LEFT JOIN graph_node sn ON sn.graph_id = e.graph_id AND sn.name = e.source_name
         LEFT JOIN graph_node tn ON tn.graph_id = e.graph_id AND tn.name = e.target_name
         WHERE e.graph_id = ?1",
    )?;
    let mut rows = stmt.query(params![graph_id])?;
    while let Some(row) = rows.next()? {
        let props: Map<String, Value> =
            serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default();
        edges.push(GraphEdge {
            uuid: row.get(0)?,
            relation_type: row.get(1)?,
            fact: props.get("fact").and_then(Value::as_str).unwrap_or_default().to_string(),
            source_node_uuid: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            target_node_uuid: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            source_node_name: row.get(6)?,
            target_node_name: row.get(7)?,
            attributes: props,
            created_at: row.get(3)?,
        });
    }

    Ok(GraphData {
        graph_id: graph_id.to_string(),
        node_count: nodes.len(),
        edge_count: edges.len(),
        nodes,
        edges,
    })
}

/// Delete a graph's nodes, edges and metadata row.
pub async fn delete_graph(db: &GraphDb, graph_id: &str) -> Result<()> {
    let c = db.conn.lock().unwrap_or_else(|e| e.into_inner());
    c.execute("DELETE FROM graph_node WHERE graph_id = ?1", params![graph_id])?;
    c.execute("DELETE FROM graph_edge WHERE graph_id = ?1", params![graph_id])?;
    c.execute("DELETE FROM graph_meta WHERE graph_id = ?1", params![graph_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_db() -> GraphDb {
        let dir = std::env::temp_dir().join(format!("popinion-graphdb-{}-{:p}", std::process::id(), &0));
        GraphDb::open(&dir.join("g.db")).unwrap()
    }

    #[test]
    fn sanitizes_identifiers() {
        assert_eq!(sanitize_identifier("Person"), Some("Person".into()));
        assert_eq!(sanitize_identifier("WORKS_FOR"), Some("WORKS_FOR".into()));
        assert_eq!(sanitize_identifier("has space"), Some("has_space".into()));
        assert_eq!(sanitize_identifier("1starts_with_digit"), None);
        assert_eq!(sanitize_identifier(""), None);
    }

    #[tokio::test]
    async fn roundtrip_build_read_delete() {
        let db = tmp_db();
        let gid = create_graph(&db, "test graph").await.unwrap();

        let mut alice = Map::new();
        alice.insert("summary".into(), json!("A tester"));
        alice.insert("role".into(), json!("qa"));
        upsert_entity(&db, &gid, "Alice", &["Person".into()], &alice).await.unwrap();
        // Second chunk adds a label + a property to the same entity → merged.
        let mut alice2 = Map::new();
        alice2.insert("location".into(), json!("Phnom Penh"));
        upsert_entity(&db, &gid, "Alice", &["Student".into()], &alice2).await.unwrap();
        upsert_entity(&db, &gid, "Acme", &["Organization".into()], &Map::new()).await.unwrap();

        let mut rel = Map::new();
        rel.insert("fact".into(), json!("Alice works for Acme"));
        upsert_relationship(&db, &gid, "Alice", "Acme", "WORKS_FOR", &rel).await.unwrap();

        let info = graph_info(&db, &gid).await.unwrap();
        assert_eq!(info.node_count, 2);
        assert_eq!(info.edge_count, 1);
        assert!(info.entity_types.contains(&"Person".to_string()));
        assert!(info.entity_types.contains(&"Student".to_string()));

        let data = get_graph_data(&db, &gid).await.unwrap();
        assert_eq!(data.nodes.len(), 2);
        let alice_node = data.nodes.iter().find(|n| n.name == "Alice").unwrap();
        assert_eq!(alice_node.summary, "A tester");
        assert_eq!(alice_node.attributes["role"], "qa");
        assert_eq!(alice_node.attributes["location"], "Phnom Penh"); // merged across chunks
        assert_eq!(alice_node.entity_types.len(), 2); // Person + Student

        assert_eq!(data.edges.len(), 1);
        assert_eq!(data.edges[0].fact, "Alice works for Acme");
        assert_eq!(data.edges[0].relation_type, "WORKS_FOR");
        assert!(!data.edges[0].source_node_uuid.is_empty());

        delete_graph(&db, &gid).await.unwrap();
        let info = graph_info(&db, &gid).await.unwrap();
        assert_eq!(info.node_count, 0);
    }
}
