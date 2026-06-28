# A2X Concurrency Model — Async, Multi-Program & Parallelism

> **How the CCS VM integrates with async Rust, how multiple programs run concurrently on the same agent, and how parallel execution is managed.**

---

## 1. Overview

The A2X ecosystem has **four levels of concurrency**:

| Level | What Runs | Mechanism | Crate |
|-------|-----------|-----------|-------|
| **Program-level** | Multiple Σ∞/Ω programs on the same agent | Async tasks + scheduler | `a2x-ccs` |
| **Instruction-level** | Parallel swarm (⥁) within a single program | Forked child VMs | `a2x-ccs` |
| **Agent-level** | Multiple agents on the same machine | Tokio tasks + bus | `a2x-agents` |
| **Node-level** | Agents across machines | Network transport (TCP/gRPC) | `a2x-bus` |

---

## 2. The CCS VM is Single-Threaded Per Instance

Each CCS VM instance runs on **one OS thread** (via `tokio::spawn`). This avoids data races within a single VM:

```rust
/// A CCS VM runs on a single async task.
/// It does NOT hold internal locks — it's !Sync by design.
pub struct CcsVm {
    world_graph: WorldGraph,
    state_field: StateField,
    instruction_pointer: usize,
    program: SigmaProgram,
    call_stack: Vec<usize>,
    memory_trace: MemoryTrace,
    policy: Box<dyn PolicyField>,
    safety: SafetyConstraints,
    probe_channel: Option<mpsc::Receiver<ProbeQuery>>,
}

impl CcsVm {
    /// Run the VM to completion (blocking within the async task).
    /// This does NOT block the tokio runtime — it's an async fn that yields.
    pub async fn run(&mut self) -> Result<SigmaProgram, VmError> {
        loop {
            // Check for incoming probe queries (non-blocking)
            if let Some(probe) = self.try_recv_probe().await? {
                self.handle_probe(probe).await?;
            }

            // Execute one instruction
            match self.step()? {
                VmStatus::Running => {
                    // Yield periodically to allow other tasks to run
                    // (every N instructions, configurable)
                    if self.instruction_pointer % YIELD_EVERY == 0 {
                        tokio::task::yield_now().await;
                    }
                },
                VmStatus::Halted => break,
                VmStatus::Yield => break,
                VmStatus::Fault(err) => return Err(err),
            }
        }
        Ok(self.program.output())
    }
}
```

**Key design decisions:**
- VM is `!Sync` — mutable state is not shared across threads
- VM can be `Send` — it can be moved between threads (e.g., for load balancing)
- Probe queries arrive on a separate channel and are checked between instructions
- `tokio::task::yield_now()` is called periodically to prevent starving other tasks

---

## 3. Running Multiple Programs on One Agent

An agent can run multiple programs concurrently by spawning multiple VM instances:

```rust
/// Agent program scheduler — manages concurrent VM instances.
pub struct ProgramScheduler {
    /// Currently running VM instances.
    running: HashMap<ProgramId, JoinHandle<Result<SigmaProgram, AgentError>>>,
    /// Maximum concurrent programs.
    max_concurrent: usize,
    /// Channel for submitting new programs.
    submit_tx: mpsc::UnboundedSender<ScheduledProgram>,
}

impl ProgramScheduler {
    /// Submit a program for execution.
    pub async fn submit(&self, program: SigmaProgram) -> Result<ProgramId, SchedulerError> {
        if self.running.len() >= self.max_concurrent {
            return Err(SchedulerError::AtCapacity(self.max_concurrent));
        }
        // Spawn a new tokio task running a CCS VM
        let handle = tokio::spawn(async move {
            let mut vm = CcsVm::new(/* ... */);
            vm.run().await
        });
        // ...
    }

    /// Get the result of a completed program (non-blocking).
    pub fn try_recv(&mut self, id: &ProgramId) -> Option<Result<SigmaProgram, AgentError>> {
        // Check if the JoinHandle is finished
    }

    /// Cancel a running program.
    pub fn cancel(&mut self, id: &ProgramId) -> Result<(), SchedulerError> {
        // Drop the JoinHandle (aborts the task)
    }
}
```

**Scheduling policies:**

