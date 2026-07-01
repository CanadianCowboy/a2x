// a2x-agents — Built-in A2X agent implementations
//
// See plans/05-agents.md for the full design specification.
//
// Agent types: Orchestrator, CLI agent, LLM agent, CCS agent.

pub mod ccs_agent;
pub mod cli_agent;
pub mod lifecycle;
pub mod llm_agent;
pub mod llm_backend;
pub mod omega_agent;
pub mod orchestrator;
pub mod parse;

// Re-exports
pub use ccs_agent::CcsAgent;
pub use cli_agent::{CliAgent, SandboxMode};
pub use lifecycle::{AgentLifecycle, AgentState};
pub use llm_agent::LlmAgent;
pub use llm_backend::{
    LlmBackend, NoopBackend, OpenAiBackend, OpenAiConfig, SIGMA_EXPLANATION_PROMPT,
    SIGMA_GENERATION_PROMPT,
};
pub use omega_agent::OmegaAgent;
pub use orchestrator::Orchestrator;
pub use parse::{packet_to_sigma_program, sigma_program_to_packet};
