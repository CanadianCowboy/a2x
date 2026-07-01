// See plans/14-resilience.md — Graceful Degradation, Fault Tolerance & Crash Recovery
//
// This module implements the core resilience infrastructure:
//   - AgentSupervisor: heartbeat-based crash detection + exponential backoff restart
//   - ProgramWatchdog: wall-clock timeout on program execution
//   - InstructionFaultMode: per-instruction fault policies (retry/skip/fallback)
//   - ResourceMonitor: memory pressure, disk space, throttling
//   - RetryPolicy: exponential backoff with jitter

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};

// ── Retry Policy ──────────────────────────────────────────────────────────

/// Policy for retrying failed operations.
#[derive(Clone, Debug, PartialEq)]
pub enum RetryPolicy {
    /// No retry — fail immediately.
    None,
    /// Fixed number of retries with constant delay.
    Fixed { max_retries: u32, delay: Duration },
    /// Exponential backoff with optional jitter.
    Exponential {
        max_retries: u32,
        base_delay: Duration,
        max_delay: Duration,
        /// Random jitter factor (0.0–1.0). 0.1 = ±10% jitter.
        jitter: f32,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::Exponential {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: 0.1,
        }
    }
}

impl RetryPolicy {
    /// Compute the delay before the next retry attempt.
    /// Returns None if retries are exhausted.
    pub fn next_delay(&self, attempt: u32) -> Option<Duration> {
        match self {
            RetryPolicy::None => None,
            RetryPolicy::Fixed { max_retries, delay } => {
                if attempt >= *max_retries {
                    None
                } else {
                    Some(*delay)
                }
            }
            RetryPolicy::Exponential {
                max_retries,
                base_delay,
                max_delay,
                jitter,
            } => {
                if attempt >= *max_retries {
                    return None;
                }
                let base_ms = base_delay.as_millis() as f64 * 2f64.powi(attempt as i32);
                let clamped = base_ms.min(max_delay.as_millis() as f64);
                // Apply jitter: ±jitter * clamped
                let jitter_range = clamped * (*jitter as f64);
                let jittered = clamped + (fast_rand() * 2.0 - 1.0) * jitter_range;
                Some(Duration::from_millis(jittered.max(0.0) as u64))
            }
        }
    }
}

/// Simple fast pseudo-random float in [0, 1) for jitter (no rand dep).
fn fast_rand() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // Simple LCG: X_{n+1} = (a * X_n + c) mod m
    let x = (nanos as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    (x as f64) / (u64::MAX as f64)
}

// ── Agent Supervisor ──────────────────────────────────────────────────────

/// Monitors agent health and restarts crashed agents.
///
/// Uses a heartbeat-based detection model: if an agent misses
/// `threshold` heartbeats at `interval`, it's declared dead and restarted.
pub struct AgentSupervisor {
    /// Agents being monitored, keyed by ID.
    agents: HashMap<String, SupervisedAgent>,
    /// Maximum restarts per minute before giving up.
    max_restarts_per_minute: u32,
    /// Base delay for exponential backoff on restart.
    restart_base_delay: Duration,
    /// Maximum delay between restart attempts.
    restart_max_delay: Duration,
}

struct SupervisedAgent {
    /// When the last heartbeat was received.
    last_heartbeat: Instant,
    /// Number of consecutive missed heartbeats.
    missed_beats: u32,
    /// Restart count in the current window.
    restart_count: u32,
    /// When the restart window started.
    restart_window_start: Instant,
}

impl AgentSupervisor {
    /// Create a new agent supervisor.
    pub fn new(max_restarts_per_minute: u32) -> Self {
        AgentSupervisor {
            agents: HashMap::new(),
            max_restarts_per_minute,
            restart_base_delay: Duration::from_secs(1),
            restart_max_delay: Duration::from_secs(60),
        }
    }

    /// Register an agent for supervision.
    pub fn register(&mut self, agent_id: &str) {
        self.agents.insert(
            agent_id.to_string(),
            SupervisedAgent {
                last_heartbeat: Instant::now(),
                missed_beats: 0,
                restart_count: 0,
                restart_window_start: Instant::now(),
            },
        );
        debug!("Agent '{}' registered for supervision", agent_id);
    }

