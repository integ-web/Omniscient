//! Full-text Search Index — Tantivy-powered indexing and retrieval

use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};
use tracing::{debug, info};

use omniscient_core::error::{OmniscientError, Result};

/// Search result from the index
#[derive(Debug, Clone)]
pub struct IndexedResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub url: String,
    pub score: f32,
}

/// Tantivy-based full-text search index
pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Option<IndexWriter>,
    schema: Schema,
    // Field handles
    id_field: Field,
    title_field: Field,
    content_field: Field,
    url_field: Field,
    source_field: Field,
    timestamp_field: Field,
}

impl SearchIndex {
    /// Create or open a search index at the given directory
    pub fn new(index_dir: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let url_field = schema_builder.add_text_field("url", STRING | STORED);
        let source_field = schema_builder.add_text_field("source", STRING | STORED);
        let timestamp_field = schema_builder.add_text_field("timestamp", STRING | STORED);

        let schema = schema_builder.build();

        // Create or open index
        std::fs::create_dir_all(index_dir)
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        let index = if Index::exists(
            &tantivy::directory::MmapDirectory::open(index_dir)
                .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?,
        )
        .map_err(|e| OmniscientError::SearchIndex(e.to_string()))? {
            Index::open_in_dir(index_dir)
                .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?
        } else {
            Index::create_in_dir(index_dir, schema.clone())
                .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        let writer = index
            .writer(50_000_000) // 50MB write buffer
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        Ok(Self {
            index,
            reader,
            writer: Some(writer),
            schema,
            id_field,
            title_field,
            content_field,
            url_field,
            source_field,
            timestamp_field,
        })
    }

    /// Create an in-memory search index (for testing or temporary use)
    pub fn in_memory() -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let url_field = schema_builder.add_text_field("url", STRING | STORED);
        let source_field = schema_builder.add_text_field("source", STRING | STORED);
        let timestamp_field = schema_builder.add_text_field("timestamp", STRING | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        Ok(Self {
            index,
            reader,
            writer: Some(writer),
            schema,
            id_field,
            title_field,
            content_field,
            url_field,
            source_field,
            timestamp_field,
        })
    }

    /// Index a document
    pub fn add_document(
        &mut self,
        id: &str,
        title: &str,
        content: &str,
        url: &str,
        source: &str,
    ) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| OmniscientError::SearchIndex("Writer not available".to_string()))?;

        let timestamp = chrono::Utc::now().to_rfc3339();

        writer
            .add_document(doc!(
                self.id_field => id,
                self.title_field => title,
                self.content_field => content,
                self.url_field => url,
                self.source_field => source,
                self.timestamp_field => timestamp,
            ))
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        Ok(())
    }

    /// Commit all pending changes to the index
    pub fn commit(&mut self) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| OmniscientError::SearchIndex("Writer not available".to_string()))?;

        writer
            .commit()
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        self.reader
            .reload()
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        info!("Search index committed");
        Ok(())
    }

    /// Search the index
    pub fn search(&self, query: &str, max_results: usize) -> Result<Vec<IndexedResult>> {
        let searcher = self.reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);

        let parsed_query = query_parser
            .parse_query(query)
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(max_results))
            .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| OmniscientError::SearchIndex(e.to_string()))?;

            let get_field = |field: Field| -> String {
                doc.get_first(field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };

            results.push(IndexedResult {
                id: get_field(self.id_field),
                title: get_field(self.title_field),
                content: get_field(self.content_field),
                url: get_field(self.url_field),
                score,
            });
        }

        debug!(query = query, results = results.len(), "Search completed");
        Ok(results)
    }

    /// Get the number of documents in the index
    pub fn document_count(&self) -> u64 {
        let searcher = self.reader.searcher();
        searcher.num_docs()
    }
}
