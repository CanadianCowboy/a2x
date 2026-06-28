# A2X Probe — Debug, Probe & Visualization Protocol

> **The debug interface for inspecting running CCS VMs — breakpoints, single-stepping, state inspection, and visualization.**

---

## 1. Overview

Every CCS VM exposes a **probe interface** that allows external tools to inspect its internal state without interrupting execution.

- **Crate:** `a2x-probe` (Phase 5)
- **Depends on:** `a2x-ccs`, `a2x-bus`, `tracing`
- **Key files:** `inspector.rs`, `tracer.rs`, `dashboard/`

---

## 2. Probe Interface

```rust
pub enum ProbeQuery {
    /// Snapshot the entire VM state.
    Snapshot,
    /// Get the current instruction pointer.
    GetIp,
    /// Dump a WorldGraph node by ID.
    GetNode(NodeId),
    /// Dump a WorldGraph node by label.
    GetNodeByLabel(String),
    /// Dump a StateField region.
    GetRegion(String),
    /// Run a graph query in the current WorldGraph.
    Query(GraphQuery),
    /// Get the program counter (MemoryTrace position).
    GetPc,
    /// Get the last N MemoryTrace entries.
    GetTraceTail(usize),
    /// Set a breakpoint at instruction index.
    SetBreakpoint(usize),
    /// Clear a breakpoint.
    ClearBreakpoint(usize),
    /// Step one instruction (when paused at breakpoint).
    Step,
    /// Continue execution (when paused at breakpoint).
    Continue,
}

pub enum ProbeSnapshot {
    VmState {
        program_id: ProgramId,
        ip: usize,
        pc: u64,
        state: VmStatus,
        safety: SafetyLevel,
    },
    Node(GraphNode),
    Region(StateRegion, ArrayD<f32>),
    QueryResult(Vec<NodeId>),
    TraceSegment(Vec<MemoryEntry>),
    BreakpointSet(usize),
    BreakpointCleared(usize),
    Stepped,
    Continued,
}
```

---

## 3. How Probe Tools Connect

```
┌──────────────────┐       ┌──────────────────┐
│  CLI Probe Tool  │──────▶│  CCS VM (Agent)  │
│  a2x-cli          │       │                   │
│  $ probe --agent │       │ ProbeQuery/Response│
│    cli-1         │       │ over transport    │
└──────────────────┘       └──────────────────┘
```

1. Probe tool discovers agent via bus discovery
2. Probe tool opens a probe channel (separate stream from program execution)
3. Probe tool sends ProbeQuery messages
4. VM responds with ProbeSnapshot messages
5. Probe tool displays or logs the state
6. Optionally: set breakpoints, single-step through programs

---

## 4. Channel Separation (Probe Channel vs Execution Channel)

Probe traffic is **separate from program execution traffic** to avoid interference:

```
┌────────────────────────────┐
│         CCS VM             │
│                            │
│  ┌──────────────────────┐  │
│  │   Execution Loop     │  │   Bus message: SigmaProgram / OmegaProgram
│  │   (running program)  │◄─┼──────────────────────────────────────
│  └──────────┬───────────┘  │
│             │              │
│  ┌──────────▼───────────┐  │
│  │  Probe Channel       │  │   Probe channel: ProbeQuery / ProbeSnapshot
│  │  (mpsc::Receiver     │◄─┼──────────────────────────────────────
│  │   checked between    │  │
│  │   instructions)      │  │
│  └──────────────────────┘  │
└────────────────────────────┘
```

### Implementation

