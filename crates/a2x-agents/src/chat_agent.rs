// ChatAgent — conversational A2X coding agent with full subsystem access
//
// The ChatAgent orchestrates ALL A2X subsystems through natural language.
// It uses an LlmBackend (local GPU via Ollama, or cloud OpenAI) to understand
// user intent, calls tools to execute A2X operations, and returns results
// conversationally.
//
// Architecture:
//   User message → ChatAgent.chat() → LLM (with tools)
//     → LLM returns text → stream to user
//     → LLM returns tool call → execute_tool() → feed result back to LLM
//   → Repeat until LLM returns text-only response
//
// Context management: before each LLM call, prune_history() applies a sliding
// window — system messages always preserved, recent messages kept, old messages
// dropped to stay within max_context_tokens. Dropped messages are mined for
// patterns (files, tools, topics) and persisted in ContextMemory for future turns.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use a2x_bus::Bus;
use a2x_ccs::CcsVm;
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;

use crate::chat_prompt::A2X_CHAT_SYSTEM_PROMPT;
use crate::chat_tools::{all_tool_defs, execute_tool, ToolContext};
use crate::cli_agent::CliAgent;
use crate::context_memory::{
    extract_message_patterns, ContextMemory, CHARS_PER_TOKEN, CONTEXT_SAFETY_MARGIN,
    DEFAULT_CONTEXT_TOKENS, MEMORY_TOKEN_BUDGET, PRESERVED_SYSTEM_MSGS,
};
use crate::llm_backend::{ChatChunk, ChatMessage, LlmBackend, ToolCall};

/// Maximum tool-calling iterations before forcing a text response.
const MAX_TOOL_ROUNDS: usize = 5;

// ── ChatAgent ─────────────────────────────────────────────────────────────

/// The ChatAgent — a conversational A2X coding agent.
///
/// Connects an LLM backend to all A2X subsystems (Sigma, Bus, Probe, CLI, CCS, Omega)
/// through a tool-use loop. Designed to run locally on GPU via Ollama or in the cloud
/// via OpenAI-compatible APIs.
pub struct ChatAgent {
    /// Agent identity.
    id: AgentId,
    /// LLM backend (OpenAI, Ollama, etc.).
    backend: Arc<dyn LlmBackend>,
    /// The system prompt that defines the agent's behavior.
    #[allow(dead_code)]
    system_prompt: String,
    /// Conversation history (messages sent to the LLM).
    history: Mutex<Vec<ChatMessage>>,
    /// Tool execution context (bus, CLI agent, CCS VM).
    tool_ctx: Mutex<ToolContext>,
    /// Maximum context window tokens for history pruning.
    max_context_tokens: std::sync::atomic::AtomicU32,
    /// Persistent context memory (files, tools, topics, decisions).
    memory: Mutex<ContextMemory>,
    /// Statistics.
    total_messages: std::sync::atomic::AtomicU64,
    total_tool_calls: std::sync::atomic::AtomicU64,
}

