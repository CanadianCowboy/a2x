// See plans/04-bus.md §6

use crate::discovery::{AgentFilter, Discovery};
use a2x_core::{AgentId, Capability};

/// How the router selects target agents from matching candidates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// First available matching agent.
    FirstMatch,
    /// Round-robin across matching agents.
    RoundRobin,
    /// Route to a specific agent by label.
    ByLabel(String),
}

/// The bus router — matches programs to agents based on capability and strategy.
pub struct Router {
    /// Next index for round-robin routing.
    round_robin_index: usize,
    /// Routing strategy.
    strategy: RoutingStrategy,
}

impl Router {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Router {
            round_robin_index: 0,
            strategy,
        }
    }

    /// Find the best target agent for a given capability requirement.
    pub fn route(
        &mut self,
        discovery: &dyn Discovery,
        required_capability: &Capability,
    ) -> Option<AgentId> {
        let candidates =
            discovery.discover(&AgentFilter::ByCapability(required_capability.clone()));

        let online: Vec<_> = candidates.into_iter().filter(|info| info.online).collect();

        if online.is_empty() {
            return None;
        }

        match &self.strategy {
            RoutingStrategy::FirstMatch => online.first().map(|info| info.id.clone()),
            RoutingStrategy::RoundRobin => {
                let idx = self.round_robin_index % online.len();
                self.round_robin_index = self.round_robin_index.wrapping_add(1);
                online.get(idx).map(|info| info.id.clone())
            }
            RoutingStrategy::ByLabel(label) => online
                .iter()
                .find(|info| info.id.as_str() == label.as_str())
                .map(|info| info.id.clone()),
        }
    }

    /// Update the routing strategy.
    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{AgentInfo, InMemoryDiscovery};
    use a2x_core::{AgentId, AgentType, Capability};

    fn setup_discovery() -> InMemoryDiscovery {
        let mut disc = InMemoryDiscovery::new();
        let agents = vec![
            AgentInfo::new(
                AgentId::new("cli-1"),
                AgentType::Cli,
                vec![Capability::Execute, Capability::FileSystem],
            ),
            AgentInfo::new(
                AgentId::new("cli-2"),
                AgentType::Cli,
                vec![Capability::Execute, Capability::Network],
            ),
            AgentInfo::new(
                AgentId::new("orch-1"),
                AgentType::Orchestrator,
                vec![Capability::Execute],
            ),
        ];
        for agent in agents {
            disc.register(agent).unwrap();
        }
        disc
    }

    #[test]
    fn test_first_match() {
        let disc = setup_discovery();
        let mut router = Router::new(RoutingStrategy::FirstMatch);
        let target = router.route(&disc, &Capability::Execute);
        assert!(target.is_some());
        // First registered agent with Execute is cli-1
        assert_eq!(target.unwrap().as_str(), "cli-1");
    }

    #[test]
    fn test_round_robin() {
        let disc = setup_discovery();
        let mut router = Router::new(RoutingStrategy::RoundRobin);
        let t1 = router.route(&disc, &Capability::Execute).unwrap();
        let t2 = router.route(&disc, &Capability::Execute).unwrap();
        let t3 = router.route(&disc, &Capability::Execute).unwrap();
        // 3 agents with Execute, should cycle
        assert_eq!(t1.as_str(), "cli-1");
        assert_eq!(t2.as_str(), "cli-2");
        assert_eq!(t3.as_str(), "orch-1");
    }

    #[test]
    fn test_by_label() {
        let disc = setup_discovery();
        let mut router = Router::new(RoutingStrategy::ByLabel("cli-2".into()));
        let target = router.route(&disc, &Capability::Execute);
        assert_eq!(target.unwrap().as_str(), "cli-2");
    }

    #[test]
    fn test_no_match() {
        let disc = setup_discovery();
        let mut router = Router::new(RoutingStrategy::FirstMatch);
        let target = router.route(&disc, &Capability::Shell);
        assert!(target.is_none());
    }
}
