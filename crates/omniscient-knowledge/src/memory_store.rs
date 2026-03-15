use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Embedded SQLite Relational Graph storage for memory integration
pub struct VectorMemoryStore {
    db: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub source: String,
}

impl VectorMemoryStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = if db_path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            Connection::open(db_path)?
        };

        // Initialize schema for a HydraDB-inspired Context Graph
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_graph (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                embedding_stub TEXT
            )",
            [],
        )?;

        Ok(Self { db: Mutex::new(conn) })
    }

    /// Adds new memories into the local SQLite knowledge graph
    pub fn add_memory(&self, entry: &MemoryEntry) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memory_graph (id, content, source, embedding_stub) VALUES (?1, ?2, ?3, ?4)",
            params![entry.id, entry.content, entry.source, "null"],
        )?;
        Ok(())
    }
}
