//! # Omniscient Web Research Engine
//!
//! Crawling, content extraction, multi-engine search, and academic DB access.
//! Designed to be a Firecrawl-killer — faster, Rust-native, fully async.

pub mod crawler;
pub mod extractor;
pub mod search;
pub mod academic;
pub mod traits;

pub use crawler::WebCrawler;
pub use extractor::ContentExtractor;
pub use search::{SearchClient, SearchResult};
pub use traits::{FastScraper, StealthScraper, ChallengeSolver, HttpFastScraper};
