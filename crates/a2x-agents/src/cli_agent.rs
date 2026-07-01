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

use crate::parse::{packet_to_sigma_program, sigma_program_to_packet};

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

/// Resource limits for CLI agent execution.
/// See plans/12-security.md §5 — host system resource enforcement.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum CPU time per program execution.
    pub max_cpu_time: Duration,
    /// Maximum estimated memory usage in bytes (world graph nodes + memory trace).
    pub max_memory_bytes: u64,
    /// Maximum output program size in bytes.
    pub max_output_size: usize,
    /// Maximum number of concurrently running programs.
    pub max_concurrent_processes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_cpu_time: Duration::from_secs(30),
            max_memory_bytes: 64 * 1024 * 1024, // 64 MiB
            max_output_size: 10 * 1024 * 1024,  // 10 MiB
            max_concurrent_processes: 8,
        }
    }
}

/// Default set of dangerous commands that are always forbidden.
/// See plans/12-security.md §5 — safety denylist for destructive shell commands.
const DEFAULT_FORBIDDEN_COMMANDS: &[&str] = &[
    "rm", "sudo", "chmod", "dd", "mkfs", "shutdown", "reboot", "kill",
];

fn default_forbidden_commands() -> Vec<String> {
    DEFAULT_FORBIDDEN_COMMANDS
        .iter()
        .map(|s| s.to_string())
        .collect()
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
    /// Commands that are always forbidden, even in CommandFilter mode.
    /// See plans/12-security.md §5 — safety denylist for dangerous commands.
    forbidden_commands: Vec<String>,
    /// Resource limits: memory, output size, concurrent processes, CPU time.
    /// See plans/12-security.md §5 — host system resource enforcement.
    resource_limits: ResourceLimits,
}

