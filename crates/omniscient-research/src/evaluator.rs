use anyhow::Result;
use rig::agent::Agent;
use rig::completion::Prompt;
use rig::providers::openai;

pub struct EvaluatorLoop {
    evaluator_agent: Agent<rig::providers::openai::CompletionModel>,
}

impl EvaluatorLoop {
    pub async fn new(client: &openai::Client, model: &str) -> Result<Self> {
        let evaluator_agent = client.agent(model)
            .preamble("You are a strict Evaluator agent specializing in corporate veil piercing.
Your objective is to verify if the scraped findings establish a clear Ultimate Beneficial Owner (UBO) connection or evidence of commingled funds.
If sufficient, reply strictly with 'SUFFICIENT: <summary>'.
If insufficient, provide exactly what details are missing and suggest the next search strategy.")
            .build();

        Ok(Self { evaluator_agent })
    }

    pub async fn evaluate_findings(&self, query: &str, findings: &str) -> Result<(bool, String)> {
        let prompt = format!(
            "Goal: {}\nFindings:\n{}\n\nEvaluate if these findings are sufficient to establish the UBO.",
            query, findings
        );

        let response = self.evaluator_agent.prompt(&prompt).await?;
        let is_sufficient = response.trim().starts_with("SUFFICIENT:");

        Ok((is_sufficient, response))
    }
}
