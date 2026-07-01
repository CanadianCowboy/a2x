// See plans/05-agents.md §4

use std::time::{Duration, Instant};

use a2x_ccs::CcsVm;
use a2x_core::error::AgentError;
use a2x_core::program_id::ProgramId;

/// Agent lifecycle state.
pub enum AgentState {
    /// Agent is initialized but idle, waiting for a program.
    Idle,
    /// Agent is executing a program. Per plans/05-agents.md §4, Running
    /// carries a reference to the VM so the agent lifecycle manager can
    /// inspect or pause the executing VM.
    Running {
        /// The program being executed.
        program_id: ProgramId,
        /// When execution started.
        started_at: Instant,
        /// The CCS VM executing the program (T3-5: plan compliance).
        /// None only after cloning — clones drop the VM reference.
        vm: Option<Box<CcsVm>>,
    },
    /// Agent encountered a recoverable error.
    Error {
        /// The error that occurred.
        error: String,
        /// How many times we've retried.
        retry_count: u32,
    },
    /// Agent is permanently stopped.
    Halted,
    /// Agent is terminated (can be restarted).
    Dead,
}

use std::fmt;

impl Clone for AgentState {
    fn clone(&self) -> Self {
        match self {
            AgentState::Idle => AgentState::Idle,
            AgentState::Running {
                program_id,
                started_at,
                ..
            } => AgentState::Running {
                program_id: *program_id,
                started_at: *started_at,
                vm: None, // VM is not cloneable — informational clones omit it
            },
            AgentState::Error { error, retry_count } => AgentState::Error {
                error: error.clone(),
                retry_count: *retry_count,
            },
            AgentState::Halted => AgentState::Halted,
            AgentState::Dead => AgentState::Dead,
        }
    }
}

impl fmt::Debug for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentState::Idle => write!(f, "Idle"),
            AgentState::Running {
                program_id,
                started_at,
                vm,
            } => f
                .debug_struct("Running")
                .field("program_id", program_id)
                .field("started_at", started_at)
                .field("vm", &vm.as_ref().map(|_| "CcsVm(...)"))
                .finish(),
            AgentState::Error { error, retry_count } => f
                .debug_struct("Error")
                .field("error", error)
                .field("retry_count", retry_count)
                .finish(),
            AgentState::Halted => write!(f, "Halted"),
            AgentState::Dead => write!(f, "Dead"),
        }
    }
}

impl AgentState {
    /// Returns true if the agent can accept new programs.
    pub fn can_accept(&self) -> bool {
        matches!(self, AgentState::Idle)
    }

    /// Returns true if the agent is actively executing.
    pub fn is_running(&self) -> bool {
        matches!(self, AgentState::Running { .. })
    }

    /// Returns the program ID being executed, if any.
    pub fn current_program(&self) -> Option<ProgramId> {
        match self {
            AgentState::Running { program_id, .. } => Some(*program_id),
            _ => None,
        }
    }
}

/// Manages agent state transitions and error recovery.
pub struct AgentLifecycle {
    /// Current agent state.
    pub state: AgentState,
    /// Maximum number of retries for recoverable errors.
    pub max_retries: u32,
    /// Heartbeat interval for announcing liveness on the bus.
    pub heartbeat_interval: Duration,
    /// When the last heartbeat was sent.
    pub last_heartbeat: Instant,
    /// When the agent was started.
    pub started_at: Instant,
}

impl AgentLifecycle {
    /// Create a new idle lifecycle.
    pub fn new(max_retries: u32, heartbeat_interval: Duration) -> Self {
        let now = Instant::now();
        AgentLifecycle {
            state: AgentState::Idle,
            max_retries,
            heartbeat_interval,
            last_heartbeat: now,
            started_at: now,
        }
    }