| Policy | Behavior |
|--------|----------|
| **Round-robin** | Each program gets equal VM steps before yielding |
| **Priority** | Programs with `⚡` (immediate) intent run first |
| **FIFO** | Strict queue order (default) |
| **Preemptive** | Priority program preempts lower-priority programs |

---

## 4. Parallel Swarm (⥁) Internals

When a program hits `⥁` (FORK):

1. Parent VM snapshots `WorldGraph + StateField` (clone)
2. N child VMs are created, each with their own `ProgramScheduler`
3. Child VMs run as independent `tokio::spawn` tasks
4. Parent VM awaits all children via `FuturesUnordered`
5. Results are merged back into parent WorldGraph
6. Merge conflicts are detected and handled (see §14-resilience.md)

```rust
// In CcsVm::execute_fork():
async fn execute_fork(&mut self, sub_programs: Vec<SigmaProgram>) -> Result<(), VmError> {
    let snapshot = VmSnapshot {
        world_graph: self.world_graph.clone(),
        state_field: self.state_field.snapshot(),
    };

    // Spawn children
    let mut handles = Vec::new();
    for program in sub_programs {
        let child_snapshot = snapshot.clone();
        handles.push(tokio::spawn(async move {
            let mut vm = CcsVm::from_snapshot(child_snapshot, program);
            vm.run().await
        }));
    }

    // Wait for all children
    let results: Vec<Result<SigmaProgram, VmError>> =
        futures::future::join_all(handles).await
            .into_iter()
            .map(|r| r.map_err(|_| VmError::TaskJoinFailed)?)
            .collect::<Result<_, _>>()?;

    // Merge results
    for result in results {
        self.merge_result(result)?;
    }

    Ok(())
}
```

---

## 5. The Bus is Async-Native

The bus uses Tokio channels for in-process communication:

```rust
// In-memory bus implementation
pub struct InMemoryBus {
    /// Registered agents by capability.
    agents: Arc<RwLock<HashMap<Capability, Vec<AgentInfo>>>>,
    /// Per-agent message channels.
    channels: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<WireMessage>>>>,
}
```

- `mpsc::UnboundedSender` — messages are sent without backpressure (bounded in Phase 2)
- `watch` channel — agent discovery events
- `tokio::select!` — multiplexing between incoming messages and discovery events

---

## 6. Async Boundary: Where We Use Tokio

| Operation | Async? | Why |
|-----------|:------:|-----|
| VM step (instruction execution) | Sometimes | Yields every N steps; most instructions are sync |
| Network transport (TCP/gRPC) | Yes | Non-blocking I/O |
| File system reads/writes | Yes | `tokio::fs` |
| Agent discovery | Yes | `watch` channel |
| Probe channel communication | Yes | `mpsc` channel |
| Gateway HTTP/WS listeners | Yes | `axum` or `warp` |
| CLI agent executing system cmds | Yes | `tokio::process::Command` |

---

## 7. Avoiding Async in Hot Paths

The VM's inner loop avoids async overhead:

```rust
impl CcsVm {
    /// Sync step — no async, no allocations in hot path.
    fn step(&mut self) -> Result<VmStatus, VmError> {
        // Pre-allocated instruction buffer (reused across steps)
        // Operator dispatch table (no pattern matching overhead)
        // Bump-allocated scratch space for intermediate results
    }
}
```

Async is used at the **boundary** of the VM (between instructions), not inside individual instructions.

---

## 8. Thread Safety Summary

| Type | Send | Sync | Notes |
|------|:----:|:----:|-------|
| `CcsVm` | ✅ | ❌ | Mutable state per instance |
| `WorldGraph` | ✅ | ❌ | Internal mutation |
| `StateField` | ✅ | ❌ | Internal mutation |
| `SigmaProgram` | ✅ | ✅ | Immutable after construction |
| `OmegaPacket` | ✅ | ✅ | Immutable data |
| `InMemoryBus` | ✅ | ✅ | Internal `Arc<RwLock<>>` |
| `CliAgent` | ✅ | ✅ | Shared state behind Arc |
| `ProbeQuery` | ✅ | ✅ | Message types |

---

*This sub-plan maps to phases 2–3 of the implementation roadmap.*
