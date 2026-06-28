// See plans/05-agents.md §3 (LLM Agent)

use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

/// The LLM agent — bridges natural language and A2X.
///
/// Converts human requests into Σ∞ programs and Σ∞ results back into
/// natural language explanations. Phase 0: stub implementation.
pub struct LlmAgent {
    /// Agent identity.
    id: AgentId,
    /// Model name (for reference/display).
    _model: String,
}

impl LlmAgent {
    /// Create a new LLM agent.
    pub fn new(id: AgentId, model: impl Into<String>) -> Self {
        LlmAgent {
            id,
            _model: model.into(),
        }
    }

    /// Convert natural language intent into a Σ∞ program.
    /// Phase 0 stub: returns an empty program.
    pub fn nl_to_sigma(&self, _intent: &str) -> SigmaProgram {
        // Phase 0 stub — in Phase 2+ this calls an LLM API
        SigmaProgram::new()
    }

    /// Convert a Σ∞ program result into natural language.
    /// Phase 0 stub: returns a placeholder description.
    pub fn sigma_to_nl(&self, _program: &SigmaProgram) -> String {
        // Phase 0 stub — in Phase 2+ this calls an LLM API
        "[LLM: program execution summary]".to_string()
    }
}

impl Agent for LlmAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Llm
    }

    fn execute(&self, _program: Packet) -> Result<Packet, AgentError> {
        // Phase 0: return empty raw result
        Ok(Packet::Raw(vec![]))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: "idle".to_string(),
            current_program: None,
            ip: None,
            world_graph_size: 0,
            memory_trace_length: 0,
            uptime: std::time::Duration::ZERO,
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Execute, Capability::Custom("plan".into())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::agent_id::AgentId;

    #[test]
    fn test_new_llm_agent() {
        let agent = LlmAgent::new(AgentId::new("llm-1"), "mock-model");
        assert_eq!(agent.id(), AgentId::new("llm-1"));
        assert_eq!(agent.agent_type(), AgentType::Llm);
    }

    #[test]
    fn test_nl_to_sigma_stub() {
        let agent = LlmAgent::new(AgentId::new("llm-1"), "mock");
        let program = agent.nl_to_sigma("scan the system");
        assert!(program.instructions.is_empty());
    }

    #[test]
    fn test_sigma_to_nl_stub() {
        let agent = LlmAgent::new(AgentId::new("llm-1"), "mock");
        let program = SigmaProgram::new();
        let desc = agent.sigma_to_nl(&program);
        assert!(desc.contains("LLM"));
    }
}
