//! Omniscient CLI — Command-line interface for the deep research agent
//!
//! Usage:
//!   omniscient research "your query here"
//!   omniscient research "query" --depth deep
//!   omniscient config init
//!   omniscient config show

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use omniscient_core::config::OmniscientConfig;
use omniscient_core::tools::ToolRegistry;
use omniscient_core::types::ResearchDepth;

use omniscient_llm::api::OllamaProvider;
use omniscient_llm::categorizer::SlmCategorizer;
use omniscient_llm::provider::LlmProvider;

use omniscient_research::report::ReportGenerator;
use omniscient_research::DeepResearchPipeline;
use omniscient_web::{WebCrawler, crawler::CrawlConfig};

/// Omniscient — The God of All Research Agents
#[derive(Parser)]
#[command(
    name = "omniscient",
    version,
    about = "🔮 Omniscient — Deep Research AI Agent",
    long_about = "Omniscient is a Rust-native deep research agent that can conduct\n\
                  comprehensive research on any topic, company, person, or domain.\n\n\
                  Built for performance on low-end hardware with cross-platform support."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, default_value = "config/omniscient.toml")]
    config: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Conduct a deep research query
    Research {
        /// The research query
        query: String,

        /// Research depth: quick, standard, deep, exhaustive
        #[arg(short, long, default_value = "standard")]
        depth: String,

        /// Output file path (default: auto-generated)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Profile a company or person
    Profile {
        /// Name of the company or person
        name: String,

        /// Type: company, person
        #[arg(short, long, default_value = "company")]
        r#type: String,
    },

    /// Compare multiple entities
    Compare {
        /// Entities to compare (comma-separated)
        entities: String,
    },

    /// Crawl a URL and extract content (Firecrawl-style)
    Crawl {
        /// URL to crawl
        url: String,

        /// Use headless browser for JS-heavy sites
        #[arg(short, long)]
        browser: bool,

        /// Maximum crawl depth
        #[arg(short, long, default_value = "0")]
        depth: usize,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show system status and information  
    Status,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize default configuration
    Init,
    /// Show current configuration
    Show,
}

fn print_banner() {
    let banner = r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ██████╗ ███╗   ███╗███╗   ██╗██╗███████╗ ██████╗██╗███████╗███╗   ██╗████████╗   ║
    ║  ██╔═══██╗████╗ ████║████╗  ██║██║██╔════╝██╔════╝██║██╔════╝████╗  ██║╚══██╔══╝   ║
    ║  ██║   ██║██╔████╔██║██╔██╗ ██║██║███████╗██║     ██║█████╗  ██╔██╗ ██║   ██║      ║
    ║  ██║   ██║██║╚██╔╝██║██║╚██╗██║██║╚════██║██║     ██║██╔══╝  ██║╚██╗██║   ██║      ║
    ║  ╚██████╔╝██║ ╚═╝ ██║██║ ╚████║██║███████║╚██████╗██║███████╗██║ ╚████║   ██║      ║
    ║   ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═══╝╚═╝╚══════╝ ╚═════╝╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝      ║
    ║                                                               ║
    ║          🔮 The God of All Research Agents 🔮                  ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#;
    println!("{}", banner.bright_cyan());
}

fn parse_depth(s: &str) -> ResearchDepth {
    match s.to_lowercase().as_str() {
        "quick" | "q" => ResearchDepth::Quick,
        "standard" | "s" => ResearchDepth::Standard,
        "deep" | "d" => ResearchDepth::Deep,
        "exhaustive" | "e" => ResearchDepth::Exhaustive,
        _ => ResearchDepth::Standard,
    }
}

fn setup_logging(verbose: bool) {
    let filter = if verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    // Load configuration
    let config = OmniscientConfig::load(&cli.config).unwrap_or_default();
    config.ensure_dirs()?;

    match cli.command {
        Commands::Research {
            query,
            depth,
            output,
        } => {
            print_banner();

            let depth = parse_depth(&depth);
            println!(
                "{} {}",
                "🔍 Research Query:".bright_green().bold(),
                query.bright_white()
            );
            println!(
                "{} {:?}",
                "📊 Depth:".bright_blue().bold(),
                depth
            );
            println!();

            // Setup LLM provider
            let llm: Box<dyn LlmProvider> = if let Some(ref ollama_cfg) = config.llm.ollama {
                Box::new(OllamaProvider::new(
                    ollama_cfg.host.clone(),
                    ollama_cfg.port,
                    ollama_cfg.model.clone(),
                ))
            } else {
                println!(
                    "{}",
                    "⚠️  No LLM provider configured. Using Ollama defaults.".yellow()
                );
                Box::new(OllamaProvider::new(
                    "localhost".to_string(),
                    11434,
                    "llama3.2:3b".to_string(),
                ))
            };

            // SLM Categorization
            if config.llm.use_slm_categorization {
                println!("{}", "🧠 Running SLM categorization...".bright_yellow());
                let categorizer_llm: Box<dyn LlmProvider> = Box::new(OllamaProvider::new(
                    "localhost".to_string(),
                    11434,
                    config
                        .llm
                        .slm_model
                        .clone()
                        .unwrap_or_else(|| "phi-3-mini".to_string()),
                ));
                let categorizer = SlmCategorizer::new(categorizer_llm);
                match categorizer.categorize(&query).await {
                    Ok(cat) => {
                        println!(
                            "   {} {:?} (confidence: {:.0}%)",
                            "Category:".bright_cyan(),
                            cat.category,
                            cat.confidence * 100.0,
                        );
                        println!(
                            "   {} {:?}",
                            "Suggested depth:".bright_cyan(),
                            cat.suggested_depth,
                        );
                        println!(
                            "   {} {:?}",
                            "Suggested tools:".bright_cyan(),
                            cat.suggested_tools,
                        );
                        println!();
                    }
                    Err(e) => {
                        println!(
                            "   {} {}",
                            "⚠️  Categorization fallback:".yellow(),
                            e,
                        );
                    }
                }
            }

            // Progress bar
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.set_message("Initializing research...");

            // Setup tools registry
            let tools = ToolRegistry::new();

            // Run research pipeline
            let pipeline = DeepResearchPipeline::new(config.clone());
            pb.set_position(20);
            pb.set_message("Conducting research...");

            match pipeline.research(&query, depth, llm, tools).await {
                Ok(report) => {
                    pb.set_position(90);
                    pb.set_message("Generating report...");

                    // Generate report
                    let reporter = ReportGenerator::new();
                    let markdown = reporter.generate_markdown(&report);

                    // Save report
                    let output_path = output.unwrap_or_else(|| {
                        let filename = format!(
                            "report_{}.md",
                            chrono::Utc::now().format("%Y%m%d_%H%M%S")
                        );
                        config.research.output_dir.join(filename)
                    });

                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&output_path, &markdown)?;

                    pb.finish_with_message("Research complete!");
                    println!();
                    println!(
                        "{} {}",
                        "📄 Report saved:".bright_green().bold(),
                        output_path.display().to_string().bright_white()
                    );
                    println!();

                    // Print summary
                    println!("{}", "━".repeat(60).bright_cyan());
                    println!(
                        "{}",
                        "📋 Executive Summary".bright_green().bold()
                    );
                    println!("{}", "━".repeat(60).bright_cyan());
                    println!("{}", report.executive_summary);
                    println!();
                    println!(
                        "   {} {}",
                        "Sources consulted:".bright_blue(),
                        report.total_sources_consulted
                    );
                    println!(
                        "   {} {}",
                        "Key findings:".bright_blue(),
                        report.findings.len()
                    );
                    println!(
                        "   {} {}",
                        "Entities discovered:".bright_blue(),
                        report.entities.len()
                    );
                }
                Err(e) => {
                    pb.finish_with_message("Research failed");
                    eprintln!(
                        "{} {}",
                        "❌ Research failed:".bright_red().bold(),
                        e,
                    );
                }
            }
        }

        Commands::Profile { name, r#type } => {
            print_banner();
            println!(
                "{} {} ({})",
                "🔍 Profiling:".bright_green().bold(),
                name.bright_white(),
                r#type.bright_cyan(),
            );
            println!(
                "{}",
                "Profile command will use the research pipeline with specialized prompts."
                    .bright_yellow()
            );

            let query = match r#type.as_str() {
                "person" => format!(
                    "Comprehensive profile of {}: career history, achievements, publications, social presence, connections, and influence",
                    name
                ),
                _ => format!(
                    "Full company analysis of {}: founding story, leadership, products, technology stack, competitors, financials, news, market position",
                    name
                ),
            };

            println!("  Query: {}", query.bright_white());
            println!(
                "\n{}",
                "Run: omniscient research \"<query>\" --depth deep".bright_yellow()
            );
        }

        Commands::Compare { entities } => {
            print_banner();
            let names: Vec<&str> = entities.split(',').map(|s| s.trim()).collect();
            println!(
                "{} {}",
                "⚖️  Comparing:".bright_green().bold(),
                names.join(" vs ").bright_white(),
            );
            println!(
                "{}",
                "Compare command will run parallel research and SWOT analysis.".bright_yellow()
            );
        }

        Commands::Config { action } => match action {
            ConfigAction::Init => {
                let default_config = OmniscientConfig::default();
                let config_path = cli.config;
                default_config.save(&config_path)?;
                println!(
                    "{} {}",
                    "✅ Configuration initialized:".bright_green().bold(),
                    config_path.display().to_string().bright_white(),
                );
            }
            ConfigAction::Show => {
                let toml_str = toml::to_string_pretty(&config)?;
                println!("{}", "📋 Current Configuration:".bright_green().bold());
                println!("{}", toml_str);
            }
        },

        Commands::Crawl { url, browser, depth } => {
            print_banner();
            println!(
                "{} {}",
                "🕷️  Crawling:".bright_green().bold(),
                url.bright_white()
            );
            println!(
                "{} {}",
                "🌐 Mode:".bright_blue().bold(),
                if browser { "Headless Browser (JS-Enabled)" } else { "Standard HTTP" }
            );

            let crawl_config = CrawlConfig {
                max_depth: depth,
                max_pages: 1, // Start with 1 for direct crawl
                use_browser: browser,
                ..Default::default()
            };

            let crawler = WebCrawler::new(crawl_config);
            
            let pb = ProgressBar::new_spinner();
            pb.set_message("Fetching page content...");
            pb.enable_steady_tick(Duration::from_millis(100));

            match crawler.fetch_page(&url).await {
                Ok(result) => {
                    pb.finish_with_message("Crawl complete!");
                    if let Some(doc) = result.document {
                        println!("\n{}", "📄 Content Preview:".bright_cyan().bold());
                        println!("{}", "━".repeat(40).bright_cyan());
                        println!("{}", doc.content.chars().take(1000).collect::<String>());
                        if doc.content.len() > 1000 {
                            println!("{}", "... [truncated]".bright_black());
                        }
                        println!("{}", "━".repeat(40).bright_cyan());
                        println!(
                            "   {} {}",
                            "Title:".bright_blue(),
                            doc.title.unwrap_or_default()
                        );
                        println!(
                            "   {} {}",
                            "Words:".bright_blue(),
                            doc.metadata.word_count
                        );
                    } else {
                        println!("{}", "⚠️  No content extracted.".yellow());
                    }
                }
                Err(e) => {
                    pb.finish_with_message("Crawl failed");
                    eprintln!("{} {}", "❌ Error:".bright_red().bold(), e);
                }
            }
        }

        Commands::Status => {
            print_banner();
            println!("{}", "📊 System Status".bright_green().bold());
            println!("{}", "━".repeat(40).bright_cyan());
            println!(
                "  {} {}",
                "Version:".bright_blue(),
                env!("CARGO_PKG_VERSION")
            );
            println!(
                "  {} {}",
                "Data directory:".bright_blue(),
                config.general.data_dir.display()
            );
            println!(
                "  {} {}",
                "Default LLM:".bright_blue(),
                config.llm.default_backend
            );
            println!(
                "  {} {}",
                "SLM categorization:".bright_blue(),
                if config.llm.use_slm_categorization {
                    "enabled ✅"
                } else {
                    "disabled ❌"
                }
            );
            println!(
                "  {} {}",
                "Search engines:".bright_blue(),
                config
                    .web
                    .search_engines
                    .iter()
                    .filter(|e| e.enabled)
                    .map(|e| e.engine.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!(
                "  {} {}",
                "Knowledge DB:".bright_blue(),
                config.knowledge.db_mode
            );
            println!(
                "  {} {:?}",
                "Default depth:".bright_blue(),
                config.research.default_depth
            );
        }
    }

    Ok(())
}
