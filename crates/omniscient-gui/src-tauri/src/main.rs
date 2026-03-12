// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::{info, Level};

use omniscient_core::config::{LlmConfig, OmniscientConfig};
use omniscient_core::orchestrator::Orchestrator;
use omniscient_core::types::ProgressEvent;
use omniscient_knowledge::KnowledgeGraph;
use omniscient_research::{DeepResearchPipeline, ReportGenerator, ResearchAgent};

#[derive(serde::Serialize)]
struct CommandError(String);

impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        CommandError(err.to_string())
    }
}

// ──────────────────────────────────────────────
// Tauri Commands
// ──────────────────────────────────────────────

#[tauri::command]
async fn start_research(
    app: tauri::AppHandle,
    query: String,
    depth: String,
) -> Result<String, CommandError> {
    info!(query, depth, "GUI initiated research command");
    
    // Send initial event
    let _ = app.emit("research-progress", format!("Initializing research pipeline for: {}", query));

    let config = OmniscientConfig::default();
    let graph = Arc::new(KnowledgeGraph::new_memory().await.map_err(|e| anyhow::anyhow!(e))?);
    
    // We assume Ollama on localhost for local demo
    let llm_config = LlmConfig {
        default_backend: "ollama".to_string(),
        api_keys: std::collections::HashMap::new(),
    };
    
    let agent = Arc::new(ResearchAgent::new(llm_config));
    let mut orchestrator = Orchestrator::new(config);
    orchestrator.register_agent("research", agent.clone());
    
    // Create progress event channel
    let (progress_tx, mut progress_rx) = mpsc::channel::<ProgressEvent>(100);
    
    // Spawn task to forward progress events to frontend
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(event) = progress_rx.recv().await {
            let msg = format!("[{}] {}", event.stage, event.message);
            info!("{}", msg);
            let _ = app_clone.emit("research-progress", msg);
        }
    });
    
    let is_deep = depth == "deep";
    let pipeline = DeepResearchPipeline::new(agent, graph.clone());

    let _ = app.emit("research-progress", "Pipeline ready. Starting execution...");

    // Run the research pipeline
    let results = pipeline
        .run_research(&query, is_deep, progress_tx)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let _ = app.emit("research-progress", format!("Research complete. Found {} sources and {} entities. Generating report...", results.sources.len(), results.entities.len()));

    // Generate markdown report
    let report = ReportGenerator::new()
        .generate_report(&query, &results, graph.clone())
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // We'll trust our Markdown parser in the UI, but returning raw HTML works best here
    // For simplicity, we convert to HTML using a minimal transformation or just return markdown
    // Let's return the markdown and parse it in frontend. 
    // BUT we didn't add marked.js, so I'll wrap it in a `<pre>` for plain text, or let's use a quick HTML formatter.
    
    let html = format!(
        "<div class='markdown-report'>\n  <pre style='white-space: pre-wrap; font-family: inherit;'>\n{}  </pre>\n</div>",
        report.markdown_content()
    );

    Ok(html)
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Omniscient GUI...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![start_research])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
