//! Model Router — selects the best LLM backend based on task, resources, and strategy

use std::sync::Arc;
use tracing::{info, warn};
use tiktoken_rs::cl100k_base;

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
                // Prefer local providers over API providers using proper capability flags
                for provider in &self.providers {
                    if provider.capabilities().is_local && provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                // Fall back to first available API
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
                // Prefer API providers (large context windows generally mean APIs)
                for provider in &self.providers {
                    if !provider.capabilities().is_local && provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }
                Err(OmniscientError::ModelNotAvailable(
                    "No providers available".to_string(),
                ))
            }
            RoutingStrategy::Auto => {
                let bpe = cl100k_base().unwrap();
                let mut total_tokens = 0;
                for m in &request.messages {
                    total_tokens += bpe.encode_with_special_tokens(&m.content).len();
                }

                // Mock Hardware Probe logic: strict 2048 MB VRAM reservation limit.
                let system_vram_limit = 2048;
                info!("Measured Context window requires {} tokens.", total_tokens);

                if total_tokens < 4096 {
                    info!("Auto-routing: Within hardware fence constraint → checking provider VRAM requirements");
                    for provider in &self.providers {
                        let caps = provider.capabilities();
                        if caps.is_local && caps.vram_requirement_mb <= system_vram_limit && provider.is_available().await {
                            return Ok(provider.clone());
                        }
                    }
                }

                info!("Auto-routing: Tokens exceed VRAM hardware threshold fence or no local model fits constraint. Delegating to API.");
                for provider in &self.providers {
                    if !provider.capabilities().is_local && provider.is_available().await {
                        return Ok(provider.clone());
                    }
                }

                // Fallback
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
