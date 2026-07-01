// See plans/05-agents.md §3 — LLM Agent
//
// T3-1: Trait-based LLM backend with OpenAI-compatible provider.
// The LlmAgent now uses a configurable backend to call real LLM APIs
// for natural language ↔ Σ∞ program translation.
//
// Uses Pin<Box<dyn Future>> for dyn-compatible async trait methods
// (avoids RPITIT compatibility issues with Arc<dyn LlmBackend>).

use std::future::Future;
use std::pin::Pin;

use a2x_core::error::AgentError;

/// Trait for LLM backends — any LLM API provider that can complete prompts.
///
/// Implementations:
/// - `OpenAiBackend` — OpenAI-compatible API (OpenAI, Ollama, vLLM, etc.)
/// - `NoopBackend` — no-op stub for testing/probing
/// - Future: `AnthropicBackend` — Anthropic Messages API
///
/// See plans/05-agents.md §3 — "LLM Agent: bridges natural language and A2X"
pub trait LlmBackend: Send + Sync {
    /// Send a completion request to the LLM.
    ///
    /// `system_prompt` sets the model's behavior (e.g., "You are a Σ∞ program
    /// generator..."). `user_prompt` is the specific request.
    ///
    /// Returns the model's text response.
    fn complete<'a>(
        &'a self,
        system_prompt: &'a str,
        user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>>;
}

// ── No-op backend (for testing/probing) ────────────────────────────────────

/// No-op LLM backend for testing/probing — always returns empty results.
/// Does not make any network calls.
pub struct NoopBackend;

impl LlmBackend for NoopBackend {
    fn complete<'a>(
        &'a self,
        _system_prompt: &'a str,
        _user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move { Ok(String::new()) })
    }
}

// ── OpenAI-compatible backend ──────────────────────────────────────────────

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Configuration for an OpenAI-compatible API endpoint.
#[derive(Clone, Debug)]
pub struct OpenAiConfig {
    /// API base URL (e.g., "https://api.openai.com/v1" or "http://localhost:11434/v1").
    pub api_url: String,
    /// API key (sent as Bearer token). Empty for local/Ollama.
    pub api_key: String,
    /// Model name (e.g., "gpt-4o", "claude-3-5-sonnet-20241022", "llama3").
    pub model: String,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        OpenAiConfig {
            api_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            model: "gpt-4o".into(),
            max_tokens: 4096,
            temperature: 0.2,
        }
    }
}

/// OpenAI-compatible chat completion backend.
///
/// Works with:
/// - OpenAI API (api.openai.com)
/// - Ollama (localhost:11434)
/// - vLLM (any self-hosted endpoint)
/// - Any OpenAI-compatible proxy (LiteLLM, OpenRouter, etc.)
pub struct OpenAiBackend {
    config: OpenAiConfig,
    client: Client,
}

impl OpenAiBackend {
    /// Create a new OpenAI-compatible backend with the given configuration.
    pub fn new(config: OpenAiConfig) -> Self {
        OpenAiBackend {
            config,
            client: Client::new(),
        }
    }
}

// ── OpenAI API types ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: String,
}

// ── LlmBackend impl for OpenAiBackend ──────────────────────────────────────

impl LlmBackend for OpenAiBackend {
    fn complete<'a>(
        &'a self,
        system_prompt: &'a str,
        user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let request = ChatRequest {
                model: self.config.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".into(),
                        content: system_prompt.to_string(),
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: user_prompt.to_string(),
                    },
                ],
                max_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
            };

            let url = format!(
                "{}/chat/completions",
                self.config.api_url.trim_end_matches('/')
            );

            let mut req = self.client.post(&url).json(&request);

            if !self.config.api_key.is_empty() {
                req = req.bearer_auth(&self.config.api_key);
            }

            let response = req
                .send()
                .await
                .map_err(|e| AgentError::TransportError(format!("LLM request failed: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AgentError::TransportError(format!(
                    "LLM API error {}: {}",
                    status, body
                )));
            }

            let chat: ChatResponse = response.json().await.map_err(|e| {
                AgentError::TransportError(format!("Failed to parse LLM response: {}", e))
            })?;

            let content = chat
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .unwrap_or_default();

            Ok(content)
        })
    }
}