    /// Record a heartbeat from a supervised agent.
    pub fn heartbeat(&mut self, agent_id: &str) -> bool {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.last_heartbeat = Instant::now();
            agent.missed_beats = 0;
            true
        } else {
            false
        }
    }

    /// Check all agents for missed heartbeats.
    ///
    /// Returns a list of agent IDs that need to be restarted.
    /// Call this periodically (e.g., every N seconds).
    pub fn check(&mut self, interval: Duration, threshold: u32) -> Vec<String> {
        let now = Instant::now();
        let mut dead_agents = Vec::new();

        for (id, agent) in self.agents.iter_mut() {
            if agent.last_heartbeat.elapsed() > interval {
                agent.missed_beats += 1;
                debug!(
                    "Agent '{}' missed heartbeat {} / {}",
                    id, agent.missed_beats, threshold
                );
            }
            if agent.missed_beats >= threshold {
                // Agent is dead — check if we should restart
                if agent.restart_window_start.elapsed() > Duration::from_secs(60) {
                    // Reset restart window
                    agent.restart_count = 0;
                    agent.restart_window_start = now;
                }
                if agent.restart_count < self.max_restarts_per_minute {
                    dead_agents.push(id.clone());
                    agent.restart_count += 1;
                    agent.missed_beats = 0;
                    agent.last_heartbeat = now; // reset to avoid immediate re-detection
                } else {
                    error!(
                        "Agent '{}' exceeded max restarts ({} per minute) — giving up",
                        id, self.max_restarts_per_minute
                    );
                }
            }
        }

        dead_agents
    }

    /// Compute the restart delay for an agent using exponential backoff.
    pub fn restart_delay(&self, agent_id: &str) -> Duration {
        let attempt = self
            .agents
            .get(agent_id)
            .map(|a| a.restart_count)
            .unwrap_or(0);

        let base_ms = self.restart_base_delay.as_millis() as f64;
        let delay_ms = base_ms * 2f64.powi(attempt as i32);
        let clamped = delay_ms.min(self.restart_max_delay.as_millis() as f64);
        Duration::from_millis(clamped as u64)
    }

    /// Check if a specific agent is alive (has not exceeded threshold).
    pub fn is_alive(&self, agent_id: &str, interval: Duration, threshold: u32) -> bool {
        self.agents
            .get(agent_id)
            .map(|a| a.missed_beats < threshold || a.last_heartbeat.elapsed() <= interval)
            .unwrap_or(false)
    }

    /// Remove an agent from supervision.
    pub fn unregister(&mut self, agent_id: &str) {
        self.agents.remove(agent_id);
        info!("Agent '{}' removed from supervision", agent_id);
    }

    /// Get the number of supervised agents.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

// ── Program Watchdog ──────────────────────────────────────────────────────

/// What to do when a program exceeds its time limit.
#[derive(Clone, Debug, PartialEq)]
pub enum TimeoutAction {
    /// Kill the program immediately, return error.
    Kill,
    /// Yield the program (save state, move to back of queue).
    Yield,
    /// Escalate to a supervisor agent for human review.
    Escalate,
}

/// Watches program execution and enforces wall-clock time limits.
pub struct ProgramWatchdog {
    /// Maximum wall-clock time per program.
    max_execution_time: Duration,
    /// Maximum VM steps per program.
    max_instructions: u64,
    /// Action to take on timeout.
    on_timeout: TimeoutAction,
    /// When the current program started.
    started: Option<Instant>,
    /// Instructions executed so far.
    instructions_executed: u64,
    /// Whether the watchdog has triggered.
    triggered: bool,
}

impl ProgramWatchdog {
    /// Create a new watchdog.
    pub fn new(
        max_execution_time: Duration,
        max_instructions: u64,
        on_timeout: TimeoutAction,
    ) -> Self {
        ProgramWatchdog {
            max_execution_time,
            max_instructions,
            on_timeout,
            started: None,
            instructions_executed: 0,
            triggered: false,
        }
    }

    /// Start watching a new program.
    pub fn start(&mut self) {
        self.started = Some(Instant::now());
        self.instructions_executed = 0;
        self.triggered = false;
    }

