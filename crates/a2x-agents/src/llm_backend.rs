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

// ── Chat / streaming types ────────────────────────────────────────────────

/// A chunk of streaming output from an LLM.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatChunk {
    /// A delta of text content.
    Text(String),
    /// A tool call request (function calling) — incremental.
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// Tool call arguments are complete.
    ToolCallDone {
        id: String,
        name: String,
        arguments: String,
    },
    /// Streaming is finished.
    Done {
        total_tokens: Option<u64>,
        finish_reason: String,
    },
}

/// A single message in a chat conversation.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        ChatMessage {
            role: ChatRole::System,
            content: content.into(),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        ChatMessage {
            role: ChatRole::User,
            content: content.into(),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        ChatMessage {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        ChatMessage {
            role: ChatRole::Tool,
            content: content.into(),
            tool_calls: vec![],
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ChatRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
        }
    }
}

/// A tool call from the assistant.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// A tool definition passed to the LLM.
#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDef {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        ToolDef {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

// ── Trait ──────────────────────────────────────────────────────────────────

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
    fn complete<'a>(
        &'a self,
        system_prompt: &'a str,
        user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>>;

    /// Stream a chat completion with full conversation history and optional tools.
    ///
    /// Each chunk is sent through `on_chunk`. The returned future resolves when
    /// streaming is complete.
    fn chat_stream<'a>(
        &'a self,
        messages: &'a [ChatMessage],
        tools: &'a [ToolDef],
        on_chunk: &'a (dyn Fn(ChatChunk) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + 'a>>;
}

// ── No-op backend (for testing/probing) ────────────────────────────────────

/// No-op LLM backend for testing/probing — always returns empty results.
pub struct NoopBackend;

impl LlmBackend for NoopBackend {
    fn complete<'a>(
        &'a self,
        _system_prompt: &'a str,
        _user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move { Ok(String::new()) })
    }

    fn chat_stream<'a>(
        &'a self,
        _messages: &'a [ChatMessage],
        _tools: &'a [ToolDef],
        on_chunk: &'a (dyn Fn(ChatChunk) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + 'a>> {
        Box::pin(async move {
            on_chunk(ChatChunk::Text("(noop backend — no LLM configured)".into()));
            on_chunk(ChatChunk::Done {
                total_tokens: Some(0),
                finish_reason: "noop".into(),
            });
            Ok(())
        })
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
    /// Model name (e.g., "gpt-4o", "llama3", "codellama").
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

    /// Convenience constructor for Ollama (local GPU).
    pub fn ollama(model: impl Into<String>) -> Self {
        OpenAiBackend {
            config: OpenAiConfig {
                api_url: "http://localhost:11434/v1".into(),
                api_key: String::new(),
                model: model.into(),
                max_tokens: 4096,
                temperature: 0.2,
            },
            client: Client::new(),
        }
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the API URL.
    pub fn api_url(&self) -> &str {
        &self.config.api_url
    }
}

// ── Internal OpenAI API types ───────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiToolDef>>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Clone)]
struct OpenAiChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    ttype: String,
    function: OpenAiFunctionCall,
}

#[derive(Serialize, Clone)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize, Clone)]
struct OpenAiToolDef {
    #[serde(rename = "type")]
    ttype: String,
    function: OpenAiFunctionDef,
}

