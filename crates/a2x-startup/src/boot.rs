// See plans/11-startup-shutdown.md §2 (Startup Sequence)
//
// BootSequence — ordered boot phase orchestrator for A2X system startup.
//
// The boot sequence runs phases in dependency order:
//   Config → Storage → Bus → Agents → Gateway → Ready
//
// Each phase can succeed or fail. On failure, the sequence stops and
// reports which phase failed with the error.

use std::fmt;
use std::time::Instant;

use tracing::{error, info, warn};

// ── BootPhase ─────────────────────────────────────────────────────────────

/// Ordered boot phases for A2X system startup.
///
/// Phases execute in declaration order. The order is fixed by dependency:
/// config must load before storage, storage before bus, etc.
///
/// See plans/11-startup-shutdown.md §2 for the full specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BootPhase {
    /// Phase 1: Load and validate configuration from ~/.a2x/.
    Config,
    /// Phase 2: Initialize storage directories, load persisted state.
    Storage,
    /// Phase 3: Start message bus with configured transport.
    Bus,
    /// Phase 4: Start agents in order (Orchestrator → CLI → CCS → LLM).
    Agents,
    /// Phase 5: Start gateway listeners (HTTP, WS, TCP, stdio).
    Gateway,
    /// Phase 6: Signal readiness — write PID file, emit Ready event.
    Ready,
}

impl BootPhase {
    /// Human-readable name for the phase.
    pub fn name(&self) -> &'static str {
        match self {
            BootPhase::Config => "Configuration",
            BootPhase::Storage => "Storage",
            BootPhase::Bus => "Message Bus",
            BootPhase::Agents => "Agents",
            BootPhase::Gateway => "Gateway",
            BootPhase::Ready => "Ready Signal",
        }
    }

    /// Description of what the phase does.
    pub fn description(&self) -> &'static str {
        match self {
            BootPhase::Config => "Load and validate ~/.a2x/config.toml and per-agent configs",
            BootPhase::Storage => {
                "Initialize directory structure, load persisted WorldGraph and MemoryTrace"
            }
            BootPhase::Bus => "Start message bus with configured transport (in-memory or TCP)",
            BootPhase::Agents => "Start agents in order: Orchestrator → CLI → CCS → LLM",
            BootPhase::Gateway => "Start gateway listeners: HTTP, WebSocket, TCP, stdio",
            BootPhase::Ready => "Write PID file, emit Ready event on bus",
        }
    }

    /// All phases in execution order.
    pub fn all() -> &'static [BootPhase] {
        &[
            BootPhase::Config,
            BootPhase::Storage,
            BootPhase::Bus,
            BootPhase::Agents,
            BootPhase::Gateway,
            BootPhase::Ready,
        ]
    }
}

impl fmt::Display for BootPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ── BootError ─────────────────────────────────────────────────────────────

/// Error returned when a boot phase fails.
#[derive(Debug)]
pub struct BootError {
    /// Which phase failed.
    pub phase: BootPhase,
    /// The error message from the phase.
    pub message: String,
    /// How long the boot sequence ran before failing.
    pub elapsed: std::time::Duration,
}

impl BootError {
    pub fn new(phase: BootPhase, message: impl Into<String>, elapsed: std::time::Duration) -> Self {
        BootError {
            phase,
            message: message.into(),
            elapsed,
        }
    }
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "boot phase '{}' failed after {:?}: {}",
            self.phase.name(),
            self.elapsed,
            self.message
        )
    }
}

impl std::error::Error for BootError {}

// ── PhaseResult ───────────────────────────────────────────────────────────

/// Result of executing a single boot phase.
#[derive(Clone, Debug)]
pub struct PhaseResult {
    /// Which phase this result is for.
    pub phase: BootPhase,
    /// How long the phase took.
    pub duration: std::time::Duration,
    /// Whether the phase succeeded.
    pub success: bool,
    /// Optional detail message (e.g., "loaded config with 3 agents").
    pub detail: Option<String>,
}

impl PhaseResult {
    fn ok(phase: BootPhase, duration: std::time::Duration, detail: Option<String>) -> Self {
        PhaseResult {
            phase,
            duration,
            success: true,
            detail,
        }
    }

    fn err(phase: BootPhase, duration: std::time::Duration, error: Option<String>) -> Self {
        PhaseResult {
            phase,
            duration,
            success: false,
            detail: error,
        }
    }
}

