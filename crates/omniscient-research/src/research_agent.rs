//! Research Agent — concrete implementation of the Agent trait
//! that uses LLMs + tools to conduct deep research

use async_trait::async_trait;
use tracing::info;

use omniscient_core::agent::*;
use omniscient_core::error::Result;
use omniscient_core::task::ResearchTask;
use omniscient_core::tools::{ToolInput, ToolRegistry};
use omniscient_core::types::{Finding, FindingCategory, Message, Role, Source};

use omniscient_llm::provider::{LlmProvider, LlmRequest};

/// The main research agent — uses LLM reasoning + tools to research any topic
pub struct ResearchAgent {
    llm: Box<dyn LlmProvider>,
    name: String,
}

impl ResearchAgent {
    pub fn new(llm: Box<dyn LlmProvider>) -> Self {
        Self {
            llm,
            name: "OmniscientResearcher".to_string(),
        }
    }
}

#[async_trait]
impl Agent for ResearchAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Deep research agent that combines web search, crawling, and LLM analysis for comprehensive research"
    }

    async fn plan(&self, query: &str, context: &AgentContext) -> Result<AgentPlan> {
        let system_prompt = r#"You are a research planning agent. Given a query, create a research plan.
Output a JSON array of steps. Each step has:
- "id": step number
- "description": what to do
- "tool_name": which tool to use (web_search, web_crawl, academic_search, analyze, synthesize)
- "tool_input": parameters for the tool as a JSON object
- "depends_on": array of step IDs this depends on

Available tools:
- web_search: Search the web. Input: {"query": "search terms"}
- web_crawl: Crawl a URL for content. Input: {"url": "https://..."}
- academic_search: Search academic papers. Input: {"query": "research topic"}
- analyze: Analyze collected content. Input: {"content": "text to analyze", "question": "what to look for"}
- synthesize: Combine findings. Input: {"findings": [...], "question": "original query"}

Create 3-8 steps for a thorough research plan. Output ONLY the JSON array."#;

        let memory_context = if context.working_memory.is_empty() {
            String::new()
        } else {
            let findings: Vec<String> = context
                .working_memory
                .iter()
                .map(|f| format!("- {}", f.content))
                .collect();
            format!(
                "\n\nPrevious findings:\n{}",
                findings.join("\n")
            )
        };

        let messages = vec![
            Message {
                role: Role::System,
                content: system_prompt.to_string(),
                timestamp: chrono::Utc::now(),
            },
            Message {
                role: Role::User,
                content: format!(
                    "Create a research plan for: {}{}",
                    query, memory_context
                ),
                timestamp: chrono::Utc::now(),
            },
        ];

        let request = LlmRequest {
            messages,
            max_tokens: Some(2048),
            temperature: Some(0.3),
            stop_sequences: Vec::new(),
            tools: None,
            stream: false,
        };

        let response = self.llm.complete(&request).await?;

        // Parse the plan from LLM response
        let steps = parse_plan_steps(&response.content);

        Ok(AgentPlan {
            steps,
            reasoning: format!("Research plan for: {}", query),
            estimated_duration_secs: Some(60),
        })
    }

    async fn execute_step(
        &self,
        step: &PlanStep,
        context: &AgentContext,
        tools: &ToolRegistry,
    ) -> Result<StepResult> {
        info!(
            step_id = step.id,
            tool = %step.tool_name,
            description = %step.description,
            "Executing research step"
        );

        let input = ToolInput {
            parameters: step.tool_input.clone(),
        };

        // Execute the tool
        match tools.execute(&step.tool_name, &input).await {
            Ok(output) => {
                // Convert tool output to findings
                let findings = vec![Finding {
                    id: uuid::Uuid::new_v4(),
                    content: output.text_summary.clone(),
                    confidence: 0.7,
                    source: Source {
                        url: None,
                        title: step.description.clone(),
                        reliability_score: 0.7,
                        access_date: chrono::Utc::now(),
                    },
                    category: FindingCategory::Fact,
                    entities: Vec::new(),
                    timestamp: chrono::Utc::now(),
                }];

                Ok(StepResult {
                    step_id: step.id,
                    success: true,
                    output: output.text_summary,
                    findings,
                    follow_up_needed: false,
                })
            }
            Err(e) => Ok(StepResult {
                step_id: step.id,
                success: false,
                output: format!("Error: {}", e),
                findings: Vec::new(),
                follow_up_needed: true,
            }),
        }
    }

    async fn synthesize(
        &self,
        results: &[StepResult],
        context: &AgentContext,
    ) -> Result<Synthesis> {
        let findings_text: Vec<String> = results
            .iter()
            .filter(|r| r.success)
            .map(|r| format!("## Finding\n{}", r.output))
            .collect();

        let combined = findings_text.join("\n\n");

        let messages = vec![
            Message {
                role: Role::System,
                content: "You are a research synthesis agent. Combine the following findings into a comprehensive summary. Identify key insights, gaps, and contradictions. Be thorough and cite your sources.".to_string(),
                timestamp: chrono::Utc::now(),
            },
            Message {
                role: Role::User,
                content: format!(
                    "Synthesize these research findings:\n\n{}\n\nQuery: {}",
                    combined,
                    context
                        .current_task
                        .as_ref()
                        .map(|t| t.query.as_str())
                        .unwrap_or("Unknown query")
                ),
                timestamp: chrono::Utc::now(),
            },
        ];

        let request = LlmRequest {
            messages,
            max_tokens: Some(4096),
            temperature: Some(0.5),
            stop_sequences: Vec::new(),
            tools: None,
            stream: false,
        };

        let response = self.llm.complete(&request).await?;

        let all_findings: Vec<Finding> = results
            .iter()
            .flat_map(|r| r.findings.clone())
            .collect();

        let failed_count = results.iter().filter(|r| !r.success).count();

        Ok(Synthesis {
            summary: response.content,
            key_findings: all_findings,
            gaps: if failed_count > 0 {
                vec![format!("{} research steps failed", failed_count)]
            } else {
                Vec::new()
            },
            confidence: if failed_count == 0 { 0.85 } else { 0.6 },
            needs_more_research: failed_count > results.len() / 2,
        })
    }
}

