//! Knowledge-graph subsystem.
//! Ingest text -> LLM entity/relation extraction -> embedded graph store (SQLite)
//! -> read back graph data for the frontend viz and the persona compiler.

pub mod builder;
pub mod db;
pub mod entity_extractor;
pub mod file_parser;
pub mod ontology;
pub mod projects;
pub mod tasks;
pub mod text_processor;