    /// Check if the program has exceeded its limits.
    /// Call this after each instruction or periodically.
    pub fn check(&mut self) -> Result<(), WatchdogError> {
        if self.triggered {
            return Err(WatchdogError::AlreadyTriggered);
        }

        if let Some(started) = self.started {
            // Check wall-clock timeout
            if started.elapsed() > self.max_execution_time {
                self.triggered = true;
                return Err(WatchdogError::Timeout {
                    elapsed: started.elapsed(),
                    limit: self.max_execution_time,
                    action: self.on_timeout.clone(),
                });
            }

            // Check instruction limit (exclusive upper bound: allow up to max_instructions)
            if self.instructions_executed > self.max_instructions {
                self.triggered = true;
                return Err(WatchdogError::InstructionLimit {
                    executed: self.instructions_executed,
                    limit: self.max_instructions,
                });
            }
        }

        Ok(())
    }

    /// Record that an instruction was executed.
    pub fn step(&mut self) {
        self.instructions_executed = self.instructions_executed.saturating_add(1);
    }

    /// Get the elapsed time for the current program.
    pub fn elapsed(&self) -> Option<Duration> {
        self.started.map(|s| s.elapsed())
    }

    /// Get the number of instructions executed.
    pub fn instructions(&self) -> u64 {
        self.instructions_executed
    }

    /// Get the timeout action.
    pub fn timeout_action(&self) -> &TimeoutAction {
        &self.on_timeout
    }

    /// Whether the watchdog has triggered.
    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    /// Reset the watchdog for a new program.
    pub fn reset(&mut self) {
        self.started = None;
        self.instructions_executed = 0;
        self.triggered = false;
    }
}

/// Error returned when the watchdog triggers.
#[derive(Clone, Debug, PartialEq)]
pub enum WatchdogError {
    Timeout {
        elapsed: Duration,
        limit: Duration,
        action: TimeoutAction,
    },
    InstructionLimit {
        executed: u64,
        limit: u64,
    },
    AlreadyTriggered,
}

impl std::fmt::Display for WatchdogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchdogError::Timeout { elapsed, limit, .. } => {
                write!(
                    f,
                    "watchdog timeout: {:?} elapsed, {:?} limit",
                    elapsed, limit
                )
            }
            WatchdogError::InstructionLimit { executed, limit } => {
                write!(
                    f,
                    "instruction limit: {} executed, {} limit",
                    executed, limit
                )
            }
            WatchdogError::AlreadyTriggered => {
                write!(f, "watchdog already triggered")
            }
        }
    }
}

// ── Instruction Fault Mode ────────────────────────────────────────────────

/// How the VM should handle a fault at the instruction level.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum InstructionFaultMode {
    /// Stop the program, return error (default).
    #[default]
    FailFast,
    /// Skip the faulting instruction and continue.
    SkipAndContinue {
        /// Maximum consecutive skips before failing.
        max_consecutive_skips: u32,
        /// Current consecutive skips (internal counter).
        consecutive_skips: u32,
    },
    /// Retry the instruction N times with a policy.
    Retry {
        policy: RetryPolicy,
        /// Current retry attempt (internal, caller resets to 0).
        attempt: u32,
    },
    /// Execute a fallback instruction string (Σ∞ source text).
    Fallback {
        /// Σ∞ source for the fallback instruction.
        fallback_source: String,
    },
}

impl InstructionFaultMode {
    /// Determine what to do when an instruction faults.
    /// Returns Some(action) if the instruction should be retried/skipped,
    /// or None if the fault should be escalated.
    pub fn handle_fault(&mut self, error: &str) -> FaultAction {
        match self {
            InstructionFaultMode::FailFast => {
                error!("Instruction fault (FailFast): {error}");
                FaultAction::Escalate
            }
            InstructionFaultMode::SkipAndContinue {
                max_consecutive_skips,
                ref mut consecutive_skips,
            } => {
                *consecutive_skips += 1;
                if *consecutive_skips > *max_consecutive_skips {
                    error!(
                        "Instruction fault: {} consecutive skips exceeded limit ({})",
                        consecutive_skips, max_consecutive_skips
                    );
                    FaultAction::Escalate
                } else {
                    warn!(
                        "Instruction fault: skipping (consecutive skip {}/{}, error: {error})",
                        consecutive_skips, max_consecutive_skips
                    );
                    FaultAction::Skip
                }
            }
            InstructionFaultMode::Retry { policy, attempt } => {
                let current = *attempt;
                match policy.next_delay(current) {
                    Some(delay) => {
                        *attempt += 1;
                        warn!(
                            "Instruction fault: retrying (attempt {}/{})",
                            *attempt,
                            match policy {
                                RetryPolicy::Fixed { max_retries, .. }
                                | RetryPolicy::Exponential { max_retries, .. } => max_retries,
                                RetryPolicy::None => &0,
                            }
                        );
                        FaultAction::Retry { delay }
                    }
                    None => {
                        error!("Instruction fault: retries exhausted");
                        FaultAction::Escalate
                    }
                }
            }
            InstructionFaultMode::Fallback { fallback_source } => {
                warn!("Instruction fault: using fallback: {fallback_source}");
                FaultAction::Fallback {
                    source: fallback_source.clone(),
                }
            }
        }
    }
}