impl CliAgent {
    /// Create a new CLI agent with default sandboxing and denylist.
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
            forbidden_commands: default_forbidden_commands(),
            resource_limits: ResourceLimits::default(),
        }
    }

    /// Create with custom sandbox configuration.
    pub fn with_sandbox(id: AgentId, sandbox: SandboxMode) -> Self {
        CliAgent {
            id,
            vm: Mutex::new(CcsVm::new()),
            sandbox,
            max_execution_time: Duration::from_secs(30),
            forbidden_commands: default_forbidden_commands(),
            resource_limits: ResourceLimits::default(),
        }
    }

    /// Create with custom resource limits.
    pub fn with_resource_limits(id: AgentId, limits: ResourceLimits) -> Self {
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
            max_execution_time: limits.max_cpu_time,
            forbidden_commands: default_forbidden_commands(),
            resource_limits: limits,
        }
    }

    /// Check if a command is allowed, considering both the allowlist and denylist.
    ///
    /// The denylist is always checked first — a forbidden command is rejected
    /// regardless of sandbox mode.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Denylist always takes precedence (security-critical).
        if self.forbidden_commands.iter().any(|c| c == command) {
            return false;
        }
        match &self.sandbox {
            SandboxMode::None => true,
            SandboxMode::CommandFilter(allowed) => allowed.iter().any(|c| c == command),
            SandboxMode::Container | SandboxMode::Vm => {
                // Future: check against container/VM allowlist
                true
            }
        }
    }

    /// Estimate memory usage of the VM in bytes.
    ///
    /// Uses the same heuristic as `SafetyConstraints::record_allocation()`:
    /// ~4 KiB per world graph node + ~1 KiB per memory trace entry.
    fn estimate_memory_usage(vm: &CcsVm) -> u64 {
        let node_bytes = vm.world_graph.node_count() as u64 * 4096;
        let trace_bytes = vm.memory_trace.len() as u64 * 1024;
        node_bytes.saturating_add(trace_bytes)
    }

    /// Check resource limits before or after execution. Returns Err if any limit is exceeded.
    ///
    /// `label` describes what's being checked (e.g., "pre-execution VM state")
    /// for clear error messages.
    fn check_resource_limits(
        &self,
        label: &str,
        program_id: a2x_core::program_id::ProgramId,
        vm: &CcsVm,
    ) -> Result<(), AgentError> {
        let mem_used = Self::estimate_memory_usage(vm);
        if mem_used >= self.resource_limits.max_memory_bytes {
            return Err(AgentError::ResourceLimitExceeded {
                program_id,
                limit: format!("memory ({})", label),
                used: mem_used,
                max: self.resource_limits.max_memory_bytes,
            });
        }
        Ok(())
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

        // Pre-execution: check accumulated memory limit.
        self.check_resource_limits("pre-execution VM state", pid, &vm)?;

        vm.load(program);
        let status = vm.run().map_err(|e| AgentError::ProgramCrash {
            program_id: pid,
            reason: e.to_string(),
        })?;

        // Post-execution: check memory limit (may have grown).
        self.check_resource_limits("post-execution", pid, &vm)?;

        // Post-execution: check CPU time.
        if start.elapsed() > self.max_execution_time {
            return Err(AgentError::Timeout {
                timeout: self.max_execution_time,
            });
        }

        match status {
            VmStatus::Halted | VmStatus::Yield | VmStatus::Suspended => {
                // Extract output from the program's D field
                let output_program = vm.program().cloned().unwrap_or_default();
                let output = output_program.output();

                // Post-execution: check output size.
                let output_size: usize = output
                    .instructions
                    .iter()
                    .map(|inst| inst.data.payload.len())
                    .sum();
                if output_size > self.resource_limits.max_output_size {
                    return Err(AgentError::OutputTooLarge {
                        program_id: pid,
                        size_bytes: output_size as u64,
                        max_bytes: self.resource_limits.max_output_size as u64,
                    });
                }

                Ok(output)
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
        let sigma_prog = packet_to_sigma_program(program)?;
        let result = self.run_program(sigma_prog)?;
        Ok(sigma_program_to_packet(&result))
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
        let agent = CliAgent::with_sandbox(AgentId::new("cli-2"), SandboxMode::None);
        // Denylist always takes precedence — even in None sandbox,
        // "rm" is forbidden. Use a non-forbidden command instead.
        assert!(!agent.is_command_allowed("rm"));
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

    // ── Resource limit tests ──────────────────────────────────────────

    #[test]
    fn test_memory_limit_rejects_pre_execution() {
        let limits = ResourceLimits {
            max_memory_bytes: 0, // No memory allowed
            ..ResourceLimits::default()
        };
        let agent = CliAgent::with_resource_limits(AgentId::new("cli-mem"), limits);

        let packet = a2x_sigma::SigmaPacket::default();
        let mut prog = SigmaProgram::new();
        prog.push(packet);

        let result = agent.run_program(prog);
        assert!(result.is_err(), "should reject with zero memory limit");
        assert!(
            matches!(result, Err(AgentError::ResourceLimitExceeded { .. })),
            "should be ResourceLimitExceeded"
        );
    }

    #[test]
    fn test_output_size_limit_rejects_large_output() {
        let limits = ResourceLimits {
            max_output_size: 1, // Only 1 byte of output allowed
            ..ResourceLimits::default()
        };
        let agent = CliAgent::with_resource_limits(AgentId::new("cli-out"), limits);

        // Create a program with a data payload that will become output.
        let mut packet = a2x_sigma::SigmaPacket::default();
        packet.data.payload = vec![0u8; 100]; // 100 bytes of output
        let mut prog = SigmaProgram::new();
        prog.push(packet);

        let result = agent.run_program(prog);
        assert!(result.is_err(), "should reject large output");
        assert!(
            matches!(result, Err(AgentError::OutputTooLarge { .. })),
            "should be OutputTooLarge, got {:?}",
            result
        );
    }

    #[test]
    fn test_default_limits_allow_normal_execution() {
        let agent = CliAgent::new(AgentId::new("cli-default"));
        let packet = a2x_sigma::SigmaPacket::default();
        let mut prog = SigmaProgram::new();
        prog.push(packet);

        let result = agent.run_program(prog);
        assert!(
            result.is_ok(),
            "default limits should allow normal execution"
        );
    }

    #[test]
    fn test_with_resource_limits_sets_max_execution_time() {
        let limits = ResourceLimits {
            max_cpu_time: Duration::from_secs(5),
            ..ResourceLimits::default()
        };
        let agent = CliAgent::with_resource_limits(AgentId::new("cli-cpu"), limits);
        assert_eq!(agent.max_execution_time, Duration::from_secs(5));
    }
}
