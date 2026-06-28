// See plans/05-agents.md §3 and §5 (CLI Agent with sandboxing)

use std::time::{Duration, Instant};

use a2x_ccs::{CcsVm, VmStatus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

/// CLI agent sandboxing mode.
#[derive(Clone, Debug, PartialEq)]
pub enum SandboxMode {
    /// No sandbox (unsafe, dev only).
    None,
    /// Filter commands against allowlist.
    CommandFilter(Vec<String>),
    /// Docker container (future).
    Container,
    /// Micro-VM (future).
    Vm,
}

/// The CLI agent — executes Σ∞ programs that interact with the host system.
///
/// Supports filesystem operations, process execution, and network operations
/// through its CCS VM. Sandboxing restricts which commands can run.
pub struct CliAgent {
    /// Agent identity.
    id: AgentId,
    /// Internal CCS VM for execution.
    vm: CcsVm,
    /// Sandbox mode.
    sandbox: SandboxMode,
    /// Maximum execution time per command.
    max_execution_time: Duration,
}

impl CliAgent {
    /// Create a new CLI agent with default sandboxing.
    pub fn new(id: AgentId) -> Self {
        CliAgent {
            id,
            vm: CcsVm::new(),
            sandbox: SandboxMode::CommandFilter(vec![
                "ls".into(),
                "ps".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
            ]),
            max_execution_time: Duration::from_secs(30),
        }
    }

    /// Create with custom sandbox configuration.
    pub fn with_sandbox(id: AgentId, sandbox: SandboxMode) -> Self {
        CliAgent {
            id,
            vm: CcsVm::new(),
            sandbox,
            max_execution_time: Duration::from_secs(30),
        }
    }

    /// Check if a command is allowed.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        match &self.sandbox {
            SandboxMode::None => true,
            SandboxMode::CommandFilter(allowed) => {
                allowed.iter().any(|c| c == command)
            }
            SandboxMode::Container | SandboxMode::Vm => {
                // Future: check against container/VM allowlist
                true
            }
        }
    }

    /// Run a program on the CLI agent's VM.
    pub fn run_program(&mut self, program: SigmaProgram) -> Result<SigmaProgram, AgentError> {
        let pid = program.id;
        let start = Instant::now();

        self.vm.load(program);
        let status = self.vm.run().map_err(|e| {
            AgentError::ProgramCrash {
                program_id: pid,
                reason: e.to_string(),
            }
        })?;

        if start.elapsed() > self.max_execution_time {
            return Err(AgentError::Timeout {
                timeout: self.max_execution_time,
            });
        }

        match status {
            VmStatus::Halted | VmStatus::Yield => {
                Ok(SigmaProgram::new())
            }
            _ => Err(AgentError::ProgramCrash {
                program_id: pid,
                reason: "unexpected VM status".into(),
            }),
        }
    }
}

impl Agent for CliAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Cli
    }

    fn execute(&self, _program: Packet) -> Result<Packet, AgentError> {
        // Phase 0: accept raw packet, return raw result
        Ok(Packet::Raw(vec![]))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: "idle".to_string(),
            current_program: None,
            ip: Some(self.vm.ip),
            world_graph_size: self.vm.world_graph.node_count(),
            memory_trace_length: self.vm.memory_trace.len(),
            uptime: self.vm.uptime(),
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Execute,
            Capability::FileSystem,
            Capability::Network,
            Capability::Shell,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::agent_id::AgentId;

    #[test]
    fn test_new_cli_agent() {
        let agent = CliAgent::new(AgentId::new("cli-1"));
        assert_eq!(agent.id(), AgentId::new("cli-1"));
        assert_eq!(agent.agent_type(), AgentType::Cli);
    }

    #[test]
    fn test_command_allowed() {
        let agent = CliAgent::new(AgentId::new("cli-1"));
        assert!(agent.is_command_allowed("ls"));
        assert!(!agent.is_command_allowed("rm"));
    }

    #[test]
    fn test_command_allowed_none_sandbox() {
        let agent = CliAgent::with_sandbox(
            AgentId::new("cli-2"),
            SandboxMode::None,
        );
        assert!(agent.is_command_allowed("rm"));
        assert!(agent.is_command_allowed("anything"));
    }

    #[test]
    fn test_capabilities() {
        let agent = CliAgent::new(AgentId::new("cli-1"));
        let caps = agent.capabilities();
        assert!(caps.contains(&Capability::Execute));
        assert!(caps.contains(&Capability::FileSystem));
        assert!(caps.contains(&Capability::Shell));
    }
}
