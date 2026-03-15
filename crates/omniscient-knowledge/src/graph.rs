//! Knowledge Graph — SurrealDB-backed entity-relationship graph

use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use std::sync::Mutex;
use tracing::{debug, info};

use omniscient_core::error::{OmniscientError, Result};
use omniscient_core::types::{Entity, EntityType};

/// A relationship between two entities in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub confidence: f64,
    pub source: String,
    pub context: Option<String>,
}

/// A stored document reference in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDocument {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub source: String,
    pub summary: Option<String>,
    pub timestamp: String,
}

/// Embedded SQLite relational Knowledge Graph for entities and relationships
pub struct KnowledgeGraph {
    db: Mutex<Connection>,
}

impl KnowledgeGraph {
    /// Create a new in-memory SQLite knowledge graph
    pub async fn new_memory() -> Result<Self> {
        let db = Connection::open_in_memory()
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS entity (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                attributes TEXT
            )",
            [],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS relationship (
                id INTEGER PRIMARY KEY,
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                source TEXT NOT NULL,
                context TEXT,
                FOREIGN KEY(from_id) REFERENCES entity(id),
                FOREIGN KEY(to_id) REFERENCES entity(id)
            )",
            [],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS document (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                url TEXT,
                source TEXT NOT NULL,
                summary TEXT,
                timestamp TEXT NOT NULL
            )",
            [],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        info!("Knowledge graph initialized (in-memory SQLite)");
        Ok(Self { db: Mutex::new(db) })
    }

    /// Add an entity to the graph
    pub async fn add_entity(&self, entity: &Entity) -> Result<()> {
        let conn = self.db.lock().unwrap();

        let attrs = serde_json::to_string(&entity.attributes)
            .unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT OR REPLACE INTO entity (id, name, entity_type, attributes) VALUES (?1, ?2, ?3, ?4)",
            params![entity.id.to_string(), entity.name, format!("{:?}", entity.entity_type), attrs],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        debug!(entity = %entity.name, "Entity added to graph");
        Ok(())
    }

    /// Add a relationship between two entities
    pub async fn add_relationship(&self, rel: &Relationship) -> Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "INSERT INTO relationship (from_id, to_id, relation_type, confidence, source, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![rel.from_id, rel.to_id, rel.relation_type, rel.confidence, rel.source, rel.context],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        debug!(
            from = %rel.from_id,
            to = %rel.to_id,
            relation = %rel.relation_type,
            "Relationship added"
        );
        Ok(())
    }

    /// Store a document reference in the graph
    pub async fn add_document(&self, doc: &GraphDocument) -> Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO document (id, title, url, source, summary, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![doc.id, doc.title, doc.url, doc.source, doc.summary, doc.timestamp],
        ).map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        Ok(())
    }

    /// Find entities by name (partial match)
    pub async fn find_entities(&self, name: &str) -> Result<Vec<serde_json::Value>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare("SELECT id, name, entity_type, attributes FROM entity WHERE name LIKE ?1")
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        let search_pattern = format!("%{}%", name);
        let mut rows = stmt.query(params![search_pattern])
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))? {
            let attrs: String = row.get(3).unwrap_or_else(|_| "{}".to_string());
            let entity_json = serde_json::json!({
                "id": row.get::<_, String>(0).unwrap_or_default(),
                "name": row.get::<_, String>(1).unwrap_or_default(),
                "entity_type": row.get::<_, String>(2).unwrap_or_default(),
                "attributes": serde_json::from_str::<serde_json::Value>(&attrs).unwrap_or(serde_json::json!({}))
            });
            results.push(entity_json);
        }

        Ok(results)
    }

    /// Get all relationships for an entity
    pub async fn get_relationships(&self, entity_id: &str) -> Result<Vec<serde_json::Value>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare("SELECT from_id, to_id, relation_type, confidence, source, context FROM relationship WHERE from_id = ?1 OR to_id = ?1")
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        let mut rows = stmt.query(params![entity_id])
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))? {
            let rel_json = serde_json::json!({
                "from_id": row.get::<_, String>(0).unwrap_or_default(),
                "to_id": row.get::<_, String>(1).unwrap_or_default(),
                "relation_type": row.get::<_, String>(2).unwrap_or_default(),
                "confidence": row.get::<_, f64>(3).unwrap_or(0.0),
                "source": row.get::<_, String>(4).unwrap_or_default(),
                "context": row.get::<_, Option<String>>(5).unwrap_or_default()
            });
            results.push(rel_json);
        }

        Ok(results)
    }

    /// Get graph statistics
    pub async fn stats(&self) -> Result<serde_json::Value> {
        let conn = self.db.lock().unwrap();

        let entities_count: i64 = conn.query_row("SELECT COUNT(*) FROM entity", [], |row| row.get(0))
            .unwrap_or(0);

        let docs_count: i64 = conn.query_row("SELECT COUNT(*) FROM document", [], |row| row.get(0))
            .unwrap_or(0);

        Ok(serde_json::json!({
            "entities": entities_count,
            "documents": docs_count,
        }))
    }
}

/// Map entity type to SurrealDB table name
fn entity_type_table(et: &EntityType) -> &'static str {
    match et {
        EntityType::Person => "entity",
        EntityType::Company => "entity",
        EntityType::Organization => "entity",
        EntityType::Location => "entity",
        EntityType::Product => "entity",
        EntityType::Technology => "entity",
        EntityType::Event => "entity",
        EntityType::Date => "entity",
        EntityType::Money => "entity",
        EntityType::Concept => "entity",
    }
}