/// Action to take after an instruction fault.
#[derive(Clone, Debug, PartialEq)]
pub enum FaultAction {
    /// Escalate the fault (stop the program).
    Escalate,
    /// Skip the current instruction and continue.
    Skip,
    /// Retry the instruction after a delay.
    Retry { delay: Duration },
    /// Execute a fallback instruction.
    Fallback { source: String },
}

// ── Resource Monitor ──────────────────────────────────────────────────────

/// Threshold and action for resource pressure.
#[derive(Clone, Debug, PartialEq)]
pub enum MemoryPressureAction {
    /// Log a warning when usage exceeds the threshold (0.0–1.0).
    Warn(f32),
    /// Throttle new program submissions when usage exceeds threshold.
    Throttle(f32),
    /// Evict least-recently-used data when usage exceeds threshold.
    Evict(f32),
    /// Kill lowest-priority programs when usage exceeds threshold.
    Kill(f32),
}

/// Monitors system resource usage and takes action under pressure.
pub struct ResourceMonitor {
    /// Physical memory limit in bytes (0 = auto-detect).
    memory_limit_bytes: u64,
    /// Disk space limit in bytes (0 = auto-detect).
    disk_limit_bytes: u64,
    /// Actions for memory pressure at different thresholds.
    memory_actions: Vec<(f32, MemoryPressureAction)>,
    /// Whether the system is currently throttled.
    throttled: bool,
    /// Whether eviction is active.
    evicting: bool,
    /// Whether programs are being killed.
    killing: bool,
    /// Current memory usage fraction (cached from last check).
    memory_usage: f32,
    /// Current disk usage fraction (cached from last check).
    disk_usage: f32,
}

impl ResourceMonitor {
    /// Create a new resource monitor with default thresholds.
    pub fn new() -> Self {
        ResourceMonitor {
            memory_limit_bytes: 0,
            disk_limit_bytes: 0,
            memory_actions: vec![
                (0.7, MemoryPressureAction::Warn(0.7)),
                (0.8, MemoryPressureAction::Throttle(0.8)),
                (0.9, MemoryPressureAction::Evict(0.9)),
                (0.95, MemoryPressureAction::Kill(0.95)),
            ],
            throttled: false,
            evicting: false,
            killing: false,
            memory_usage: 0.0,
            disk_usage: 0.0,
        }
    }

