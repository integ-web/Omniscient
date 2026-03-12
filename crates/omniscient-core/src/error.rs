//! Error types for Omniscient

use thiserror::Error;

/// Main error type for Omniscient operations
#[derive(Error, Debug)]
pub enum OmniscientError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("LLM inference error: {0}")]
    Inference(String),

    #[error("Web request error: {0}")]
    Web(String),

    #[error("Parsing error: {0}")]
    Parse(String),

    #[error("Search index error: {0}")]
    SearchIndex(String),

    #[error("Knowledge graph error: {0}")]
    KnowledgeGraph(String),

    #[error("Tool execution error: {tool} — {message}")]
    ToolExecution { tool: String, message: String },

    #[error("Agent planning error: {0}")]
    Planning(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Task cancelled")]
    Cancelled,

    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for Omniscient operations
pub type Result<T> = std::result::Result<T, OmniscientError>;