#[derive(Serialize, Clone)]
struct OpenAiFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
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
#[allow(dead_code)]
struct ChatMessageContent {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCallResponse>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiToolCallResponse {
    id: String,
    #[serde(rename = "type")]
    ttype: String,
    function: FunctionCallResponse,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FunctionCallResponse {
    name: String,
    arguments: String,
}

/// SSE streaming delta.
#[derive(Deserialize)]
struct StreamDelta {
    choices: Vec<StreamChoice>,
    usage: Option<StreamUsage>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDeltaContent,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDeltaContent {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<StreamToolCallDelta>,
}

#[derive(Deserialize)]
struct StreamToolCallDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    function: Option<StreamFunctionDelta>,
}

#[derive(Deserialize)]
struct StreamFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct StreamUsage {
    total_tokens: Option<u64>,
}

// ── LlmBackend impl for OpenAiBackend ──────────────────────────────────────

impl LlmBackend for OpenAiBackend {
    fn complete<'a>(
        &'a self,
        system_prompt: &'a str,
        user_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let request = OpenAiChatRequest {
                model: self.config.model.clone(),
                messages: vec![
                    OpenAiChatMessage {
                        role: "system".into(),
                        content: Some(system_prompt.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    OpenAiChatMessage {
                        role: "user".into(),
                        content: Some(user_prompt.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                tools: None,
                max_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
                stream: false,
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

    fn chat_stream<'a>(
        &'a self,
        messages: &'a [ChatMessage],
        tools: &'a [ToolDef],
        on_chunk: &'a (dyn Fn(ChatChunk) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + 'a>> {
        Box::pin(async move {
            // Build messages
            let api_messages: Vec<OpenAiChatMessage> = messages
                .iter()
                .map(|m| {
                    let tool_calls = if m.tool_calls.is_empty() {
                        None
                    } else {
                        Some(
                            m.tool_calls
                                .iter()
                                .map(|tc| OpenAiToolCall {
                                    id: tc.id.clone(),
                                    ttype: "function".into(),
                                    function: OpenAiFunctionCall {
                                        name: tc.name.clone(),
                                        arguments: tc.arguments.clone(),
                                    },
                                })
                                .collect(),
                        )
                    };
                    OpenAiChatMessage {
                        role: m.role.as_str().into(),
                        content: if m.content.is_empty() {
                            None
                        } else {
                            Some(m.content.clone())
                        },
                        tool_calls,
                        tool_call_id: m.tool_call_id.clone(),
                    }
                })
                .collect();

            // Build tools
            let api_tools: Vec<OpenAiToolDef> = if tools.is_empty() {
                vec![]
            } else {
                tools
                    .iter()
                    .map(|t| OpenAiToolDef {
                        ttype: "function".into(),
                        function: OpenAiFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect()
            };

            let request = OpenAiChatRequest {
                model: self.config.model.clone(),
                messages: api_messages,
                tools: if api_tools.is_empty() {
                    None
                } else {
                    Some(api_tools)
                },
                max_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
                stream: true,
            };

            let url = format!(
                "{}/chat/completions",
                self.config.api_url.trim_end_matches('/')
            );

            let mut req = self.client.post(&url).json(&request);

            if !self.config.api_key.is_empty() {
                req = req.bearer_auth(&self.config.api_key);
            }

            let response = req.send().await.map_err(|e| {
                AgentError::TransportError(format!("LLM stream request failed: {}", e))
            })?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AgentError::TransportError(format!(
                    "LLM API error {}: {}",
                    status, body
                )));
            }

            // Process SSE stream
            use futures_util::StreamExt;
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut current_tool_calls: Vec<(usize, String, String, String)> = vec![]; // (index, id, name, args)
            let mut total_tokens: Option<u64> = None;
            let mut finish_reason = "stop".to_string();

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result
                    .map_err(|e| AgentError::TransportError(format!("stream read error: {}", e)))?;

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete SSE lines
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    if line == "data: [DONE]" {
                        break;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        match serde_json::from_str::<StreamDelta>(data) {
                            Ok(delta) => {
                                for choice in &delta.choices {
                                    // Text content
                                    if let Some(ref content) = choice.delta.content {
                                        if !content.is_empty() {
                                            on_chunk(ChatChunk::Text(content.clone()));
                                        }
                                    }

                                    // Tool calls
                                    for tc in &choice.delta.tool_calls {
                                        // Ensure we have an entry for this tool call index
                                        while current_tool_calls.len() <= tc.index {
                                            current_tool_calls.push((
                                                current_tool_calls.len(),
                                                String::new(),
                                                String::new(),
                                                String::new(),
                                            ));
                                        }

                                        let entry = &mut current_tool_calls[tc.index];

                                        if let Some(ref id) = tc.id {
                                            entry.1 = id.clone();
                                        }
                                        if let Some(ref func) = tc.function {
                                            if let Some(ref name) = func.name {
                                                entry.2 = name.clone();
                                            }
                                            if let Some(ref args) = func.arguments {
                                                entry.3.push_str(args);
                                                on_chunk(ChatChunk::ToolCall {
                                                    id: entry.1.clone(),
                                                    name: entry.2.clone(),
                                                    arguments: entry.3.clone(),
                                                });
                                            }
                                        }
                                    }

                                    if let Some(ref reason) = choice.finish_reason {
                                        finish_reason = reason.clone();
                                    }
                                }

                                // Usage
                                if let Some(ref usage) = delta.usage {
                                    total_tokens = usage.total_tokens;
                                }
                            }
                            Err(_) => {
                                // Some providers send non-JSON data lines (e.g., Ollama healthchecks)
                                // Ignore parse errors for non-critical data
                            }
                        }
                    }
                }
            }

            // Emit ToolCallDone for each completed tool call
            for (_idx, id, name, args) in &current_tool_calls {
                if !id.is_empty() && !name.is_empty() {
                    on_chunk(ChatChunk::ToolCallDone {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args.clone(),
                    });
                }
            }

            on_chunk(ChatChunk::Done {
                total_tokens,
                finish_reason,
            });

            Ok(())
        })
    }
}

// ── Σ∞ program generation prompt ───────────────────────────────────────────

/// System prompt that instructs the LLM to generate Σ∞ programs.
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
    fn test_ollama_constructor() {
        let backend = OpenAiBackend::ollama("codellama:7b");
        assert_eq!(backend.model(), "codellama:7b");
        assert_eq!(backend.api_url(), "http://localhost:11434/v1");
    }

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("you are helpful");
        assert_eq!(sys.role, ChatRole::System);
        assert_eq!(sys.content, "you are helpful");

        let usr = ChatMessage::user("hello");
        assert_eq!(usr.role, ChatRole::User);

        let ast = ChatMessage::assistant("hi there");
        assert_eq!(ast.role, ChatRole::Assistant);

        let tool = ChatMessage::tool("call_1", "result");
        assert_eq!(tool.role, ChatRole::Tool);
        assert_eq!(tool.tool_call_id, Some("call_1".into()));
    }

    #[test]
    fn test_tool_def_constructor() {
        let td = ToolDef::new(
            "test_tool",
            "a test tool",
            serde_json::json!({"type": "object", "properties": {}}),
        );
        assert_eq!(td.name, "test_tool");
        assert_eq!(td.description, "a test tool");
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

    #[test]
    fn test_noop_backend_chat_stream() {
        use std::sync::{Arc, Mutex};
        let rt = tokio::runtime::Runtime::new().unwrap();
        let backend = NoopBackend;
        rt.block_on(async {
            let chunks = Arc::new(Mutex::new(vec![]));
            let c = Arc::clone(&chunks);
            backend
                .chat_stream(&[ChatMessage::user("hello")], &[], &move |chunk| {
                    c.lock().unwrap().push(chunk);
                })
                .await
                .unwrap();
            let chunks = chunks.lock().unwrap();
            assert!(!chunks.is_empty());
            assert!(matches!(chunks.last(), Some(ChatChunk::Done { .. })));
        });
    }
}
