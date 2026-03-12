//! SLM Categorizer — uses a Small Language Model to classify tasks
//! before routing to expensive models. Dramatically reduces load.

use omniscient_core::error::Result;
use omniscient_core::types::{CategoryResult, ResearchDepth, TaskCategory};

use crate::provider::{LlmProvider, LlmRequest};
use omniscient_core::types::{Message, Role};

/// SLM-based task categorizer — routes tasks to the right pipeline
/// before involving expensive LLMs
pub struct SlmCategorizer {
    provider: Box<dyn LlmProvider>,
}

impl SlmCategorizer {
    pub fn new(provider: Box<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// Categorize a research query using the SLM
    pub async fn categorize(&self, query: &str) -> Result<CategoryResult> {
        let system_prompt = r#"You are a research task categorizer. Given a query, classify it into exactly one category and suggest a research depth.

Categories:
- CompanyResearch: queries about companies, businesses, startups
- PersonResearch: queries about specific people, their work, career
- TechnologyAnalysis: queries about tech, frameworks, tools, software
- AcademicLiterature: queries about research papers, scientific topics
- MarketAnalysis: queries about markets, industries, trends
- CompetitiveIntelligence: comparing companies/products
- ReverseEngineering: understanding how something works internally
- GeneralKnowledge: general factual questions
- CurrentEvents: recent news, current happenings
- FactChecking: verifying claims or statements

Research Depths:
- Quick: simple factual lookup (1-2 sources)
- Standard: moderate research (5-10 sources)
- Deep: thorough investigation (20-50 sources)
- Exhaustive: comprehensive analysis (100+ sources)

Reply in JSON format ONLY:
{"category": "<category>", "confidence": <0.0-1.0>, "depth": "<depth>", "tools": ["<tool1>", "<tool2>"]}"#;

        let messages = vec![
            Message {
                role: Role::System,
                content: system_prompt.to_string(),
                timestamp: chrono::Utc::now(),
            },
            Message {
                role: Role::User,
                content: format!("Categorize this research query: {}", query),
                timestamp: chrono::Utc::now(),
            },
        ];

        let request = LlmRequest {
            messages,
            max_tokens: Some(200),
            temperature: Some(0.1), // Low temp for classification
            stop_sequences: Vec::new(),
            tools: None,
            stream: false,
        };

        let response = self.provider.complete(&request).await?;

        // Parse the JSON response
        Self::parse_category_response(&response.content, query)
    }

    fn parse_category_response(response: &str, _query: &str) -> Result<CategoryResult> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            let category = match parsed["category"].as_str().unwrap_or("GeneralKnowledge") {
                "CompanyResearch" => TaskCategory::CompanyResearch,
                "PersonResearch" => TaskCategory::PersonResearch,
                "TechnologyAnalysis" => TaskCategory::TechnologyAnalysis,
                "AcademicLiterature" => TaskCategory::AcademicLiterature,
                "MarketAnalysis" => TaskCategory::MarketAnalysis,
                "CompetitiveIntelligence" => TaskCategory::CompetitiveIntelligence,
                "ReverseEngineering" => TaskCategory::ReverseEngineering,
                "CurrentEvents" => TaskCategory::CurrentEvents,
                "FactChecking" => TaskCategory::FactChecking,
                _ => TaskCategory::GeneralKnowledge,
            };

            let depth = match parsed["depth"].as_str().unwrap_or("Standard") {
                "Quick" => ResearchDepth::Quick,
                "Standard" => ResearchDepth::Standard,
                "Deep" => ResearchDepth::Deep,
                "Exhaustive" => ResearchDepth::Exhaustive,
                _ => ResearchDepth::Standard,
            };

            let tools = parsed["tools"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(CategoryResult {
                category,
                confidence: parsed["confidence"].as_f64().unwrap_or(0.7),
                suggested_depth: depth,
                suggested_tools: tools,
            })
        } else {
            // Fallback: keyword-based categorization
            Ok(Self::keyword_categorize(_query))
        }
    }

    /// Fallback keyword-based categorization (when SLM is unavailable)
    fn keyword_categorize(query: &str) -> CategoryResult {
        let q = query.to_lowercase();

        let category = if q.contains("company")
            || q.contains("startup")
            || q.contains("business")
            || q.contains("inc")
            || q.contains("corp")
        {
            TaskCategory::CompanyResearch
        } else if q.contains("person")
            || q.contains("who is")
            || q.contains("biography")
            || q.contains("career")
        {
            TaskCategory::PersonResearch
        } else if q.contains("technology")
            || q.contains("framework")
            || q.contains("programming")
            || q.contains("software")
        {
            TaskCategory::TechnologyAnalysis
        } else if q.contains("paper")
            || q.contains("research")
            || q.contains("study")
            || q.contains("academic")
        {
            TaskCategory::AcademicLiterature
        } else if q.contains("market")
            || q.contains("industry")
            || q.contains("trend")
        {
            TaskCategory::MarketAnalysis
        } else if q.contains("compare")
            || q.contains("vs")
            || q.contains("versus")
            || q.contains("competitor")
        {
            TaskCategory::CompetitiveIntelligence
        } else if q.contains("reverse engineer")
            || q.contains("how does")
            || q.contains("architecture")
            || q.contains("teardown")
        {
            TaskCategory::ReverseEngineering
        } else {
            TaskCategory::GeneralKnowledge
        };

        CategoryResult {
            category,
            confidence: 0.6,
            suggested_depth: ResearchDepth::Standard,
            suggested_tools: vec!["web_search".to_string(), "web_crawl".to_string()],
        }
    }
}
