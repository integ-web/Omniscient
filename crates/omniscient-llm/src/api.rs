//! API-based LLM providers — OpenAI, Anthropic, Ollama

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use omniscient_core::error::{OmniscientError, Result};
use omniscient_core::types::{Message, Role};

use crate::provider::*;

/// Helper to convert reqwest errors
fn web_err(e: reqwest::Error) -> OmniscientError {
    OmniscientError::Web(e.to_string())
}

// ──────────────────────────────────────────────
// OpenAI Provider
// ──────────────────────────────────────────────

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            model,
        }
    }

    fn build_messages(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    "content": m.content,
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }
    fn model(&self) -> &str { &self.model }

    async fn is_available(&self) -> bool {
        self.client
            .get(&format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": self.build_messages(&request.messages),
            "temperature": request.temperature.unwrap_or(0.7),
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if !request.stop_sequences.is_empty() {
            body["stop"] = serde_json::json!(request.stop_sequences);
        }

        if let Some(ref tools) = request.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tool_defs);
        }

        debug!("OpenAI request to {}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OmniscientError::Inference(format!(
                "OpenAI API error {}: {}", status, text
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;
        let choice = &data["choices"][0];

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = if let Some(calls) = choice["message"]["tool_calls"].as_array() {
            calls
                .iter()
                .map(|tc| ToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: serde_json::from_str(
                        tc["function"]["arguments"].as_str().unwrap_or("{}"),
                    )
                    .unwrap_or(serde_json::Value::Null),
                })
                .collect()
        } else {
            Vec::new()
        };

        let usage = TokenUsage {
            prompt_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: data["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize,
        };

        let finish_reason = match choice["finish_reason"].as_str() {
            Some("stop") => FinishReason::Stop,
            Some("tool_calls") => FinishReason::ToolCall,
            Some("length") => FinishReason::Length,
            _ => FinishReason::Stop,
        };

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            tool_calls,
            usage,
            finish_reason,
        })
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            is_local: false,
            vram_requirement_mb: 0,
            context_window: match self.model.as_str() {
                m if m.contains("gpt-4") => 128_000,
                m if m.contains("gpt-3.5") => 16_385,
                _ => 8_192,
            },
        }
    }
}

// ──────────────────────────────────────────────
// Anthropic Provider
// ──────────────────────────────────────────────

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }
    fn model(&self) -> &str { &self.model }

    async fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
        });

        if !system_msg.is_empty() {
            body["system"] = serde_json::json!(system_msg);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2024-10-22")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OmniscientError::Inference(format!(
                "Anthropic API error {}: {}", status, text
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;

        let content = data["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: 0,
        };

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            tool_calls: Vec::new(),
            usage,
            finish_reason: FinishReason::Stop,
        })
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            is_local: false,
            vram_requirement_mb: 0,
            context_window: 200_000,
        }
    }
}

// ──────────────────────────────────────────────
// Ollama Provider (local)
// ──────────────────────────────────────────────

pub struct OllamaProvider {
    client: Client,
    host: String,
    port: u16,
    model: String,
}

impl OllamaProvider {
    pub fn new(host: String, port: u16, model: String) -> Self {
        Self {
            client: Client::new(),
            host,
            port,
            model,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str { "ollama" }
    fn model(&self) -> &str { &self.model }

    async fn is_available(&self) -> bool {
        self.client
            .get(&format!("{}/api/tags", self.base_url()))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Ollama chat API
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "user",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "num_predict": request.max_tokens.unwrap_or(4096),
            }
        });

        info!("Sending request to Ollama at {}", self.base_url());

        let response = self
            .client
            .post(&format!("{}/api/chat", self.base_url()))
            .json(&body)
            .send()
            .await
            .map_err(web_err)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OmniscientError::Inference(format!(
                "Ollama error {}: {}", status, text
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(web_err)?;

        let content = data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: data["prompt_eval_count"].as_u64().unwrap_or(0) as usize,
            completion_tokens: data["eval_count"].as_u64().unwrap_or(0) as usize,
            total_tokens: 0,
        };

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            tool_calls: Vec::new(),
            usage,
            finish_reason: FinishReason::Stop,
        })
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            is_local: true,
            // Assuming default local execution requires ~4096 MB VRAM footprint minimum for 7B models.
            vram_requirement_mb: 4096,
            context_window: 8192,
        }
    }
}
