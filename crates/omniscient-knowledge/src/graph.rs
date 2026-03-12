//! Knowledge Graph — SurrealDB-backed entity-relationship graph

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;
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

/// SurrealDB-backed knowledge graph for entities and relationships
pub struct KnowledgeGraph {
    db: Surreal<Db>,
}

impl KnowledgeGraph {
    /// Create a new in-memory knowledge graph
    pub async fn new_memory() -> Result<Self> {
        let db = Surreal::new::<Mem>(())
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        db.use_ns("omniscient")
            .use_db("research")
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        info!("Knowledge graph initialized (in-memory)");
        Ok(Self { db })
    }

    /// Add an entity to the graph
    pub async fn add_entity(&self, entity: &Entity) -> Result<()> {
        let table = entity_type_table(&entity.entity_type);

        let _: Option<serde_json::Value> = self
            .db
            .create((table, entity.id.to_string()))
            .content(serde_json::json!({
                "name": entity.name,
                "entity_type": format!("{:?}", entity.entity_type),
                "attributes": entity.attributes,
            }))
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        debug!(entity = %entity.name, "Entity added to graph");
        Ok(())
    }

    /// Add a relationship between two entities
    pub async fn add_relationship(&self, rel: &Relationship) -> Result<()> {
        let query = format!(
            "RELATE {}:{}->{}->{}:{}",
            "entity", rel.from_id, rel.relation_type, "entity", rel.to_id
        );

        // Clone everything to owned values for SurrealDB's 'static requirement
        let confidence = rel.confidence;
        let source = rel.source.clone();
        let context = rel.context.clone();

        self.db
            .query(&query)
            .bind(("confidence", confidence))
            .bind(("source", source))
            .bind(("context", context))
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

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
        // Clone to owned for SurrealDB's 'static requirement
        let doc_owned = doc.clone();
        let doc_id = doc.id.clone();

        let _: Option<serde_json::Value> = self
            .db
            .create(("document", doc_id))
            .content(doc_owned)
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        Ok(())
    }

    /// Find entities by name (partial match)
    pub async fn find_entities(&self, name: &str) -> Result<Vec<serde_json::Value>> {
        // Clone to owned for SurrealDB's 'static requirement
        let name_owned = name.to_string();

        let results: Vec<serde_json::Value> = self
            .db
            .query("SELECT * FROM entity WHERE name CONTAINS $name")
            .bind(("name", name_owned))
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?
            .take(0)
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        Ok(results)
    }

    /// Get all relationships for an entity
    pub async fn get_relationships(&self, entity_id: &str) -> Result<Vec<serde_json::Value>> {
        // Clone to owned for SurrealDB's 'static requirement
        let id_owned = entity_id.to_string();

        let results: Vec<serde_json::Value> = self
            .db
            .query("SELECT * FROM relationship WHERE from_id = $id OR to_id = $id")
            .bind(("id", id_owned))
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?
            .take(0)
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        Ok(results)
    }

    /// Get graph statistics
    pub async fn stats(&self) -> Result<serde_json::Value> {
        let entities: Vec<serde_json::Value> = self
            .db
            .query("SELECT count() FROM entity GROUP ALL")
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?
            .take(0)
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        let docs: Vec<serde_json::Value> = self
            .db
            .query("SELECT count() FROM document GROUP ALL")
            .await
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?
            .take(0)
            .map_err(|e| OmniscientError::KnowledgeGraph(e.to_string()))?;

        Ok(serde_json::json!({
            "entities": entities.first().unwrap_or(&serde_json::Value::Null),
            "documents": docs.first().unwrap_or(&serde_json::Value::Null),
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