impl ChatAgent {
    /// Create a new ChatAgent with the given backend and subsystems.
    pub fn new(
        id: AgentId,
        backend: Arc<dyn LlmBackend>,
        bus: Arc<Mutex<Bus>>,
        cli_agent: Arc<CliAgent>,
        ccs_vm: Arc<Mutex<CcsVm>>,
    ) -> Self {
        let mut history = vec![ChatMessage::system(A2X_CHAT_SYSTEM_PROMPT)];
        let tool_desc = build_tool_description();
        history.push(ChatMessage::system(tool_desc));

        ChatAgent {
            id,
            backend,
            system_prompt: A2X_CHAT_SYSTEM_PROMPT.into(),
            history: Mutex::new(history),
            tool_ctx: Mutex::new(ToolContext::new(bus, cli_agent, ccs_vm)),
            max_context_tokens: std::sync::atomic::AtomicU32::new(DEFAULT_CONTEXT_TOKENS),
            memory: Mutex::new(ContextMemory::default()),
            total_messages: std::sync::atomic::AtomicU64::new(0),
            total_tool_calls: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Create a ChatAgent with a short system prompt (for smaller local models).
    pub fn new_compact(
        id: AgentId,
        backend: Arc<dyn LlmBackend>,
        bus: Arc<Mutex<Bus>>,
        cli_agent: Arc<CliAgent>,
        ccs_vm: Arc<Mutex<CcsVm>>,
    ) -> Self {
        use crate::chat_prompt::A2X_CHAT_SYSTEM_PROMPT_SHORT;
        let mut history = vec![ChatMessage::system(A2X_CHAT_SYSTEM_PROMPT_SHORT)];
        let tool_desc = build_tool_description();
        history.push(ChatMessage::system(tool_desc));

        ChatAgent {
            id,
            backend,
            system_prompt: A2X_CHAT_SYSTEM_PROMPT_SHORT.into(),
            history: Mutex::new(history),
            tool_ctx: Mutex::new(ToolContext::new(bus, cli_agent, ccs_vm)),
            max_context_tokens: std::sync::atomic::AtomicU32::new(DEFAULT_CONTEXT_TOKENS),
            memory: Mutex::new(ContextMemory::default()),
            total_messages: std::sync::atomic::AtomicU64::new(0),
            total_tool_calls: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Set the maximum context window size in tokens.
    pub fn set_max_context_tokens(&mut self, tokens: u32) {
        self.max_context_tokens
            .store(tokens, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the current max context window size.
    pub fn max_context_tokens(&self) -> u32 {
        self.max_context_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the full conversation history.
    pub fn history(&self) -> Vec<ChatMessage> {
        self.history
            .lock()
            .unwrap_or_else(|e| panic!("history lock poisoned: {}", e))
            .clone()
    }

    /// Clear conversation history (keep system prompt, preserve memory).
    pub fn reset_history(&self) {
        let mut hist = self
            .history
            .lock()
            .unwrap_or_else(|e| panic!("history lock poisoned: {}", e));
        hist.truncate(PRESERVED_SYSTEM_MSGS);
    }

    /// Get a snapshot of the accumulated context memory.
    pub fn memory_snapshot(&self) -> ContextMemory {
        self.memory
            .lock()
            .unwrap_or_else(|e| panic!("memory lock poisoned: {}", e))
            .clone()
    }

    /// Estimate token count for a string using the chars/4 heuristic.
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.chars().count() as f64 / CHARS_PER_TOKEN).ceil() as u32
    }

    /// Estimate token count for a ChatMessage (content + role + tool calls).
    pub fn estimate_msg_tokens(msg: &ChatMessage) -> u32 {
        let mut tokens = Self::estimate_tokens(&msg.content);
        for tc in &msg.tool_calls {
            tokens += Self::estimate_tokens(&tc.name);
            tokens += Self::estimate_tokens(&tc.arguments);
            tokens += 4;
        }
        tokens + 4
    }

    /// Get the estimated token usage of the current history.
    pub fn context_tokens_used(&self) -> u32 {
        let hist = self
            .history
            .lock()
            .unwrap_or_else(|e| panic!("history lock: {}", e));
        hist.iter().map(|m| Self::estimate_msg_tokens(m)).sum()
    }

    /// Get the current message count in the history.
    pub fn history_length(&self) -> usize {
        self.history.lock().map(|h| h.len()).unwrap_or(0)
    }

    /// Apply a sliding window to the conversation history with smart memory.
    ///
    /// * System messages (first PRESERVED_SYSTEM_MSGS) are always kept.
    /// * Messages that don't fit are mined for patterns into ContextMemory.
    /// * A dynamic memory summary is injected as context after system messages.
    /// * Recent messages are kept within the remaining budget.
    pub fn prune_history(&self, messages: &[ChatMessage]) -> Vec<ChatMessage> {
        let max_tokens = self.max_context_tokens();
        let budget = ((max_tokens as f64) * (1.0 - CONTEXT_SAFETY_MARGIN)) as u32;

        if messages.len() <= PRESERVED_SYSTEM_MSGS {
            return messages.to_vec();
        }

        // Always preserve static system messages
        let system_msgs: Vec<&ChatMessage> = messages[..PRESERVED_SYSTEM_MSGS].iter().collect();
        let system_tokens: u32 = system_msgs
            .iter()
            .map(|m| Self::estimate_msg_tokens(m))
            .sum();

        if system_tokens >= budget {
            return system_msgs.into_iter().cloned().collect();
        }

        // Build dynamic memory message
        let memory_msg = {
            let mem = self
                .memory
                .lock()
                .unwrap_or_else(|e| panic!("memory lock: {}", e));
            mem.to_system_message()
        };
        let memory_tokens = memory_msg
            .as_ref()
            .map(|m| Self::estimate_msg_tokens(m))
            .unwrap_or(0);
        let memory_budget = memory_tokens.min(MEMORY_TOKEN_BUDGET);

        let mut remaining = budget
            .saturating_sub(system_tokens)
            .saturating_sub(memory_budget);
        let mut result: Vec<ChatMessage> = system_msgs.into_iter().cloned().collect();

        // Inject memory after system messages
        if let Some(mem) = memory_msg {
            result.push(mem);
        }

        // Walk messages newest-first, include those that fit
        let conversation: Vec<&ChatMessage> = messages[PRESERVED_SYSTEM_MSGS..].iter().collect();
        let mut included: Vec<(usize, &ChatMessage)> = Vec::new();
        let mut dropped: Vec<(usize, &ChatMessage)> = Vec::new();

        for (idx, msg) in conversation.iter().enumerate().rev() {
            let tokens = Self::estimate_msg_tokens(msg);
            if tokens <= remaining {
                remaining = remaining.saturating_sub(tokens);
                included.push((idx, msg));
            } else {
                dropped.push((idx, msg));
            }
        }

        // Mine dropped messages for patterns into persistent memory
        if !dropped.is_empty() {
            if let Ok(mut mem) = self.memory.lock() {
                for (_, msg) in &dropped {
                    extract_message_patterns(msg, &mut mem);
                }
            }
            tracing::info!(
                dropped_count = dropped.len(),
                kept_count = included.len() + PRESERVED_SYSTEM_MSGS,
                "ChatAgent: mined dropped messages for context patterns"
            );
        }

        // Sort by original index to maintain chronological order
        included.sort_by_key(|(idx, _)| *idx);
        for (_, msg) in included {
            result.push((*msg).clone());
        }

        if result.len() < messages.len() {
            tracing::info!(
                pruned = messages.len() - result.len(),
                kept = result.len(),
                max_context = max_tokens,
                "ChatAgent: pruned conversation history"
            );
        }

        result
    }

    /// Send a user message and get the full response (blocking, non-streaming).
    pub fn chat(&self, user_message: &str) -> Result<String, AgentError> {
        self.total_messages
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let start = Instant::now();

        {
            let mut hist = self
                .history
                .lock()
                .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
            hist.push(ChatMessage::user(user_message));
        }

        let mut full_response = String::new();
        let mut rounds = 0;

        loop {
            rounds += 1;
            if rounds > MAX_TOOL_ROUNDS {
                full_response
                    .push_str("\n\n[Max tool rounds reached — please refine your request.]");
                break;
            }

            let tool_defs = all_tool_defs();
            let hist = self.history();
            let pruned = self.prune_history(&hist);

            let result = tokio::runtime::Handle::try_current()
                .map(|handle| {
                    let backend = Arc::clone(&self.backend);
                    let tools = tool_defs.clone();
                    handle.block_on(async move {
                        let text_mtx = Arc::new(Mutex::new(String::new()));
                        let tc_mtx = Arc::new(Mutex::new(Vec::<ToolCall>::new()));
                        {
                            let tm = Arc::clone(&text_mtx);
                            let tcm = Arc::clone(&tc_mtx);
                            backend
                                .chat_stream(&pruned, &tools, &move |chunk| match chunk {
                                    ChatChunk::Text(t) => {
                                        tm.lock().unwrap().push_str(&t);
                                    }
                                    ChatChunk::ToolCallDone {
                                        id,
                                        name,
                                        arguments,
                                    } => {
                                        tcm.lock().unwrap().push(ToolCall {
                                            id,
                                            name,
                                            arguments,
                                        });
                                    }
                                    _ => {}
                                })
                                .await?;
                        }
                        let text = text_mtx.lock().unwrap().clone();
                        let tool_calls = tc_mtx.lock().unwrap().clone();
                        Ok::<_, AgentError>((text, tool_calls))
                    })
                })
                .unwrap_or_else(|_| {
                    Err(AgentError::TransportError(
                        "ChatAgent::chat requires a tokio runtime".into(),
                    ))
                })?;

            let (text, tool_calls) = result;

            if !tool_calls.is_empty() {
                let mut assistant_msg = ChatMessage::assistant("");
                assistant_msg.tool_calls = tool_calls.clone();

                {
                    let mut hist = self
                        .history
                        .lock()
                        .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
                    hist.push(assistant_msg);

                    for tc in &tool_calls {
                        self.total_tool_calls
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.arguments).unwrap_or(serde_json::Value::Null);
                        {
                            let mut ctx = self.tool_ctx.lock().map_err(|e| {
                                AgentError::TransportError(format!("tool ctx lock: {}", e))
                            })?;
                            let result = execute_tool(&tc.name, &args, &tc.id, &mut ctx);
                            hist.push(ChatMessage::tool(tc.id.clone(), result.content));
                        }
                    }
                }
            } else if !text.is_empty() {
                full_response = text.clone();
                {
                    let mut hist = self
                        .history
                        .lock()
                        .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
                    hist.push(ChatMessage::assistant(text));
                }
                break;
            } else {
                full_response = "[No response from LLM]".into();
                break;
            }
        }

        let elapsed = start.elapsed();
        tracing::info!(
            agent = %self.id.as_str(),
            rounds = rounds,
            elapsed_ms = elapsed.as_millis(),
            "ChatAgent::chat completed"
        );

        Ok(full_response)
    }

    /// Send a user message and stream the response through a callback.
    pub async fn chat_streaming(
        &self,
        user_message: &str,
        on_chunk: &(dyn Fn(serde_json::Value) + Send + Sync),
    ) -> Result<(), AgentError> {
        self.total_messages
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        {
            let mut hist = self
                .history
                .lock()
                .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
            hist.push(ChatMessage::user(user_message));
        }

        let mut rounds = 0;
        let tool_defs = all_tool_defs();

        loop {
            rounds += 1;
            if rounds > MAX_TOOL_ROUNDS {
                on_chunk(serde_json::json!({
                    "type": "warning",
                    "content": "Max tool rounds reached."
                }));
                break;
            }

            let messages = self.prune_history(&self.history());
            let tools = tool_defs.clone();
            let backend = Arc::clone(&self.backend);

            let text_buf = Arc::new(Mutex::new(String::new()));
            let tool_calls_buf = Arc::new(Mutex::new(Vec::<ToolCall>::new()));

            {
                let tb = Arc::clone(&text_buf);
                let tcb = Arc::clone(&tool_calls_buf);

                backend
                    .chat_stream(&messages, &tools, &move |chunk| match chunk {
                        ChatChunk::Text(delta) => {
                            tb.lock().unwrap().push_str(&delta);
                            on_chunk(serde_json::json!({
                                "type": "text", "content": delta
                            }));
                        }
                        ChatChunk::ToolCall {
                            name, arguments, ..
                        } => {
                            on_chunk(serde_json::json!({
                                "type": "tool_call", "tool": name,
                                "args_preview": &arguments[..arguments.len().min(80)]
                            }));
                        }
                        ChatChunk::ToolCallDone {
                            id,
                            name,
                            arguments,
                        } => {
                            tcb.lock().unwrap().push(ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                arguments: arguments.clone(),
                            });
                            on_chunk(serde_json::json!({
                                "type": "tool_call_done", "tool": name, "id": id
                            }));
                        }
                        ChatChunk::Done {
                            total_tokens,
                            finish_reason,
                        } => {
                            on_chunk(serde_json::json!({
                                "type": "stream_done",
                                "total_tokens": total_tokens,
                                "finish_reason": finish_reason
                            }));
                        }
                    })
                    .await
                    .map_err(|e| AgentError::TransportError(format!("stream error: {}", e)))?;
            }

            let final_text = text_buf
                .lock()
                .map_err(|e| AgentError::TransportError(format!("text lock: {}", e)))?
                .clone();
            let tool_calls = tool_calls_buf
                .lock()
                .map_err(|e| AgentError::TransportError(format!("tool calls lock: {}", e)))?
                .clone();

            if !tool_calls.is_empty() {
                let mut assistant_msg = ChatMessage::assistant("");
                assistant_msg.tool_calls = tool_calls.clone();

                {
                    let mut hist = self
                        .history
                        .lock()
                        .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
                    hist.push(assistant_msg);

                    for tc in &tool_calls {
                        self.total_tool_calls
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.arguments).unwrap_or(serde_json::Value::Null);
                        {
                            let mut ctx = self.tool_ctx.lock().map_err(|e| {
                                AgentError::TransportError(format!("tool ctx lock: {}", e))
                            })?;
                            let result = execute_tool(&tc.name, &args, &tc.id, &mut ctx);

                            on_chunk(serde_json::json!({
                                "type": "tool_result",
                                "tool": tc.name, "id": tc.id,
                                "success": result.success,
                                "content_preview": &result.content[..result.content.len().min(200)]
                            }));

                            hist.push(ChatMessage::tool(tc.id.clone(), result.content));
                        }
                    }
                }
            } else if !final_text.is_empty() {
                {
                    let mut hist = self
                        .history
                        .lock()
                        .map_err(|e| AgentError::TransportError(format!("history lock: {}", e)))?;
                    hist.push(ChatMessage::assistant(&final_text));
                }
                on_chunk(serde_json::json!({"type": "done"}));
                break;
            } else {
                on_chunk(serde_json::json!({
                    "type": "error", "content": "No response from LLM"
                }));
                break;
            }
        }

        Ok(())
    }

    /// Save conversation history to a JSON file.
    pub fn save_conversation(&self, path: &std::path::Path) -> Result<(), String> {
        let hist = self.history();
        let mem = self.memory_snapshot();
        let data = serde_json::json!({
            "version": 1,
            "max_context_tokens": self.max_context_tokens(),
            "history": hist.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    crate::llm_backend::ChatRole::System => "system",
                    crate::llm_backend::ChatRole::User => "user",
                    crate::llm_backend::ChatRole::Assistant => "assistant",
                    crate::llm_backend::ChatRole::Tool => "tool",
                },
                "content": m.content,
                "tool_calls": m.tool_calls.iter().map(|tc| serde_json::json!({
                    "id": tc.id, "name": tc.name, "arguments": tc.arguments
                })).collect::<Vec<_>>(),
                "tool_call_id": m.tool_call_id,
            })).collect::<Vec<_>>(),
            "memory": {
                "accessed_files": mem.accessed_files.iter().collect::<Vec<_>>(),
                "tool_usage": mem.tool_usage,
                "topics": mem.topics,
                "working_dir": mem.working_dir,
                "messages_processed": mem.messages_processed,
            },
        });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        let json =
            serde_json::to_string_pretty(&data).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("Write error: {}", e))?;
        tracing::info!(path = %path.display(), "ChatAgent: conversation saved");
        Ok(())
    }

    /// Load conversation history from a JSON file, appending to existing history.
    pub fn load_conversation(&self, path: &std::path::Path) -> Result<usize, String> {
        let json_str = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
        let data: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| format!("Parse error: {}", e))?;

        let mut loaded = 0usize;
        if let Some(history) = data["history"].as_array() {
            for entry in history {
                let role_str = entry["role"].as_str().unwrap_or("user");
                let content = entry["content"].as_str().unwrap_or("");
                let msg = match role_str {
                    "system" => ChatMessage::system(content),
                    "user" => ChatMessage::user(content),
                    "assistant" => {
                        let mut msg = ChatMessage::assistant(content);
                        if let Some(tcs) = entry["tool_calls"].as_array() {
                            for tc_entry in tcs {
                                msg.tool_calls.push(ToolCall {
                                    id: tc_entry["id"].as_str().unwrap_or("").into(),
                                    name: tc_entry["name"].as_str().unwrap_or("").into(),
                                    arguments: tc_entry["arguments"]
                                        .as_str()
                                        .unwrap_or("{}")
                                        .into(),
                                });
                            }
                        }
                        msg
                    }
                    "tool" => {
                        let tc_id = entry["tool_call_id"].as_str().unwrap_or("").to_string();
                        let mut msg = ChatMessage::tool(tc_id.clone(), content);
                        msg.tool_call_id = Some(tc_id);
                        msg
                    }
                    _ => continue,
                };
                {
                    let mut hist = self.history.lock().map_err(|e| format!("lock: {}", e))?;
                    hist.push(msg);
                }
                loaded += 1;
            }
        }

        // Restore context memory
        if let Some(mem) = data.get("memory") {
            if let Ok(mut memory) = self.memory.lock() {
                if let Some(files) = mem["accessed_files"].as_array() {
                    for f in files {
                        if let Some(s) = f.as_str() {
                            memory.accessed_files.insert(s.to_string());
                        }
                    }
                }
                if let Some(tools) = mem["tool_usage"].as_object() {
                    for (k, v) in tools {
                        if let Some(c) = v.as_u64() {
                            memory.tool_usage.insert(k.clone(), c as usize);
                        }
                    }
                }
                if let Some(topics) = mem["topics"].as_array() {
                    for t in topics {
                        if let Some(s) = t.as_str() {
                            memory.topics.push(s.to_string());
                        }
                    }
                }
                if let Some(wd) = mem["working_dir"].as_str() {
                    memory.working_dir = Some(wd.to_string());
                }
                if let Some(mp) = mem["messages_processed"].as_u64() {
                    memory.messages_processed = mp;
                }
            }
        }

        // Restore context token setting
        if let Some(ctx) = data["max_context_tokens"].as_u64() {
            self.max_context_tokens
                .store(ctx as u32, std::sync::atomic::Ordering::Relaxed);
        }

        tracing::info!(path = %path.display(), loaded, "ChatAgent: conversation loaded");
        Ok(loaded)
    }

    /// Get call statistics.
    pub fn stats(&self) -> ChatStats {
        ChatStats {
            total_messages: self
                .total_messages
                .load(std::sync::atomic::Ordering::Relaxed),
            total_tool_calls: self
                .total_tool_calls
                .load(std::sync::atomic::Ordering::Relaxed),
            history_length: self.history.lock().map(|h| h.len()).unwrap_or(0),
        }
    }
}

