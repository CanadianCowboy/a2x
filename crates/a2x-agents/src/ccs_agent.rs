// See plans/05-agents.md §3 (CCS Agent)

use std::sync::{Arc, Mutex};

use a2x_ccs::CcsVm;
use a2x_core::agent::Agent;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

/// The CCS agent — long-running cognitive agent.
///
/// Maintains a persistent WorldGraph, continuously executing Evolve/Reflect
/// cycles. Builds up a rich world-model over time and responds to queries.
pub struct CcsAgent {
    /// Agent identity.
    id: AgentId,
    /// Persistent CCS VM (long-running).
    vm: Arc<Mutex<CcsVm>>,
    /// Whether the agent is running its cognitive loop.
    running: Arc<Mutex<bool>>,
}

impl CcsAgent {
    /// Create a new CCS agent with a persistent WorldGraph.
    pub fn new(id: AgentId) -> Self {
        CcsAgent {
            id,
            vm: Arc::new(Mutex::new(CcsVm::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the continuous cognitive loop (Evolve + Reflect).
    /// Phase 0 stub: marks as running but doesn't start a background thread.
    pub fn start_cognitive_loop(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = true;
        }
    }

    /// Stop the cognitive loop.
    pub fn stop_cognitive_loop(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }
    }

    /// Check if the cognitive loop is running.
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Query the agent's WorldGraph.
    /// Phase 0 stub: returns an empty result.
    pub fn query(&self, _query: &str) -> Result<SigmaProgram, AgentError> {
        // Phase 0 stub — in Phase 2+ this runs graph queries
        Ok(SigmaProgram::new())
    }
}

impl Agent for CcsAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Ccs
    }

    fn execute(&self, _program: Packet) -> Result<Packet, AgentError> {
        // Phase 0: return empty raw result
        Ok(Packet::Raw(vec![]))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let vm = self.vm.lock().ok()?;
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: if self.is_running() {
                "running".into()
            } else {
                "idle".into()
            },
            current_program: None,
            ip: Some(vm.ip),
            world_graph_size: vm.world_graph.node_count(),
            memory_trace_length: vm.memory_trace.len(),
            uptime: vm.uptime(),
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Execute, Capability::Custom("plan".into()), Capability::Custom("schedule".into())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::agent_id::AgentId;

    #[test]
    fn test_new_ccs_agent() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        assert_eq!(agent.id(), AgentId::new("ccs-1"));
        assert_eq!(agent.agent_type(), AgentType::Ccs);
        assert!(!agent.is_running());
    }

    #[test]
    fn test_start_stop_loop() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        assert!(!agent.is_running());
        agent.start_cognitive_loop();
        assert!(agent.is_running());
        agent.stop_cognitive_loop();
        assert!(!agent.is_running());
    }

    #[test]
    fn test_query_stub() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.query("find: anomaly");
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_summary() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let summary = agent.state_summary();
        assert!(summary.is_some());
    }
}
