// Phase 7.4: ParallelSwarm — fork-join execution for ⥁ (FORK) operator
//
// See plans/10-concurrency.md §4 — "Parallel Swarm (⥁) Internals"
//
// When a program hits Opcode::Fork, the parent VM snapshots its
// WorldGraph + StateField, spawns N child VMs each with a copy of the
// snapshot, runs them concurrently via tokio::spawn, collects results,
// and merges them back.
//
// The core snapshot/merge infrastructure (VmSnapshot, snapshot(),
// from_snapshot(), merge_swarm_results()) lives in vm.rs so the
// synchronous Fork/Merge path in step() works without tokio.

use std::time::Duration;

use a2x_core::state::StateField;
use a2x_sigma::program::SigmaProgram;
use tracing::{debug, info, warn};

use crate::async_vm::{AsyncRunConfig, AsyncRunResult};
use crate::error::VmError;
use crate::vm::CcsVm;

/// Result from a single child VM in a parallel swarm.
#[derive(Clone, Debug)]
pub struct SwarmChildResult {
    /// Index of this child in the swarm.
    pub index: usize,
    /// The async run result.
    pub result: AsyncRunResult,
    /// WorldGraph snapshot from the child (for merging).
    pub world_graph_data: Vec<u8>,
    /// StateField snapshot from the child.
    pub state_field_data: Vec<f32>,
}

/// Configuration for parallel swarm execution.
#[derive(Clone, Debug)]
pub struct SwarmConfig {
    /// Maximum concurrent child VMs.
    pub max_concurrency: usize,
    /// Timeout per child VM.
    pub child_timeout: Option<Duration>,
    /// Yield interval for child VMs.
    pub yield_interval: usize,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        SwarmConfig {
            max_concurrency: 4,
            child_timeout: Some(Duration::from_secs(30)),
            yield_interval: 32,
        }
    }
}

impl CcsVm {
    /// Execute a parallel swarm (async): fork N child VMs via tokio::spawn,
    /// collect results, and merge back into the parent.
    ///
    /// This is the async version — for synchronous fork execution within
    /// step(), see the `Opcode::Fork` arm in vm.rs.
    pub async fn execute_fork(
        &mut self,
        sub_programs: Vec<SigmaProgram>,
        config: SwarmConfig,
    ) -> Result<Vec<AsyncRunResult>, VmError> {
        if sub_programs.is_empty() {
            return Ok(Vec::new());
        }

        let parent_snapshot = self.snapshot();
        let n = sub_programs.len();

        debug!(
            parent_ip = self.ip,
            child_count = n,
            "parallel swarm: forking"
        );

        // Spawn child VMs
        let mut handles = Vec::with_capacity(n);
        for (i, program) in sub_programs.into_iter().enumerate() {
            let snapshot = parent_snapshot.clone();
            let child_config = AsyncRunConfig {
                yield_interval: config.yield_interval,
                timeout: config.child_timeout,
            };

            let handle = tokio::spawn(async move {
                let mut vm = CcsVm::from_snapshot(&snapshot);
                vm.load(program);
                vm.run_async(child_config)
                    .await
                    .map(|result| SwarmChildResult {
                        index: i,
                        result,
                        world_graph_data: vm.serialize_world_graph(),
                        state_field_data: vm.state_field.raw_data().to_vec(),
                    })
                    .map_err(|e| (i, e))
            });
            handles.push(handle);
        }

        // Wait for all children
        let mut results = Vec::with_capacity(n);
        let mut child_snapshots = Vec::with_capacity(n);

        for handle in handles {
            match handle.await {
                Ok(Ok(swarm_result)) => {
                    debug!(
                        index = swarm_result.index,
                        steps = swarm_result.result.steps_executed,
                        "child VM completed"
                    );
                    child_snapshots.push((
                        swarm_result.index,
                        swarm_result.world_graph_data,
                        swarm_result.state_field_data,
                    ));
                    results.push(swarm_result.result);
                }
                Ok(Err((idx, err))) => {
                    warn!(index = idx, error = %err, "child VM failed");
                    return Err(err);
                }
                Err(join_err) => {
                    warn!(error = %join_err, "child VM task panicked");
                    return Err(VmError::Other(format!("task join error: {}", join_err)));
                }
            }
        }

        // Merge child results back into parent
        self.merge_swarm_results(&child_snapshots)?;

        info!(
            child_count = results.len(),
            "parallel swarm: all children completed, merged"
        );

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::VmStatus;
    use a2x_sigma::SigmaPacket;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn test_vm_snapshot() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(SigmaPacket::default());
        vm.load(prog);
        vm.step().unwrap();

        let snapshot = vm.snapshot();
        assert!(!snapshot.state_field_data.is_empty());
    }

    #[test]
    fn test_execute_fork_empty() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            vm.load(SigmaProgram::new());
            let results = vm
                .execute_fork(vec![], SwarmConfig::default())
                .await
                .unwrap();
            assert!(results.is_empty());
        });
    }

    #[test]
    fn test_execute_fork_single_child() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            vm.load(SigmaProgram::new()); // empty parent program

            let mut child_prog = SigmaProgram::new();
            child_prog.push(SigmaPacket::default()); // NOP
            child_prog.push(SigmaPacket::default()); // NOP

            let config = SwarmConfig {
                max_concurrency: 2,
                child_timeout: Some(Duration::from_secs(5)),
                yield_interval: 1,
            };

            let results = vm.execute_fork(vec![child_prog], config).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].status, VmStatus::Halted);
            assert_eq!(results[0].steps_executed, 2);
        });
    }

    #[test]
    fn test_execute_fork_multiple_children() {
        rt().block_on(async {
            let mut vm = CcsVm::new();
            vm.load(SigmaProgram::new());

            let mut children = Vec::new();
            for _ in 0..3 {
                let mut prog = SigmaProgram::new();
                prog.push(SigmaPacket::default());
                children.push(prog);
            }

            let config = SwarmConfig {
                max_concurrency: 4,
                child_timeout: Some(Duration::from_secs(5)),
                yield_interval: 1,
            };

            let results = vm.execute_fork(children, config).await.unwrap();
            assert_eq!(results.len(), 3);
            for r in &results {
                assert_eq!(r.status, VmStatus::Halted);
                assert_eq!(r.steps_executed, 1);
            }
        });
    }

    #[test]
    fn test_swarm_config_default() {
        let config = SwarmConfig::default();
        assert_eq!(config.max_concurrency, 4);
        assert!(config.child_timeout.is_some());
        assert_eq!(config.yield_interval, 32);
    }
}