/// Parse plan steps from LLM JSON response
fn parse_plan_steps(response: &str) -> Vec<PlanStep> {
    // Try to extract JSON array from response
    let json_str = if let Some(start) = response.find('[') {
        if let Some(end) = response.rfind(']') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        response
    };

    if let Ok(steps) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        steps
            .into_iter()
            .enumerate()
            .map(|(idx, step)| PlanStep {
                id: step["id"].as_u64().unwrap_or(idx as u64 + 1) as usize,
                description: step["description"]
                    .as_str()
                    .unwrap_or("Research step")
                    .to_string(),
                tool_name: step["tool_name"]
                    .as_str()
                    .unwrap_or("web_search")
                    .to_string(),
                tool_input: step["tool_input"].clone(),
                depends_on: step["depends_on"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as usize))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect()
    } else {
        // Fallback: create a basic 3-step plan
        vec![
            PlanStep {
                id: 1,
                description: "Search the web for relevant information".to_string(),
                tool_name: "web_search".to_string(),
                tool_input: serde_json::json!({"query": response}),
                depends_on: Vec::new(),
            },
            PlanStep {
                id: 2,
                description: "Analyze the search results".to_string(),
                tool_name: "analyze".to_string(),
                tool_input: serde_json::json!({"content": "", "question": response}),
                depends_on: vec![1],
            },
            PlanStep {
                id: 3,
                description: "Synthesize findings into a report".to_string(),
                tool_name: "synthesize".to_string(),
                tool_input: serde_json::json!({"findings": [], "question": response}),
                depends_on: vec![2],
            },
        ]
    }
}