// ── Σ∞ program generation prompt ───────────────────────────────────────────

/// System prompt that instructs the LLM to generate Σ∞ programs.
///
/// This is the core prompt engineering for T3-1. It teaches the LLM the Σ∞
/// syntax and provides examples so it can generate valid programs from
/// natural language intent.
pub const SIGMA_GENERATION_PROMPT: &str = r#"You are a Σ∞ (Sigma Infinity) program generator.
Σ∞ is an AI-native programming language using Unicode operators for ultra-dense instruction encoding.

## Σ∞ Syntax
Each instruction has the format: ⟦Σ∞⟧⟬I:<intent> ∷ C:<context> ∷ P:<plan> ∷ D:<data>⟭

### Intent Operators (I: field)
- ⚡ Lightning — actuate/immediate action
- ✦ Star — ground/embed data from the world
- ✣ Synthesis — bind/merge concepts
- ✕ Cancel — halt/stop execution
- ⩫ Split — differentiate/split concepts
- ⟁ Contradiction — reflect/self-model
- ⧖ Delay — evolve/time-step
- ⥁ Parallel — plan/generate actions
- ⫴ Merge — merge results

### Context Labels (C: field)
Labels are written ⟨label⟩. Multiple labels separated by spaces.

### Plan Operators (P: field)
- ⥂ Sequential — execute in order
- ⤐ Branch — conditional jump
- ⤈ Descend — call sub-program
- ⤉ Ascend — return from sub-program
- ⥁ Swarm — parallel fork
- ⤑ Merge — join results

### Data (D: field)
- ⌵ Empty — no payload
- ⌮ GraphDelta, ⌳ Scalar, ⌱ Vector, ⌴ Payload

## Task
Convert the user's natural language request into a sequence of Σ∞ instructions.
Output ONLY the Σ∞ program text, one instruction per line. No explanations, no markdown.
If the request is simple, output a single instruction. If complex, output a multi-instruction program.

Example: "stop" → ⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭
Example: "reflect on what happened" → ⟦Σ∞⟧⟬I:⟁ ∷ P:⥂⟭
Example: "merge concepts A and B" → ⟦Σ∞⟧⟬I:✣ ∷ C:⟨A⟩⟨B⟩ ∷ P:⥂⟭
"#;

/// System prompt that instructs the LLM to explain Σ∞ program results.
pub const SIGMA_EXPLANATION_PROMPT: &str = r#"You are a Σ∞ (Sigma Infinity) program analyst.
Given a Σ∞ program execution result, explain what happened in plain natural language.
Be concise — aim for 1-3 sentences. Focus on what actions were taken and what the result means.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_config_default() {
        let config = OpenAiConfig::default();
        assert_eq!(config.api_url, "https://api.openai.com/v1");
        assert_eq!(config.model, "gpt-4o");
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_openai_config_custom() {
        let config = OpenAiConfig {
            api_url: "http://localhost:11434/v1".into(),
            api_key: String::new(),
            model: "llama3".into(),
            max_tokens: 2048,
            temperature: 0.0,
        };
        assert_eq!(config.model, "llama3");
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_sigma_prompt_contains_operators() {
        assert!(SIGMA_GENERATION_PROMPT.contains("⟦Σ∞⟧"));
        assert!(SIGMA_GENERATION_PROMPT.contains("Lightning"));
        assert!(SIGMA_GENERATION_PROMPT.contains("Synthesis"));
    }

    #[test]
    fn test_explanation_prompt_is_concise() {
        assert!(SIGMA_EXPLANATION_PROMPT.len() < 500);
    }

    #[test]
    fn test_noop_backend_returns_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let backend = NoopBackend;
        rt.block_on(async {
            let result = backend.complete("sys", "user").await.unwrap();
            assert!(result.is_empty());
        });
    }
}
