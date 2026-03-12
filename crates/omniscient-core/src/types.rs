//! Shared types used across the Omniscient system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for any entity in the system
pub type EntityId = Uuid;

/// A document extracted from the web or any source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: EntityId,
    pub url: Option<String>,
    pub title: String,
    pub content: String,
    pub content_type: ContentType,
    pub metadata: DocumentMetadata,
    pub extracted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub author: Option<String>,
    pub published_date: Option<DateTime<Utc>>,
    pub source: String,
    pub word_count: usize,
    pub language: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    WebPage,
    AcademicPaper,
    NewsArticle,
    BlogPost,
    Documentation,
    SocialMedia,
    ForumPost,
    PDF,
    Unknown,
}

/// A research finding — an atomic piece of knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: EntityId,
    pub content: String,
    pub confidence: f64,
    pub source: Source,
    pub category: FindingCategory,
    pub entities: Vec<Entity>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: Option<String>,
    pub title: String,
    pub reliability_score: f64,
    pub access_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindingCategory {
    Fact,
    Claim,
    Opinion,
    Statistic,
    Quote,
    Definition,
    Relationship,
    Event,
    Contradiction,
}

/// An extracted entity (person, company, location, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Person,
    Company,
    Organization,
    Location,
    Product,
    Technology,
    Event,
    Date,
    Money,
    Concept,
}

/// A research report — the final output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    pub id: EntityId,
    pub title: String,
    pub query: String,
    pub executive_summary: String,
    pub sections: Vec<ReportSection>,
    pub sources: Vec<Source>,
    pub findings: Vec<Finding>,
    pub entities: Vec<Entity>,
    pub generated_at: DateTime<Utc>,
    pub research_depth: ResearchDepth,
    pub total_sources_consulted: usize,
    pub total_pages_crawled: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
    pub citations: Vec<usize>, // indices into sources
    pub subsections: Vec<ReportSection>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResearchDepth {
    Quick,      // 1-2 sources, ~30 seconds
    Standard,   // 5-10 sources, ~2 minutes
    Deep,       // 20-50 sources, ~10 minutes
    Exhaustive, // 100+ sources, ~30+ minutes
}

/// Message in a conversation with the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// SLM categorization result — used to route tasks efficiently
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResult {
    pub category: TaskCategory,
    pub confidence: f64,
    pub suggested_depth: ResearchDepth,
    pub suggested_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskCategory {
    CompanyResearch,
    PersonResearch,
    TechnologyAnalysis,
    AcademicLiterature,
    MarketAnalysis,
    CompetitiveIntelligence,
    ReverseEngineering,
    GeneralKnowledge,
    CurrentEvents,
    FactChecking,
}
