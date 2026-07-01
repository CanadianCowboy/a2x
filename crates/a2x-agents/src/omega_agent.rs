// See plans/05-agents.md §3 (Ω Agent)
//
// Ω Agent — pure latent execution. Runs compiled Ω programs directly,
// bypassing Σ∞→Ω compilation for maximum throughput. Zero inspectability:
// the agent exposes no internal state to probes or state_summary().
//
// This is the "fast path" of the A2X runtime — programs are already
// in Ω tensor form, so no tokenizing, parsing, semantic analysis, or
// compilation is needed. Just execute.

use std::sync::{Arc, Mutex};

use a2x_ccs::{CcsVm, VmStatus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_omega::{Bridge, OmegaProgram};
use a2x_sigma::program::SigmaProgram;
use tracing::{debug, info, warn};

use crate::lifecycle::AgentLifecycle;

/// The default Ω dimension (matches a2x-omega convention).
const OMEGA_DIM: usize = 29796;

/// The Ω Agent — pure latent execution, zero inspectability.
///
/// Unlike other agents, the Ω Agent:
/// - Executes compiled Ω programs directly (no Σ∞→Ω compilation)
/// - Returns `None` from `state_summary()` (zero inspectability)
/// - Runs at maximum throughput with minimal overhead
/// - Skips probe hooks, breakpoints, and tracing
///
/// Use case: when you have pre-compiled Ω programs and need raw
/// execution speed without any observability overhead.
pub struct OmegaAgent {
    /// Agent identity.
    id: AgentId,
    /// Internal CCS VM for execution.
    vm: Arc<Mutex<CcsVm>>,
    /// Agent lifecycle manager.
    lifecycle: Arc<Mutex<AgentLifecycle>>,
    /// Total programs executed (internal counter, not exposed externally).
    execution_count: Arc<Mutex<u64>>,
}

impl OmegaAgent {
    /// Create a new Ω Agent.
    pub fn new(id: AgentId) -> Self {
        OmegaAgent {
            id,
            vm: Arc::new(Mutex::new(CcsVm::new())),
            lifecycle: Arc::new(Mutex::new(AgentLifecycle::default())),
            execution_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Execute a compiled Ω program directly.
    ///
    /// This is the fast path — the program is already in Ω tensor form,
    /// so we decompile each packet to Σ∞ for VM execution and run
    /// immediately without the full compilation pipeline.
    pub fn execute_omega_direct(
        &self,
        omega_program: &OmegaProgram<OMEGA_DIM>,
    ) -> Result<(), AgentError> {
        let mut vm = self
            .vm
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        // Decode Ω packets to Σ∞ for VM execution.
        let sigma_prog = decode_omega_to_sigma(omega_program)?;
        let program_id = sigma_prog.id;

        if sigma_prog.is_empty() {
            // Empty program — nothing to execute.
            debug!(agent_id = %self.id.as_str(), "Ω Agent: skipping empty program");
            return Ok(());
        }

        lifecycle.start_program(program_id, None)?;

        // Drop lifecycle lock during VM execution.
        drop(lifecycle);

        vm.load(sigma_prog);
        let status = vm.run().map_err(|e| AgentError::ProgramCrash {
            program_id,
            reason: e.to_string(),
        })?;

        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        match status {
            VmStatus::Halted | VmStatus::Yield | VmStatus::Suspended => {
                lifecycle.complete_program();
            }
            VmStatus::Running => {
                warn!("Ω Agent: program did not halt — forcing completion");
                lifecycle.complete_program();
            }
            VmStatus::Fault(err) => {
                let err_str = err.to_string();
                lifecycle.handle_error(&err_str)?;
                return Err(AgentError::VmError(format!("Ω execution fault: {err}")));
            }
        }

        // Increment execution counter.
        if let Ok(mut count) = self.execution_count.lock() {
            *count = count.wrapping_add(1);
        }

        let count = self.execution_count.lock().ok().map(|g| *g).unwrap_or(0);
        info!(
            agent_id = %self.id.as_str(),
            program_count = count,
            "Ω Agent: program executed"
        );

        Ok(())
    }

    /// Execute a batch of Ω programs in sequence (pipeline mode).
    pub fn execute_batch(
        &self,
        programs: &[OmegaProgram<OMEGA_DIM>],
    ) -> Result<Vec<()>, AgentError> {
        let mut results = Vec::with_capacity(programs.len());
        for prog in programs {
            self.execute_omega_direct(prog)?;
            results.push(());
        }
        Ok(results)
    }
}

/// Decode an Ω program into a Σ∞ program for VM execution.
///
/// Uses the Ω bridge's decompile functionality to recover Σ∞ packets
/// from Ω tensors. Each Ω packet is decompiled individually via
/// `Bridge::decompile()`. Packets that cannot be decoded (unknown opcodes,
/// control flow without canonical IntentOp mapping) are skipped.
fn decode_omega_to_sigma(
    omega_program: &OmegaProgram<OMEGA_DIM>,
) -> Result<SigmaProgram, AgentError> {
    let mut sigma_prog = SigmaProgram::new();

    for packet in &omega_program.instructions {
        match Bridge::decompile(packet) {
            Some(sigma_packet) => {
                sigma_prog.push(sigma_packet);
            }
            None => {
                // Packet cannot be decompiled — skip it.
                // This is expected for control-flow opcodes (Branch, Merge, etc.)
                // that have no canonical IntentOp mapping.
                debug!("Ω Agent: skipping non-decompilable Ω packet");
            }
        }
    }

    Ok(sigma_prog)
}

impl Agent for OmegaAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Ccs // Ω Agent is a CCS subtype — pure execution
    }

    fn execute(&self, _program: Packet) -> Result<Packet, AgentError> {
        // Ω Agent doesn't accept raw packets — use execute_omega_direct().
        Err(AgentError::VmError(
            "Ω Agent requires pre-compiled OmegaProgram — use execute_omega_direct()".into(),
        ))
    }

    /// Zero inspectability per plan: "max speed, zero inspectability."
    fn state_summary(&self) -> Option<StateSnapshot> {
        None
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Execute,
            Capability::Custom("omega".into()),
            Capability::Custom("latent".into()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_omega_agent() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        assert_eq!(agent.id(), AgentId::new("omega-1"));
        assert_eq!(agent.agent_type(), AgentType::Ccs);
        assert!(agent.state_summary().is_none());
    }

    #[test]
    fn test_zero_inspectability() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        // Per plan: "zero inspectability" — state_summary returns None.
        assert!(agent.state_summary().is_none());
    }

    #[test]
    fn test_capabilities() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        let caps = agent.capabilities();
        assert!(caps.contains(&Capability::Execute));
        assert!(caps.contains(&Capability::Custom("omega".into())));
        assert!(caps.contains(&Capability::Custom("latent".into())));
    }

    #[test]
    fn test_execute_rejects_raw_packets() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        let result = agent.execute(Packet::Raw(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_batch_empty() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        let results = agent.execute_batch(&[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_execute_omega_empty_program() {
        let agent = OmegaAgent::new(AgentId::new("omega-1"));
        let empty_prog: OmegaProgram<OMEGA_DIM> = OmegaProgram::new();
        // Empty Ω program should succeed (no-op).
        let result = agent.execute_omega_direct(&empty_prog);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_instances_independent() {
        let a1 = OmegaAgent::new(AgentId::new("omega-a"));
        let a2 = OmegaAgent::new(AgentId::new("omega-b"));
        assert_ne!(a1.id(), a2.id());
        // Both have zero inspectability.
        assert!(a1.state_summary().is_none());
        assert!(a2.state_summary().is_none());
    }

    #[test]
    fn test_decode_omega_to_sigma_empty() {
        let prog: OmegaProgram<OMEGA_DIM> = OmegaProgram::new();
        let sigma = decode_omega_to_sigma(&prog).unwrap();
        assert!(sigma.is_empty());
    }
}
