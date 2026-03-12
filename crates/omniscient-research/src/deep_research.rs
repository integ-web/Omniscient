//! Deep Research Pipeline — multi-pass iterative research

use omniscient_core::config::OmniscientConfig;
use omniscient_core::error::Result;
use omniscient_core::orchestrator::Orchestrator;
use omniscient_core::task::ResearchTask;
use omniscient_core::tools::ToolRegistry;
use omniscient_core::types::{ResearchDepth, ResearchReport};

use omniscient_llm::provider::LlmProvider;

use crate::research_agent::ResearchAgent;

/// Deep research pipeline — coordinates the full research workflow
pub struct DeepResearchPipeline {
    config: OmniscientConfig,
}

impl DeepResearchPipeline {
    pub fn new(config: OmniscientConfig) -> Self {
        Self { config }
    }

    /// Execute a full deep research session
    pub async fn research(
        &self,
        query: &str,
        depth: ResearchDepth,
        llm: Box<dyn LlmProvider>,
        tools: ToolRegistry,
    ) -> Result<ResearchReport> {
        let mut task = ResearchTask::new(query, depth);

        let agent = ResearchAgent::new(llm);
        let mut orchestrator = Orchestrator::new(self.config.clone(), tools);

        let report = orchestrator.run(&agent, &mut task).await?;

        Ok(report)
    }
}