// ── BootSequence ──────────────────────────────────────────────────────────

/// Orchestrates ordered system startup.
///
/// Consumed by `execute()` — a boot sequence runs exactly once.
///
/// # Example
/// ```ignore
/// let mut boot = BootSequence::new();
/// boot.add_phase(BootPhase::Config, "Load config", || {
///     let config = A2xConfig::load()?;
///     Ok(Some("loaded 3 agents".to_string()))
/// });
/// boot.add_phase(BootPhase::Storage, "Init storage", || {
///     A2xConfig::initialize()?;
///     Ok(Some("storage ready".to_string()))
/// });
/// let results = boot.execute()?;
/// for r in &results {
///     println!("ok {} ({:?})", r.phase.name(), r.duration);
/// }
/// ```
pub struct BootSequence {
    /// Phases to execute, in order.
    phases: Vec<BootStep>,
    /// Whether to stop on first failure (true) or continue (false).
    stop_on_error: bool,
    /// When the boot sequence started.
    started_at: Option<Instant>,
}

/// A single step in the boot sequence.
struct BootStep {
    phase: BootPhase,
    /// Human-readable label for logging.
    label: String,
    /// The function to execute. Returns Ok(Some(detail)) on success,
    /// Ok(None) if nothing to report, or Err(message) on failure.
    action: Box<dyn FnOnce() -> Result<Option<String>, String>>,
}

impl BootSequence {
    /// Create a new, empty boot sequence.
    pub fn new() -> Self {
        BootSequence {
            phases: Vec::new(),
            stop_on_error: true,
            started_at: None,
        }
    }

    /// If true (default), stop execution on the first phase failure.
    /// If false, continue executing phases even if some fail.
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Add a boot phase with a label and an action function.
    ///
    /// The action function returns:
    /// - `Ok(Some(detail))` — phase succeeded with a detail message
    /// - `Ok(None)` — phase succeeded with no detail
    /// - `Err(message)` — phase failed
    ///
    /// Phases execute in the order they are added.
    pub fn add_phase<F>(&mut self, phase: BootPhase, label: impl Into<String>, action: F)
    where
        F: FnOnce() -> Result<Option<String>, String> + 'static,
    {
        self.phases.push(BootStep {
            phase,
            label: label.into(),
            action: Box::new(action),
        });
    }

    /// Execute all phases in order.
    ///
    /// Consumes `self` — a boot sequence runs exactly once.
    /// Returns the results for all executed phases. If `stop_on_error` is true
    /// (default), returns `Err(BootError)` on the first failure and stops.
    /// If false, collects all results and returns them regardless of failures.
    pub fn execute(mut self) -> Result<Vec<PhaseResult>, BootError> {
        self.started_at = Some(Instant::now());
        let mut results: Vec<PhaseResult> = Vec::new();

        let total_phases = self.phases.len();
        info!("Boot sequence starting ({} phases)", total_phases);

        while !self.phases.is_empty() {
            let step = self.phases.remove(0); // O(n) but n ≤ 6, negligible
            let phase = step.phase;
            let label = step.label;
            let started = Instant::now();

            info!("Boot phase: {} — {}", phase.name(), label);

            match (step.action)() {
                Ok(detail) => {
                    let duration = started.elapsed();
                    info!(
                        "ok {} complete ({:?}){}",
                        phase.name(),
                        duration,
                        detail
                            .as_deref()
                            .map(|d| format!(" — {d}"))
                            .unwrap_or_default()
                    );
                    results.push(PhaseResult::ok(phase, duration, detail));
                }
                Err(err_msg) => {
                    let duration = started.elapsed();
                    let elapsed = self.started_at.unwrap().elapsed();
                    error!("FAIL {} failed ({:?}): {}", phase.name(), duration, err_msg);
                    results.push(PhaseResult::err(phase, duration, Some(err_msg.clone())));

                    if self.stop_on_error {
                        return Err(BootError::new(phase, err_msg, elapsed));
                    }
                    // Continue to next phase if stop_on_error is false
                    warn!(
                        "Continuing after {} failure (stop_on_error=false)",
                        phase.name()
                    );
                }
            }
        }

        let total_elapsed = self.started_at.unwrap().elapsed();
        let succeeded = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();

        info!(
            "Boot sequence complete ({:?}): {}/{} succeeded, {} failed",
            total_elapsed, succeeded, total_phases, failed
        );

        Ok(results)
    }
}

