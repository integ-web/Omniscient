use anyhow::Result;

pub struct ReactLoop {
    // This structure coordinates the ReAct (Reason, Plan, Act, Observe) loop.
    // It's meant to test a single agent's capability to take steps before
    // multi-agent orchestration via DAG.
}

impl ReactLoop {
    pub async fn run_react_agent(agent: &impl rig::completion::Prompt, query: &str) -> Result<String> {
        // Minimal ReAct loop abstraction: The agent prompts itself to plan, act, observe.
        // For MVP testing, we just prompt the agent with the user's query and a ReAct prompt.
        let prompt = format!("You are an autonomous ReAct agent.
Follow this format strictly:
Thought: think about what to do
Action: the action to take (e.g., search Google)
Observation: the result of the action

Begin!

User Query: {}", query);

        let response = agent.prompt(&prompt).await?;
        Ok(response)
    }
}
