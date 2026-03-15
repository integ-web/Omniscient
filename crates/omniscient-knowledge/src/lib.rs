//! # Omniscient Knowledge System
//!
//! Full-text search (Tantivy), knowledge graph (SurrealDB), and entity extraction.

pub mod graph;
pub mod index;

pub use graph::KnowledgeGraph;
pub use index::SearchIndex;
