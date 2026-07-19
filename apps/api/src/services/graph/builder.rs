//! Graph build orchestration.
//! Spawns a tokio task that chunks the text, runs LLM extraction per chunk,
//! upserts into the graph store, and reports progress through the task registry.
//! No fixed per-chunk rate-limit sleep is needed: the Llm client already
//! retries 429s with backoff.

use super::{db, dedupe, entity_extractor, projects, tasks};
use crate::llm::Llm;
use serde_json::{json, Value};

pub struct BuildParams {
    pub project_id: String,
    pub text: String,
    pub ontology: Value,
    pub graph_name: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

/// Kick off an async graph build; returns the task_id to poll.
pub fn spawn_build(graph: db::GraphDb, llm: Llm, params: BuildParams) -> String {
    let task_id = tasks::create(
        "graph_build",
        json!({
            "project_id": params.project_id,
            "graph_name": params.graph_name,
            "chunk_size": params.chunk_size,
            "chunk_overlap": params.chunk_overlap,
            "text_length": params.text.chars().count(),
        }),
    );

    let tid = task_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_build(&graph, &llm, &tid, params).await {
            tracing::error!("graph build {tid} failed: {e:#}");
            tasks::fail(&tid, format!("{e:#}"));
        }
    });

    task_id
}

async fn run_build(
    graph: &db::GraphDb,
    llm: &Llm,
    task_id: &str,
    params: BuildParams,
) -> anyhow::Result<()> {
    tasks::update(task_id, 5, "Starting graph build...");

    // 1. Create the graph's metadata row.
    let graph_id = db::create_graph(graph, &params.graph_name).await?;
    tasks::update(task_id, 10, format!("Graph created: {graph_id}"));

    // 2. Store the ontology on the metadata row.
    db::set_ontology(graph, &graph_id, &params.ontology).await?;
    tasks::update(task_id, 15, "Ontology set");

    // 3. Chunk the text.
    let chunks = super::text_processor::split_text(&params.text, params.chunk_size, params.chunk_overlap);
    let total_chunks = chunks.len();
    tasks::update(task_id, 20, format!("Text split into {total_chunks} chunks"));

    // 4. Extract entities per chunk and upsert into the graph store.
    let mut failed_chunks = 0usize;
    for (idx, chunk) in chunks.iter().enumerate() {
        let progress = 20 + ((idx + 1) as f64 / total_chunks.max(1) as f64 * 70.0) as u8;
        tasks::update(
            task_id,
            progress.min(90),
            format!("Processing chunk {}/{total_chunks}...", idx + 1),
        );

        match entity_extractor::extract(llm, chunk, &params.ontology).await {
            Ok(extraction) => {
                if let Err(e) = write_extraction(graph, &graph_id, &extraction).await {
                    failed_chunks += 1;
                    tracing::error!("failed to write chunk {idx} to graph store: {e:#}");
                }
            }
            Err(e) => {
                failed_chunks += 1;
                tracing::error!("extraction failed for chunk {idx}: {e:#}");
            }
        }
    }

    if total_chunks > 0 && failed_chunks * 2 > total_chunks {
        anyhow::bail!(
            "Extraction failed: {failed_chunks}/{total_chunks} chunks failed. \
             This is likely due to rate limiting. Please try again later."
        );
    }

    // 5. Fold alias entities together. Runs here, once, with every name visible
    //    at the same time: "EDC" is only resolvable against the full roster, and
    //    leaving it to persona compilation would leave the graph itself showing
    //    one actor three times for every later reader.
    tasks::update(task_id, 92, "Merging duplicate entities...");
    let merged = match dedupe::run(graph, &graph_id, llm).await {
        Ok(m) => m,
        Err(e) => {
            // A graph with aliases still beats no graph at all.
            tracing::warn!("dedupe pass failed for {graph_id}, keeping duplicates: {e:#}");
            Vec::new()
        }
    };

    // 6. Collect graph info; refuse to complete on an empty graph.
    tasks::update(task_id, 95, "Getting graph information...");
    let info = db::graph_info(graph, &graph_id).await?;
    if info.node_count == 0 {
        anyhow::bail!("Graph build failed: no entities were extracted. Please try again later.");
    }

    // 7. Persist graph info onto the project.
    if let Some(mut project) = projects::get(&params.project_id) {
        project.graph_id = Some(graph_id.clone());
        project.node_count = Some(info.node_count);
        project.edge_count = Some(info.edge_count);
        project.status = projects::status::GRAPH_BUILT.to_string();
        if let Err(e) = projects::save(&project) {
            tracing::warn!("could not update project {}: {e:#}", params.project_id);
        }
    }

    tasks::complete(
        task_id,
        json!({
            "graph_id": graph_id,
            // Every merge is reported: a pass that quietly rewrites who is in
            // the population should be inspectable, and reversible from the
            // `aliases` recorded on each survivor.
            "merged_entities": merged.iter().map(|(k, a)| json!({"kept": k, "absorbed": a})).collect::<Vec<_>>(),
            "graph_information": {
                "graph_id": graph_id,
                "node_count": info.node_count,
                "edge_count": info.edge_count,
                "entity_types": info.entity_types,
            },
            "chunks_processed": total_chunks,
        }),
    );
    Ok(())
}