impl Default for BootSequence {
    fn default() -> Self {
        Self::new()
    }
}

// ── Convenience: standard boot sequence ──────────────────────────────────

/// The canonical 6-phase boot order from plans/11-startup-shutdown.md §2:
///   Config → Storage → Bus → Agents → Gateway → Ready
///
/// Use with `BootSequence::new()` + `add_phase()` in a loop to get
/// the correct order without manually listing phases.
pub fn standard_boot_order() -> Vec<(BootPhase, &'static str)> {
    vec![
        (BootPhase::Config, "Load configuration"),
        (BootPhase::Storage, "Initialize storage"),
        (BootPhase::Bus, "Start message bus"),
        (BootPhase::Agents, "Start agents"),
        (BootPhase::Gateway, "Start gateway"),
        (BootPhase::Ready, "Signal readiness"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_phase_names() {
        assert_eq!(BootPhase::Config.name(), "Configuration");
        assert_eq!(BootPhase::Storage.name(), "Storage");
        assert_eq!(BootPhase::Bus.name(), "Message Bus");
        assert_eq!(BootPhase::Agents.name(), "Agents");
        assert_eq!(BootPhase::Gateway.name(), "Gateway");
        assert_eq!(BootPhase::Ready.name(), "Ready Signal");
    }

    #[test]
    fn test_boot_phase_display() {
        assert_eq!(format!("{}", BootPhase::Config), "Configuration");
    }

    #[test]
    fn test_all_phases_in_order() {
        let phases = BootPhase::all();
        assert_eq!(phases.len(), 6);
        assert_eq!(phases[0], BootPhase::Config);
        assert_eq!(phases[1], BootPhase::Storage);
        assert_eq!(phases[2], BootPhase::Bus);
        assert_eq!(phases[3], BootPhase::Agents);
        assert_eq!(phases[4], BootPhase::Gateway);
        assert_eq!(phases[5], BootPhase::Ready);
    }

    #[test]
    fn test_boot_error_format() {
        let err = BootError::new(
            BootPhase::Bus,
            "connection refused",
            std::time::Duration::from_secs(2),
        );
        let msg = err.to_string();
        assert!(msg.contains("Message Bus"));
        assert!(msg.contains("connection refused"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn test_empty_boot_sequence() {
        let boot = BootSequence::new();
        let results = boot.execute().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_successful_boot_sequence() {
        let mut boot = BootSequence::new();
        boot.add_phase(BootPhase::Config, "test config", || {
            Ok(Some("loaded 2 agents".to_string()))
        });
        boot.add_phase(BootPhase::Storage, "test storage", || {
            Ok(Some("storage ready".to_string()))
        });
        boot.add_phase(BootPhase::Bus, "test bus", || Ok(None));

        let results = boot.execute().unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
        assert_eq!(results[0].detail.as_deref(), Some("loaded 2 agents"));
        assert_eq!(results[1].detail.as_deref(), Some("storage ready"));
        assert_eq!(results[2].detail.as_deref(), None);
    }

    #[test]
    fn test_boot_sequence_stops_on_error() {
        let mut boot = BootSequence::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let cc1 = call_count.clone();
        boot.add_phase(BootPhase::Config, "test", move || {
            cc1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(None)
        });

        let cc2 = call_count.clone();
        boot.add_phase(BootPhase::Storage, "failing", move || {
            cc2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err("disk full".to_string())
        });

        let cc3 = call_count.clone();
        boot.add_phase(BootPhase::Bus, "should not run", move || {
            cc3.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(None)
        });

        let result = boot.execute();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.phase, BootPhase::Storage);
        assert!(err.message.contains("disk full"));

        // Only first 2 phases should have run.
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn test_boot_sequence_continues_on_error() {
        let mut boot = BootSequence::new().stop_on_error(false);

        boot.add_phase(BootPhase::Config, "test", || Ok(None));
        boot.add_phase(BootPhase::Storage, "failing", || {
            Err("disk full".to_string())
        });
        boot.add_phase(BootPhase::Bus, "should run", || {
            Ok(Some("bus started".to_string()))
        });

        let results = boot.execute().unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[2].success);
    }

    #[test]
    fn test_standard_boot_order() {
        let order = standard_boot_order();
        assert_eq!(order.len(), 6);
        assert_eq!(order[0].0, BootPhase::Config);
        assert_eq!(order[5].0, BootPhase::Ready);
    }
}
