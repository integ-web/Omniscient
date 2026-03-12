//! Tool system — extensible plugin architecture for research tools

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{OmniscientError, Result};

/// Input to a tool — JSON-based for flexibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub parameters: serde_json::Value,
}

/// Output from a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub data: serde_json::Value,
    pub text_summary: String,
    pub artifacts: Vec<ToolArtifact>,
}

/// An artifact produced by a tool (e.g., a downloaded file, extracted content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArtifact {
    pub name: String,
    pub content_type: String,
    pub data: String,
}

/// Describes a tool's parameter for LLM understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String, // "string", "number", "boolean", "array", "object"
    pub required: bool,
    pub default: Option<serde_json::Value>,
}

/// Describes a tool for function-calling LLMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub category: ToolCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Search,
    WebAccess,
    Analysis,
    Knowledge,
    Utility,
}

/// Core Tool trait — implement this for any new capability
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's description for LLM function calling
    fn describe(&self) -> ToolDescription;

    /// Execute the tool with given input
    async fn execute(&self, input: &ToolInput) -> Result<ToolOutput>;

    /// Validate input before execution
    fn validate_input(&self, input: &ToolInput) -> Result<()> {
        let _ = input;
        Ok(())
    }
}

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a new tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, input: &ToolInput) -> Result<ToolOutput> {
        let tool = self.get(name).ok_or_else(|| {
            OmniscientError::ToolExecution {
                tool: name.to_string(),
                message: "Tool not found in registry".to_string(),
            }
        })?;

        tool.validate_input(input)?;
        tool.execute(input).await
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get descriptions of all tools (for LLM function calling)
    pub fn describe_all(&self) -> Vec<ToolDescription> {
        self.tools.values().map(|t| t.describe()).collect()
    }

    /// Get the count of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
