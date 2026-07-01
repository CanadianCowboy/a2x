// See plans/04-bus.md §5

use crate::transport::TransportError;
use a2x_core::{AgentId, AgentType, Capability};

/// Information about a registered agent on the bus.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentInfo {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub capabilities: Vec<Capability>,
    pub online: bool,
}

impl AgentInfo {
    pub fn new(id: AgentId, agent_type: AgentType, capabilities: Vec<Capability>) -> Self {
        AgentInfo {
            id,
            agent_type,
            capabilities,
            online: true,
        }
    }
}

/// Agent card — typed metadata for agent discovery and capability negotiation.
///
/// Based on the A2A (Agent-to-Agent) AgentCard pattern. Provides structured
/// information about an agent's identity, capabilities, endpoints, and
/// supported modalities for protocol-level handshake.
///
/// See plans/04-bus.md §5 and the comprehensive audit T2-1 recommendation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentCard {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Human-readable display name.
    pub name: String,
    /// Semantic version of the agent implementation.
    pub version: String,
    /// Agent type classification.
    pub agent_type: AgentType,
    /// Capabilities this agent provides.
    pub capabilities: Vec<Capability>,
    /// Network endpoints where this agent can be reached.
    pub endpoints: Vec<String>,
    /// Supported authentication methods.
    pub auth_methods: Vec<String>,
    /// Modalities this agent can process (e.g., "text", "sigma", "omega").
    pub modalities: Vec<String>,
    /// Short description of the agent's purpose.
    pub description: String,
}

impl AgentCard {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AgentId,
        name: impl Into<String>,
        version: impl Into<String>,
        agent_type: AgentType,
        capabilities: Vec<Capability>,
        endpoints: Vec<String>,
        auth_methods: Vec<String>,
        modalities: Vec<String>,
        description: impl Into<String>,
    ) -> Self {
        AgentCard {
            id,
            name: name.into(),
            version: version.into(),
            agent_type,
            capabilities,
            endpoints,
            auth_methods,
            modalities,
            description: description.into(),
        }
    }

    /// Convert to a lightweight AgentInfo for bus registration.
    pub fn to_agent_info(&self) -> AgentInfo {
        AgentInfo {
            id: self.id.clone(),
            agent_type: self.agent_type,
            capabilities: self.capabilities.clone(),
            online: true,
        }
    }
}

/// Agent handshake — protocol-level capability negotiation.
///
/// Based on the MCP/A2A initialization handshake pattern. Before an agent
/// can send programs to another agent, they exchange AgentCards to verify
/// compatibility: supported modalities, auth methods, and protocol versions.
///
/// See plans/04-bus.md §5 and the comprehensive audit T1-2.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentHandshake {
    /// The agent card this agent presents.
    pub card: AgentCard,
    /// Protocol version(s) this agent supports.
    pub protocol_versions: Vec<String>,
    /// Nonce for preventing replay attacks.
    pub nonce: [u8; 32],
}

impl AgentHandshake {
    pub fn new(card: AgentCard) -> Self {
        let mut nonce = [0u8; 32];
        // Deterministic nonce from card properties.
        // In production, this would use a CSPRNG.
        for (i, byte) in nonce.iter_mut().enumerate() {
            *byte = (i as u8)
                .wrapping_mul(17)
                .wrapping_add(card.id.as_str().len() as u8);
        }
        AgentHandshake {
            card,
            protocol_versions: vec!["0.6.0".into(), "0.7.0-alpha".into()],
            nonce,
        }
    }

    /// Verify compatibility between two handshakes.
    /// Returns true if they share at least one protocol version.
    pub fn is_compatible_with(&self, other: &AgentHandshake) -> bool {
        self.protocol_versions
            .iter()
            .any(|v| other.protocol_versions.contains(v))
    }
}

/// Filter for discovering agents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentFilter {
    ByCapability(Capability),
    ByType(AgentType),
    ById(AgentId),
    All,
}