```rust
impl CcsVm {
    /// The probe channel — checked between each instruction.
    /// If a message is waiting, it's processed BEFORE the next instruction.
    probe_rx: Option<mpsc::UnboundedReceiver<ProbeQuery>>,
}

impl CcsVm {
    async fn run_probe_aware(&mut self) -> Result<SigmaProgram, VmError> {
        loop {
            // Check probe channel (non-blocking)
            if let Some(probe_rx) = &mut self.probe_rx {
                loop {
                    match probe_rx.try_recv() {
                        Ok(query) => self.handle_probe(query).await?,
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            // Probe tool disconnected
                            self.probe_rx = None;
                            break;
                        }
                    }
                }
            }

            // If paused at breakpoint, wait for Continue or Step
            if self.paused_at_breakpoint {
                self.wait_for_probe_command().await?;
            }

            // Execute one instruction
            match self.step()? {
                VmStatus::Running => {
                    if self.instruction_pointer % YIELD_EVERY == 0 {
                        tokio::task::yield_now().await;
                    }
                },
                VmStatus::Halted => break,
                VmStatus::BreakpointHit => {
                    self.paused_at_breakpoint = true;
                    // Notify probe tool
                    if let Some(tx) = &self.probe_event_tx {
                        let _ = tx.send(ProbeEvent::BreakpointHit {
                            ip: self.instruction_pointer,
                            instruction: self.program.instructions[self.instruction_pointer].clone(),
                            state: self.snapshot(),
                        });
                    }
                },
                VmStatus::Fault(err) => return Err(err),
            }
        }
        Ok(self.program.output())
    }
}
```

### Probe Channel vs Execution Channel: Key Differences

