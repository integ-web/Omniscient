use anyhow::Result;
use rig::agent::Agent;
use rig::completion::Prompt;
use rig::providers::openai;
use crate::rig_agents::{build_evaluator_agent, build_planner_agent, build_scraper_agent, build_synthesizer_agent};
use serde::{Deserialize, Serialize};

use rig::providers::openai::Client;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvestigationState {
    pub user_query: String,
    pub planner_strategy: String,
    pub scraper_findings: String,
    pub evaluator_feedback: String,
    pub final_report: String,
}

pub struct RigOrchestrator {
    pub planner: Agent<rig::providers::openai::CompletionModel>,
    pub scraper: Agent<rig::providers::openai::CompletionModel>,
    pub evaluator: Agent<rig::providers::openai::CompletionModel>,
    pub synthesizer: Agent<rig::providers::openai::CompletionModel>,
}

impl RigOrchestrator {
    pub async fn new(client: &Client, model: &str) -> Result<Self> {
        let planner = build_planner_agent(client, model).await;
        let scraper = build_scraper_agent(client, model).await;
        let evaluator = build_evaluator_agent(client, model).await;
        let synthesizer = build_synthesizer_agent(client, model).await;

        Ok(Self {
            planner,
            scraper,
            evaluator,
            synthesizer,
        })
    }

    // DAG Workflow loop using standard prompt capabilities for agentic behavior
    pub async fn execute_investigation(&self, query: &str) -> Result<InvestigationState> {
        let mut state = InvestigationState {
            user_query: query.to_string(),
            planner_strategy: "".into(),
            scraper_findings: "".into(),
            evaluator_feedback: "".into(),
            final_report: "".into(),
        };

        // 1. Plan
        let plan_prompt = format!("Create a research plan for: {}", query);
        let plan_res = self.planner.prompt(&plan_prompt).await?;
        state.planner_strategy = plan_res.clone();

        // Loop max 3 times for MVP evaluation cycle
        let mut iterations = 0;
        let max_iterations = 3;
        let mut found_sufficient_evidence = false;

        while iterations < max_iterations && !found_sufficient_evidence {
            iterations += 1;

            // 2. Scrape
            let scrape_prompt = format!("Using this strategy: {}\nExecute search to find corporate UBO data or relevant details. Previous findings: {}", plan_res, state.scraper_findings);
            let scrape_res = self.scraper.prompt(&scrape_prompt).await?;
            state.scraper_findings.push_str(&format!("\nIteration {}: {}", iterations, scrape_res));

            // 3. Evaluate
            let eval_prompt = format!("Evaluate these findings against the goal of piercing the corporate veil: {}. Goal: {}. If sufficient, start response with 'SUFFICIENT:'. Otherwise, suggest what else to scrape.", state.scraper_findings, query);
            let eval_res = self.evaluator.prompt(&eval_prompt).await?;
            state.evaluator_feedback = eval_res.clone();

            if eval_res.starts_with("SUFFICIENT:") {
                found_sufficient_evidence = true;
            }
        }

        // 4. Synthesize
        let synth_prompt = format!("Synthesize a final report based on these verified findings: {}\nGoal: {}", state.scraper_findings, query);
        let final_report = self.synthesizer.prompt(&synth_prompt).await?;
        state.final_report = final_report;

        Ok(state)
    }
}
