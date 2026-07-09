//! Graph build orchestration (port of graph_builder.py).
//! Spawns a tokio task that chunks the text, runs LLM extraction per chunk,
//! upserts into Neo4j, and reports progress through the task registry.
//! The Python version's fixed 3s-per-chunk rate-limit sleep is dropped: the
//! Llm client already retries 429s with backoff.

use super::{entity_extractor, neo4j, projects, tasks};
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
pub fn spawn_build(graph: neo4rs::Graph, llm: Llm, params: BuildParams) -> String {
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
    graph: &neo4rs::Graph,
    llm: &Llm,
    task_id: &str,
    params: BuildParams,
) -> anyhow::Result<()> {
    tasks::update(task_id, 5, "Starting graph build...");

    // 1. Create graph metadata + constraints.
    let graph_id = neo4j::create_graph(graph, &params.graph_name).await?;
    tasks::update(task_id, 10, format!("Graph created: {graph_id}"));

    // 2. Store ontology on the metadata node.
    neo4j::set_ontology(graph, &graph_id, &params.ontology).await?;
    tasks::update(task_id, 15, "Ontology set");

    // 3. Chunk the text.
    let chunks = super::text_processor::split_text(&params.text, params.chunk_size, params.chunk_overlap);
    let total_chunks = chunks.len();
    tasks::update(task_id, 20, format!("Text split into {total_chunks} chunks"));

    // 4. Extract entities per chunk and upsert into Neo4j.
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
                    tracing::error!("failed to write chunk {idx} to Neo4j: {e:#}");
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

    // 5. Collect graph info; refuse to complete on an empty graph.
    tasks::update(task_id, 95, "Getting graph information...");
    let info = neo4j::graph_info(graph, &graph_id).await?;
    if info.node_count == 0 {
        anyhow::bail!("Graph build failed: no entities were extracted. Please try again later.");
    }

    // 6. Persist graph info onto the project.
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

/// Write one normalized extraction ({entities, relationships}) to Neo4j.
async fn write_extraction(
    graph: &neo4rs::Graph,
    graph_id: &str,
    extraction: &Value,
) -> anyhow::Result<()> {
    let empty = vec![];
    for entity in extraction["entities"].as_array().unwrap_or(&empty) {
        let name = entity["name"].as_str().unwrap_or_default();
        let labels: Vec<String> = entity["labels"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default();
        let props = entity["properties"].as_object().cloned().unwrap_or_default();
        if name.is_empty() || labels.is_empty() {
            continue;
        }
        neo4j::upsert_entity(graph, graph_id, name, &labels, &props).await?;
    }

    for rel in extraction["relationships"].as_array().unwrap_or(&empty) {
        let source = rel["source_name"].as_str().unwrap_or_default();
        let target = rel["target_name"].as_str().unwrap_or_default();
        let rel_type = rel["type"].as_str().unwrap_or("RELATED_TO");
        let props = rel["properties"].as_object().cloned().unwrap_or_default();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        neo4j::upsert_relationship(graph, graph_id, source, target, rel_type, &props).await?;
    }
    Ok(())
}
