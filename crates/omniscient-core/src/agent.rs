//! Agent trait — the core abstraction for all AI agents in Omniscient

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::task::ResearchTask;
use crate::tools::ToolRegistry;
use crate::types::{Finding, Message};

/// Context provided to agents during execution
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Conversation history
    pub messages: Vec<Message>,
    /// Current research task being worked on
    pub current_task: Option<ResearchTask>,
    /// Working memory — key facts discovered so far
    pub working_memory: Vec<Finding>,
    /// Available tool names
    pub available_tools: Vec<String>,
    /// Maximum iterations to prevent runaway loops
    pub max_iterations: usize,
    /// Current iteration
    pub iteration: usize,
}

impl Default for AgentContext {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            current_task: None,
            working_memory: Vec::new(),
            available_tools: Vec::new(),
            max_iterations: 50,
            iteration: 0,
        }
    }
}

/// The result of an agent's planning step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    /// Ordered list of subtasks to execute
    pub steps: Vec<PlanStep>,
    /// Reasoning for this plan
    pub reasoning: String,
    /// Estimated total time
    pub estimated_duration_secs: Option<u64>,
}

/// A single step in an agent's plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub description: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub depends_on: Vec<usize>,
}

/// The result of an agent's execution of a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: usize,
    pub success: bool,
    pub output: String,
    pub findings: Vec<Finding>,
    pub follow_up_needed: bool,
}

/// The result of an agent's synthesis/reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synthesis {
    pub summary: String,
    pub key_findings: Vec<Finding>,
    pub gaps: Vec<String>,
    pub confidence: f64,
    pub needs_more_research: bool,
}

/// Core Agent trait — every research agent must implement this
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's name
    fn name(&self) -> &str;

    /// Get the agent's description
    fn description(&self) -> &str;

    /// Plan how to approach a research query
    async fn plan(
        &self,
        query: &str,
        context: &AgentContext,
    ) -> Result<AgentPlan>;

    /// Execute a single step from the plan
    async fn execute_step(
        &self,
        step: &PlanStep,
        context: &AgentContext,
        tools: &ToolRegistry,
    ) -> Result<StepResult>;

    /// Reflect on results and synthesize findings
    async fn synthesize(
        &self,
        results: &[StepResult],
        context: &AgentContext,
    ) -> Result<Synthesis>;

    /// Decide whether to continue researching or stop
    fn should_continue(&self, synthesis: &Synthesis, context: &AgentContext) -> bool {
        synthesis.needs_more_research
            && context.iteration < context.max_iterations
            && synthesis.confidence < 0.85
    }
}
