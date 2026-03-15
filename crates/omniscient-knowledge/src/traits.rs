use async_trait::async_trait;
use omniscient_core::error::Result;

/// Provides deterministic resolution of messy real-world entities
#[async_trait]
pub trait RecordLinker: Send + Sync {
    async fn link_records(&self, current_records: Vec<String>, new_record: String) -> Result<Vec<String>>;
}

/// Allows fast traversing of highly relational data structures
#[async_trait]
pub trait GraphTraverser: Send + Sync {
    async fn find_path(&self, source_entity: &str, target_entity: &str) -> Result<Vec<String>>;
}
