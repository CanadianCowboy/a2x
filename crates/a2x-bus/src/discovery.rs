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
