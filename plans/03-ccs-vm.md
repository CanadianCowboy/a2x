# A2X CCS VM — Cognitive Substrate Runtime

> **The runtime virtual machine that executes A2X programs. Manages memory (WorldGraph = heap), registers (StateField), and execution history (MemoryTrace).**

---

## 1. Overview

**CCS** (CryoCore Cognitive Substrate) is the runtime VM. It executes Σ∞ and Ω programs, maintains the agent's world-model, and provides the execution environment.

| Component | Role | Analogy |
|-----------|------|---------|
| WorldGraph | Persistent graph memory | Heap |
| StateField | High-dimensional working memory | CPU registers |
| Instruction Pointer (IP) | Current instruction index | Program counter |
| MemoryTrace | Execution history | Execution log |
| Call Stack | Sub-plan return addresses | Call stack |
| PolicyField | JIT compiler + optimizer | Compiler/optimizer |

- **Crate:** `a2x-ccs`
- **Depends on:** `a2x-core`, `petgraph`, `ndarray` (optional)
- **Key files:** `vm.rs`, `world_graph.rs`, `state.rs`, `memory.rs`, `policy.rs`, `safety.rs`, `probe.rs`, `operators/`

---

## 2. CCS Primitives (from `a2x-core`)

| Type | Definition | Notes |
|------|-----------|-------|
| `ConceptVector` | `Vec<f32>` | Dense embedding for concepts |
| `RelationEdge` | `(usize, usize, RelationType, WeightMatrix)` | Directed edge between nodes |
| `RelationType` | `enum { Causal, Spatial, Temporal, Logical, Hierarchical }` | Type tag |
| `WorldGraph` | Graph of ConceptVector nodes + RelationEdge edges | The agent's world-model |
| `StateField` | `ArrayD<f32>` | High-dimensional tensor |
| `PolicyField` | Trait: `(StateField, WorldGraph) → ActionDistribution` | Neural mapping |
| `MemoryTrace` | Time-indexed sequence of `(StateField, WorldGraphDelta)` | Execution history |
| `ActionDistribution` | Probability distribution over action space | What the agent wants to do |

---

## 3. VM Execution Loop

### Fetch-Decode-Execute Cycle

```rust
struct CcsVm {
    world_graph: WorldGraph,
    state_field: StateField,
    instruction_pointer: usize,
    program: SigmaProgram,
    call_stack: Vec<usize>,
    memory_trace: MemoryTrace,
    policy: PolicyField,
    safety: SafetyConstraints,
}

impl CcsVm {
    fn step(&mut self) -> Result<VmStatus, VmError> {
        // 1. FETCH: Get the current instruction
        let instruction = self.program.get(self.instruction_pointer)?;

        // 2. DECODE: Parse instruction fields
        let opcode = instruction.intent();
        let operand = instruction.context();
        let control = instruction.plan();
        let immediate = instruction.data();

        // 3. SAFETY CHECK: Is this instruction allowed?
        self.safety.check_instruction(&instruction)?;

        // 4. EXECUTE: Dispatch to operator
        match opcode { /* ... */ }

        // 5. UPDATE STATE: Apply operand to WorldGraph/StateField
        self.apply_operand(operand, immediate)?;

        // 6. CONTROL FLOW: Update instruction pointer
        self.update_ip(control)?;

        // 7. TRACE: Log to MemoryTrace
        self.memory_trace.push(instruction, self.state_field.snapshot())?;

        Ok(VmStatus::Running)
    }

    fn run(&mut self) -> Result<SigmaProgram, VmError> {
        loop {
            match self.step()? {
                VmStatus::Running => continue,
                VmStatus::Halted => break,
                VmStatus::Yield => break,
                VmStatus::Fault(err) => return Err(err),
            }
        }
        Ok(self.program.output())
    }
}
```

### Execution Modes

| Mode | Set By | Behavior |
|------|--------|----------|
| Normal | Default | Standard fetch-decode-execute |
| Immediate | `⚡` | Skip safety checks, max priority |
| Explore | `✦` | Non-deterministic branching, try multiple paths |
| Safe | `⚠` | Enable all safety constraints, log everything |
| Parallel | `⩫` | Fork VM for each sub-goal, merge results |

---

## 4. VM Instruction Set

### Opcode Table

| Code | Mnemonic | Symbol | CCS Operator | Description |
|:----:|----------|:------:|-------------|-------------|
| 0x0 | `NOP` | — | — | No operation |
| 0x1 | `BIND` | `✣` | `bind()` | Merge concepts into composite |
| 0x2 | `DIFF` | `⩨` | `differentiate()` | Split a concept |
| 0x3 | `GRND` | `✦` | `ground()` | Perceptual grounding |
| 0x4 | `EVOL` | `⟢` | `evolve()` | Time-step world state |
| 0x5 | `REFL` | `⟡` | `reflect()` | Meta-learning from history |
| 0x6 | `PLAN` | `⥂` | `plan()` | Generate action sequence |
| 0x7 | `ACT` | `⤊` | `actuate()` | Emit side effect |
| 0x8 | `JMP` | `→` | — | Unconditional jump |
| 0x9 | `BR` | `⤐` | — | Conditional branch |
| 0xA | `CALL` | `⤈` | — | Call sub-program |
| 0xB | `RET` | `⤉` | — | Return from sub-program |
| 0xC | `FORK` | `⥁` | — | Fork parallel execution |
| 0xD | `MERGE` | `⤑` | — | Merge parallel branches |
| 0xE | `HALT` | `✕` | — | Halt execution |
| 0xF | `CUSTOM` | — | — | Custom extension |

