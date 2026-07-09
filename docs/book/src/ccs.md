# CCS Virtual Machine

The **Cognitive Control Substrate** (CCS) executes Ω tensor programs through
7 cognitive operators, maintaining a WorldGraph and StateField.

## Architecture

```
┌────────────────────────────────┐
│         CCS VM (CcsVm)         │
│                                │
│  ┌──────────┐  ┌────────────┐  │
│  │WorldGraph│  │ StateField │  │
│  │(petgraph)│  │ (ndarray)  │  │
│  └──────────┘  └────────────┘  │
│  ┌──────────────────────────┐  │
│  │     MemoryTrace           │  │
│  │  (RLE + hash dedupe)     │  │
│  └──────────────────────────┘  │
│  ┌──────────────────────────┐  │
│  │   Scheduler + Safety      │  │
│  └──────────────────────────┘  │
└────────────────────────────────┘
```

## Execution Loop

```
LOAD Ω program → map opcodes → iterate:

  1. BIND          → bind(context, state)
  2. DIFFERENTIATE → differentiate(context, state)
  3. GROUND        → ground(context, state)
  4. EVOLVE        → evolve(state)
  5. REFLECT       → reflect(state)
  6. PLAN          → plan(state)
  7. ACTUATE       → actuate(state) → ExternalCommand
```

## The 7 Operators

| # | Operator | Description |
|---|----------|-------------|
| 1 | **Bind** | Average concept vectors — merge ideas |
| 2 | **Differentiate** | Split concepts — find distinctions |
| 3 | **Ground** | Import external data into concepts |
| 4 | **Evolve** | Time-step concepts forward (attention decay) |
| 5 | **Reflect** | Allocate self-model node |
| 6 | **Plan** | Generate action sequence |
| 7 | **Actuate** | Produce external commands |

## Fork/Merge (Parallel Execution)

The VM supports parallel execution via Fork and Merge:

```rust
let child_vm = vm.fork()?;              // Snapshot child VM
child_vm.load(parallel_program)?;
child_vm.run()?;
vm.merge(child_vm)?;                    // Merge results back
```

Multiple child VMs can execute simultaneously, then merge their WorldGraph
deltas into the parent.

## Async VM

The `AsyncCcsVm` wraps the synchronous VM for use in async contexts (tokio):

```rust
use a2x_ccs::AsyncCcsVm;

let vm = AsyncCcsVm::new(safety_level)?;
vm.load(&omega_program).await?;
vm.run().await?;
```

Uses `block_in_place` to avoid tokio worker starvation during compute-heavy
execution.
