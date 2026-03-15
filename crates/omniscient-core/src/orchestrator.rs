//! Orchestrator — coordinates agents, tools, and the research pipeline

use std::sync::Arc;
use tracing::{info, warn, error};

use crate::agent::{Agent, AgentContext, Synthesis};
use crate::config::OmniscientConfig;
use crate::error::Result;
use crate::memory::Memory;
use crate::task::{ResearchTask, TaskStatus};
use crate::tools::ToolRegistry;
use crate::types::ResearchReport;
use crate::todo::TodoManager;

/// The Orchestrator — runs the Observe-Reason-Gate-Act (ORGA) cycle
pub struct Orchestrator {
    config: OmniscientConfig,
    tools: ToolRegistry,
    memory: Memory,
    todo: TodoManager,
}

impl Orchestrator {
    pub fn new(config: OmniscientConfig, tools: ToolRegistry) -> Self {
        Self {
            config,
            tools,
            memory: Memory::new(100),
            todo: TodoManager::new(),
        }
    }

    /// Run a full research task with a given agent
    pub async fn run(
        &mut self,
        agent: &dyn Agent,
        task: &mut ResearchTask,
    ) -> Result<ResearchReport> {
        info!(
            task_id = %task.id,
            query = %task.query,
            "Starting research task"
        );

        task.mark_running();

        let mut context = AgentContext {
            messages: Vec::new(),
            current_task: Some(task.clone()),
            working_memory: Vec::new(),
            available_tools: self.tools.list(),
            max_iterations: 50,
            iteration: 0,
        };

        let mut all_step_results = Vec::new();
        let mut final_synthesis: Option<Synthesis> = None;

        // ORGA research loop
        loop {
            context.iteration += 1;
            info!(iteration = context.iteration, "ORGA iteration");

            // 1. Observe/Reason (Plan)
            if self.todo.is_empty() {
                task.status = TaskStatus::Planning;
                let plan = match agent.plan(&task.query, &context).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Planning failed: {}", e);
                        break;
                    }
                };

                info!(
                    steps = plan.steps.len(),
                    reasoning = %plan.reasoning,
                    "Agent queued plan into TodoManager"
                );

                self.todo.push_plan(plan);
            }

            // 2. Act
            task.status = TaskStatus::Running;
            if let Some(step) = self.todo.next_step() {
                info!(step_id = step.id, tool = %step.tool_name, "Executing ORGA Act step");

                match agent.execute_step(&step, &context, &self.tools).await {
                    Ok(result) => {
                        for finding in &result.findings {
                            self.memory.add_finding(finding.clone());
                            context.working_memory.push(finding.clone());
                        }
                        all_step_results.push(result);
                    }
                    Err(e) => {
                        error!(step_id = step.id, error = %e, "Step execution failed");
                    }
                }

                task.update_progress(0.5); // Abstract progress
            }

            // 3. Gate (Evaluate and prune via Synthesis/Pruning logic)
            if self.todo.is_empty() {
                task.status = TaskStatus::Synthesizing;
                let synthesis = agent.synthesize(&all_step_results, &context).await?;

                info!(
                    confidence = synthesis.confidence,
                    gaps = synthesis.gaps.len(),
                    needs_more = synthesis.needs_more_research,
                    "Gate check complete"
                );

                // Sunk Cost Immunity / Pruning
                if synthesis.confidence < 0.3 {
                    warn!("Confidence extremely low. Pruning current thought tree.");
                    self.todo.prune();
                }

                if !agent.should_continue(&synthesis, &context) {
                    final_synthesis = Some(synthesis);
                    break;
                }

                for gap in &synthesis.gaps {
                    self.memory
                        .remember_short(format!("Research gap: {}", gap), 0.9);
                }

                final_synthesis = Some(synthesis);
            }
        }

        // Build the final report
        task.mark_completed();
        task.update_progress(1.0);

        let synthesis = final_synthesis.unwrap_or_else(|| Synthesis {
            summary: "Research completed with limited findings.".to_string(),
            key_findings: Vec::new(),
            gaps: Vec::new(),
            confidence: 0.5,
            needs_more_research: false,
        });

        // Consolidate important memories
        self.memory.consolidate(0.7);

        let report = ResearchReport {
            id: uuid::Uuid::new_v4(),
            title: format!("Research Report: {}", task.query),
            query: task.query.clone(),
            executive_summary: synthesis.summary,
            sections: Vec::new(), // Filled by report generator
            sources: Vec::new(),
            findings: synthesis.key_findings,
            entities: Vec::new(),
            generated_at: chrono::Utc::now(),
            research_depth: task.depth,
            total_sources_consulted: all_step_results.len(),
            total_pages_crawled: 0,
        };

        info!(
            report_id = %report.id,
            sources = report.total_sources_consulted,
            "Research complete"
        );

        Ok(report)
    }

    /// Get the tool registry (mutable access for adding tools)
    pub fn tools_mut(&mut self) -> &mut ToolRegistry {
        &mut self.tools
    }

    /// Get memory stats
    pub fn memory_stats(&self) -> crate::memory::MemoryStats {
        self.memory.stats()
    }
}
