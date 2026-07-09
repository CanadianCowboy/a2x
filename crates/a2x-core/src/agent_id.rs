// See plans/09-core-types.md §2

/// Unique identifier for an agent or entity in the A2X ecosystem.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentId(String);

impl AgentId {
    /// Create a new AgentId.
    pub fn new(id: impl Into<String>) -> Self {
        AgentId(id.into())
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        AgentId(s.to_string())
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        AgentId(s)
    }
}

/// Type tag for an agent in the A2X ecosystem.
///
/// Each agent type has different execution semantics:
/// - Orchestrator: writes and dispatches A2X programs
/// - Llm: generates Σ∞ programs from natural language intent
/// - Cli: executes system commands through sandboxed shell
/// - Ccs: maintains a persistent WorldGraph for cognitive programs
/// - Omega: pure latent execution, no symbolic layer///   - Chat: conversational coding agent that orchestrates all A2X subsystems
///   - Entity: external system connected through the gateway
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AgentType {
    Orchestrator,
    Llm,
    Cli,
    Ccs,
    Omega,
    Chat,
    Entity,
    /// 4-byte namespace for custom/third-party agent types.
    Custom([u8; 4]),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id = AgentId::new("orchestrator-1");
        assert_eq!(id.as_str(), "orchestrator-1");
    }

    #[test]
    fn test_agent_id_from_str() {
        let id: AgentId = "cli-1".into();
        assert_eq!(id.as_str(), "cli-1");
    }
}