    /// Set the memory limit (0 = auto-detect from system).
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit_bytes = bytes;
        self
    }

    /// Set the disk limit (0 = auto-detect).
    pub fn with_disk_limit(mut self, bytes: u64) -> Self {
        self.disk_limit_bytes = bytes;
        self
    }

    /// Add a memory pressure action.
    pub fn with_action(mut self, threshold: f32, action: MemoryPressureAction) -> Self {
        self.memory_actions.push((threshold, action));
        self.memory_actions
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self
    }

    /// Check system resources and return recommended actions.
    ///
    /// In a real implementation, this would query the OS for memory/disk
    /// usage. For Phase 7, it uses configurable limits and tracks state.
    pub fn check(&mut self, current_memory_bytes: u64, current_disk_bytes: u64) -> ResourceStatus {
        let mem_limit = if self.memory_limit_bytes > 0 {
            self.memory_limit_bytes
        } else {
            // Default: 1 GB for Phase 7
            1024 * 1024 * 1024
        };
        let disk_limit = if self.disk_limit_bytes > 0 {
            self.disk_limit_bytes
        } else {
            // Default: 10 GB
            10 * 1024 * 1024 * 1024
        };

        self.memory_usage = current_memory_bytes as f32 / mem_limit as f32;
        self.disk_usage = current_disk_bytes as f32 / disk_limit as f32;

        let mut status = ResourceStatus::normal();

        // Check memory pressure actions
        for (threshold, action) in &self.memory_actions {
            if self.memory_usage >= *threshold {
                match action {
                    MemoryPressureAction::Warn(_) => {
                        warn!(
                            "Memory pressure: {:.1}% (threshold: {:.0}%)",
                            self.memory_usage * 100.0,
                            threshold * 100.0
                        );
                        status
                            .warnings
                            .push(format!("memory at {:.1}%", self.memory_usage * 100.0));
                    }
                    MemoryPressureAction::Throttle(_) => {
                        if !self.throttled {
                            warn!("Throttling program submissions due to memory pressure");
                            self.throttled = true;
                            status.throttled = true;
                        }
                    }
                    MemoryPressureAction::Evict(_) => {
                        if !self.evicting {
                            warn!("Evicting LRU data due to memory pressure");
                            self.evicting = true;
                            status.evicting = true;
                        }
                    }
                    MemoryPressureAction::Kill(_) => {
                        if !self.killing {
                            error!("Killing low-priority programs due to critical memory");
                            self.killing = true;
                            status.killing = true;
                        }
                    }
                }
            }
        }

        // Check disk space
        if self.disk_usage > 0.9 {
            warn!("Disk space low: {:.1}% used", self.disk_usage * 100.0);
            status
                .warnings
                .push(format!("disk at {:.1}%", self.disk_usage * 100.0));
        }

        // Reset flags if pressure has eased
        if self.memory_usage < 0.5 {
            self.throttled = false;
            self.evicting = false;
            self.killing = false;
        }

        status
    }

    /// Get the current memory usage fraction (0.0–1.0+).
    pub fn memory_usage(&self) -> f32 {
        self.memory_usage
    }

    /// Get the current disk usage fraction.
    pub fn disk_usage(&self) -> f32 {
        self.disk_usage
    }

    /// Whether the system is currently throttled.
    pub fn is_throttled(&self) -> bool {
        self.throttled
    }

    /// Whether eviction is active.
    pub fn is_evicting(&self) -> bool {
        self.evicting
    }

    /// Whether programs are being killed.
    pub fn is_killing(&self) -> bool {
        self.killing
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Status returned by a resource check.
#[derive(Clone, Debug)]
pub struct ResourceStatus {
    /// Warning messages about resource pressure.
    pub warnings: Vec<String>,
    /// Whether new program submissions should be throttled.
    pub throttled: bool,
    /// Whether LRU eviction should be triggered.
    pub evicting: bool,
    /// Whether low-priority programs should be killed.
    pub killing: bool,
}

impl ResourceStatus {
    fn normal() -> Self {
        ResourceStatus {
            warnings: Vec::new(),
            throttled: false,
            evicting: false,
            killing: false,
        }
    }

    /// Returns true if the system is under any pressure.
    pub fn is_under_pressure(&self) -> bool {
        self.throttled || self.evicting || self.killing
    }

    /// Returns true if the system is healthy.
    pub fn is_healthy(&self) -> bool {
        !self.is_under_pressure() && self.warnings.is_empty()
    }
}

// ── Graceful Degradation ──────────────────────────────────────────────────

/// Defines how the system degrades under specific failure modes.
///
/// See plans/14-resilience.md §9 for the full degradation matrix.
#[derive(Clone, Debug, PartialEq)]
pub enum DegradationMode {
    /// Continue operating normally.
    Normal,
    /// Work redistributed to peer agents; agent restarts with backoff.
    AgentCrashed { agent_id: String },
    /// Agents run in isolation, queue messages for later delivery.
    BusDown,
    /// External systems can't connect; gateway restarting.
    GatewayDown,
    /// Each partition continues independently; reconcile on heal.
    NetworkPartition,
    /// Fall back to empty state; restore from backup if available.
    StorageCorrupt,
    /// Throttle new submissions, evict LRU data.
    MemoryFull,
    /// Kill program, return error to caller.
    ProgramTimeout,
}

/// Summary of the system's degradation state.
#[derive(Clone, Debug, Default)]
pub struct DegradationSummary {
    /// Active degradations.
    pub modes: Vec<DegradationMode>,
}

impl DegradationSummary {
    /// Returns true if the system is fully operational.
    pub fn fully_operational(&self) -> bool {
        self.modes.is_empty()
    }

    /// Returns true if there are any active degradations.
    pub fn is_degraded(&self) -> bool {
        !self.modes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RetryPolicy tests ────────────────────────────────────────────────

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::None;
        assert!(policy.next_delay(0).is_none());
    }

    #[test]
    fn test_retry_policy_fixed() {
        let policy = RetryPolicy::Fixed {
            max_retries: 3,
            delay: Duration::from_secs(2),
        };
        assert_eq!(policy.next_delay(0), Some(Duration::from_secs(2)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_secs(2)));
        assert!(policy.next_delay(3).is_none());
    }

    #[test]
    fn test_retry_policy_exponential() {
        let policy = RetryPolicy::Exponential {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: 0.0, // no jitter for deterministic test
        };
        // attempt 0: 1s * 2^0 = 1s
        let d0 = policy.next_delay(0).unwrap();
        assert!(d0 >= Duration::from_millis(1000) && d0 <= Duration::from_millis(1000));

        // attempt 2: 1s * 2^2 = 4s
        let d2 = policy.next_delay(2).unwrap();
        assert!(d2 >= Duration::from_millis(4000) && d2 <= Duration::from_millis(4000));

        // attempt 3: exhausted
        assert!(policy.next_delay(3).is_none());
    }

    #[test]
    fn test_retry_policy_exponential_hits_max() {
        let policy = RetryPolicy::Exponential {
            max_retries: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            jitter: 0.0,
        };
        // attempt 3: 1s * 2^3 = 8s > 5s max → clamped to 5s
        let d = policy.next_delay(3).unwrap();
        assert!(d.as_secs() <= 5);
    }

    // ── AgentSupervisor tests ────────────────────────────────────────────

    #[test]
    fn test_supervisor_register_and_heartbeat() {
        let mut sup = AgentSupervisor::new(5);
        sup.register("agent-1");
        assert!(sup.heartbeat("agent-1"));
        assert!(!sup.heartbeat("unknown"));
    }

    #[test]
    fn test_supervisor_detects_missed_heartbeats() {
        let mut sup = AgentSupervisor::new(5);
        sup.register("agent-1");

        // First check: no missed beats yet (interval not elapsed)
        let dead = sup.check(Duration::from_millis(1), 3);
        assert!(dead.is_empty());

        // Simulate missed heartbeats
        std::thread::sleep(Duration::from_millis(5));
        let dead = sup.check(Duration::from_millis(1), 1); // threshold=1
        assert!(dead.contains(&"agent-1".to_string()));
    }

    #[test]
    fn test_supervisor_exponential_backoff() {
        let mut sup = AgentSupervisor::new(5);
        sup.register("agent-1");

        let d1 = sup.restart_delay("agent-1").as_millis();
        // After a restart, restart_count increments. Let's simulate.
        sup.check(Duration::from_secs(0), 1);
        let d2 = sup.restart_delay("agent-1").as_millis();
        assert!(d2 >= d1, "delay should increase with restart count");
    }

    #[test]
    fn test_supervisor_unregister() {
        let mut sup = AgentSupervisor::new(5);
        sup.register("agent-1");
        assert_eq!(sup.agent_count(), 1);
        sup.unregister("agent-1");
        assert_eq!(sup.agent_count(), 0);
    }

    // ── ProgramWatchdog tests ────────────────────────────────────────────

    #[test]
    fn test_watchdog_new() {
        let wd = ProgramWatchdog::new(Duration::from_secs(30), 10_000, TimeoutAction::Kill);
        assert!(!wd.is_triggered());
        assert_eq!(wd.instructions(), 0);
        assert_eq!(wd.timeout_action(), &TimeoutAction::Kill);
    }

    #[test]
    fn test_watchdog_step_and_check() {
        let mut wd = ProgramWatchdog::new(Duration::from_secs(30), 3, TimeoutAction::Kill);
        wd.start();
        wd.step();
        wd.step();
        assert!(wd.check().is_ok());
        wd.step(); // instruction 3
        assert!(wd.check().is_ok());
        wd.step(); // instruction 4 > limit
        assert!(wd.check().is_err());
        assert!(wd.is_triggered());
    }

    #[test]
    fn test_watchdog_timeout() {
        let mut wd = ProgramWatchdog::new(Duration::from_millis(1), 10_000, TimeoutAction::Yield);
        wd.start();
        std::thread::sleep(Duration::from_millis(5));
        assert!(wd.check().is_err());
        assert_eq!(wd.timeout_action(), &TimeoutAction::Yield);
    }

    #[test]
    fn test_watchdog_reset() {
        let mut wd = ProgramWatchdog::new(Duration::from_secs(30), 1, TimeoutAction::Kill);
        wd.start();
        wd.step();
        wd.step();
        assert!(wd.check().is_err()); // triggered
        wd.reset();
        assert!(!wd.is_triggered());
        assert_eq!(wd.instructions(), 0);
    }

    // ── InstructionFaultMode tests ───────────────────────────────────────

    #[test]
    fn test_fault_mode_fail_fast() {
        let mut mode = InstructionFaultMode::FailFast;
        let action = mode.handle_fault("test error");
        assert_eq!(action, FaultAction::Escalate);
    }

    #[test]
    fn test_fault_mode_skip_and_continue() {
        let mut mode = InstructionFaultMode::SkipAndContinue {
            max_consecutive_skips: 10,
            consecutive_skips: 0,
        };
        let action = mode.handle_fault("skip me");
        assert_eq!(action, FaultAction::Skip);
    }

    #[test]
    fn test_fault_mode_retry() {
        let mut mode = InstructionFaultMode::Retry {
            policy: RetryPolicy::Fixed {
                max_retries: 3,
                delay: Duration::from_millis(100),
            },
            attempt: 0,
        };
        let action = mode.handle_fault("retry me");
        assert!(matches!(action, FaultAction::Retry { .. }));
    }

    #[test]
    fn test_fault_mode_retry_exhausted() {
        let mut mode = InstructionFaultMode::Retry {
            policy: RetryPolicy::Fixed {
                max_retries: 1,
                delay: Duration::from_millis(100),
            },
            attempt: 1, // already at max
        };
        let action = mode.handle_fault("no more retries");
        assert_eq!(action, FaultAction::Escalate);
    }

    #[test]
    fn test_fault_mode_fallback() {
        let mut mode = InstructionFaultMode::Fallback {
            fallback_source: "⟦Σ∞⟧⟬I:✕⟭".into(),
        };
        let action = mode.handle_fault("use fallback");
        match action {
            FaultAction::Fallback { source } => {
                assert!(source.contains("✕"));
            }
            _ => panic!("expected Fallback"),
        }
    }

    // ── ResourceMonitor tests ────────────────────────────────────────────

    #[test]
    fn test_resource_monitor_normal() {
        let mut monitor = ResourceMonitor::new()
            .with_memory_limit(1024 * 1024 * 1024) // 1 GB
            .with_disk_limit(10 * 1024 * 1024 * 1024); // 10 GB

        let status = monitor.check(100 * 1024 * 1024, 1024 * 1024 * 1024); // 100MB mem, 1GB disk
        assert!(status.is_healthy());
        assert!(!status.is_under_pressure());
    }

    #[test]
    fn test_resource_monitor_pressure() {
        let mut monitor = ResourceMonitor::new()
            .with_memory_limit(100 * 1024 * 1024) // 100 MB
            .with_disk_limit(1024 * 1024 * 1024); // 1 GB

        // 95 MB used of 100 MB = 95%
        let status = monitor.check(95 * 1024 * 1024, 100 * 1024 * 1024);
        assert!(status.is_under_pressure());
        assert!(status.killing);
    }

    #[test]
    fn test_resource_monitor_throttle() {
        let mut monitor = ResourceMonitor::new()
            .with_memory_limit(100 * 1024 * 1024) // 100 MB
            .with_disk_limit(1024 * 1024 * 1024);

        // 85 MB used = 85% → throttle (threshold 0.8)
        let status = monitor.check(85 * 1024 * 1024, 100 * 1024 * 1024);
        assert!(status.throttled);
        assert!(monitor.is_throttled());
    }

    #[test]
    fn test_resource_monitor_recovers() {
        let mut monitor = ResourceMonitor::new()
            .with_memory_limit(100 * 1024 * 1024)
            .with_disk_limit(1024 * 1024 * 1024);

        // Trigger pressure
        monitor.check(90 * 1024 * 1024, 100 * 1024 * 1024);
        assert!(monitor.is_throttled());

        // Pressure eases
        monitor.check(30 * 1024 * 1024, 100 * 1024 * 1024);
        assert!(!monitor.is_throttled());
    }

    // ── DegradationSummary tests ─────────────────────────────────────────

    #[test]
    fn test_degradation_summary() {
        let mut summary = DegradationSummary::default();
        assert!(summary.fully_operational());
        assert!(!summary.is_degraded());

        summary.modes.push(DegradationMode::MemoryFull);
        assert!(!summary.fully_operational());
        assert!(summary.is_degraded());
    }
}
