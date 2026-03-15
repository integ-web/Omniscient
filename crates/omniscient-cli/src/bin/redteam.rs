use omniscient_research::dag_orchestrator::RigOrchestrator;
use rig::providers::openai;

#[tokio::main]
async fn main() {
    println!("Starting Red Team / Stress Test logic for Rig Orchestrator...");

    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-mock-redteam-key");
    }
    let client = openai::Client::from_env();

    let orchestrator = match RigOrchestrator::new(&client, "gpt-4").await {
        Ok(orch) => orch,
        Err(e) => {
            println!("Failed to initialize orchestrator: {}", e);
            return;
        }
    };

    let query = "Find the ultimate beneficial owner of the shell company 'Oceanic Oceanic Holdings LLC' registered in the BVI.";
    println!("Executing query: {}", query);

    let result = orchestrator.execute_investigation(query).await;

    match result {
        Ok(state) => println!("Investigation State generated successfully:\n{:?}", state),
        Err(e) => println!("Investigation naturally failed on mock API call (expected behavior for unit test): {}", e),
    }
}
