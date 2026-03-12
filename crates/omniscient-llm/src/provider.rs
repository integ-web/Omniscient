//! LLM Provider trait — abstraction over all LLM backends

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use omniscient_core::error::Result;
use omniscient_core::types::Message;

/// A request to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub stop_sequences: Vec<String>,
    /// Available tools for function calling
    pub tools: Option<Vec<ToolDefinition>>,
    /// Whether to stream the response
    pub stream: bool,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// Response from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
}

/// A tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    ToolCall,
    Length,
    Error(String),
}

/// Core LLM provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the model name
    fn model(&self) -> &str;

    /// Check if the provider is available and ready
    async fn is_available(&self) -> bool;

    /// Generate a completion
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Get the context window size
    fn context_window(&self) -> usize;

    /// Estimate token count for a string
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 chars per token
        text.len() / 4
    }
}
