//! Configuration system for Omniscient
//!
//! Loads from TOML config files with environment variable overrides.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Top-level configuration for the entire Omniscient system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniscientConfig {
    pub general: GeneralConfig,
    pub llm: LlmConfig,
    pub web: WebConfig,
    pub knowledge: KnowledgeConfig,
    pub research: ResearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Data directory for storing indexes, databases, cache
    pub data_dir: PathBuf,
    /// Maximum concurrent operations
    pub max_concurrency: usize,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Enable telemetry/progress reporting
    pub show_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default model backend to use
    pub default_backend: String,
    /// OpenAI API configuration
    pub openai: Option<ApiEndpointConfig>,
    /// Anthropic API configuration
    pub anthropic: Option<ApiEndpointConfig>,
    /// Ollama local server configuration
    pub ollama: Option<OllamaConfig>,
    /// Local model path for Candle inference
    pub local_model_path: Option<PathBuf>,
    /// Use SLM for task categorization to reduce load
    pub use_slm_categorization: bool,
    /// SLM model name for categorization (e.g., "phi-3-mini")
    pub slm_model: Option<String>,
    /// Maximum tokens for generation
    pub max_tokens: usize,
    /// Temperature for generation
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpointConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_retries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// User agent string for requests
    pub user_agent: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum pages to crawl per domain
    pub max_pages_per_domain: usize,
    /// Maximum crawl depth
    pub max_crawl_depth: usize,
    /// Respect robots.txt
    pub respect_robots_txt: bool,
    /// Delay between requests (ms) for politeness
    pub request_delay_ms: u64,
    /// Search engines to use
    pub search_engines: Vec<SearchEngineConfig>,
    /// Firecrawl API configuration (optional)
    pub firecrawl: Option<FirecrawlConfig>,
    /// Browser-based scraping configuration
    pub browser: BrowserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngineConfig {
    pub engine: String, // "duckduckgo", "brave", "google", "serper", "searxng"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrawlConfig {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Enable headless browser for JS-heavy pages
    pub enabled: bool,
    /// Browser to use ("chromium", "firefox")
    pub browser_type: String,
    /// Headless mode
    pub headless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    /// Directory for Tantivy search indexes
    pub index_dir: PathBuf,
    /// SurrealDB storage mode
    pub db_mode: String, // "memory", "file", "rocksdb"
    /// SurrealDB file path (if file/rocksdb mode)
    pub db_path: Option<PathBuf>,
    /// Enable vector embeddings
    pub enable_vectors: bool,
    /// Embedding model name
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConfig {
    /// Default research depth
    pub default_depth: String, // "quick", "standard", "deep", "exhaustive"
    /// Maximum sources to consult per research task
    pub max_sources: usize,
    /// Enable fact checking / cross-referencing
    pub enable_fact_checking: bool,
    /// Report output format
    pub report_format: String, // "markdown", "html", "pdf"
    /// Output directory for reports
    pub output_dir: PathBuf,
}

impl Default for OmniscientConfig {
    fn default() -> Self {
        let home = dirs_home();
        let data_dir = home.join(".omniscient");

        Self {
            general: GeneralConfig {
                data_dir: data_dir.clone(),
                max_concurrency: 10,
                log_level: "info".to_string(),
                show_progress: true,
            },
            llm: LlmConfig {
                default_backend: "ollama".to_string(),
                openai: None,
                anthropic: None,
                ollama: Some(OllamaConfig {
                    host: "localhost".to_string(),
                    port: 11434,
                    model: "llama3.2:3b".to_string(),
                }),
                local_model_path: None,
                use_slm_categorization: true,
                slm_model: Some("phi-3-mini".to_string()),
                max_tokens: 4096,
                temperature: 0.7,
            },
            web: WebConfig {
                user_agent: "Omniscient-Research-Agent/0.1 (https://github.com/omniscient-agent)"
                    .to_string(),
                timeout_secs: 30,
                max_pages_per_domain: 50,
                max_crawl_depth: 3,
                respect_robots_txt: true,
                request_delay_ms: 500,
                search_engines: vec![
                    SearchEngineConfig {
                        engine: "duckduckgo".to_string(),
                        api_key: None,
                        base_url: None,
                        enabled: true,
                    },
                    SearchEngineConfig {
                        engine: "brave".to_string(),
                        api_key: None,
                        base_url: None,
                        enabled: false,
                    },
                    SearchEngineConfig {
                        engine: "searxng".to_string(),
                        api_key: None,
                        base_url: Some("http://localhost:8888".to_string()),
                        enabled: false,
                    },
                ],
                firecrawl: None,
                browser: BrowserConfig {
                    enabled: false,
                    browser_type: "chromium".to_string(),
                    headless: true,
                },
            },
            knowledge: KnowledgeConfig {
                index_dir: data_dir.join("index"),
                db_mode: "memory".to_string(),
                db_path: Some(data_dir.join("db")),
                enable_vectors: false,
                embedding_model: None,
            },
            research: ResearchConfig {
                default_depth: "standard".to_string(),
                max_sources: 20,
                enable_fact_checking: true,
                report_format: "markdown".to_string(),
                output_dir: data_dir.join("reports"),
            },
        }
    }
}

impl OmniscientConfig {
    /// Load configuration from a TOML file, falling back to defaults
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| crate::error::OmniscientError::Config(e.to_string()))?;
            let config: Self = toml::from_str(&content)
                .map_err(|e| crate::error::OmniscientError::Config(e.to_string()))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::OmniscientError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Ensure all required directories exist
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.general.data_dir)?;
        std::fs::create_dir_all(&self.knowledge.index_dir)?;
        std::fs::create_dir_all(&self.research.output_dir)?;
        if let Some(ref db_path) = self.knowledge.db_path {
            std::fs::create_dir_all(db_path)?;
        }
        Ok(())
    }
}

/// Get the user's home directory
fn dirs_home() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    }
}
