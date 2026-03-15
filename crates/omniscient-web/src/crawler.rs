//! Web Crawler — async recursive crawler with rate limiting and polite scraping

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;
use futures::StreamExt;

use omniscient_core::error::{OmniscientError, Result};
use omniscient_core::types::{ContentType, Document, DocumentMetadata};

use crate::extractor::ContentExtractor;

/// Configuration for crawling behavior
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    pub max_depth: usize,
    pub max_pages: usize,
    pub timeout: Duration,
    pub delay_between_requests: Duration,
    pub user_agent: String,
    pub respect_robots_txt: bool,
    pub allowed_domains: Option<Vec<String>>,
    pub max_concurrent: usize,
    pub use_browser: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_pages: 50,
            timeout: Duration::from_secs(30),
            delay_between_requests: Duration::from_millis(500),
            user_agent: "Omniscient-Research-Agent/0.1".to_string(),
            respect_robots_txt: true,
            allowed_domains: None,
            max_concurrent: 5,
            use_browser: false,
        }
    }
}

/// Result of crawling a single page
#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub url: String,
    pub status: u16,
    pub document: Option<Document>,
    pub links: Vec<String>,
    pub error: Option<String>,
}

/// Async web crawler with depth control and rate limiting
pub struct WebCrawler {
    client: Client,
    config: CrawlConfig,
    extractor: ContentExtractor,
}

impl WebCrawler {
    pub fn new(config: CrawlConfig) -> Self {
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::limited(10))
            .gzip(true)
            .brotli(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            config,
            extractor: ContentExtractor::new(),
        }
    }

    /// Fetch a single page using pure-Rust heuristics engine (SPA fallback)
    pub async fn fetch_page_with_heuristics(&self, html: &str, url: &str) -> Result<CrawlResult> {
        info!("Applying SPA/JSON heuristics for: {}", url);

        let mut text_content = String::new();

        // Very basic Next.js / React JSON state data extraction
        if let Some(start) = html.find(r#"<script id="__NEXT_DATA__""#) {
            if let Some(data_start) = html[start..].find('>') {
                if let Some(end) = html[start + data_start + 1..].find("</script>") {
                    let json_str = &html[start + data_start + 1..start + data_start + 1 + end];
                    if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                        text_content.push_str(&format!("SPA JSON Data:\n{:#?}", json_data));
                    }
                }
            }
        }

        // Add standard extracted fallback
        let extracted = self.extractor.extract(html, url);
        let links = self.extractor.extract_links(html, url);

        if !text_content.is_empty() {
            text_content.push_str("\n\nExtracted Visible Text:\n");
        }
        text_content.push_str(&extracted.clean_text);

        let document = Document {
            id: uuid::Uuid::new_v4(),
            url: Some(url.to_string()),
            title: extracted.title.clone(),
            content: text_content,
            content_type: ContentType::WebPage,
            metadata: DocumentMetadata {
                author: extracted.author.clone(),
                published_date: None,
                source: url.to_string(),
                word_count: extracted.clean_text.split_whitespace().count(),
                language: None,
                tags: Vec::new(),
            },
            extracted_at: chrono::Utc::now(),
        };

        Ok(CrawlResult {
            url: url.to_string(),
            status: 200,
            document: Some(document),
            links,
            error: None,
        })
    }

    /// Fetch a single page and extract content
    pub async fn fetch_page(&self, url: &str) -> Result<CrawlResult> {
        debug!("Fetching: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| OmniscientError::Web(e.to_string()))?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            return Ok(CrawlResult {
                url: url.to_string(),
                status,
                document: None,
                links: Vec::new(),
                error: Some(format!("HTTP {}", status)),
            });
        }

        let html = response.text().await.map_err(|e| OmniscientError::Web(e.to_string()))?;

        // Pass through heuristics to grab any SPA embedded JSON config if useful
        let mut result = self.fetch_page_with_heuristics(&html, url).await?;
        result.status = status;

        Ok(result)
    }

    /// Crawl a site starting from a URL, up to max_depth and max_pages
    pub async fn crawl(&self, start_url: &str) -> Result<Vec<CrawlResult>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut results: Vec<CrawlResult> = Vec::new();

        // Normalize and add start URL
        let start = Self::normalize_url(start_url);
        queue.push_back((start.clone(), 0));

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent));

        while let Some((url, depth)) = queue.pop_front() {
            if visited.contains(&url) {
                continue;
            }
            if visited.len() >= self.config.max_pages {
                info!("Reached max pages limit ({})", self.config.max_pages);
                break;
            }
            if depth > self.config.max_depth {
                continue;
            }

            // Check domain restrictions
            if let Some(ref domains) = self.config.allowed_domains {
                if let Ok(parsed) = Url::parse(&url) {
                    if let Some(host) = parsed.host_str() {
                        if !domains.iter().any(|d| host.contains(d)) {
                            continue;
                        }
                    }
                }
            }

            visited.insert(url.clone());

            // Rate limiting
            tokio::time::sleep(self.config.delay_between_requests).await;

            let _permit = semaphore.acquire().await.unwrap();

            match self.fetch_page(&url).await {
                Ok(result) => {
                    // Queue discovered links
                    if depth < self.config.max_depth {
                        for link in &result.links {
                            if !visited.contains(link) {
                                queue.push_back((link.clone(), depth + 1));
                            }
                        }
                    }
                    results.push(result);
                }
                Err(e) => {
                    warn!("Failed to fetch {}: {}", url, e);
                    results.push(CrawlResult {
                        url: url.clone(),
                        status: 0,
                        document: None,
                        links: Vec::new(),
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        info!(
            "Crawl complete: {} pages fetched, {} URLs visited",
            results.len(),
            visited.len()
        );

        Ok(results)
    }

    /// Normalize a URL for deduplication
    fn normalize_url(url: &str) -> String {
        url.trim_end_matches('/').to_string()
    }
}
