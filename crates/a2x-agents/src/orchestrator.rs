// See plans/05-agents.md §3 (Orchestrator)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a2x_ccs::{CcsVm, VmStatus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::program_id::ProgramId;
use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

use crate::lifecycle::AgentLifecycle;

/// The Orchestrator agent — top-level coordinator.
///
/// Receives high-level goals, decomposes them into Σ∞ programs, dispatches
/// to other agents for execution, and collects results.
pub struct Orchestrator {
    /// Agent identity.
    id: AgentId,
    /// Internal VM for planning/coordination.
    vm: Arc<Mutex<CcsVm>>,
    /// Agent lifecycle manager.
    lifecycle: Arc<Mutex<AgentLifecycle>>,
    /// Results collected from dispatched programs.
    results: Arc<Mutex<HashMap<ProgramId, SigmaProgram>>>,
}

impl Orchestrator {
    /// Create a new Orchestrator agent.
    pub fn new(id: AgentId) -> Self {
        Orchestrator {
            id,
            vm: Arc::new(Mutex::new(CcsVm::new())),
            lifecycle: Arc::new(Mutex::new(AgentLifecycle::default())),
            results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Dispatch a Σ∞ program to this agent for execution.
    /// In Phase 0, the orchestrator runs the program on its own VM.
    pub fn dispatch(&self, program: SigmaProgram) -> Result<SigmaProgram, AgentError> {
        let pid = program.id;
        let mut lc = self
            .lifecycle
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;
        lc.start_program(pid)?;
        drop(lc);

        let mut vm = self
            .vm
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;
        vm.load(program);
        let status = vm.run().map_err(|e| {
            AgentError::ProgramCrash {
                program_id: pid,
                reason: e.to_string(),
            }
        })?;

        let mut lc = self
            .lifecycle
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;
        match status {
            VmStatus::Halted => lc.complete_program(),
            VmStatus::Yield => lc.complete_program(),
            _ => {}
        }

        // Return result (last instruction's data as a new program)
        let result = SigmaProgram::new();
        Ok(result)
    }

    /// Store a result from a dispatched program.
    pub fn store_result(&self, program_id: ProgramId, result: SigmaProgram) {
        if let Ok(mut results) = self.results.lock() {
            results.insert(program_id, result);
        }
    }

    /// Get a previously stored result.
    pub fn get_result(&self, program_id: &ProgramId) -> Option<SigmaProgram> {
        self.results
            .lock()
            .ok()
            .and_then(|r| r.get(program_id).cloned())
    }
}

impl Agent for Orchestrator {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Orchestrator
    }

    fn execute(&self, _program: Packet) -> Result<Packet, AgentError> {
        // Phase 0: accept raw packet, run empty program, return raw result
        let prog = SigmaProgram::new();
        let _result = self.dispatch(prog)?;
        Ok(Packet::Raw(vec![]))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let lc = self.lifecycle.lock().ok()?;
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: format!("{:?}", lc.state),
            current_program: lc.state.current_program(),
            ip: None,
            world_graph_size: 0,
            memory_trace_length: 0,
            uptime: lc.uptime(),
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Execute, Capability::Custom("schedule".into()), Capability::Custom("plan".into())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::agent_id::AgentId;

    #[test]
    fn test_new_orchestrator() {
        let orch = Orchestrator::new(AgentId::new("orch-1"));
        assert_eq!(orch.id(), AgentId::new("orch-1"));
        assert_eq!(orch.agent_type(), AgentType::Orchestrator);
        assert!(orch.state_summary().is_some());
    }

    #[test]
    fn test_orchestrator_capabilities() {
        let orch = Orchestrator::new(AgentId::new("orch-1"));
        let caps = orch.capabilities();
        assert!(caps.contains(&Capability::Execute));
    }

    #[test]
    fn test_dispatch_empty_program() {
        let orch = Orchestrator::new(AgentId::new("orch-1"));
        let program = SigmaProgram::new();
        let result = orch.dispatch(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_and_get_result() {
        let orch = Orchestrator::new(AgentId::new("orch-1"));
        let pid = ProgramId::new([1u8; 32]);
        let result = SigmaProgram::new();
        orch.store_result(pid, result.clone());
        let got = orch.get_result(&pid);
        assert!(got.is_some());
    }
}
