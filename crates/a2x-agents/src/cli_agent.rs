// See plans/05-agents.md §3 and §5 (CLI Agent with sandboxing)
//
// Phase 1: Real execution — parses Sigma programs from packets, runs them
// on the CCS VM, and returns output programs.

use std::sync::Mutex;
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
///
/// Phase 1: Uses `Mutex<CcsVm>` for thread-safe interior mutability so
/// `execute(&self)` can run programs without requiring `&mut self`.
pub struct CliAgent {
    /// Agent identity.
    id: AgentId,
    /// Internal CCS VM for execution (thread-safe interior mutability).
    vm: Mutex<CcsVm>,
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
            vm: Mutex::new(CcsVm::new()),
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
            vm: Mutex::new(CcsVm::new()),
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
    ///
    /// Loads the program into the VM, executes it to completion, and returns
    /// the output program (extracted from the last instruction's D field).
    ///
    /// **Note:** Must not be called concurrently with `execute()`. In practice
    /// this is guaranteed because the bus processes messages sequentially per agent.
    pub fn run_program(&self, program: SigmaProgram) -> Result<SigmaProgram, AgentError> {
        let pid = program.id;
        let start = Instant::now();

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

        if start.elapsed() > self.max_execution_time {
            return Err(AgentError::Timeout {
                timeout: self.max_execution_time,
            });
        }

        match status {
            VmStatus::Halted | VmStatus::Yield => {
                // Extract output from the program's D field
                let program = vm.program().cloned().unwrap_or_default();
                Ok(program.output())
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

    fn execute(&self, program: Packet) -> Result<Packet, AgentError> {
        match program {
            Packet::Raw(bytes) => {
                // Parse raw bytes as Sigma text, execute, return result
                let text = String::from_utf8(bytes)
                    .map_err(|e| AgentError::ProgramCrash {
                        program_id: a2x_core::ProgramId::zero(),
                        reason: format!("invalid UTF-8 in packet: {}", e),
                    })?;

                let sigma_prog = a2x_sigma::parse_program(&text).map_err(|e| {
                    AgentError::ProgramCrash {
                        program_id: a2x_core::ProgramId::zero(),
                        reason: format!("parse error: {}", e),
                    }
                })?;

                let result = self.run_program(sigma_prog)?;

                // Serialize the result program back as raw bytes
                let output_text = result
                    .instructions
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                Ok(Packet::Raw(output_text.into_bytes()))
            }
        }
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let vm = self.vm.lock().ok()?;
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: "idle".to_string(),
            current_program: None,
            ip: Some(vm.ip),
            world_graph_size: vm.world_graph.node_count(),
            memory_trace_length: vm.memory_trace.len(),
            uptime: vm.uptime(),
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

    #[test]
    fn test_execute_nop_program() {
        use a2x_sigma::SigmaPacket;

        let agent = CliAgent::new(AgentId::new("cli-test"));
        let packet = SigmaPacket::default();
        let mut prog = SigmaProgram::new();
        prog.push(packet);

        let result = agent.run_program(prog);
        assert!(result.is_ok(), "NOP program should execute successfully");
    }

    #[test]
    fn test_execute_valid_packet() {
        let agent = CliAgent::new(AgentId::new("cli-test"));
        let sigma_text = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨test⟩ ∷ P:⥂ ∷ D:⌵⟭";
        let packet = Packet::Raw(sigma_text.as_bytes().to_vec());
        let result = agent.execute(packet);
        assert!(result.is_ok(), "valid Sigma packet should execute");
    }

    #[test]
    fn test_execute_invalid_packet() {
        let agent = CliAgent::new(AgentId::new("cli-test"));
        // Malformed Sigma: has boundaries but missing protocol identifier
        let packet = Packet::Raw("⟦not valid sigma⟧".as_bytes().to_vec());
        let result = agent.execute(packet);
        assert!(result.is_err(), "invalid packet should fail");
    }

    #[test]
    fn test_execute_unknown_character() {
        let agent = CliAgent::new(AgentId::new("cli-test"));
        // § is not a valid Sigma character
        let packet = Packet::Raw(b"\xc2\xa7".to_vec());
        let result = agent.execute(packet);
        assert!(result.is_err(), "unknown character should fail");
    }
}