| Aspect | Execution Channel | Probe Channel |
|--------|:-----------------:|:-------------:|
| Purpose | Send programs, receive results | Inspect VM state |
| Direction | Bidirectional (req/response) | Bidirectional |
| Backpressure | Yes (bounded send) | No (unbounded — probe can't block execution) |
| Authentication | Same as bus | Requires `can_probe: true` permission |
| Message types | SigmaProgram / OmegaProgram | ProbeQuery / ProbeSnapshot |
| Routing | Via bus router | Direct VM-to-tool (no routing) |
| Transport | Bus transport | Same transport, different message type |

---

## 5. Breakpoint Semantics

### Breakpoint Types

```rust
pub enum BreakpointType {
    /// Stop at a specific instruction index.
    Instruction(usize),
    /// Stop when a WorldGraph node matching a label is accessed.
    NodeAccess {
        label: String,
        access_type: AccessType,
    },
    /// Stop when a StateField region is read/written.
    RegionAccess {
        region: String,
        access_type: AccessType,
    },
    /// Stop when a specific condition is met (predicate on VM state).
    Conditional(Box<dyn Fn(&VmSnapshot) -> bool + Send>),
}

pub enum AccessType {
    Read,
    Write,
    Both,
}
```

### Breakpoint Lifecycle

```
1. Probe tool sends SetBreakpoint(ip=5)  ──►  VM stores breakpoint in HashMap
2. VM executes instructions 0,1,2,3,4    ──►  (checks breakpoints each step)
3. VM reaches IP=5, checks breakpoints   ──►  Match found!
4. VM pauses BEFORE executing instruction 5
5. VM sends BreakpointHit event on probe channel
6. VM blocks on probe_rx, waiting for: Step | Continue | other ProbeQuery
7a. If Step:  execute instruction 5, pause again
7b. If Continue: resume normal execution
7c. If Query: respond, stay paused
8. Probe tool sends ClearBreakpoint(5) or ClearAllBreakpoints
```

### Multiple Breakpoints

- Unlimited breakpoints (stored in `HashMap<usize, BreakpointType>`)
- Each breakpoint is checked in O(1) (hash lookup by IP)
- Breakpoints persist until cleared, even across program executions
- On program reload (new SigmaProgram), breakpoints by IP shift if program size changes

### Conditional Breakpoints

```rust
pub enum Condition {
    /// IP == N (equivalent to instruction breakpoint).
    AtInstruction(usize),
    /// StateField region value == expected (within epsilon).
    StateFieldEquals { region: String, expected: Vec<f32>, epsilon: f32 },
    /// WorldGraph label exists.
    NodeExists(String),
    /// Program has executed for > N instructions.
    AfterSteps(u64),
    /// Custom predicate (WASM plugin or serialized closure).
    Custom(String), // DSL or WASM bytecode
}
```

---

## 6. Performance Impact

### Probe Overhead

| Operation | Overhead (no probe) | Overhead (with probe) | Impact |
|-----------|:-------------------:|:---------------------:|:------:|
| Checking probe channel (once per instruction) | N/A | ~50ns | try_recv on empty channel is fast |
| No breakpoints set | None | ~50ns per instruction | Negligible |
| 1 breakpoint set, not hit | None | ~50ns + 1 HashMap lookup (~20ns) | Negligible |
| 100 breakpoints set, not hit | None | ~50ns + 1 HashMap lookup (~20ns) | Negligible (same O(1)) |
| Breakpoint hit | None | ~1µs to snapshot + event send | Acceptable (pauses anyway) |
| Snapshot entire VM | None | ~100µs–1ms (depends on graph size) | Only on demand |
| Tracer: log every instruction | None | ~200ns per instruction | ~20% slowdown @ 1M inst/s |

**Key insight:** Empty probe channel check is ~50ns — negligible compared to instruction execution time (typically 1–100µs for a CCS operator like EVOL or PLAN).

### Tracer Modes

```rust
pub enum TracerMode {
    /// No tracing (fastest).
    Off,
    /// Log instruction IP + opcode only (~50ns/inst).
    Light,
    /// Log full instruction + state delta (~200ns/inst).
    Full,
    /// Log everything + MemoryTrace entry (~500ns/inst).
    Verbose,
}
```

### Recommended Usage

| Scenario | Tracer Mode | Probe Channel | Breakpoints |
|----------|:-----------:|:-------------:|:-----------:|
| Production | Off | Closed | None |
| Development | Light | Open | As needed |
| Debugging specific bug | Full | Open | Set targeted |
| Demoing / teaching | Verbose | Open | Demonstrate control flow |

---

## 7. Probe Tool CLI Commands

```bash
# Connect to an agent and probe its state
$ a2x probe cli-1

# Show current VM status
> status
VM: cli-1 | IP: 42 | State: Running | Safety: Bounded

# Dump WorldGraph
> graph
Node #0: "sys" (edges: 3)
Node #1: "port:22" (edges: 1)
...

# View StateField regions
> regions
goal    [0..64]      belief  [64..320]
scratch [576..1024]

# Set a breakpoint
> break 47
Breakpoint set at instruction 47

# Continue execution
> continue

# Step one instruction
> step
Executed instruction 47: ⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⟘⟭

# Trace last 10 instructions
> trace 10
[38] ⟦Σ∞⟧⟬I:✦ ∷ C:⟨⟩ ∷ P:⥂ ∷ D:⌵⟭
[39] ⟦Σ∞⟧⟬I:✣ ∷ C:⟨⟩ ∷ P:⤈ ∷ D:⟘⟭
...

# Watch a specific state region
> watch goal
Watching region "goal" — will show updates every instruction

# Exit probe
> quit
```

---

## 8. Programmatic Probing (for AI Agents)

Probing isn't just for humans — AI agents can probe each other:

```rust
// Agent A probes Agent B programmatically
#[async_trait]
pub trait ProbeExt: Agent {
    /// Probe another agent's state.
    async fn probe(&self, target: AgentId, query: ProbeQuery)
        -> Result<ProbeSnapshot, ProbeError>;

    /// Stream probe snapshots from another agent.
    async fn stream_probe(&self, target: AgentId, interval: Duration)
        -> BoxStream<ProbeSnapshot>;

    /// Set a breakpoint on another agent.
    async fn set_breakpoint(&self, target: AgentId, bp: BreakpointType)
        -> Result<(), ProbeError>;
}
```

This enables **self-debugging agents** — an agent that encounters an error can probe its own state, generate a diagnostic program, and send it to another agent for analysis.

---

## 9. Visualization (Phase 5+)

The probe crate will provide:

- **WorldGraph visualizer** — graphviz dot output or egui interactive graph
- **StateField heatmap** — visualize tensor regions as color grids
- **Instruction tracer** — step through program execution instruction by instruction
- **MemoryTrace timeline** — scroll through state history, see how concepts evolved

---

*This sub-plan maps to Phase 5 of the implementation roadmap.*
