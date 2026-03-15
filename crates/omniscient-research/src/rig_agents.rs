use rig::providers::openai::Client;
use rig::agent::Agent;

pub async fn build_planner_agent(client: &Client, model: &str) -> Agent<rig::providers::openai::CompletionModel> {
    client.agent(model)
        .preamble("You are an expert private investigator Planner agent.
Your objective is to decompose high-level investigation goals into targeted, actionable research steps.
You must focus on asset tracing, corporate veil piercing, and identifying Ultimate Beneficial Owners (UBOs).
Output a clear plan detailing the search queries and entities to track.")
        .build()
}

pub async fn build_scraper_agent(client: &Client, model: &str) -> Agent<rig::providers::openai::CompletionModel> {
    client.agent(model)
        .preamble("You are an elite Data Scraper agent.
Your objective is to execute research tasks, using available OSINT tools and scraping techniques to gather facts.
You must find and document hidden assets, company ownership details, and exact financial traces.")
        .build()
}

pub async fn build_evaluator_agent(client: &Client, model: &str) -> Agent<rig::providers::openai::CompletionModel> {
    client.agent(model)
        .preamble("You are a strict Evaluator agent.
Your objective is to check the Scraper's findings against the original goal.
If the data is insufficient to establish UBOs or pierce the corporate veil, you must pinpoint the missing links and suggest the next search strategy. If sufficient, output 'SUFFICIENT: ' followed by a summary.")
        .build()
}

pub async fn build_synthesizer_agent(client: &Client, model: &str) -> Agent<rig::providers::openai::CompletionModel> {
    client.agent(model)
        .preamble("You are a master Synthesizer agent.
Your objective is to produce a cohesive, legally sound report linking disparate facts into a unified narrative.
You must highlight multi-hop relational logic that proves control over hidden entities.")
        .build()
}
