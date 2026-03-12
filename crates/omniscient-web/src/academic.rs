//! Academic Database Connectors — arXiv, Semantic Scholar, CrossRef

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use omniscient_core::error::{OmniscientError, Result};

/// Helper to convert reqwest errors
fn web_err(e: reqwest::Error) -> OmniscientError {
    OmniscientError::Web(e.to_string())
}

/// An academic paper result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicPaper {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub year: Option<u32>,
    pub url: String,
    pub pdf_url: Option<String>,
    pub doi: Option<String>,
    pub citation_count: Option<u32>,
    pub source: String,
    pub venue: Option<String>,
}

/// Academic search engine trait
#[async_trait]
pub trait AcademicSearch: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<AcademicPaper>>;
}

/// Multi-source academic search client
pub struct AcademicClient {
    sources: Vec<Box<dyn AcademicSearch>>,
}

impl AcademicClient {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: Box<dyn AcademicSearch>) {
        self.sources.push(source);
    }

    /// Search across all academic databases
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<AcademicPaper>> {
        let mut all_results = Vec::new();
        for source in &self.sources {
            match source.search(query, max_results).await {
                Ok(results) => all_results.extend(results),
                Err(e) => {
                    tracing::warn!(source = source.name(), error = %e, "Academic search failed");
                }
            }
        }
        Ok(all_results)
    }

    /// Build a default client with free APIs
    pub fn default_client() -> Self {
        let mut client = Self::new();
        client.add_source(Box::new(ArxivSearch::new()));
        client.add_source(Box::new(SemanticScholarSearch::new()));
        client
    }
}

impl Default for AcademicClient {
    fn default() -> Self {
        Self::default_client()
    }
}

// ──────────────────────────────────────────────
// arXiv API (free, no key needed)
// ──────────────────────────────────────────────

pub struct ArxivSearch {
    client: Client,
}

impl ArxivSearch {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AcademicSearch for ArxivSearch {
    fn name(&self) -> &str {
        "arxiv"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<AcademicPaper>> {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            query.replace(' ', "+"),
            max_results
        );

        let response = self.client.get(&url).send().await.map_err(web_err)?;
        let xml = response.text().await.map_err(web_err)?;

        // Simple XML parsing for arXiv Atom feed
        let mut papers = Vec::new();
        let mut current_title = String::new();
        let mut current_summary = String::new();
        let mut current_authors = Vec::new();
        let mut current_url = String::new();
        let mut current_pdf = None;
        let mut in_entry = false;

        for line in xml.lines() {
            let line = line.trim();
            if line.contains("<entry>") {
                in_entry = true;
                current_title.clear();
                current_summary.clear();
                current_authors.clear();
                current_url.clear();
                current_pdf = None;
            } else if line.contains("</entry>") && in_entry {
                in_entry = false;
                if !current_title.is_empty() {
                    papers.push(AcademicPaper {
                        title: current_title.clone(),
                        authors: current_authors.clone(),
                        abstract_text: current_summary.clone(),
                        year: None,
                        url: current_url.clone(),
                        pdf_url: current_pdf.clone(),
                        doi: None,
                        citation_count: None,
                        source: "arxiv".to_string(),
                        venue: None,
                    });
                }
            } else if in_entry {
                if line.starts_with("<title>") {
                    current_title = line
                        .replace("<title>", "")
                        .replace("</title>", "")
                        .trim()
                        .to_string();
                } else if line.starts_with("<summary>") {
                    current_summary = line
                        .replace("<summary>", "")
                        .replace("</summary>", "")
                        .trim()
                        .to_string();
                } else if line.contains("<name>") {
                    let name = line
                        .replace("<name>", "")
                        .replace("</name>", "")
                        .trim()
                        .to_string();
                    current_authors.push(name);
                } else if line.contains("rel=\"alternate\"") {
                    if let Some(href) = extract_href(line) {
                        current_url = href;
                    }
                } else if line.contains("title=\"pdf\"") {
                    if let Some(href) = extract_href(line) {
                        current_pdf = Some(href);
                    }
                }
            }
        }

        info!(count = papers.len(), "arXiv search results");
        Ok(papers)
    }
}

// ──────────────────────────────────────────────
// Semantic Scholar API (free, no key needed for basic)
// ──────────────────────────────────────────────

pub struct SemanticScholarSearch {
    client: Client,
}

impl SemanticScholarSearch {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AcademicSearch for SemanticScholarSearch {
    fn name(&self) -> &str {
        "semantic_scholar"
    }

    async fn search(&self, query: &str, max_results: usize) -> Result<Vec<AcademicPaper>> {
        let response = self
            .client
            .get("https://api.semanticscholar.org/graph/v1/paper/search")
            .query(&[
                ("query", query),
                ("limit", &max_results.to_string()),
                (
                    "fields",
                    "title,authors,abstract,year,url,externalIds,citationCount,venue",
                ),
            ])
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            return Err(OmniscientError::Parse(
                "Semantic Scholar API error".to_string(),
            ));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        let papers: Vec<AcademicPaper> = data["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|p| AcademicPaper {
                        title: p["title"].as_str().unwrap_or("").to_string(),
                        authors: p["authors"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|author| {
                                        author["name"].as_str().unwrap_or("").to_string()
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        abstract_text: p["abstract"].as_str().unwrap_or("").to_string(),
                        year: p["year"].as_u64().map(|y| y as u32),
                        url: p["url"].as_str().unwrap_or("").to_string(),
                        pdf_url: None,
                        doi: p["externalIds"]["DOI"]
                            .as_str()
                            .map(|s| s.to_string()),
                        citation_count: p["citationCount"].as_u64().map(|c| c as u32),
                        source: "semantic_scholar".to_string(),
                        venue: p["venue"].as_str().map(|s| s.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        info!(count = papers.len(), "Semantic Scholar results");
        Ok(papers)
    }
}

/// Helper to extract href from an XML link element
fn extract_href(line: &str) -> Option<String> {
    if let Some(start) = line.find("href=\"") {
        let rest = &line[start + 6..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