### Operator Implementations

Each operator is a Rust module in `operators/`:

| Operator | Module | Signature |
|----------|--------|-----------|
| `bind` | `operators/bind.rs` | `(&[ConceptVector]) → ConceptVector` |
| `differentiate` | `operators/differentiate.rs` | `(&ConceptVector, usize) → Vec<ConceptVector>` |
| `ground` | `operators/ground.rs` | `(&[f32], Modality) → ConceptVector` |
| `evolve` | `operators/evolve.rs` | `(&WorldGraph, &StateField, Duration) → (WorldGraph, StateField)` |
| `reflect` | `operators/reflect.rs` | `(&[MemoryTrace]) → PolicyUpdate` |
| `plan` | `operators/plan.rs` | `(&WorldGraph, &StateField, &Goal) → Vec<Action>` |
| `actuate` | `operators/actuate.rs` | `(&ActionDistribution, &Embodiment) → ExternalCommand` |

---

## 5. Memory Model

### WorldGraph — The Heap

```rust
#[derive(Clone)]
pub struct GraphNode {
    pub id: NodeId,
    pub concept: ConceptVector,
    pub label: Option<String>,
    pub edges: Vec<RelationEdge>,
    pub metadata: NodeMetadata,
}

#[derive(Clone)]
pub struct WorldGraph {
    nodes: Vec<GraphNode>,
    label_index: HashMap<String, NodeId>,
    incoming: HashMap<NodeId, Vec<RelationEdge>>,
    next_id: u64,
}
```

| Operation | Signature | Description |
|-----------|-----------|-------------|
| `allocate` | `(&mut self, concept) -> NodeId` | Allocate a new node (malloc) |
| `deallocate` | `(&mut self, id)` | Remove a node (free) |
| `add_edge` | `(&mut self, from, to, relation)` | Add directed edge |
| `lookup` | `(&self, id) -> Option<&GraphNode>` | Get node by ID |
| `lookup_label` | `(&self, label) -> Option<NodeId>` | Get node by label |
| `neighbors` | `(&self, id) -> Vec<NodeId>` | Get adjacent nodes |
| `query` | `(&self, query) -> Vec<NodeId>` | Complex graph query |

### StateField — The Registers

```rust
#[derive(Clone)]
pub struct StateField {
    data: ArrayD<f32>,
    regions: HashMap<String, StateRegion>,
}

pub struct StateRegion {
    pub name: String,
    pub offset: usize,
    pub shape: Vec<usize>,
}
```

Default regions:

| Region | Offset | Shape | Purpose |
|--------|--------|-------|---------|
| `goal` | 0 | `[64]` | Current goal embedding |
| `belief` | 64 | `[256]` | Current belief state |
| `uncertainty` | 320 | `[64]` | Uncertainty estimates |
| `attention` | 384 | `[128]` | Attention focus (NodeId weights) |
| `temporal` | 512 | `[64]` | Temporal context |
| `scratch` | 576 | `[448]` | General-purpose working memory |

---

## 6. Parallel Execution (Swarm)

When the VM encounters `⥁` (parallel swarm):

1. Current VM snapshots its WorldGraph and StateField
2. Forks N child VMs, each with a copy of the snapshot
3. Each child executes the sub-program independently
4. Results are collected and merged back into parent WorldGraph
5. Conflicts detected during merge may trigger `⟁` (contradiction) and escalate

---

## 7. ISA Binary Encoding

### Header Byte

```
 7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ Protocol │ Opcode     │ Flags │
└───┴───┴───┴───┴───┴───┴───┴───┘
```

### Full Instruction Layout

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│  Header  │  Operand │  Control │   Data   │ Checksum │
│  1 byte  │ 4 bytes  │ 2 bytes  │ variable │  4 bytes │
└──────────┴──────────┴──────────┴──────────┴──────────┘
Minimum: 11 bytes
```

### Addressing Modes

| Mode | Bits | Description |
|------|:----:|-------------|
| Label index | 00 | Into program's label table |
| Direct NodeId | 01 | Numeric WorldGraph node ID |
| StateField region | 10 | Named region index |
| Immediate value | 11 | Literal float/int |

### Control Flow Ops

| Code | Mnemonic | Description |
|:----:|----------|-------------|
| 0000 | Sequential | IP += 1 |
| 0001 | Jump abs | IP = target |
| 0010 | Jump rel | IP += target (signed) |
| 0101 | Call | Push IP, jump to target |
| 0110 | Return | Pop IP |
| 0111 | Fork | Fork N child VMs |
| 1001 | Halt | Stop execution |

---

*This sub-plan maps to phases 0 and 2 of the implementation roadmap.*
