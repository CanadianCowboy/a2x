// a2x-agents — Built-in A2X agent implementations
//
// See plans/05-agents.md for the full design specification.
//
// Agent types: Orchestrator, CLI agent, LLM agent, CCS agent.

pub mod ccs_agent;
pub mod chat_agent;
pub mod chat_prompt;
pub mod chat_tools;
pub mod cli_agent;
pub mod context_memory;
pub mod lifecycle;
pub mod llm_agent;
pub mod llm_backend;
pub mod omega_agent;
pub mod orchestrator;
pub mod parse;

// Re-exports
pub use ccs_agent::CcsAgent;
pub use chat_agent::ChatAgent;
pub use chat_prompt::{A2X_CHAT_SYSTEM_PROMPT, A2X_CHAT_SYSTEM_PROMPT_SHORT};
pub use chat_tools::{all_tool_defs, execute_tool, ToolContext};
pub use cli_agent::{CliAgent, SandboxMode};
pub use context_memory::{extract_message_patterns, extract_topics, scan_for_paths, ContextMemory};
pub use lifecycle::{AgentLifecycle, AgentState};
pub use llm_agent::LlmAgent;
pub use llm_backend::{
    ChatChunk, ChatMessage, ChatRole, LlmBackend, NoopBackend, OpenAiBackend, OpenAiConfig,
    ToolCall, ToolDef, SIGMA_EXPLANATION_PROMPT, SIGMA_GENERATION_PROMPT,
};
pub use omega_agent::OmegaAgent;
pub use orchestrator::Orchestrator;
pub use parse::{packet_to_sigma_program, sigma_program_to_packet};
