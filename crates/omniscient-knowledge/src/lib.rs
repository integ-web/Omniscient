//! # Omniscient Knowledge System
//!
//! Full-text search (Tantivy), knowledge graph (SurrealDB), and entity extraction.

pub mod graph;
pub mod index;
pub mod traits;
pub mod memory_store;

pub use graph::KnowledgeGraph;
pub use index::SearchIndex;
pub use traits::{RecordLinker, GraphTraverser};
pub use memory_store::VectorMemoryStore;