/// Error from discovery operations.
#[derive(Clone, Debug, PartialEq)]
pub enum DiscoveryError {
    NotFound,
    AlreadyRegistered,
    Transport(TransportError),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::NotFound => write!(f, "agent not found"),
            DiscoveryError::AlreadyRegistered => write!(f, "agent already registered"),
            DiscoveryError::Transport(err) => write!(f, "transport error: {}", err),
        }
    }
}

impl std::error::Error for DiscoveryError {}

pub trait Discovery: Send + Sync {
    fn register(&mut self, agent: AgentInfo) -> Result<(), DiscoveryError>;
    fn discover(&self, filter: &AgentFilter) -> Vec<AgentInfo>;
    fn mark_offline(&mut self, id: &AgentId);
    fn mark_online(&mut self, id: &AgentId);
}

/// In-memory agent registry.
#[derive(Default)]
pub struct InMemoryDiscovery {
    agents: std::collections::HashMap<AgentId, AgentInfo>,
}

impl InMemoryDiscovery {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Discovery for InMemoryDiscovery {
    fn register(&mut self, agent: AgentInfo) -> Result<(), DiscoveryError> {
        if self.agents.contains_key(&agent.id) {
            return Err(DiscoveryError::AlreadyRegistered);
        }
        self.agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    fn discover(&self, filter: &AgentFilter) -> Vec<AgentInfo> {
        let mut results: Vec<_> = self
            .agents
            .values()
            .filter(|info| match filter {
                AgentFilter::ByCapability(cap) => info.capabilities.contains(cap),
                AgentFilter::ByType(t) => info.agent_type == *t,
                AgentFilter::ById(id) => info.id == *id,
                AgentFilter::All => true,
            })
            .cloned()
            .collect();
        // Sort by AgentId for deterministic ordering (HashMap iteration is non-deterministic).
        results.sort_by(|a, b| a.id.cmp(&b.id));
        results
    }

    fn mark_offline(&mut self, id: &AgentId) {
        if let Some(info) = self.agents.get_mut(id) {
            info.online = false;
        }
    }

    fn mark_online(&mut self, id: &AgentId) {
        if let Some(info) = self.agents.get_mut(id) {
            info.online = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_discover() {
        let mut disc = InMemoryDiscovery::new();
        let info = AgentInfo::new(
            AgentId::new("agent-1"),
            AgentType::Cli,
            vec![Capability::Execute, Capability::FileSystem],
        );
        disc.register(info).unwrap();
        let results = disc.discover(&AgentFilter::ByCapability(Capability::Execute));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.as_str(), "agent-1");
    }

    #[test]
    fn test_discover_by_type() {
        let mut disc = InMemoryDiscovery::new();
        disc.register(AgentInfo::new(AgentId::new("a"), AgentType::Cli, vec![]))
            .unwrap();
        disc.register(AgentInfo::new(
            AgentId::new("b"),
            AgentType::Orchestrator,
            vec![],
        ))
        .unwrap();
        let results = disc.discover(&AgentFilter::ByType(AgentType::Cli));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_duplicate_registration() {
        let mut disc = InMemoryDiscovery::new();
        let info = AgentInfo::new(AgentId::new("agent-1"), AgentType::Cli, vec![]);
        disc.register(info.clone()).unwrap();
        assert!(disc.register(info).is_err());
    }

    #[test]
    fn test_mark_offline() {
        let mut disc = InMemoryDiscovery::new();
        let info = AgentInfo::new(
            AgentId::new("agent-1"),
            AgentType::Cli,
            vec![Capability::Execute],
        );
        disc.register(info).unwrap();
        disc.mark_offline(&AgentId::new("agent-1"));
        let results = disc.discover(&AgentFilter::ByCapability(Capability::Execute));
        assert_eq!(results.len(), 1);
        assert!(!results[0].online);
    }
}
