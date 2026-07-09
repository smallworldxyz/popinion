//! Knowledge-graph subsystem.
//! Ports the Neo4j + entity-extraction pipeline (neo4j_service, graph_builder,
//! ontology_generator, llm_entity_extractor, text_processor, file_parser).
//! Ingest text -> LLM entity/relation extraction -> Neo4j write -> read back
//! graph data for the frontend viz.

pub mod builder;
pub mod entity_extractor;
pub mod file_parser;
pub mod neo4j;
pub mod ontology;
pub mod projects;
pub mod tasks;
pub mod text_processor;
