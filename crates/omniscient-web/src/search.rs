//! Multi-engine Search Client — DuckDuckGo, Brave, SearXNG, Serper, Firecrawl

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use omniscient_core::error::{OmniscientError, Result};

/// Helper to convert reqwest errors
fn web_err(e: reqwest::Error) -> OmniscientError {
    OmniscientError::Web(e.to_string())
}

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source_engine: String,
    pub rank: usize,
}

/// Search engine trait — implement for each search provider
#[async_trait]
pub trait SearchEngine: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>>;
}

/// Multi-engine search client — queries multiple engines and merges results
pub struct SearchClient {
    engines: Vec<Box<dyn SearchEngine>>,
}

impl SearchClient {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    pub fn add_engine(&mut self, engine: Box<dyn SearchEngine>) {
        self.engines.push(engine);
    }

    /// Search across all engines and deduplicate results
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        for engine in &self.engines {
            info!(engine = engine.name(), query = query, "Searching");
            match engine.search(query, max_results).await {
                Ok(results) => {
                    info!(
                        engine = engine.name(),
                        count = results.len(),
                        "Search results received"
                    );
                    all_results.extend(results);
                }
                Err(e) => {
                    warn!(engine = engine.name(), error = %e, "Search engine failed");
                }
            }
        }

        // Deduplicate by URL
        let mut seen_urls = std::collections::HashSet::new();
        all_results.retain(|r| seen_urls.insert(r.url.clone()));

        // Truncate to max_results
        all_results.truncate(max_results);

        Ok(all_results)
    }

    /// Get the number of configured engines
    pub fn engine_count(&self) -> usize {
        self.engines.len()
    }
}

impl Default for SearchClient {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// DuckDuckGo (no API key needed)
// ──────────────────────────────────────────────

pub struct DuckDuckGoEngine {
    client: Client,
}

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Omniscient-Research-Agent/0.1")
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoEngine {
    fn name(&self) -> &str {
        "duckduckgo"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await.map_err(web_err)?;
        let html = response.text().await.map_err(web_err)?;

        let document = scraper::Html::parse_document(&html);
        let result_selector = scraper::Selector::parse(".result").unwrap();
        let title_selector = scraper::Selector::parse(".result__title a").unwrap();
        let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();

        let mut results = Vec::new();

        for (idx, result) in document.select(&result_selector).enumerate() {
            if idx >= max_results {
                break;
            }

            let title = result
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let link = result
                .select(&title_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| {
                    if let Some(pos) = href.find("uddg=") {
                        let encoded = &href[pos + 5..];
                        if let Some(end) = encoded.find('&') {
                            urlencoding::decode(&encoded[..end])
                                .unwrap_or_default()
                                .to_string()
                        } else {
                            urlencoding::decode(encoded)
                                .unwrap_or_default()
                                .to_string()
                        }
                    } else {
                        href.to_string()
                    }
                })
                .unwrap_or_default();

            let snippet = result
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !title.is_empty() && !link.is_empty() {
                results.push(SearchResult {
                    title,
                    url: link,
                    snippet,
                    source_engine: "duckduckgo".to_string(),
                    rank: idx + 1,
                });
            }
        }

        Ok(results)
    }
}

// ──────────────────────────────────────────────
// Brave Search API
// ──────────────────────────────────────────────

pub struct BraveSearchEngine {
    client: Client,
    api_key: String,
}

impl BraveSearchEngine {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for BraveSearchEngine {
    fn name(&self) -> &str {
        "brave"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", query), ("count", &max_results.to_string())])
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            return Err(OmniscientError::Web(format!(
                "Brave API error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        let results = data["web"]["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(idx, r)| SearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["url"].as_str().unwrap_or("").to_string(),
                        snippet: r["description"].as_str().unwrap_or("").to_string(),
                        source_engine: "brave".to_string(),
                        rank: idx + 1,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

// ──────────────────────────────────────────────
// SearXNG (self-hosted meta search)
// ──────────────────────────────────────────────

pub struct SearxngEngine {
    client: Client,
    base_url: String,
}

impl SearxngEngine {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
}

#[async_trait]
impl SearchEngine for SearxngEngine {
    fn name(&self) -> &str {
        "searxng"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .get(&format!("{}/search", self.base_url))
            .query(&[
                ("q", query),
                ("format", "json"),
                ("pageno", "1"),
            ])
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            return Err(OmniscientError::Parse("SearXNG search failed".into()));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        let results = data["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(max_results)
                    .enumerate()
                    .map(|(idx, r)| SearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["url"].as_str().unwrap_or("").to_string(),
                        snippet: r["content"].as_str().unwrap_or("").to_string(),
                        source_engine: "searxng".to_string(),
                        rank: idx + 1,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

// ──────────────────────────────────────────────
// Serper API (Google Search)
// ──────────────────────────────────────────────

pub struct SerperEngine {
    client: Client,
    api_key: String,
}

impl SerperEngine {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchEngine for SerperEngine {
    fn name(&self) -> &str {
        "serper"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let body = serde_json::json!({
            "q": query,
            "num": max_results,
        });

        let response = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            return Err(OmniscientError::Parse("Serper search failed".into()));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        let results = data["organic"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(max_results)
                    .enumerate()
                    .map(|(idx, r)| SearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["link"].as_str().unwrap_or("").to_string(),
                        snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                        source_engine: "serper".to_string(),
                        rank: idx + 1,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

// ──────────────────────────────────────────────
// Firecrawl API (crawl and extract)
// ──────────────────────────────────────────────

pub struct FirecrawlClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl FirecrawlClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.firecrawl.dev".to_string()),
        }
    }

    /// Scrape a single URL using Firecrawl
    pub async fn scrape(&self, url: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "url": url,
            "formats": ["markdown", "html"],
        });

        let response = self
            .client
            .post(&format!("{}/v1/scrape", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(web_err)?;

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        Ok(data)
    }
}

// URL encoding helpers for DuckDuckGo
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => encoded.push('+'),
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        encoded
    }

    pub fn decode(s: &str) -> std::result::Result<std::borrow::Cow<str>, ()> {
        let mut result = String::new();
        let mut chars = s.chars();
        while let Some(ch) = chars.next() {
            if ch == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                }
            } else if ch == '+' {
                result.push(' ');
            } else {
                result.push(ch);
            }
        }
        Ok(std::borrow::Cow::Owned(result))
    }
}
