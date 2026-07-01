# A2X CCS Virtual Machine

> Reference for the Cognitive Control Substrate (CCS) — the virtual machine
> that executes cognitive programs using a WorldGraph and StateField.

## Architecture

The CCS VM (`CcsVm`) executes Ω tensor programs through 7 cognitive operators:

| Operator | Function | Description |
|----------|----------|-------------|
| **Bind** | `bind()` | Average concept vectors — merge ideas |
| **Differentiate** | `differentiate()` | Split concepts — find distinctions |
| **Ground** | `ground()` | Import external data into concepts |
| **Evolve** | `evolve()` | Time-step concepts forward |
| **Reflect** | `reflect()` | Allocate self-model node |
| **Plan** | `plan()` | Generate action sequence |
| **Actuate** | `actuate()` | Produce external commands |

## WorldGraph

The WorldGraph is a directed graph where:
- **Nodes** are concepts (identified by `NodeId`)
- **Edges** are relations (causal, spatial, temporal, logical, hierarchical)
- **ConceptVectors** store the latent representation of each concept

### Query Types

- `ByLabel(name)` — find nodes by label
- `NeighborsOf(id, hops)` — traverse the graph
- `ByRelation(relation_type)` — filter by relation
- `Custom(bytes)` — parse structured query (e.g., `"neighbors:<id>:<hops>"`)

## StateField

The StateField stores typed regions:
- **belief** — current belief state
- **goal** — target state
- **memory** — compressed memory trace
- `region_0` through `region_7` — general-purpose regions

## MemoryTrace

The MemoryTrace records execution history. Each entry stores:
- The opcode that executed
- The instruction pointer before execution
- StateField snapshot (optional, for debugging)

Capacity is configurable (default: 1000 entries).

## Safety Levels

| Level | Enforcement |
|-------|-------------|
| `Production` | Checks nodes_allocated * 4 KiB against max_memory_bytes |
| `Bounded { max_memory_bytes, max_ip }` | Enforces both memory and IP bounds |
| `Unsafe` | No enforcement (development only) |

## Execution Loop

```
LOAD program → map opcodes → iterate instructions:
  1. BIND → bind(context, state)
  2. DIFFERENTIATE → differentiate(context, state)
  3. GROUND → ground(context, state)
  4. EVOLVE → evolve(state)
  5. REFLECT → reflect(state)
  6. PLAN → plan(state)
  7. ACTUATE → actuate(actions) → ExternalCommand
```

## See Also

- [Σ∞ Protocol Reference](sigma-protocol.md)
- [Ω Compilation Pipeline](omega-compilation.md)
- `plans/03-ccs-vm.md` — Full VM design spec
