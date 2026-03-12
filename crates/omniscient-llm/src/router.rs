//! Model Router — selects the best LLM backend based on task, resources, and strategy

use std::sync::Arc;
use tracing::{info, warn};

use omniscient_core::error::{OmniscientError, Result};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse};

/// Routing strategy for model selection
#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Always use the first available model
    FirstAvailable,
    /// Use the fastest model
    Fastest,
    /// Use the cheapest model (local > API)
    Cheapest,
    /// Use the best quality model
    BestQuality,
    /// Automatic: use SLM for simple tasks, powerful model for complex ones
    Auto,
}

/// Routes LLM requests to the best available backend
pub struct ModelRouter {
    providers: Vec<Arc<dyn LlmProvider>>,
    strategy: RoutingStrategy,
}

impl ModelRouter {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            providers: Vec::new(),
            strategy,
        }
    }

    /// Add a provider to the router
    pub fn add_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// Complete a request using the best available provider
    pub async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let provider = self.select_provider(request).await?;
        info!(provider = provider.name(), model = provider.model(), "Routing to provider");
        provider.complete(request).await
    }

    /// Select the best provider based on strategy
    async fn select_provider(&self, request: &LlmRequest) -> Result<Arc<dyn LlmProvider>> {
        if self.providers.is_empty() {
            return Err(OmniscientError::ModelNotAvailable(
                "No LLM providers configured".to_string(),
            ));
        }

        match self.strategy {
            RoutingStrategy::FirstAvailable => {
                for provider in &self.providers {
                    if provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
            RoutingStrategy::Cheapest => {
                // Prefer local providers (Ollama) over API providers
                for provider in &self.providers {
                    if provider.name() == "ollama" && provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                // Fall back to first available
                for provider in &self.providers {
                    if provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
            RoutingStrategy::BestQuality => {
                // Prefer API providers (Anthropic > OpenAI > Ollama)
                let priority = ["anthropic", "openai", "ollama"];
                for name in &priority {
                    for provider in &self.providers {
                        if provider.name() == *name && provider.is_available().await {
                            return Ok(provider.clone());
                        }
                    }
                }
                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
            RoutingStrategy::Auto => {
                // Estimate complexity from message length
                let total_tokens: usize = request
                    .messages
                    .iter()
                    .map(|m| m.content.len() / 4)
                    .sum();

                if total_tokens < 500 {
                    // Simple task → cheap local model
                    info!("Auto-routing: simple task → local model");
                    for provider in &self.providers {
                        if provider.name() == "ollama" && provider.is_available().await {
                            return Ok(provider.clone());
                        }
                    }
                }

                // Complex task → best quality
                info!("Auto-routing: complex task → best quality model");
                for provider in &self.providers {
                    if provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }

                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
            RoutingStrategy::Fastest => {
                // For now, same as FirstAvailable
                for provider in &self.providers {
                    if provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
        }
    }

    /// List all configured providers
    pub fn list_providers(&self) -> Vec<(&str, &str)> {
        self.providers
            .iter()
            .map(|p| (p.name(), p.model()))
            .collect()
    }
}
