//! # Omniscient LLM Layer
//!
//! Hybrid inference — local models (Candle/BitNet) + API (OpenAI/Anthropic/Ollama).
//! Includes SLM-based task categorization for intelligent load routing.

pub mod api;
pub mod categorizer;
pub mod provider;
pub mod router;

pub use categorizer::SlmCategorizer;
pub use provider::{LlmProvider, LlmRequest, LlmResponse};
pub use router::ModelRouter;
