use async_trait::async_trait;
use reqwest::Client;

use omniscient_core::error::{Result, OmniscientError};

/// Provides high-speed HTTP scraping to bypass basic WAFs
#[async_trait]
pub trait FastScraper: Send + Sync {
    async fn fetch_page(&self, url: &str) -> Result<String>;
}

/// Provides headless browser capability for JS-heavy sites
#[async_trait]
pub trait StealthScraper: Send + Sync {
    async fn fetch_dynamic_page(&self, url: &str) -> Result<String>;
}

/// Provides skills to bypass captchas, paywalls, and logins
#[async_trait]
pub trait ChallengeSolver: Send + Sync {
    async fn solve_challenge(&self, page_content: &str) -> Result<String>;
}

/// High-speed scraper using reqwest
pub struct HttpFastScraper {
    client: Client,
}

impl HttpFastScraper {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

#[async_trait]
impl FastScraper for HttpFastScraper {
    async fn fetch_page(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await.map_err(|e| OmniscientError::Web(e.to_string()))?;
        response.text().await.map_err(|e| OmniscientError::Web(e.to_string()))
    }
}
