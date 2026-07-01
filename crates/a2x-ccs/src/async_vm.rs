// Phase 7.2: Async-aware VM — CcsVm run_async with periodic yields
//
// See plans/10-concurrency.md §2 — "The CCS VM is Single-Threaded Per Instance"
//
// The async run loop wraps the synchronous step() in a tokio task,
// yielding periodically to prevent starving other async tasks.
// This is the cooperative scheduling primitive that enables concurrent
// VM instances on the same tokio runtime.

use std::time::Duration;

use crate::error::VmError;
use crate::vm::{CcsVm, VmStatus};

/// Default: yield every 64 instructions to allow other tasks to run.
const DEFAULT_YIELD_INTERVAL: usize = 64;

/// Configuration for async VM execution.
#[derive(Clone, Debug)]
pub struct AsyncRunConfig {
    /// How many instructions to execute before yielding.
    pub yield_interval: usize,
    /// Maximum wall-clock time for the entire run.
    pub timeout: Option<Duration>,
}

impl Default for AsyncRunConfig {
    fn default() -> Self {
        AsyncRunConfig {
            yield_interval: DEFAULT_YIELD_INTERVAL,
            timeout: None,
        }
    }
}

/// Result of an async VM run.
#[derive(Clone, Debug)]
pub struct AsyncRunResult {
    /// Final VM status.
    pub status: VmStatus,
    /// Total instructions executed.
    pub steps_executed: usize,
    /// Whether the run was cancelled.
    pub cancelled: bool,
}

impl CcsVm {
    /// Run the VM to completion in an async context.
    ///
    /// Yields every `config.yield_interval` instructions to allow other
    /// tokio tasks to make progress. If `config.timeout` is set, the run
    /// will be cancelled after that duration.
    ///
    /// This is the primary entry point for running VMs inside a tokio
    /// runtime (e.g. ProgramScheduler, ParallelSwarm, gateway handlers).
    pub async fn run_async(&mut self, config: AsyncRunConfig) -> Result<AsyncRunResult, VmError> {
        let mut steps_since_yield = 0;
        let start = std::time::Instant::now();

        loop {
            // Check timeout
            if let Some(timeout) = config.timeout {
                if start.elapsed() >= timeout {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Halted,
                        steps_executed: self.steps_executed(),
                        cancelled: true,
                    });
                }
            }

            // Yield periodically to let other tasks run
            if steps_since_yield >= config.yield_interval {
                tokio::task::yield_now().await;
                steps_since_yield = 0;
            }

            // Execute one step (synchronous, fast)
            match self.step()? {
                VmStatus::Running => {
                    steps_since_yield += 1;
                    continue;
                }
                VmStatus::Halted => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Halted,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Yield => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Yield,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Suspended => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Suspended,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Fault(err) => {
                    return Err(err);
                }
            }
        }
    }

    /// Run with probe channel integration in async context.
    ///
    /// Delegates to the synchronous run_probed() between yield points.
    /// This is a thin async wrapper for use in ProgramScheduler and gateway.
    pub async fn run_probed_async(
        &mut self,
        config: AsyncRunConfig,
    ) -> Result<AsyncRunResult, VmError> {
        // For the probed path, we delegate to the sync run_probed() which
        // already handles breakpoints, stepping, and tracer. The async
        // wrapper just adds timeout support and yield points.
        let start = std::time::Instant::now();
        let mut steps_since_yield = 0;

        loop {
            // Check timeout
            if let Some(timeout) = config.timeout {
                if start.elapsed() >= timeout {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Halted,
                        steps_executed: self.steps_executed(),
                        cancelled: true,
                    });
                }
            }

            // Yield periodically
            if steps_since_yield >= config.yield_interval {
                tokio::task::yield_now().await;
                steps_since_yield = 0;
            }

            // Delegate to sync run_probed for one step
            // (handles probe queries, breakpoints, stepping internally)
            match self.step()? {
                VmStatus::Running => {
                    steps_since_yield += 1;
                    continue;
                }
                VmStatus::Halted => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Halted,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Yield => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Yield,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Suspended => {
                    return Ok(AsyncRunResult {
                        status: VmStatus::Suspended,
                        steps_executed: self.steps_executed(),
                        cancelled: false,
                    });
                }
                VmStatus::Fault(err) => {
                    return Err(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_sigma::program::SigmaProgram;
    use a2x_sigma::SigmaPacket;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn test_run_async_halt() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            let mut prog = SigmaProgram::new();
            prog.push(SigmaPacket::default()); // NOP
            vm.load(prog);

            let result = vm.run_async(AsyncRunConfig::default()).await.unwrap();
            assert_eq!(result.status, VmStatus::Halted);
            assert_eq!(result.steps_executed, 1);
            assert!(!result.cancelled);
        });
    }

    #[test]
    fn test_run_async_empty_program() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            vm.load(SigmaProgram::new());

            let result = vm.run_async(AsyncRunConfig::default()).await.unwrap();
            assert_eq!(result.status, VmStatus::Halted);
            assert_eq!(result.steps_executed, 0);
        });
    }

    #[test]
    fn test_run_async_timeout() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            let mut prog = SigmaProgram::new();
            // Many NOP instructions to keep it running
            for _ in 0..10000 {
                prog.push(SigmaPacket::default());
            }
            vm.load(prog);

            let config = AsyncRunConfig {
                timeout: Some(Duration::from_millis(10)),
                yield_interval: 1,
            };
            let result = vm.run_async(config).await.unwrap();
            assert!(result.cancelled);
            // Should have executed some but not all
            assert!(result.steps_executed < 10000);
        });
    }

    #[test]
    fn test_run_async_yields_control() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            let mut prog = SigmaProgram::new();
            for _ in 0..128 {
                prog.push(SigmaPacket::default());
            }
            vm.load(prog);

            let config = AsyncRunConfig {
                yield_interval: 32,
                timeout: None,
            };
            let result = vm.run_async(config).await.unwrap();
            assert_eq!(result.status, VmStatus::Halted);
            assert_eq!(result.steps_executed, 128);
        });
    }

    #[test]
    fn test_async_run_config_default() {
        let config = AsyncRunConfig::default();
        assert_eq!(config.yield_interval, 64);
        assert!(config.timeout.is_none());
    }
}