    /// Transition to Running state with the given VM.
    pub fn start_program(
        &mut self,
        program_id: ProgramId,
        vm: Option<Box<CcsVm>>,
    ) -> Result<(), AgentError> {
        if !self.state.can_accept() {
            return Err(AgentError::AtCapacity { max: 1 });
        }
        self.state = AgentState::Running {
            program_id,
            started_at: Instant::now(),
            vm,
        };
        Ok(())
    }

    /// Transition back to Idle after successful execution.
    pub fn complete_program(&mut self) {
        self.state = AgentState::Idle;
    }

    /// Handle an error — either retry or escalate.
    pub fn handle_error(&mut self, error: &str) -> Result<(), AgentError> {
        let retry_count = match &self.state {
            AgentState::Error { retry_count, .. } => *retry_count,
            _ => 0,
        };

        if retry_count < self.max_retries {
            self.state = AgentState::Error {
                error: error.to_string(),
                retry_count: retry_count + 1,
            };
            Ok(())
        } else {
            self.state = AgentState::Halted;
            Err(AgentError::SafetyViolation(error.to_string()))
        }
    }

    /// Gracefully halt the agent.
    pub fn halt(&mut self) {
        self.state = AgentState::Halted;
    }

    /// Mark the agent as dead.
    pub fn mark_dead(&mut self) {
        self.state = AgentState::Dead;
    }

    /// Check if it's time to send a heartbeat.
    pub fn should_heartbeat(&self) -> bool {
        self.last_heartbeat.elapsed() >= self.heartbeat_interval
    }

    /// Record that a heartbeat was sent.
    pub fn heartbeat_sent(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// Get uptime since agent start.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

impl Default for AgentLifecycle {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_program_id() -> ProgramId {
        ProgramId::new([1u8; 32])
    }

    #[test]
    fn test_initial_state_is_idle() {
        let lc = AgentLifecycle::default();
        assert!(lc.state.can_accept());
        assert!(matches!(lc.state, AgentState::Idle));
    }

    #[test]
    fn test_start_program() {
        let mut lc = AgentLifecycle::default();
        let pid = test_program_id();
        lc.start_program(pid, None).unwrap();
        assert!(lc.state.is_running());
        assert_eq!(lc.state.current_program(), Some(pid));
    }

    #[test]
    fn test_start_program_with_vm() {
        let mut lc = AgentLifecycle::default();
        let pid = test_program_id();
        let vm = Box::new(CcsVm::new());
        lc.start_program(pid, Some(vm)).unwrap();
        assert!(lc.state.is_running());
        // Verify Running carries the VM
        match &lc.state {
            AgentState::Running { vm, .. } => assert!(vm.is_some()),
            _ => panic!("expected Running"),
        }
    }

    #[test]
    fn test_cannot_start_when_running() {
        let mut lc = AgentLifecycle::default();
        lc.start_program(test_program_id(), None).unwrap();
        assert!(lc.start_program(test_program_id(), None).is_err());
    }

    #[test]
    fn test_complete_program() {
        let mut lc = AgentLifecycle::default();
        lc.start_program(test_program_id(), None).unwrap();
        lc.complete_program();
        assert!(matches!(lc.state, AgentState::Idle));
    }

    #[test]
    fn test_clone_running_drops_vm() {
        let mut lc = AgentLifecycle::default();
        let pid = test_program_id();
        let vm = Box::new(CcsVm::new());
        lc.start_program(pid, Some(vm)).unwrap();
        let cloned = lc.state.clone();
        // Clone should have vm = None (VM not cloneable)
        match cloned {
            AgentState::Running { vm, .. } => assert!(vm.is_none()),
            _ => panic!("expected Running"),
        }
    }

    #[test]
    fn test_should_heartbeat() {
        let lc = AgentLifecycle::new(3, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(lc.should_heartbeat());
    }

    #[test]
    fn test_halt_and_dead() {
        let mut lc = AgentLifecycle::default();
        lc.halt();
        assert!(matches!(lc.state, AgentState::Halted));
        lc.mark_dead();
        assert!(matches!(lc.state, AgentState::Dead));
    }
}