/// Build a text description of all available tools (for ReAct fallback).
fn build_tool_description() -> String {
    let tools = all_tool_defs();
    let mut desc = String::from("## Available Tools\n\n");
    for tool in &tools {
        desc.push_str(&format!("- **{}**: {}\n", tool.name, tool.description));
    }
    desc.push_str("\nWhen you need to use a tool, write exactly:\n");
    desc.push_str("TOOL: tool_name {\"arg\": \"value\"}\n");
    desc.push_str("\nAfter tools complete, respond normally to the user.\n");
    desc
}

/// Chat agent usage statistics.
#[derive(Clone, Debug, Default)]
pub struct ChatStats {
    pub total_messages: u64,
    pub total_tool_calls: u64,
    pub history_length: usize,
}

impl Agent for ChatAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }
    fn agent_type(&self) -> AgentType {
        AgentType::Chat
    }

    fn execute(&self, program: Packet) -> Result<Packet, AgentError> {
        match &program {
            Packet::Raw(bytes) => {
                let text = String::from_utf8_lossy(bytes).to_string();
                let response = self.chat(&text)?;
                Ok(Packet::Raw(response.into_bytes()))
            }
        }
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let stats = self.stats();
        let mem = self.memory.lock().ok()?;
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: format!(
                "chat (msgs: {}, tools: {}, history: {}, ctx: {}, files: {}, topics: {})",
                stats.total_messages,
                stats.total_tool_calls,
                stats.history_length,
                self.max_context_tokens(),
                mem.accessed_files.len(),
                mem.topics.len(),
            ),
            current_program: None,
            ip: None,
            world_graph_size: 0,
            memory_trace_length: 0,
            uptime: std::time::Duration::ZERO,
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::Execute,
            Capability::Generate,
            Capability::Probe,
            Capability::Reflect,
            Capability::Custom("nl_to_sigma".into()),
            Capability::Custom("sigma_to_nl".into()),
            Capability::Custom("tool_use".into()),
            Capability::Custom("streaming".into()),
            Capability::Custom("context_memory".into()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_backend::{ChatRole, NoopBackend};

    fn make_agent() -> ChatAgent {
        let backend = Arc::new(NoopBackend);
        let bus = Arc::new(Mutex::new(Bus::new()));
        let cli = Arc::new(CliAgent::new(AgentId::new("test-cli")));
        let vm = Arc::new(Mutex::new(CcsVm::new()));
        ChatAgent::new(AgentId::new("chat-1"), backend, bus, cli, vm)
    }

    #[test]
    fn test_new_chat_agent() {
        let agent = make_agent();
        assert_eq!(agent.id(), AgentId::new("chat-1"));
        assert_eq!(agent.agent_type(), AgentType::Chat);
    }

    #[test]
    fn test_history_starts_with_system_prompts() {
        let agent = make_agent();
        let hist = agent.history();
        assert!(hist.len() >= 2);
        assert_eq!(hist[0].role, ChatRole::System);
    }

    #[test]
    fn test_chat_adds_to_history() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let agent = make_agent();
        let result = rt.block_on(async { agent.chat_streaming("hello", &|_| {}).await });
        assert!(result.is_ok());
        let hist = agent.history();
        assert!(hist.len() >= 3);
    }

    #[test]
    fn test_capabilities() {
        let agent = make_agent();
        let caps = agent.capabilities();
        assert!(caps.contains(&Capability::Chat));
        assert!(caps.contains(&Capability::Execute));
    }

    #[test]
    fn test_state_summary() {
        let agent = make_agent();
        let summary = agent.state_summary().unwrap();
        assert!(summary.state.contains("chat"));
    }

    #[test]
    fn test_reset_history() {
        let agent = make_agent();
        agent.chat("hello").ok();
        agent.reset_history();
        let hist = agent.history();
        assert_eq!(hist.len(), 2);
    }

    #[test]
    fn test_compact_constructor() {
        let backend = Arc::new(NoopBackend);
        let bus = Arc::new(Mutex::new(Bus::new()));
        let cli = Arc::new(CliAgent::new(AgentId::new("test-cli")));
        let vm = Arc::new(Mutex::new(CcsVm::new()));
        let agent = ChatAgent::new_compact(AgentId::new("chat-compact"), backend, bus, cli, vm);
        assert_eq!(agent.agent_type(), AgentType::Chat);
    }

    #[test]
    fn test_stats() {
        let agent = make_agent();
        agent.chat("hello").ok();
        let stats = agent.stats();
        assert!(stats.total_messages > 0);
    }

    #[test]
    fn test_estimate_tokens() {
        let short = ChatAgent::estimate_tokens("hello");
        assert!(short > 0 && short <= 3);
        let longer = ChatAgent::estimate_tokens("This is a longer sentence with many words.");
        assert!(longer >= 5);
    }

    #[test]
    fn test_context_tokens_used() {
        let agent = make_agent();
        let used = agent.context_tokens_used();
        assert!(used > 0, "should have tokens from system prompts");
    }

    // ── Pruning tests ──────────────────────────────────────────────────

    #[test]
    fn test_prune_small_history() {
        let agent = make_agent();
        let messages = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user("short question"),
            ChatMessage::assistant("short answer"),
        ];
        let pruned = agent.prune_history(&messages);
        assert_eq!(pruned.len(), messages.len());
    }

    #[test]
    fn test_prune_preserves_system_msgs() {
        let mut agent = make_agent();
        agent.set_max_context_tokens(500);
        let big_content = "x".repeat(600);
        let messages = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user(&big_content),
            ChatMessage::assistant("answer"),
        ];
        let pruned = agent.prune_history(&messages);
        assert!(pruned.len() >= 2);
        assert_eq!(pruned[0].role, ChatRole::System);
        assert_eq!(pruned[1].role, ChatRole::System);
    }

    #[test]
    fn test_prune_system_msgs_exceed_budget() {
        let mut agent = make_agent();
        let huge_sys = "x".repeat(200);
        agent.set_max_context_tokens(30);
        let messages = vec![
            ChatMessage::system(&huge_sys),
            ChatMessage::system(&huge_sys),
            ChatMessage::user("hello"),
        ];
        let pruned = agent.prune_history(&messages);
        assert_eq!(pruned.len(), 2, "only system msgs when they fill budget");
    }

    #[test]
    fn test_prune_skips_oversized_not_drops_all() {
        let mut agent = make_agent();
        let huge = "x".repeat(400);
        agent.set_max_context_tokens(500);
        let messages = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user(&huge),
            ChatMessage::assistant(&huge),
            ChatMessage::user("small hello"),
        ];
        let pruned = agent.prune_history(&messages);
        let has_small = pruned.iter().any(|m| m.content == "small hello");
        assert!(has_small, "small recent message should be kept");
    }

    #[test]
    fn test_prune_drops_old_messages() {
        let mut agent = make_agent();
        agent.set_max_context_tokens(50);
        let messages = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user("msg1 that should be dropped"),
            ChatMessage::assistant("resp1"),
            ChatMessage::user("msg2 that should be dropped"),
            ChatMessage::assistant("resp2"),
            ChatMessage::user("recent message kept"),
            ChatMessage::assistant("recent response kept"),
        ];
        let pruned = agent.prune_history(&messages);
        assert!(pruned.len() < messages.len());
        assert_eq!(pruned[0].role, ChatRole::System);
        let has_recent = pruned.iter().any(|m| m.content.contains("recent"));
        assert!(has_recent);
        let has_old = pruned.iter().any(|m| m.content.contains("msg1"));
        assert!(!has_old);
    }

    #[test]
    fn test_set_max_context_tokens() {
        let mut agent = make_agent();
        assert_eq!(agent.max_context_tokens(), 32768);
        agent.set_max_context_tokens(4096);
        assert_eq!(agent.max_context_tokens(), 4096);
    }

    #[test]
    fn test_memory_accumulates_across_prunes() {
        let mut agent = make_agent();
        agent.set_max_context_tokens(40); // tight window to force dropping

        // First prune: messages with file paths (some will be dropped)
        let msgs1 = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user("check /src/main.rs"),
            ChatMessage::assistant("ok"),
            ChatMessage::user("also check /src/lib.rs"),
            ChatMessage::assistant("done"),
            ChatMessage::user("one more /tests/integration.rs"),
            ChatMessage::assistant("sure"),
        ];
        agent.prune_history(&msgs1);

        // Second prune: more messages with a new file
        let msgs2 = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user("run shell on /config/app.toml"),
            ChatMessage::assistant("ran"),
        ];
        agent.prune_history(&msgs2);

        // Memory should accumulate across both prunes
        let mem = agent.memory_snapshot();
        assert!(mem.accessed_files.len() >= 2);
        assert!(mem.accessed_files.contains("/src/main.rs"));
        assert!(mem.messages_processed >= 3);
    }

    #[test]
    fn test_prune_injects_memory_as_system_msg() {
        let mut agent = make_agent();
        agent.set_max_context_tokens(50);

        // Pre-populate memory
        {
            let mut mem = agent.memory.lock().unwrap();
            mem.accessed_files.insert("/src/main.rs".into());
            mem.tool_usage.insert("fs_read".into(), 3);
        }

        let messages = vec![
            ChatMessage::system("sys1"),
            ChatMessage::system("sys2"),
            ChatMessage::user("hello"),
            ChatMessage::assistant("hi"),
        ];
        let pruned = agent.prune_history(&messages);

        // Should have sys1, sys2, memory, user, assistant (5 messages)
        assert!(pruned.len() >= 3);
        // Find the memory message
        let has_memory = pruned.iter().any(|m| m.content.contains("Context Memory"));
        assert!(
            has_memory,
            "pruned output should include context memory message"
        );
        let has_working_files = pruned.iter().any(|m| m.content.contains("main.rs"));
        assert!(has_working_files, "memory should mention accessed files");
    }

    #[test]
    fn test_memory_system_message_token_budget() {
        let mut mem = ContextMemory::default();
        // Fill with lots of data
        for i in 0..30 {
            mem.accessed_files.insert(format!("/project/file_{}.rs", i));
            mem.tool_usage.insert(format!("tool_{}", i), i);
            mem.topics.push(format!("Topic{}", i));
        }
        let msg = mem.to_system_message().unwrap();
        let tokens = ChatAgent::estimate_tokens(&msg.content);
        // Should be reasonably sized — well under the 300 token budget
        assert!(
            tokens <= 400,
            "memory message should be compact, got {} tokens",
            tokens
        );
    }
}