/// Write one normalized extraction into the graph store. `normalize` already
/// guarantees non-empty names/types, so this is a straight typed write.
async fn write_extraction(
    graph: &db::GraphDb,
    graph_id: &str,
    extraction: &entity_extractor::Extraction,
) -> anyhow::Result<()> {
    for entity in &extraction.entities {
        db::upsert_entity(graph, graph_id, &entity.name, &entity.entity_types, &entity.attributes).await?;
    }
    for rel in &extraction.relationships {
        db::upsert_relationship(
            graph,
            graph_id,
            &rel.source_name,
            &rel.target_name,
            &rel.relation_type,
            &rel.attributes,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the extractor→builder→db glue: a representative
    /// LLM extraction (including the quirks `normalize` must absorb — a bare
    /// string `entity_types`, a top-level `summary`, malformed items) flows
    /// through normalize → write_extraction → get_graph_data and lands with
    /// the right entity_type / relation_type / attributes.
    #[tokio::test]
    async fn extraction_glue_roundtrips_into_graph() {
        let dir = std::env::temp_dir()
            .join(format!("popinion-glue-{}", uuid::Uuid::new_v4().simple()));
        let graph = db::GraphDb::open(&dir.join("g.db")).unwrap();
        let graph_id = db::create_graph(&graph, "glue test").await.unwrap();

        let raw = json!({
            "entities": [
                {"name": "Minister X", "entity_types": ["Politician"],
                 "summary": "Proposed the policy.", "attributes": {"role": "minister"}},
                {"name": "Labor Union", "entity_types": "Organization",
                 "summary": "Represents factory workers."},
                {"name": "", "entity_types": ["Ghost"]}
            ],
            "relationships": [
                {"source_name": "Labor Union", "target_name": "Minister X",
                 "relation_type": "OPPOSES",
                 "attributes": {"fact": "The union opposes the policy."}},
                {"source_name": "Nobody", "target_name": "", "relation_type": "KNOWS"}
            ]
        });

        let extraction = entity_extractor::normalize(raw);
        write_extraction(&graph, &graph_id, &extraction).await.unwrap();

        let data = db::get_graph_data(&graph, &graph_id).await.unwrap();
        assert_eq!(data.nodes.len(), 2, "empty-name entity dropped");
        let minister = data.nodes.iter().find(|n| n.name == "Minister X").unwrap();
        assert_eq!(minister.entity_types, vec!["Politician".to_string()]);
        assert_eq!(minister.attributes["role"], "minister");
        assert_eq!(minister.summary, "Proposed the policy.");
        let union = data.nodes.iter().find(|n| n.name == "Labor Union").unwrap();
        assert_eq!(
            union.entity_types,
            vec!["Organization".to_string()],
            "string entity_types coerced to list"
        );

        assert_eq!(data.edges.len(), 1, "relationship without target dropped");
        let e = &data.edges[0];
        assert_eq!(e.relation_type, "OPPOSES");
        assert_eq!(e.fact, "The union opposes the policy.");
        assert_eq!(e.source_node_name, "Labor Union");
        assert_eq!(e.target_node_name, "Minister X");
        assert!(!e.source_node_uuid.is_empty() && !e.target_node_uuid.is_empty());
    }
}
