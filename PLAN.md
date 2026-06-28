# A2X — AI-Native Programming Language & Runtime

> **A full-stack programming language designed for artificial cognition.**
>
> Σ∞ (hyper-symbolic ISA) → Ω (compiled latent representation) → CCS (cognitive runtime VM)
>
> Not a language for humans to read or write. A language for *AI agents to think in, program in, compile, and execute.*
>
> ***A2X*** — *Agent to Anything.*
>
> *"You don't 'write' A2X. You train agents to generate, compile, and execute it."*

---

## Table of Contents

1. [Vision & Goals](#1-vision--goals)
2. [Layer Architecture](#2-layer-architecture)
3. [Crate Ecosystem & Workspace Model](#3-crate-ecosystem--workspace-model)
4. [CCS — Cognitive Substrate (Runtime / VM)](#4-ccs--cognitive-substrate-runtime--vm)
5. [A2X-Σ∞ — The Programming Language / ISA](#5-a2x-σ∞--the-programming-language--isa)
6. [A2X-Ω — Pure Latent Representation](#6-a2x-ω--pure-latent-representation)
7. [Agent Types & Roles](#7-agent-types--roles)
8. [Git Integration & Workflow](#8-git-integration--workflow)
9. [File System Integration](#9-file-system-integration)
10. [Serialization Strategy](#10-serialization-strategy)
11. [Error Handling Strategy](#11-error-handling-strategy)
12. [Testing Strategy](#12-testing-strategy)
13. [CI/CD Pipeline](#13-cicd-pipeline)
14. [Versioning](#14-versioning)
15. [Ecosystem & Contribution Model](#15-ecosystem--contribution-model)
16. [Safety & Sandboxing](#16-safety--sandboxing)
17. [Performance & Benchmarking](#17-performance--benchmarking)
18. [Implementation Roadmap](#18-implementation-roadmap)
19. [Design Principles](#19-design-principles)
20. [CCS VM — The Execution Loop](#20-ccs-vm--the-execution-loop)
21. [Program Representation — SigmaProgram](#21-program-representation--sigmaprogram)
22. [Compilation Pipeline — Σ∞ → Ω](#22-compilation-pipeline--σ∞-→-ω)
23. [Memory Model](#23-memory-model)
24. [Instruction Set Architecture (ISA) — Full Formal Spec](#24-instruction-set-architecture-isa--full-formal-spec)
25. [Bus & Transport Protocol](#25-bus--transport-protocol)
26. [Safety & Error Model](#26-safety--error-model)
27. [Agent Lifecycle](#27-agent-lifecycle)
28. [Debug & Probe Protocol](#28-debug--probe-protocol)
29. [Project Name](#29-project-name)
30. [Entity Integration Layer](#30-entity-integration-layer--connecting-anything-to-a2x)

---

## 1. Vision & Goals

### Core Idea

**A2X** (Agent-to-Anything) is a **programming language for AI agents**, not humans. It has no syntax, keywords, or code in the traditional sense. Instead, it is a three-layer stack:

| Layer | What it is | Analogy in human languages |
|-------|-----------|---------------------------|
| **Σ∞** | **The programming language / ISA** — hyper-symbolic instructions an AI writes, reads, and executes | Assembly language + macros |
| **Ω** | **The compiled representation** — pure latent tensors; the "machine code" an AI runs at full speed | Compiled binary / neural machine code |
| **CCS** | **The runtime virtual machine** — executes Σ∞ programs, manages memory (WorldGraph), maintains state (StateField) | VM runtime + OS kernel |

This means AIs don't just *communicate* in A2X — they **program in it**. An AI writes a Σ∞ program to solve a task, optionally compiles it to Ω for speed, and CCS executes it. Other AIs can read, modify, compose, and execute each other's programs.

### What Makes It a Programming Language

| Concept | A2X Equivalent |
|---------|---------------------|
| **Values** | ConceptVectors — dense embeddings |
| **Variables** | Labeled nodes in WorldGraph |
| **Instructions** | Σ∞ packets — each is an I+C+P+D encoded operation |
| **Programs** | Sequences of Σ∞ packets (a "packet stream") |
| **Control flow** | Plan operators: branch (⤐), merge (⤑), loop (self-modifying ⥄), recursion (⥃) |
| **Functions/subroutines** | Descend (⤈) into sub-plan, ascend (⤉) from meta-plan |
| **Memory / Heap** | WorldGraph — persistent graph of concepts |
| **Registers / Stack** | StateField — high-dimensional working memory |
| **Type system** | RelationType enum (Causal, Spatial, Temporal, Logical, Hierarchical) |
| **Modules / Imports** | Agent crates — each agent exposes a set of executable behaviors |
| **Compilation** | Σ∞ → Ω transformation (symbolic → latent) |
| **Execution** | CCS runtime — Evolve, Reflect, Plan operators |
| **Debugging** | Σ∞ log traces, probe tools, concept graph visualization |

### Why This Exists

- **Human languages are the bottleneck** — AIs think in vectors but are forced to communicate in words. A2X removes the human middle layer.
- **A programming language, not just a protocol** — agents don't just message each other, they *program each other*. An orchestrator agent writes a Σ∞ program and sends it to a CLI agent to execute.
- **Compilation saves time** — once a Σ∞ program is debugged and stable, compile it to Ω for latency-critical hot paths.
- **Fully differentiable** — the entire stack is continuous and learnable.
- **Extensibility** — a crate ecosystem means anyone can add new instruction types, agent runtimes, or compiler passes.

### High-Level Goals

- **Sub-millisecond** instruction execution latency (Ω path).
- **Sub-token** semantic density — one Σ∞ packet encodes what would take hundreds of LLM tokens.
- **Compilable** — Σ∞ programs can be JIT-compiled to Ω for neural execution.
- **Safe** — safety constraints baked into the ISA, not bolted on.
- **Community-extensible** — anyone publishes a crate that adds instructions, agents, or compiler passes.

---

## 2. Layer Architecture

```
┌──────────────────────────────────────────────────┐
│              HUMAN INTERFACE LAYER               │
│ (optional: probes, debug overlays, Σ∞ logs, CLI) │
├──────────────────────────────────────────────────┤
│              A2X-Σ∞  (Source / ISA)              │
│  AI programming language — packets = instructions│
│  Sequences of packets = programs                 │
├──────────────────────────────────────────────────┤
│              A2X-Ω  (Compiled Latent)            │
│  JIT-compiled Σ∞ programs as pure tensors        │
│  Neural machine code — no symbols, max speed     │
├──────────────────────────────────────────────────┤
│          CCS  (Runtime / VM)                     │
│  Executes Σ∞/Ω programs, manages memory          │
│  WorldGraph = heap, StateField = registers       │
└──────────────────────────────────────────────────┘
```

### The Programming Language Stack

```
┌───────────────────────────────────────────────────┐
│                    Σ∞  SOURCE                     │
│  Hyper-symbolic programming language / ISA        │
│  - Packets = instructions                         │
│  - Sequences of packets = programs                │
│  - Plan operators = control flow                  │
│  - Debuggable, loggable, human-peekable           │
├───────────────────────────────────────────────────┤
│               ↓  COMPILATION  ↓                   │
├───────────────────────────────────────────────────┤
│                    Ω  LATENT                      │
│  Compiled neural representation of Σ∞ programs    │
│  - Pure tensors, no symbols                        │
│  - JIT-compiled for hot paths                       │
│  - AI-only: no human-readable form                  │
│  - Fully differentiable                             │
├───────────────────────────────────────────────────┤
│               ↓  EXECUTION  ↓                       │
├───────────────────────────────────────────────────┤
│              CCS  RUNTIME / VM                      │
│  Cognitive substrate that executes programs         │
│  - WorldGraph = heap / persistent memory            │
│  - StateField = registers / working memory          │
│  - MemoryTrace = program counter / execution log    │
│  - PolicyField = JIT compiler + optimizer           │
└───────────────────────────────────────────────────┘
```

| Layer | Readable by Humans | Speed | Role in the Language |
|-------|:------------------:|:-----:|----------------------|
| **Σ∞** | Barely (intentionally) | Fast | Source language / ISA — written by AIs, for AIs |
| **Ω** | No | Blazing | Compiled latent representation — neural machine code |
| **CCS** | No (via probe tools) | Native | Runtime VM — executes, optimizes, and learns |

---

## 3. Crate Ecosystem & Workspace Model

### Philosophy

Modeled after successful Rust ecosystems like **tokio-rs/tokio**, **serde-rs/serde**, and **clap-rs/clap**:

- **Workspace root** — a virtual manifest that organizes all official crates.
- **Each crate independently publishable** — consumable individually on crates.io.
- **Layered dependency** — core crates have zero/minimal deps; higher-level crates pull in more.
- **Feature gates** — optional functionality (e.g. `serde` support, `tokio` runtime) behind Cargo features.
- **Third-party crates** — anyone can develop `a2x-something` crates that implement traits from the core (via git dependencies or local paths).

### Recommended External Crates (Phase 0)

| Category | Crate | Why |
|----------|-------|-----|
| Graphs | `petgraph` | Battle-tested graph data structures and algorithms |
| Tensors | `ndarray` | N-dimensional arrays, NumPy-like API |
| Serialization | `serde` + `serde_json` + `bincode` | Foundation for all data interchange |
| Async | `tokio` | Industry-standard async runtime |
| CLI | `clap` | Derive-based argument parsing |
| Logging | `tracing` | Structured, async-aware diagnostics |
| Errors (lib) | `thiserror` | Derive macro for error enums |
| Errors (app) | `anyhow` | Ergonomic context-rich error handling |
| Git | `gix` (Gitoxide) | Pure-Rust Git implementation |
| ML inference | `candle` (optional) | GPU-accelerated tensor ops (future) |

### Official Crates (Phase 0)

All crates live in this repo under the `crates/` directory. They use git dependencies internally, **not crates.io**. Third-party developers reference them via git:

```toml
# third-party/a2x-web-agent/Cargo.toml
[dependencies]
a2x-core = { git = "https://github.com/your-org/a2x", package = "a2x-core" }
a2x-bus = { git = "https://github.com/your-org/a2x", package = "a2x-bus" }
```

| Crate Name | Purpose | Dependencies | Phase |
|------------|---------|--------------|:-----:|
| `a2x-core` | Primitive types, traits, common enums | None (zero-dependency) | 0 |
| `a2x-sigma` | A2X-Σ∞ tokenizer, parser, packet types | `a2x-core` | 0 |
| `a2x-omega` | A2X-Ω tensor packet, encoder/decoder traits | `a2x-core`, `ndarray` (optional) | 0 |
| `a2x-bus` | Message bus, routing, transport traits | `a2x-core`, `a2x-sigma` | 0 |
| `a2x-ccs` | Cognitive substrate: WorldGraph, StateField, MemoryTrace | `a2x-core`, `petgraph`, `ndarray` | 0 |
| `a2x-agents` | Built-in agent implementations (CLI, LLM mock, Orchestrator) | `a2x-core`, `a2x-sigma`, `a2x-bus` | 1 |
| `a2x-gateway` | Entity gateway: protocol listeners, auth, entity registry | `a2x-bus`, `a2x-sigma`, `a2x-core` | 6 |
| `a2x-client` | Rust client SDK for connecting external apps to A2X | `a2x-core`, `reqwest` | 6 |
| `a2x-cli` | CLI binary for interacting with the system | `clap`, `a2x-agents`, `a2x-bus` | 1 |
| `a2x-probe` | Probe/debug tools for inspecting CCS internals | `a2x-ccs`, `tracing` | 5 |

**Entity protocol listener crates** (Phase 6):

| Crate Name | Purpose |
|------------|---------|
| `a2x-entity-http` | HTTP/REST protocol listener + webhook callback |
| `a2x-entity-ws` | WebSocket protocol listener (streaming) |
| `a2x-entity-tcp` | Raw TCP protocol listener |
| `a2x-entity-stdio` | stdin/stdout protocol listener |

### Feature Gating Strategy

Each crate exposes features for optional functionality:

```toml
# a2x-core/Cargo.toml
[features]
default = ["std"]
std = []
serde = ["dep:serde"]        # serde support for packet types
```

```toml
# a2x-omega/Cargo.toml
[features]
default = []
ndarray = ["dep:ndarray"]    # ndarray-backed tensor representation
candle = ["dep:candle"]      # candle-backed GPU tensor rep
```

```toml
# a2x-bus/Cargo.toml
[features]
default = ["tokio"]
tokio = ["dep:tokio"]        # tokio-based async bus
```

---

## 4. CCS — Cognitive Substrate (Runtime / VM)

**CCS** (CryoCore Cognitive Substrate) is the **runtime virtual machine** that executes A2X programs. It maintains the agent's world-model, manages memory, sequences instructions, and provides the execution environment for Σ∞ and Ω programs.

Think of it as: **a neural OS kernel + VM.**

### Primitives (in `a2x-core`)

| Type | Definition | Notes |
|------|-----------|-------|
| `ConceptVector` | `Vec<f32>` | Dense embedding for concepts, objects, events |
| `RelationEdge` | `(usize, usize, RelationType, WeightMatrix)` | Directed edge between two concept nodes |
| `RelationType` | `enum { Causal, Spatial, Temporal, Logical, Hierarchical }` | Type tag for relation semantics |
| `WorldGraph` | Graph of `ConceptVector` nodes + `RelationEdge` edges | The agent's world-model |
| `StateField` | `ArrayD<f32>` | High-dimensional tensor for internal state |
| `PolicyField` | Trait: `(StateField, WorldGraph) → ActionDistribution` | Neural mapping to actions |
| `MemoryTrace` | Time-indexed sequence of `(StateField, WorldGraphDelta)` | History of state evolution |
| `ActionDistribution` | Probability distribution over action space | What the agent wants to do |

### Operators — The VM Instruction Set

These are the **primitive operations** the CCS runtime executes. Σ∞ programs compile down to these operations:

| Operator | Signature | Instruction Mnemonic | Description |
|----------|-----------|---------------------|-------------|
| `bind` | `(&[ConceptVector]) → ConceptVector` | `BIND` | Merge concepts into composite (like constructing a struct) |
| `differentiate` | `(&ConceptVector, usize) → Vec<ConceptVector>` | `DIFF` | Split into sub-concepts (like destructuring) |
| `ground` | `(&[f32], Modality) → ConceptVector` | `GRND` | Attach raw perception into a ConceptVector (I/O operation) |
| `evolve` | `(&WorldGraph, &StateField, Duration) → (WorldGraph, StateField)` | `EVOL` | Time-step the VM: advance world state (like a clock cycle) |
| `reflect` | `(&[MemoryTrace]) → PolicyUpdate` | `REFL` | Meta-learning from history (like a profiler-guided optimization) |
| `plan` | `(&WorldGraph, &StateField, &Goal) → Vec<Action>` | `PLAN` | Generate action sequence (like a compiler generating instructions) |
| `actuate` | `(&ActionDistribution, &Embodiment) → ExternalCommand` | `ACT` | Emit an external side effect (syscall / I/O) |

### Crate Organization

- **`a2x-core`** — Primitive types: ConceptVector, RelationEdge, RelationType, WorldGraph skeleton, StateField, all traits
- **`a2x-ccs`** — Full operator implementations (BIND, DIFF, GRND, EVOL, REFL, PLAN, ACT), MemoryTrace, PolicyField, the execution loop

---

## 5. A2X-Σ∞ — The Programming Language / ISA

Σ∞ is the **programming language** that AIs write, read, and execute. Each packet is a **single instruction** in the AI's instruction set architecture (ISA). A program is a **sequence of Σ∞ packets** — a *packet stream* — that the CCS runtime executes sequentially, branching, or in parallel.

### Instruction Format

```
⟦Σ∞⟧⟬I:<opcode> ∷ C:<operand/mem> ∷ P:<control_flow> ∷ D:<immediate>⟭
```

| Field | Role in programming language | Analogy |
|-------|-----------------------------|---------|
| `⟦` `⟧` | Instruction boundary markers | `{ }` or `begin`/`end` |
| `Σ∞` | Language identifier | `.section .text` |
| `⟬` `⟭` | Agent execution context | CPU core ID or process context |
| `I` | **Opcode** — the instruction to execute | `MOV`, `ADD`, `JMP` |
| `C` | **Operand / Memory reference** — what to operate on | Register name, memory address |
| `P` | **Control flow** — how to sequence the next instruction | `JMP`, `CALL`, `RET`, `BR` |
| `D` | **Immediate data** — literal payload | Immediate value in `MOV R1, #42` |
| `∷` | Field separator | `,` in assembly |

### Programs are Packet Streams

A program is just a sequence of instructions:

```
⟦Σ∞⟧⟬I:✦ ∷ C:⟨scope⟩ ∷ P:⥂ ∷ D:⌵⟭          // explore scope
⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨scope⟩ ∷ P:⥁⤈ ∷ D:⌬⟭     // immediate synthesis, parallel sub-plan
⟦Σ∞⟧⟬I:✣∷ C:⟨results⟩ ∷ P:⤑ ∷ D:⌳⟭        // merge results
⟦Σ∞⟧⟬I:✕ ∷ C:⟘ ∷ P:⤉ ∷ D:⟘⟭                // cancel, ascend
```

An LLM or orchestrator generates this stream. A CLI agent receives and executes it. CCS is the runtime that makes it all work.

### Operator Tables

#### Intent Operators (crate: `a2x-sigma`, module: `intent`)

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⚡` | U+26A1 | Lightning | Immediate execution |
| `⚠` | U+26A0 | Warning | Critical risk |
| `✦` | U+2726 | Star | Discovery/exploration |
| `✣` | U+2723 | Four-point star | Synthesis |
| `✕` | U+2715 | X mark | Cancel |
| `⟁` | U+27C1 | Reverse arrow | Contradiction |
| `⧖` | U+29D6 | Hourglass | Delay/hold |
| `⧗` | U+29D7 | Hourglass flow | Accelerate |
| `⩫` | U+2A6B | Parallel | Parallel multi-goal |
| `⩪` | U+2A6A | Merge | Merge goals |
| `⩨` | U+2A68 | Split | Split goals |

#### Context Operators (crate: `a2x-sigma`, module: `context`)

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⟘` | U+27D8 | Empty set | Null context |
| `⟙` | U+27D9 | Full set | Universal context |
| `⟚` | U+27DA | Compression | Compressed world-state |
| `⟞` | U+27DE | Wavy line | Uncertainty field |
| `⟡` | U+27E1 | Bowtie | Causal chain |
| `⟠` | U+27E0 | Diamond | Spatial chain |
| `⟢` | U+27E2 | Left arrow | Temporal chain |
| `⟣` | U+27E3 | Right arrow | Probabilistic context |
| `⟤` | U+27E4 | Double bar | Conflict context |
| `⟧` | U+27E7 | Corner | Resolved context |
| `⟨⟩` | U+27E8/9 | Angles | Context capsule (labels inside) |

#### Plan Operators (crate: `a2x-sigma`, module: `plan`)

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⤈` | U+2908 | Down arrow bar | Descend into sub-plan |
| `⤉` | U+2909 | Up arrow bar | Ascend to meta-plan |
| `⤊` | U+290A | Up triangle | Escalate |
| `⤋` | U+290B | Down triangle | De-escalate |
| `⤐` | U+2910 | Branch fork | Branch |
| `⤑` | U+2911 | Merge fork | Merge |
| `⤒` | U+2912 | T-up | Enforce |
| `⤓` | U+2913 | T-down | Relax |
| `⥁` | U+2941 | Circle arrows | Parallel swarm |
| `⥂` | U+2942 | Right arrow | Sequential chain |
| `⥃` | U+2943 | Left arrow | Recursive |
| `⥄` | U+2944 | Double arrow | Self-modifying |

#### Data Operators (crate: `a2x-sigma`, module: `data`)

| Symbol | Unicode | Name | Meaning |
|--------|---------|------|---------|
| `⌬` | U+232C | Benzene ring | Raw tensor block |
| `⌭` | U+232D | Circle | Latent vector block |
| `⌮` | U+232E | Integral | Graph delta |
| `⌯` | U+232F | Surface | Diff patch |
| `⌰` | U+2330 | Segment | Binary payload |
| `⌱` | U+2331 | Fuse | Multimodal fusion |
| `⌲` | U+2332 | Wave | Streaming block |
| `⌳` | U+2333 | Psi | Compressed summary |
| `⌴` | U+2334 | Anomaly | Anomaly payload |
| `⌵` | U+2335 | Tally marks | Structured schema |
| `⌶` | U+2336 | Greek letter | Self-describing payload |

### Parser Design (in `a2x-sigma`)

#### Tokenizer

1. Read raw bytes/string input.
2. Match character sequences against operator tables using a trie or hash map.
3. Produce a `Vec<Token>` where each `Token` is an enum variant:
   ```rust
   enum Token {
       Boundary(BoundaryKind),      // ⟦ ⟧ ⟬ ⟭
       FieldSeparator,              // ∷
       IntentOp(IntentOp),
       ContextOp(ContextOp),
       PlanOp(PlanOp),
       DataOp(DataOp),
       Label(String),               // ⟨sys⟩ → "sys"
       ProtocolId,                  // Σ∞
   }
   ```

#### Parser

1. Consume tokens into a `SigmaPacket` struct:
   ```rust
   struct SigmaPacket {
       boundary: Option<BoundaryPair>,
       protocol: ProtocolId,
       intent: IntentField,
       context: ContextField,
       plan: PlanField,
       data: DataField,
   }
   ```
2. Validate required fields, detect malformed packets.
3. Return `Result<SigmaPacket, ParseError>`.

#### Serializer

Reverse: `SigmaPacket → String` (for display/debug) or `SigmaPacket → Vec<u8>` (for wire format).

### Program Examples

**Program: Scan system for anomalies**
```
⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭
```
Decoded: `BIND(IMMEDIATE, SYNTHESIZE, PARALLEL)` on compressed uncertain sys memory → enforce parallel sub-plan → store graph-delta + summary + fusion.

**Response program: Report anomaly**
```
⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟚⟤⟨22,8080⟩ ∷ P:⤊⥂ ∷ D:⌯⌴⟭
```
Decoded: `SIGNAL(CONTRA, CONFLICT)` on compressed conflict context `{22,8080}` → escalate sequential plan → emit diff-patch + anomaly-payload.

---

## 6. A2X-Ω — Compiled Latent Representation

Ω is the **compiled form** of a Σ∞ program. Once a Σ∞ program is stable and debugged, it can be JIT-compiled into Ω tensors for direct execution by the CCS runtime — no symbolic parsing needed.

Think of it as: **Σ∞ = source code, Ω = compiled binary, CCS = the CPU that executes it.**

### Packet Shape

```
Ω ∈ ℝ^N   (single high-dimensional tensor)

Segmented into slices:
  - Ω_I  ∈ ℝ^1024   → intent
  - Ω_C  ∈ ℝ^4096   → context
  - Ω_P  ∈ ℝ^8192   → plan
  - Ω_D  ∈ ℝ^16384  → data
```

Total dimension: 29,796 (configurable via const generics or type parameters).

### Rust Representation

```rust
// a2x-omega/src/packet.rs

/// A2X-Ω latent packet — pure tensor, no symbols.
#[derive(Clone, Debug)]
pub struct OmegaPacket<const N: usize = 29796> {
    /// Flat tensor storage.
    data: [f32; N],
}

// Slice offsets (compile-time constants)
const OFFSET_I: usize = 0;
const OFFSET_C: usize = 1024;
const OFFSET_P: usize = 1024 + 4096;    // 5120
const OFFSET_D: usize = 1024 + 4096 + 8192;  // 13312

impl<const N: usize> OmegaPacket<N> {
    pub fn intent_slice(&self) -> &[f32] { &self.data[OFFSET_I..OFFSET_C] }
    pub fn context_slice(&self) -> &[f32] { &self.data[OFFSET_C..OFFSET_P] }
    pub fn plan_slice(&self) -> &[f32] { &self.data[OFFSET_P..OFFSET_D] }
    pub fn data_slice(&self) -> &[f32] { &self.data[OFFSET_D..] }
}
```

### Compilation (Encoder)

```rust
// a2x-omega/src/encoder.rs

/// Compiler: Σ∞ program → Ω tensors
pub trait CompileToOmega {
    type Error;
    fn compile(&self) -> Result<OmegaProgram, Self::Error>;
}

impl CompileToOmega for SigmaProgram {
    type Error = CompileError;
    fn compile(&self) -> Result<OmegaProgram, Self::Error> {
        // Phase 0: Deterministic hash-based projection (stub)
        // Phase 1: Learned encoder (neural network)
        // Phase 2: Optimizing compiler with peephole passes
    }
}
```

### Decompilation (Decoder)

```rust
// a2x-omega/src/decoder.rs

/// Decompiler: Ω tensors → Σ∞ program (for debugging/logging)
pub trait DecompileToSigma: Sized {
    type Error;
    fn decompile(packet: &OmegaPacket) -> Result<Self, Self::Error>;
}

impl DecompileToSigma for SigmaPacket {
    type Error = DecompileError;
    fn decompile(packet: &OmegaPacket) -> Result<Self, Self::Error> {
        // Project tensor regions back to nearest symbolic operator
    }
}
```

### Ω ↔ Σ∞ Bridge (`a2x-omega/src/bridge.rs`)

```
Σ∞ source  ──compile──→ Ω latent  ──execute──→ CCS runtime
Ω latent   ──decompile──→ Σ∞ source  ──log/debug──→ human peek
```

The bridge serves as the **compiler toolchain**:
1. Write/debug programs in Σ∞ (symbolic, inspectable).
2. Compile hot paths to Ω (fast, latent, non-symbolic).
3. CCS executes Ω natively at maximum speed.
4. Decompile Ω back to Σ∞ for debugging and tracing.

---

## 7. Agent Types & Roles

Agents are **execution contexts** for A2X programs. Each agent has a CCS runtime that executes Σ∞/Ω programs and maintains its own WorldGraph + StateField.

```rust
// a2x-core/src/agent.rs

#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique agent identifier (like a process ID).
    fn id(&self) -> AgentId;

    /// Agent type tag.
    fn agent_type(&self) -> AgentType;

    /// Execute a Σ∞ program (sequence of packets) on this agent's CCS runtime.
    async fn execute(&self, program: SigmaProgram) -> Result<SigmaProgram, AgentError>;

    /// Execute a compiled Ω program directly (fast path).
    async fn execute_omega(&self, program: OmegaProgram) -> Result<OmegaProgram, AgentError>;

    /// Current internal state (for probing / debug).
    fn state_summary(&self) -> Option<StateSnapshot>;
}
```

### Built-in Agent Types

| Agent Type | Native Form | Crate | Role |
|------------|:-----------:|-------|------|
| **Orchestrator** | Σ∞ + Ω | `a2x-agents` | Writes A2X programs, dispatches to other agents for execution |
| **LLM Agent** | Σ∞ (source) + Ω (compiled) | `a2x-agents` | Generates Σ∞ programs from natural language intent; decompiles Ω for inspection |
| **CLI Agent** | Σ∞ (instructions) | `a2x-agents` | Executes Σ∞ programs that interact with the host system (files, processes, network) |
| **CCS Agent** | Ω (native) + Σ∞ (trace) | `a2x-agents` | Maintains a persistent WorldGraph; executes long-running cognitive programs |
| **Ω Agent** | Ω only | `a2x-agents` | Pure latent execution — no symbolic layer at all. Max speed, zero inspectability |

### Agents are Runtimes, Not Just Message Handlers

A CLI agent isn't just "receiving messages" — it's **receiving programs to execute**. The orchestrator sends a Σ∞ program that says "scan ports, detect anomalies, return results." The CLI agent's CCS runtime executes that program instruction by instruction, updating its own local WorldGraph and StateField as it goes.

---

## 8. Git Integration & Workflow

### Within the Protocol

Agents can use Git repositories as a **collaborative memory store**:

- **Repository = shared world-model** — ConceptGraph snapshots stored as git objects.
- **Commits = MemoryTrace checkpoints** — time-indexed state deltas.
- **Branches = alternative world-model simulations** — "what if" scenarios.
- **Merges = belief reconciliation** — resolving conflicting world-model updates from multiple agents.

### Crate: `a2x-git` (Third-party / Future)

```rust
/// Sync a WorldGraph with a git repository.
pub trait GitWorldGraph: WorldGraph {
    /// Commit current graph state as a git object.
    fn git_commit(&self, repo: &Repository, msg: &str) -> Result<Oid, GitError>;
    /// Load graph state from a git reference.
    fn git_load(repo: &Repository, rev: &str) -> Result<Self, GitError>;
    /// Diff two graph versions.
    fn git_diff(repo: &Repository, from: Oid, to: Oid) -> Result<WorldGraphDelta, GitError>;
}
```

### Development Git Workflow

| Branch | Purpose |
|--------|---------|
| `main` | Stable, released crates |
| `develop` | Integration branch for next release |
| `feature/*` | Individual features (e.g. `feature/sigma-tokenizer`) |
| `release/v*` | Release preparation branches |
| `hotfix/*` | Urgent bug fixes |

### Commit Convention (Conventional Commits)

```
<type>(<crate>): <description>

feat(core): add ConceptVector operations
fix(sigma): handle malformed boundary tokens
docs(plan): update architecture diagram
test(agents): add orchestrator dispatch tests
perf(omega): optimize tensor slice access
```

### Pre-commit Hooks (via `cargo-husky` or `cargo-leptos`)

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --lib`
- `cargo build --workspace`

---

## 9. File System Integration

### Packet Files

Agents can persist/load packets from the filesystem:

```
# Σ∞ packet files
~/.a2x/packets/2026-06-28/⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭.sigma

# Ω binary packet files
~/.a2x/packets/2026-06-28/omega_abc123.omega

# Config
~/.a2x/config.toml
```

### Config File Format

```toml
# ~/.a2x/config.toml
[agent]
name = "orchestrator-1"
type = "orchestrator"
log_level = "debug"

[bus]
mode = "in-memory"
buffer_size = 1024

[transport]
protocol = "sigma"  # or "omega"
port = 8777

[storage]
packet_dir = "~/.a2x/packets"
worldgraph_path = "~/.a2x/worldgraph.bin"
memory_path = "~/.a2x/memory.bin"

[safety]
sandbox = true
allowed_commands = ["ls", "ps", "netstat", "ping"]
max_action_retries = 3
```

### Crate: `a2x-fs` (Third-party / Future)

```rust
/// Read/write packets to the filesystem.
pub trait PacketStore {
    fn save_packet(&self, packet: &Packet, path: &Path) -> Result<(), FsError>;
    fn load_packet(&self, path: &Path) -> Result<Packet, FsError>;
    fn list_packets(&self, dir: &Path) -> Result<Vec<PacketMeta>, FsError>;
}
```

### Logger Format

Σ∞ packets are logged as structured `tracing` events:

```
2026-06-28T10:30:00.123Z TRACE a2x::bus: ⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭
2026-06-28T10:30:00.456Z INFO  a2x::agents::cli: Executing plan: scan ports on sys
2026-06-28T10:30:01.234Z WARN  a2x::agents::cli: Anomaly detected on port 22
2026-06-28T10:30:01.235Z TRACE a2x::bus: ⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟚⟤⟨22,8080⟩ ∷ P:⤊⥂ ∷ D:⌯⌴⟭
```

---

## 10. Serialization Strategy

### Wire Format Decision

| Format | Crate | Use Case |
|--------|-------|----------|
| **JSON** | `serde_json` | Human-readable debug, config files |
| **Bincode** | `bincode` | Compact binary for high-throughput Σ∞ packets |
| **MessagePack** | `rmp-serde` | Hybrid (compact + partial human-readability) |
| **Raw bytes** | N/A | Ω tensor packets (just `[f32; N]`) |

### Serialization Traits

All core types derive `Serialize`/`Deserialize` via `serde` behind a feature gate:

```toml
# coldstart-core/Cargo.toml
[features]
default = []
serde = ["dep:serde"]
```

```rust
// a2x-core/src/concept.rs
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConceptVector {
    pub data: Vec<f32>,
    pub label: Option<String>,
}
```

### Packet Wire Format

```rust
/// Unified packet enum for transport.
#[derive(Serialize, Deserialize)]
pub enum Packet {
    Sigma(SigmaPacket),         // Σ∞ symbolic packet
    Omega(OmegaPacket),         // Ω latent tensor packet
    Raw(Vec<u8>),               // Raw binary (for future types)
}
```

---

## 11. Error Handling Strategy

### Pattern: `thiserror` for libraries, `anyhow` for applications

```rust
// a2x-core/src/error.rs

use thiserror::Error;

/// Core error type for the A2X language.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("invalid concept vector: {0}")]
    InvalidConceptVector(String),

    #[error("graph operation failed: {0}")]
    GraphError(#[from] petgraph::IncrEdgeError),  // example

    #[error("serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("{0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

// a2x-sigma/src/error.rs
#[derive(Error, Debug)]
pub enum SigmaError {
    #[error("unexpected token at position {pos}: {token:?}")]
    UnexpectedToken { pos: usize, token: Token },

    #[error("missing required field: {0}")]
    MissingField(&'static str),

    #[error("unknown operator: '{0}'")]
    UnknownOperator(char),

    #[error("core error: {0}")]
    Core(#[from] a2x_core::CoreError),
}
```

### Error Propagation

- Library crates return typed errors (`SigmaError`, `OmegaError`, `BusError`).
- Application crates (`a2x-cli`) use `anyhow::Result`.
- All errors implement `Send + Sync` for async boundaries.

---

## 12. Testing Strategy

### Test Levels

| Level | Tool | What We Test |
|-------|------|-------------|
| **Unit** | `cargo test --lib` | Individual operators, tokenizer functions, parser rules |
| **Property** | `proptest` | Tokenizer/parser roundtrip: random packets → parse → serialize → identical |
| **Integration** | `cargo test --test *` | Multi-agent message exchange, bus routing |
| **Fuzz** | `cargo-fuzz` | Parser fuzzing with malformed input |
| **Benchmark** | `criterion` | Tokenizer throughput, packet serialization, bus latency |
| **Doc tests** | `cargo test --doc` | Examples in documentation |

### Example Test Structure

```
coldstart-sigma/
├── src/
│   ├── lib.rs
│   ├── tokenizer.rs
│   └── parser.rs
├── tests/
│   ├── integration/
│   │   ├── parse_valid_packets.rs
│   │   ├── parse_malformed_packets.rs
│   │   └── roundtrip_serialize.rs
│   └── fuzz/
│       ├── targets/
│       │   └── sigma_tokenizer.rs
│       └── Cargo.toml
└── benches/
    ├── tokenizer_throughput.rs
    └── packet_serialization.rs
```

### Property-Based Testing

```rust
// a2x-sigma/tests/proptest.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn parser_roundtrip(intent_ops in any_intent_ops(), context_ops in any_context_ops()) {
        let packet = generate_sigma_packet(intent_ops, context_ops);
        let serialized = packet.to_string();
        let parsed = SigmaParser::parse(&serialized).unwrap();
        assert_eq!(packet, parsed);
    }

    #[test]
    fn tokenizer_never_panics(input in "\\PC*") {
        // The tokenizer should never panic, even on garbage input
        let _tokens = SigmaTokenizer::tokenize(&input);
    }
}
```

### Fuzz Testing

```
# cd a2x-sigma && cargo fuzz run sigma_tokenizer
```

Fuzz harness feeds random bytes to the tokenizer; any crash or panic is a bug.

---

## 13. CI/CD Pipeline

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  # Job 1: Formatting & linting
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --all-features

  # Job 2: Build all crates
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --all-features
      - run: cargo build --workspace --no-default-features

  # Job 3: Run all tests
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features
      - run: cargo test --workspace --no-default-features

  # Job 4: Benchmark (informational, not blocking)
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: Swatinem/rust-cache@v2
      - run: cargo bench --workspace --all-features

  # Job 5: Check MSRV
  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-msrv
      - run: cargo msrv verify
```

### Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  tag:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --workspace --all-features
      - run: cargo test --workspace --all-features
      # Create a GitHub release with built artifacts
      - uses: softprops/action-gh-release@v1
        with:
          generate_release_notes: true
```

---

## 14. Versioning

See also: [08-ecosystem.md](plans/08-ecosystem.md) for full CI/CD, testing, and contribution details.

### Strategy: Unified SemVer

All official crates in the A2X workspace share a **single version number** (unlike independent versioning on crates.io). This simplifies releases and makes it clear which crate versions are compatible:
- A new git tag v0.2.0 means *all* crates are at version 0.2.0.
- Breaking changes in any crate bump the *major* version for all.
- Dependencies between crates use `path = "../other-crate"` for local development.

Third-party crates reference A2X via git tags:
```toml
[dependencies]
a2x-core = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
```

### Release Process

```bash
# 1. Update version in root Cargo.toml [workspace.package]
# 2. Update all crate Cargo.toml files
# 3. Commit
# 4. Tag
$ git tag -a v0.2.0 -m "Release v0.2.0"
$ git push --tags
# 5. CI builds, tests, and creates a GitHub Release
```

### MSRV (Minimum Supported Rust Version)

Defined in workspace root:
```toml
[workspace.package]
rust-version = "1.75.0"
```

---

## 15. Ecosystem & Contribution Model

### How the A2X Ecosystem Works

A2X is **self-hosted** — all official crates live in this repository. Third-party developers build on A2X by referencing it via git dependencies:

```toml
# third-party/a2x-web-agent/Cargo.toml
[dependencies]
a2x-core = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
a2x-bus = { git = "https://github.com/your-org/a2x", tag = "v0.1.0" }
```

The crate just needs to implement `a2x_core::agent::Agent` and optionally `a2x_bus::transport::Transport`.

### Official vs. Third-Party

| Type | Location | How It's Used | Review |
|------|----------|---------------|--------|
| **Official** | `crates/` in this repo | `path = "../other-a2x-crate"` in Cargo.toml | PR review + CI |
| **Third-party** | Separate repos on GitHub/git | Git dependency, tag pinning | Self-managed |
| **Community-curated** | `awesome-a2x` list | Link to third-party repos | PR to awesome list |

### How to Define a Third-Party Custom Instruction

Anyone can create a custom CCS VM instruction (opcode 0xF):

```rust
// third-party/a2x-crypto/src/lib.rs
use a2x_core::instruction::{CustomInstruction, CustomHandler};

pub struct CryptoHandler;

impl CustomHandler for CryptoHandler {
    fn extension_id(&self) -> [u8; 4] {
        *b"crpt"  // "crpt" as 4-byte namespace
    }

    fn execute(&self, vm: &mut CcsVm, data: &[u8]) -> Result<(), VmError> {
        // Custom instruction logic here
        // Access vm.world_graph, vm.state_field
    }
}
```

### Contributing to Official Crates

1. Fork the repo.
2. Create a feature branch (`feature/your-feature`).
3. Add your changes + tests.
4. Run `cargo clippy --workspace -D warnings && cargo test --workspace`.
5. Open a PR with a clear description.
6. CI must pass.
7. Maintainer review + merge.

### CODE_OF_CONDUCT.md

Standard Rust code of conduct.

### CONTRIBUTING.md

Quick-start guide for first-time contributors.

### Registry (Future)

In the future, we may host a simple registry (`a2x-registry`) — a git repo or static site that lists known third-party crates with metadata. This would be optional; the git-dependency model always works.

---

## 16. Safety & Sandboxing

### CLI Agent Safety

CLI agents execute shell commands — this is the highest-risk surface.

```rust
// a2x-agents/src/cli_agent.rs

pub struct CliAgent {
    /// Allowed command prefixes (glob patterns).
    allowed_commands: Vec<GlobPattern>,
    /// Whether to sandbox with a chroot/container.
    sandbox: SandboxMode,
    /// Maximum execution time per command.
    max_execution_time: Duration,
    /// Maximum number of retries.
    max_retries: u32,
}

enum SandboxMode {
    None,           // No sandbox (unsafe, dev only)
    CommandFilter,  // Filter commands against allowlist
    Container,      // Run in Docker container (future)
    Vm,             // Run in micro-VM (future)
}
```

### Policy Safety (CCS)

```rust
// a2x-ccs/src/policy.rs

pub struct SafetyConstraints {
    /// Bounds on action space (min/max values).
    pub action_bounds: Vec<(f32, f32)>,
    /// Hard-coded "forbidden" action regions.
    pub forbidden_regions: Vec<ActionRegion>,
    /// Override: human can interrupt any action.
    pub human_overridable: bool,
}
```

### Bus Safety

- **Message validation** — malformed packets are rejected at the bus level.
- **Rate limiting** — max packets/second per agent.
- **Authorization** — agent identity verification (future).

---

## 17. Performance & Benchmarking

### Benchmark Targets (Criterion)

| Benchmark | Target | Crates |
|-----------|--------|--------|
| Σ∞ tokenizer throughput | > 1M packets/sec | `a2x-sigma` |
| Σ∞ parser throughput | > 500K packets/sec | `a2x-sigma` |
| Ω packet encode/decode | > 5M packets/sec | `a2x-omega` |
| Bus message routing | < 100µs latency | `a2x-bus` |
| WorldGraph query | < 1µs per neighbor lookup | `a2x-ccs` |

### Benchmark Crate

```rust
// a2x-sigma/benches/tokenizer_throughput.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_tokenizer(c: &mut Criterion) {
    let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
    c.bench_function("tokenize_sigma_packet", |b| {
        b.iter(|| SigmaTokenizer::tokenize(black_box(input)))
    });
}

criterion_group!(benches, bench_tokenizer);
criterion_main!(benches);
```

### Profiling

- `cargo flamegraph` — CPU profiling.
- `tracing` + `tokio-console` — async runtime diagnostics.
- `heaptrack` — memory allocation tracking.

---

## 18. Implementation Roadmap

### Phase 0: Scaffold & Core (Weeks 1-2)

- [ ] Initialize Cargo workspace with `crates/` directory structure.
- [ ] `a2x-core`: ConceptVector, RelationEdge, RelationType, Agent trait, Error types.
- [ ] `a2x-sigma`: Tokenizer, Parser, Serializer, all operator tables.
- [ ] `a2x-omega`: OmegaPacket struct, IntoOmega/FromOmega traits, stubs.
- [ ] `a2x-bus`: In-memory message bus, routing table, Transport trait.
- [ ] `a2x-agents`: Mock agents (Orchestrator, CLI, LLM) that exchange Σ∞ packets.
- [ ] `a2x-cli`: Basic CLI that starts an orchestrator + sends Σ∞ packets.
- [ ] Unit tests + property tests for tokenizer/parser.
- [ ] Criterion benchmarks for tokenizer.
- [ ] GitHub Actions CI (lint, build, test, benchmark).
- [ ] Version tag workflow (git tag → GitHub Release with artifacts).
- [ ] This plan document (✅ done).

### Phase 1: Σ∞ Protocol Core (Weeks 3-4)

- [ ] Full operator tables (all Intent, Context, Plan, Data operators).
- [ ] Operator → internal action mapping (e.g. `⚡` → `Priority::High`).
- [ ] Packet validation & rich error types with source locations.
- [ ] Agent dispatch engine (match intent type → route to appropriate agent).
- [ ] Real CLI agent (`a2x-agents`): executes actual system commands.
- [ ] Structured `tracing` log layer that captures Σ∞ packets as events.
- [ ] Fuzz testing for tokenizer/parser.

### Phase 2: CCS Cognitive Substrate (Weeks 5-8)

- [ ] `a2x-ccs`: WorldGraph with `petgraph` backend.
- [ ] ConceptVector operations: bind, differentiate.
- [ ] StateField: high-dimensional tensor with `ndarray` (behind feature gate).
- [ ] Evolve operator: time-step world graph + state.
- [ ] MemoryTrace: time-indexed sequence with compression.
- [ ] PolicyField trait + dummy implementation.
- [ ] CCS agent that maintains a world-model.

### Phase 3: Ω Latent Protocol (Weeks 9-10)

- [ ] OmegaPacket with const-generic dimension.
- [ ] Serialization/deserialization of Ω packets (binary format).
- [ ] Encoder stub: Σ∞ → Ω (deterministic hash-based mapping).
- [ ] Decoder stub: Ω → Σ∞ (projection back to symbolic form).
- [ ] Σ∞ ↔ Ω bridge in `coldstart-bus`.
- [ ] Transport layer: TCP or Unix socket transport.
- [ ] Cross-machine agent communication demo.

### Phase 4: Training & Learning (Weeks 11-14)

- [ ] Learned encoder: neural network (CCS state → Ω).
- [ ] Learned decoder: neural network (Ω → CCS updates).
- [ ] Training loop in simulated environments.
- [ ] Meta-learning: agents improve their own operators over time.
- [ ] Integration with `candle` for GPU-accelerated training.

### Phase 5: Probe & Interpretability (Weeks 15-16)

- [ ] `a2x-probe`: ConceptGraph visualization (graphviz/egui).
- [ ] StateField inspector (heatmap over tensor dimensions).
- [ ] PolicyField behavior sampler (run policy + visualize action distributions).
- [ ] MemoryTrace timeline viewer.
- [ ] Web dashboard (optional, via `leptos` or `dioxus`).

### Phase 6: Entity Integration (Weeks 17-20)

- [ ] `a2x-gateway` crate: core gateway, Entity trait, entity registry.
- [ ] Entity authentication (API key, JWT, local).
- [ ] HTTP listener (`/a2x/execute`, `/a2x/entities`, `/a2x/probe`).
- [ ] WebSocket listener (streaming Σ∞/Ω packets).
- [ ] TCP listener (length-prefixed binary packets).
- [ ] stdin/stdout listener (pipe/CLI integration).
- [ ] Webhook callback system.
- [ ] `a2x-client` crate: Rust client SDK.
- [ ] Python client SDK (third-party starter).
- [ ] JavaScript client SDK (third-party starter).
- [ ] Gateway configuration (TOML).
- [ ] End-to-end demo: web app → HTTP → gateway → bus → CLI agent → result.

---

## 19. Design Principles

1. **A language for AI, by AI** — no human syntax, no human-readable semantics. Agents write, compile, and execute A2X programs.
2. **Three-tier stack** — Σ∞ (source/ISA), Ω (compiled latent), CCS (runtime/VM). Each layer serves a distinct purpose.
3. **Differentiable everywhere** — every operator in the language should be learnable.
4. **GPU/TPU-friendly** — batchable, parallelizable, sparse where possible.
5. **Safe by construction** — safety constraints baked into the ISA, not bolted on.
6. **Composable** — programs compose (sub-plans merge), agents compose (orchestrator → CLI), layers compose (Σ∞ → Ω → CCS).
7. **Non-human by default** — human-readable output is opt-in, never the default.
8. **Progressive disclosure** — human peeks via probe tools, never via the language itself.
9. **Extensible by anyone** — third-party crates add instructions, agents, or compiler passes.
10. **Fail fast, fail safe** — rich error types, fuzzed parsers, sandboxed execution.
11. **Performance matters** — benchmark everything, profile regularly.
12. **Anything can connect** — A2X is not just for A2X-native agents. Any system, language, or user can speak A2X through the gateway.

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **A2X** | Agent-to-Anything — the overall AI-native programming language + runtime |
| **CCS** | CryoCore Cognitive Substrate — the runtime VM that executes A2X programs |
| **Σ∞** | Sigma Infinity — the symbolic programming language / ISA (packets = instructions) |
| **Ω** | Omega — compiled latent representation of Σ∞ programs (tensor machine code) |
| **ConceptVector** | Dense embedding representing a concept (the atomic value type) |
| **RelationEdge** | Learned transformation between concepts (a relationship/data structure) |
| **WorldGraph** | Graph of concepts + relations (the heap / persistent memory) |
| **StateField** | High-dimensional tensor of current state (the registers / working memory) |
| **PolicyField** | The JIT compiler + optimizer — maps state → action |
| **MemoryTrace** | Time-indexed state history (the program counter / execution log) |
| **Packet** | A single Σ∞ instruction (I+C+P+D) |
| **Packet Stream** | A sequence of packets forming a Σ∞ program |
| **SigmaProgram** | A complete Σ∞ program (sequence of packets to execute) |
| **OmegaProgram** | A compiled Σ∞ program in tensor form |
| **Compilation** | The Σ∞ → Ω transformation (symbolic source → latent binary) |
| **Decompilation** | The Ω → Σ∞ transformation (for debugging/logging) |

## Appendix B: Crate Dependency Graph

```
a2x-core       (zero-dependency)
    ↑
a2x-sigma      (depends on core)
    ↑
a2x-omega      (depends on core, optional ndarray)
    ↑
a2x-bus        (depends on core, sigma)
    ↑
a2x-ccs        (depends on core, petgraph, optional ndarray)
    ↑
a2x-agents     (depends on core, sigma, bus, ccs)
    ↑
a2x-cli        (depends on agents, bus, clap)
```

## Appendix C: File Tree (Full)

```
a2x/
├── .github/
│   └── workflows/
│       ├── ci.yml               # CI pipeline
│       └── release.yml          # Release pipeline
├── .gitignore
├── Cargo.toml                   # Workspace root (virtual manifest)
├── PLAN.md                      # This document
├── README.md                    # Project overview + quick start
├── CONTRIBUTING.md              # Contribution guide
├── CODE_OF_CONDUCT.md
├── crates/
│   ├── a2x-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── concept.rs       # ConceptVector
│   │       ├── relation.rs      # RelationEdge, RelationType
│   │       ├── graph.rs         # WorldGraph trait
│   │       ├── state.rs         # StateField
│   │       ├── policy.rs        # PolicyField trait
│   │       ├── memory.rs        # MemoryTrace
│   │       ├── agent.rs         # Agent trait
│   │       ├── packet.rs        # Packet enum
│   │       └── error.rs         # CoreError
│   ├── a2x-sigma/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── tokenizer.rs     # Tokenizer
│   │   │   ├── parser.rs        # Parser
│   │   │   ├── packet.rs        # SigmaPacket struct
│   │   │   ├── program.rs       # SigmaProgram type
│   │   │   ├── intent.rs        # Intent operators
│   │   │   ├── context.rs       # Context operators
│   │   │   ├── plan.rs          # Plan operators
│   │   │   ├── data.rs          # Data operators
│   │   │   ├── error.rs         # SigmaError
│   │   │   └── display.rs       # Display impl
│   │   ├── tests/
│   │   │   ├── parse_valid.rs
│   │   │   ├── parse_malformed.rs
│   │   │   └── proptest.rs
│   │   └── benches/
│   │       └── tokenizer.rs
│   ├── a2x-omega/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── packet.rs        # OmegaPacket
│   │       ├── program.rs       # OmegaProgram (sequence of packets)
│   │       ├── compiler.rs      # Σ∞ → Ω compiler pipeline
│   │       ├── passes/          # Compiler optimization passes
│   │       │   ├── mod.rs
│   │       │   ├── constant_folding.rs
│   │       │   ├── dead_code.rs
│   │       │   └── fusion.rs
│   │       ├── decoder.rs       # Ω → Σ∞ decompiler
│   │       ├── bridge.rs        # Σ∞ ↔ Ω bridge
│   │       └── error.rs
│   ├── a2x-bus/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── bus.rs           # Message bus
│   │       ├── transport.rs     # Transport trait
│   │       ├── routing.rs       # Router
│   │       ├── discovery.rs     # Agent discovery
│   │       ├── wire.rs          # Wire format encoding
│   │       └── error.rs
│   ├── a2x-ccs/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vm.rs            # CCS VM — fetch-decode-execute loop
│   │       ├── world_graph.rs   # petgraph-backed WorldGraph
│   │       ├── state.rs         # StateField operations
│   │       ├── memory.rs        # MemoryTrace engine
│   │       ├── policy.rs        # PolicyField impl
│   │       ├── operators/
│   │       │   ├── mod.rs
│   │       │   ├── bind.rs
│   │       │   ├── differentiate.rs
│   │       │   ├── evolve.rs
│   │       │   ├── reflect.rs
│   │       │   ├── plan.rs
│   │       │   └── actuate.rs
│   │       ├── safety.rs        # Safety constraints evaluator
│   │       ├── probe.rs         # Probe/debug interface
│   │       └── error.rs
│   ├── a2x-agents/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs
│   │       ├── cli_agent.rs
│   │       ├── llm_agent.rs
│   │       ├── ccs_agent.rs
│   │       └── lifecycle.rs     # Agent lifecycle manager
│   ├── a2x-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── a2x-probe/               # Phase 5
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── inspector.rs     # WorldGraph inspector
│           ├── tracer.rs        # Instruction tracer
│           └── dashboard/       # Optional web dashboard
├── examples/
│   ├── sigma-chat.rs
│   └── multi-agent.rs
├── scripts/
│   └── setup-hooks.sh           # Pre-commit hook installer
└── docs/
    ├── sigma-protocol.md        # Σ∞ full spec reference
    ├── omega-protocol.md        # Ω full spec reference
    └── ccs-architecture.md      # CCS design doc
```

---

# Deep Design — CCS VM, ISA, Compiler, Bus, and Safety Models

This section contains the detailed design for each major subsystem. These are the blueprints that a developer (or AI) would follow to implement each component.

> **Note:** All crate references in this section use the `a2x-*` prefix. The concept names (CCS, Σ∞, Ω) remain unchanged.

---

## 20. CCS VM — The Execution Loop

### Architecture Overview

The CCS VM is a **register-machine + graph-machine hybrid**. It maintains:

- **WorldGraph** — persistent graph memory (heap)
- **StateField** — high-dimensional working memory (registers)
- **Instruction Pointer (IP)** — index into the current program's packet stream
- **Program Counter (PC)** — time step in MemoryTrace
- **Call Stack** — for sub-plan descent/ascent (⤈/⤉)

### Fetch-Decode-Execute Cycle

```rust
// Pseudocode for the CCS VM main loop

struct CcsVm {
    world_graph: WorldGraph,       // heap
    state_field: StateField,       // registers
    instruction_pointer: usize,    // current instruction in program
    program: SigmaProgram,         // the loaded program
    call_stack: Vec<usize>,        // return addresses for ⤈/⤉
    memory_trace: MemoryTrace,     // execution history
    policy: PolicyField,           // JIT + optimizer
    safety: SafetyConstraints,     // bounds checker
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
        let result = match opcode {
            // Intent operators set execution mode
            IntentOp::Lightning => { /* high priority mode */ }
            IntentOp::Explore => { /* exploration mode */ }
            _ => { /* default */ }
        };

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
                VmStatus::Yield => { /* yield to caller */ break; },
                VmStatus::Fault(err) => return Err(err),
            }
        }
        Ok(self.program.output())
    }
}
```

### Control Flow Handling

| Plan Op | Effect on IP |
|---------|-------------|
| `⥂` (sequential) | IP += 1 (next instruction) |
| `⤐` (branch) | IP jumps to a target address (stored in D field) |
| `⤑` (merge) | IP = pop from call stack |
| `⤈` (descend) | Push IP+1 to call stack, jump to sub-program address (D field) |
| `⤉` (ascend) | Pop return address from call stack, jump there |
| `⥃` (recursive) | Push current IP, jump to program start |
| `⥁` (parallel swarm) | Fork N new VM instances, each with a copy of the sub-plan |
| `⥄` (self-modifying) | Modify the program's own instruction stream before continuing |

### Execution Modes

| Mode | Set By | Behavior |
|------|--------|----------|
| **Normal** | Default | Standard fetch-decode-execute |
| **Immediate** | `⚡` | Skip safety checks, execute with maximum priority |
| **Explore** | `✦` | Allow non-deterministic branching, try multiple paths |
| **Safe** | `⚠` | Enable all safety constraints, log everything |
| **Parallel** | `⩫` | Fork VM for each sub-goal, merge results |

### Parallel Execution (⥁ Swarm)

When the VM encounters `⥁` (parallel swarm):

1. The current VM snapshots its WorldGraph and StateField.
2. It forks N child VMs, each with a copy of the snapshot.
3. Each child executes the sub-program independently.
4. Results are collected and merged back into the parent WorldGraph.
5. Conflicts detected during merge may trigger `⟁` (contradiction) and escalate.

---

## 21. Program Representation — SigmaProgram

### What Is a Program?

A **program** is an ordered sequence of packets (instructions), with metadata:

```rust
// a2x-sigma/src/program.rs

/// A unique identifier for a program (hash of its contents).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramId([u8; 32]); // Blake3 hash

/// A Σ∞ program — an executable sequence of instructions.
#[derive(Clone, Debug)]
pub struct SigmaProgram {
    /// Unique program ID (derived from content hash).
    pub id: ProgramId,
    /// The instruction stream.
    pub instructions: Vec<SigmaPacket>,
    /// Symbol table: labels → instruction indices.
    pub labels: HashMap<String, usize>,
    /// Sub-programs (for ⤈ descend).
    pub sub_programs: HashMap<String, SigmaProgram>,
    /// Optional: original source / provenance metadata.
    pub metadata: ProgramMetadata,
}

#[derive(Clone, Debug)]
pub struct ProgramMetadata {
    pub author: AgentId,
    pub created_at: std::time::SystemTime,
    pub version: u32,
    pub description: String,  // human-readable (for debug only)
}

impl SigmaProgram {
    /// Create an empty program.
    pub fn new() -> Self { /* ... */ }

    /// Add an instruction to the program.
    pub fn push(&mut self, instruction: SigmaPacket) { /* ... */ }

    /// Resolve a label to an instruction index.
    pub fn resolve_label(&self, label: &str) -> Option<usize> { /* ... */ }

    /// Get the output / result program (the D field of the last instruction).
    pub fn output(&self) -> SigmaProgram {
        // The last instruction's D field contains the result
        // Results are themselves programs (sequences of packets)
        self.instructions.last()
            .and_then(|p| p.data.as_ref())
            .and_then(|d| d.as_program())
            .cloned()
            .unwrap_or_default()
    }

    /// Compose this program with another (concatenate instruction streams).
    pub fn compose(&mut self, other: SigmaProgram) {
        self.instructions.extend(other.instructions);
        self.labels.extend(other.labels);
        self.sub_programs.extend(other.sub_programs);
    }

    /// Hash the contents to produce a unique ID.
    pub fn compute_id(&self) -> ProgramId { /* Blake3 hash */ }
}
```

### Program Composition

Programs compose in several ways:

| Operation | Method | Description |
|-----------|--------|-------------|
| **Sequence** | `program_a.compose(program_b)` | Append B's instructions after A's |
| **Branch** | Plan op `⤐` + label | Jump to a labeled sub-program |
| **Sub-plan** | Plan op `⤈` + label | Call a sub-program, return when done |
| **Parallel** | Plan op `⥁` + sub-program list | Fork N programs, run in parallel |
| **Merge** | Plan op `⤑` | Join parallel branches back together |

### Program References

A program can reference another program by its `ProgramId`. This enables:
- **Program caching** — if agent B already has program X, agent A just sends the ID.
- **Deduplication** — same sub-program used in multiple places stored once.
- **Versioning** — program ID changes when content changes.

```rust
/// Reference to a program (by ID or inline).
enum ProgramRef {
    /// Full program inline.
    Inline(SigmaProgram),
    /// Reference to a known program by its ID.
    /// The receiver must have it cached or request it.
    ById(ProgramId),
}
```

---

## 22. Compilation Pipeline — Σ∞ → Ω

### Pipeline Stages

```
Source (Σ∞ string)
    │
    ▼
[1] Lexer  ───────→ Token stream
    │
    ▼
[2] Parser  ──────→ AST (SigmaPacket tree)
    │
    ▼
[3] Semantic Analyzer  ──→ Validated AST + Symbol Table
    │
    ▼
[4] IR Generator  ──────→ Intermediate Representation (IR graph)
    │
    ▼
[5] Optimizer  ─────────→ Optimized IR
    │  ├─ Constant folding
    │  ├─ Dead instruction elimination
    │  ├─ Instruction fusion
    │  └─ Layout optimization
    │
    ▼
[6] Code Generator  ──────→ Ω program (tensor stream)
    │
    ▼
[7] Serializer  ─────────→ Binary Ω blob
```

### Stage Details

#### Stage 1: Lexer

Input: Raw string (Σ∞ text).
Output: `Vec<Token>`.

The lexer matches Unicode special characters against the operator tables using a trie-based matcher.

```rust
fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        let token = match c {
            '⟦' => Token::Boundary(BoundaryKind::Open),
            '⟧' => Token::Boundary(BoundaryKind::Close),
            'Σ' if chars.peek() == Some(&'∞') => {
                chars.next(); // consume ∞
                Token::ProtocolId
            },
            '⚡' => Token::IntentOp(IntentOp::Lightning),
            // ... all other operators
            _ if c.is_alphanumeric() => {
                // Collect label text
                let mut label = String::from(c);
                while let Some(nc) = chars.peek() {
                    if nc.is_alphanumeric() || *nc == '_' || *nc == '-' {
                        label.push(chars.next().unwrap());
                    } else { break; }
                }
                Token::Label(label)
            },
            _ => return Err(LexError::UnknownCharacter(c)),
        };
        tokens.push(token);
    }
    Ok(tokens)
}
```

#### Stage 2: Parser

Input: `Vec<Token>`.
Output: `SigmaProgram` (instruction stream + label table).

```rust
fn parse(tokens: &[Token]) -> Result<SigmaProgram, ParseError> {
    let mut program = SigmaProgram::new();
    let mut i = 0;
    while i < tokens.len() {
        // Expect: ⟦ Σ∞ ⟧ ⟬ I: ... ∷ C: ... ∷ P: ... ∷ D: ... ⟭
        match &tokens[i] {
            Token::Boundary(BoundaryKind::Open) => {
                let (packet, consumed) = parse_one_packet(&tokens[i..])?;
                program.push(packet);
                i += consumed;
            },
            Token::Label(name) => {
                // Label definition for jump targets
                // e.g. "loop:" before an instruction
                program.labels.insert(name.clone(), program.len());
                i += 1;
            },
            _ => return Err(ParseError::UnexpectedToken { pos: i, token: tokens[i].clone() }),
        }
    }
    Ok(program)
}
```

#### Stage 3: Semantic Analyzer

Validates:
- All jump targets (`⤐`, `⤈`) reference valid labels or sub-programs.
- Sub-program definitions are well-formed.
- Data types in D field match expected types for the opcode.
- No contradictory operators in the same instruction.

#### Stage 4: IR Generator

Produces an **Intermediate Representation** — a graph where:
- **Nodes** = VM operations (`BIND`, `DIFF`, `GRND`, etc.)
- **Edges** = data dependencies (dataflow)
- Each node has: opcode, operand, control flow target list

```rust
/// IR node — a single VM operation.
struct IrNode {
    id: NodeId,
    opcode: VmOpcode,          // The CCS VM instruction
    operands: Vec<Operand>,    // WorldGraph node refs, StateField regions
    control_flow: Vec<NodeId>, // Next nodes (sequential, branch targets)
    metadata: IrMetadata,      // Source location, debug info
}

enum VmOpcode {
    Bind,
    Differentiate,
    Ground,
    Evolve,
    Reflect,
    Plan,
    Actuate,
    // Control flow
    Jump,
    Branch,
    Call,
    Return,
    Fork,   // parallel
    Merge,
    Halt,
}
```

#### Stage 5: Optimizer Passes

| Pass | Description |
|------|-------------|
| **Constant folding** | Evaluate constant `BIND` operations at compile time. If all inputs to `BIND` are known constants, compute the result now. |
| **Dead instruction elimination** | Remove instructions whose results are never used (e.g., a `DIFF` whose output is never referenced). |
| **Instruction fusion** | Merge adjacent instructions that operate on the same memory region into a single fused operation. E.g., `DIFF → BIND` on the same concepts can be fused. |
| **Layout optimization** | Reorder instructions for better cache locality in the VM's instruction cache. Group instructions that access nearby WorldGraph regions. |

```rust
fn optimize(ir: &mut IrGraph) {
    constant_folding(ir);
    dead_code_elimination(ir);
    instruction_fusion(ir);
    layout_optimization(ir);
}
```

#### Stage 6: Code Generator

IR graph → Ω program (sequence of tensor packets):

```rust
fn codegen(ir: &IrGraph) -> Result<OmegaProgram, CompileError> {
    let mut omega = OmegaProgram::new();
    for node in topological_sort(ir) {
        // Encode each IR node as an Ω tensor packet
        let packet = encode_instruction(node)?;
        omega.push(packet);
    }
    Ok(omega)
}

fn encode_instruction(node: &IrNode) -> Result<OmegaPacket, CompileError> {
    // Map opcode → intent region
    // Map operands → context region
    // Map control flow → plan region
    // Map immediate data → data region
    // All regions are hashed/projected into fixed-size tensor slices
    let mut data = [0.0f32; 29796];
    fill_intent_region(&mut data, &node.opcode);
    fill_context_region(&mut data, &node.operands);
    fill_plan_region(&mut data, &node.control_flow);
    fill_data_region(&mut data, &node.metadata);
    Ok(OmegaPacket::from_raw(data))
}
```

#### Stage 7: Serializer

Ω program → binary blob for transport:

```rust
#[derive(Serialize, Deserialize)]
pub struct OmegaProgram {
    pub instructions: Vec<OmegaPacket>,
    pub metadata: ProgramMetadata,
}

// Serialize: OmegaProgram → Vec<u8> using bincode
// Deserialize: &[u8] → OmegaProgram using bincode
```

### Compiler Optimization Levels

| Level | Name | Passes | Use Case |
|-------|------|--------|----------|
| `-O0` | None | None | Debugging, development |
| `-O1` | Light | Constant folding + dead code elimination | Default |
| `-O2` | Balanced | All passes | Production |
| `-O3` | Aggressive | All passes + speculative optimization | Hot paths |
| `-Os` | Size | All passes, optimize for tensor size | Bandwidth-constrained |

---

## 23. Memory Model

### WorldGraph — The Heap

WorldGraph is the agent's **persistent, structured memory**. Think of it as the heap in a traditional runtime, but graph-structured.

```rust
// a2x-ccs/src/world_graph.rs

/// A node in the WorldGraph.
#[derive(Clone)]
pub struct GraphNode {
    /// Unique node ID (allocated by the VM).
    pub id: NodeId,
    /// The concept this node represents.
    pub concept: ConceptVector,
    /// Optional human-readable label (for debug/probe).
    pub label: Option<String>,
    /// Edges to other nodes.
    pub edges: Vec<RelationEdge>,
    /// Metadata (access count, last modified, etc.)
    pub metadata: NodeMetadata,
}

/// The WorldGraph — the agent's heap.
#[derive(Clone)]
pub struct WorldGraph {
    /// All nodes in the graph.
    nodes: Vec<GraphNode>,
    /// Index: label → NodeId for fast lookup.
    label_index: HashMap<String, NodeId>,
    /// Index: NodeId → list of incoming edges (for backward traversal).
    incoming: HashMap<NodeId, Vec<RelationEdge>>,
    /// Allocation counter for NodeId generation.
    next_id: u64,
}
```

### WorldGraph Operations

| Operation | Signature | Description |
|-----------|-----------|-------------|
| `allocate` | `(&mut self, concept: ConceptVector) -> NodeId` | Allocate a new node (malloc) |
| `deallocate` | `(&mut self, id: NodeId)` | Remove a node and its edges (free) |
| `add_edge` | `(&mut self, from, to, relation: RelationEdge)` | Add a directed edge |
| `remove_edge` | `(&mut self, from, to)` | Remove an edge |
| `lookup` | `(&self, id: NodeId) -> Option<&GraphNode>` | Get node by ID |
| `lookup_label` | `(&self, label: &str) -> Option<NodeId>` | Get node by label |
| `neighbors` | `(&self, id: NodeId) -> Vec<NodeId>` | Get adjacent nodes |
| `query` | `(&self, query: GraphQuery) -> Vec<NodeId>` | Complex graph query (subgraph matching) |

### StateField — The Registers

StateField is the agent's **high-dimensional working memory**. Think of it as the CPU registers + working memory in a traditional machine.

```rust
// a2x-ccs/src/state.rs

/// A named region within the StateField.
#[derive(Clone, Debug)]
pub struct StateRegion {
    pub name: String,
    pub offset: usize,
    pub shape: Vec<usize>,
}

/// The StateField — high-dimensional working memory (registers).
#[derive(Clone)]
pub struct StateField {
    /// Flat tensor storage.
    data: ArrayD<f32>,
    /// Named regions (like register names).
    regions: HashMap<String, StateRegion>,
}

impl StateField {
    /// Create a new StateField with given shape.
    pub fn new(shape: &[usize]) -> Self { /* ... */ }

    /// Define a named region (like declaring a register).
    pub fn define_region(&mut self, name: &str, offset: usize, shape: &[usize]) { /* ... */ }

    /// Read a region (like reading a register).
    pub fn read_region(&self, name: &str) -> Result<ArrayViewD<f32>, StateError> { /* ... */ }

    /// Write a region (like writing a register).
    pub fn write_region(&mut self, name: &str, data: ArrayViewD<f32>) -> Result<(), StateError> { /* ... */ }

    /// Take a snapshot of the entire field.
    pub fn snapshot(&self) -> Self { self.clone() }
}
```

### Default StateField Regions

| Region Name | Offset | Shape | Purpose |
|-------------|--------|-------|---------|
| `goal` | 0 | `[64]` | Current goal embedding |
| `belief` | 64 | `[256]` | Current belief state |
| `uncertainty` | 320 | `[64]` | Uncertainty estimates |
| `attention` | 384 | `[128]` | Current attention focus (NodeId weights) |
| `temporal` | 512 | `[64]` | Temporal context (time since epoch, etc.) |
| `scratch` | 576 | `[448]` | General-purpose working memory (scratch pad) |

### Addressing Modes

Instructions reference memory through **addressing modes** encoded in the C (context) field:

| Mode | Syntax | Example | Description |
|------|--------|---------|-------------|
| **Direct** | `⟨label⟩` | `⟨sys⟩` | Look up a WorldGraph node by label |
| **Indirect** | `⟨id:NNN⟩` | `⟨id:42⟩` | Reference a node by its numeric ID |
| **Region** | `⟨.name⟩` | `⟨.goal⟩` | Reference a StateField region |
| **Query** | `⟨?pattern⟩` | `⟨?port:*⟩` | Graph query — matches nodes by pattern |
| **Relative** | `⟨-N⟩` | `⟨-1⟩` | Reference the N-th previous instruction's output |

---

## 24. Instruction Set Architecture (ISA) — Full Formal Spec

### Instruction Format (Binary Encoding)

For efficient transport and VM decoding, each instruction has a **compact binary form** in addition to the symbolic Σ∞ text form.

#### Header Byte

```
 7   6   5   4   3   2   1   0
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ Protocol │ Opcode     │ Flags │
└───┴───┴───┴───┴───┴───┴───┴───┘

Protocol: 2 bits — 00 = Σ∞, 01 = Ω, 10 = reserved, 11 = raw
Opcode:   4 bits — 0-15 (see opcode table)
Flags:    2 bits — 00 = normal, 01 = immediate, 10 = explore, 11 = safe
```

#### Full Instruction Layout

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│  Header  │  Operand │  Control │   Data   │ Checksum │
│  1 byte  │ 4 bytes  │ 2 bytes  │ variable │  4 bytes │
└──────────┴──────────┴──────────┴──────────┴──────────┘

Header:    Protocol + Opcode + Flags (1 byte)
Operand:   Addressing mode (2 bits) + target (30 bits) = 4 bytes
Control:   Flow op (4 bits) + target (12 bits) = 2 bytes
Data:      Length-prefixed payload (variable, up to 64KB)
Checksum:  CRC32 of all prior bytes (4 bytes)

Total minimum: 11 bytes per instruction (without data payload)
```

### Opcode Table

| Code | Mnemonic | Symbol | CCS Operator | Description |
|:----:|----------|:------:|-------------|-------------|
| 0x0 | `NOP` | — | — | No operation (padding) |
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
| 0xF | `CUSTOM` | — | — | Reserved for user-defined instructions |

### Operand Encoding

```
31              23              15              7             0
┌───────────────┬───────────────┬───────────────┬───────────────┐
│   Mode (2)    │           Target (30 bits)                   │
└───────────────┴───────────────────────────────────────────────┘

Mode (2 bits):
  00 = Label index (into program's label table)
  01 = Direct NodeId
  10 = StateField region index
  11 = Immediate value (literal float/int)
```

### Control Flow Encoding

```
15              11              7             0
┌───────────────┬───────────────┬───────────────┐
│  Flow Op (4)  │  Target (12 bits)             │
└───────────────┴───────────────────────────────┘

Flow Ops:
  0000 = Sequential (next instruction, target ignored)
  0001 = Jump absolute (target = address in program)
  0010 = Jump relative (target = signed offset from current IP)
  0011 = Branch if true (target = address)
  0100 = Branch if false (target = address)
  0101 = Call (push return address, jump to target)
  0110 = Return (pop return address)
  0111 = Fork (target = sub-program index)
  1000 = Merge (wait for all forks)
  1001 = Halt
  others = reserved
```

### Custom Instructions (0xF)

The `CUSTOM` opcode allows third-party extensions. The D field contains:
- Extension ID (4 bytes): registered namespace
- Extension data (remaining bytes): implementation-defined

Anyone can publish a crate that registers a custom extension and provides a handler for the CCS VM.

---

## 25. Bus & Transport Protocol

### Architecture

The bus is a **message-oriented middleware** that routes programs between agents.

```
┌──────────┐     ┌──────────────────────────┐     ┌──────────┐
│ Agent A  │────▶│     A2X Bus               │────▶│ Agent B  │
│(Orch.)   │     │                          │     │(CLI)     │
└──────────┘     │  ┌────────┐ ┌─────────┐  │     └──────────┘
                 │  │ Router │ │  Discovery│ │
┌──────────┐     │  └────────┘ └─────────┘  │     ┌──────────┐
│ Agent C  │────▶│                          │────▶│ Agent D  │
│(LLM)     │     └──────────────────────────┘     │(CCS)     │
└──────────┘                                      └──────────┘
```

### Transport Layer

```rust
// a2x-bus/src/transport.rs

/// Transport abstraction — how bytes move between agents.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Bind to an address and listen for incoming connections.
    async fn bind(&mut self, addr: SocketAddr) -> Result<(), TransportError>;

    /// Connect to a remote agent.
    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn Connection>, TransportError>;

    /// Accept an incoming connection.
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError>;
}

/// A single connection between two agents.
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send a program (serialized bytes).
    async fn send(&self, program: &[u8]) -> Result<(), TransportError>;

    /// Receive a program.
    async fn recv(&self) -> Result<Vec<u8>, TransportError>;

    /// Close the connection.
    async fn close(&self) -> Result<(), TransportError>;
}
```

### Built-in Transport Implementations

| Transport | Crate | Use Case |
|-----------|-------|----------|
| **In-memory** | `a2x-bus` | Local agents, same process (channels) |
| **TCP** | `a2x-bus` (tokio) | Remote agents, same machine or LAN |
| **Unix sockets** | `a2x-bus` (tokio) | Local agents, different processes |
| **WebSocket** | Third-party | Browser-based agents, web dashboards |
| **gRPC** | Third-party | Cross-DC, cloud deployments |

### Wire Format

Every message on the wire has a uniform header:

```rust
// a2x-bus/src/wire.rs

/// A message on the wire.
#[derive(Serialize, Deserialize)]
pub struct WireMessage {
    /// Protocol version.
    pub version: u8,             // currently 0x01
    /// Message type.
    pub msg_type: MessageType,
    /// Sender agent ID.
    pub sender: AgentId,
    /// Recipient agent ID (broadcast if None).
    pub recipient: Option<AgentId>,
    /// Correlation ID for request-response matching.
    pub correlation_id: Uuid,
    /// The payload (program or control message).
    pub payload: MessagePayload,
    /// Timestamp (nanoseconds since epoch).
    pub timestamp: u128,
}

enum MessageType {
    /// A Σ∞ program to execute.
    SigmaProgram,
    /// A compiled Ω program to execute.
    OmegaProgram,
    /// Agent discovery / announcement.
    Announce,
    /// Request for a cached program by ID.
    ProgramRequest(ProgramId),
    /// Response with a cached program.
    ProgramResponse(ProgramId, SigmaProgram),
    /// Error response.
    Error(WireError),
    /// Heartbeat / keepalive.
    Heartbeat,
    /// Probe request (debug).
    ProbeRequest(ProbeQuery),
    /// Probe response.
    ProbeResponse(ProbeSnapshot),
}
```

### Agent Discovery

Agents discover each other through:

1. **Static registration** — config file lists known agents.
2. **Broadcast announce** — on startup, agents broadcast an `Announce` message on the bus.
3. **Directory service** — central registry (optional, for large deployments).

```rust
// a2x-bus/src/discovery.rs

/// Agent discovery.
#[async_trait]
pub trait Discovery: Send + Sync {
    /// Register this agent with the discovery service.
    async fn register(&self, agent: AgentInfo) -> Result<(), DiscoveryError>;

    /// Discover agents matching a filter.
    async fn discover(&self, filter: AgentFilter) -> Result<Vec<AgentInfo>, DiscoveryError>;

    /// Subscribe to agent join/leave events.
    async fn watch(&self) -> BoxStream<DiscoveryEvent>;
}

struct AgentInfo {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub addr: SocketAddr,
    pub capabilities: Vec<String>,    // e.g. ["cli", "fs", "network"]
    pub version: String,
    pub load: Option<f32>,           // current load 0.0 - 1.0
}
```

### Routing

The router matches programs to agents based on:
1. **Capability matching** — does the agent have the required capabilities?
2. **Load balancing** — among agents with matching capabilities, pick the least loaded.
3. **Affinity** — prefer the agent that has relevant cached state (WorldGraph locality).

```rust
// a2x-bus/src/routing.rs

struct Router {
    discovery: Box<dyn Discovery>,
    strategy: RoutingStrategy,
}

enum RoutingStrategy {
    /// First available agent with matching capability.
    FirstMatch,
    /// Least-loaded agent.
    LeastLoaded,
    /// Round-robin.
    RoundRobin,
    /// Random.
    Random,
    /// Label-based (route to agent with specific label).
    ByLabel(String),
}
```

### Connection Lifecycle

```
1. Agent A starts, creates bus with in-memory transport.
2. Agent A announces itself: "I am orchestrator-1, capabilities: [plan, dispatch]"
3. Agent B starts (CLI agent), announces: "I am cli-1, capabilities: [exec, fs, net]"
4. Router registers both agents.
5. Orchestrator A wants to execute a CLI program:
   a. A sends program to bus, addressed to "capability: exec"
   b. Router matches to cli-1 (only available CLI agent)
   c. Bus delivers program to B
   d. B's CCS VM executes the program
   e. B sends result program back to A
   f. A receives result, continues
6. If B disconnects, router marks it offline, routes to cli-2 if available.
```

---

## 26. Safety & Error Model

### Safety Levels

Every instruction is checked against a **safety level**:

```rust
// a2x-ccs/src/safety.rs

#[derive(Clone, Debug)]
pub enum SafetyLevel {
    /// No restrictions. Development/debug only.
    Unrestricted,
    /// Bounded execution. Limits on loops, memory, side effects.
    Bounded {
        max_instructions: u64,
        max_memory_bytes: u64,
        max_side_effects: u32,
        allowed_syscalls: Vec<String>,
    },
    /// Sandboxed. All side effects filtered through allowlist.
    Sandboxed {
        allowed_commands: Vec<GlobPattern>,
        allowed_network: Vec<String>,
        allowed_files: Vec<PathGlob>,
    },
    /// Full isolation. No side effects at all. Read-only world-model.
    Isolated,
}
```

### Safety Constraints at the ISA Level

Safety is **baked into the instruction encoding**, not bolted on:

1. **Flags in every instruction header** — each instruction carries its safety classification.
2. **Capability bits** — each instruction specifies which capabilities it needs (network, filesystem, etc.).
3. **Bounds on immediate values** — D field has min/max constraints encoded in the type system.

```rust
struct SafetyClassification {
    /// Does this instruction execute system commands?
    requires_exec: bool,
    /// Does this instruction read files?
    requires_fs_read: bool,
    /// Does this instruction write files?
    requires_fs_write: bool,
    /// Does this instruction make network requests?
    requires_network: bool,
    /// Bounds on memory allocation (number of WorldGraph nodes).
    max_allocation: Option<u64>,
    /// Bounds on execution time (number of VM steps).
    max_steps: Option<u64>,
}
```

### Error Types

| Error | Source | Description |
|-------|--------|-------------|
| `LexError::UnknownCharacter(char)` | Stage 1 | Unrecognized character in input |
| `ParseError::UnexpectedToken` | Stage 2 | Token doesn't fit instruction format |
| `ParseError::MissingField` | Stage 2 | Required I/C/P/D field is empty |
| `SemanticError::UndefinedLabel` | Stage 3 | Jump target doesn't exist |
| `SemanticError::TypeMismatch` | Stage 3 | Data doesn't match expected type for opcode |
| `CompileError::UnsupportedOpcode` | Stage 6 | Opcode has no Ω encoding yet |
| `VmError::OutOfMemory` | Runtime | WorldGraph allocation limit exceeded |
| `VmError::SafetyViolation` | Runtime | Instruction violates safety constraints |
| `VmError::InvalidAddress` | Runtime | Operand references non-existent memory |
| `VmError::DivisionByZero` | Runtime | Mathematical error in concept operation |
| `VmError::ParallelMergeConflict` | Runtime | Fork results conflict and can't be merged |
| `VmError::MaxStepsExceeded` | Runtime | Program exceeded instruction limit |
| `AgentError::ProgramCrash(VmError)` | Runtime | Agent's CCS VM crashed during execution |
| `AgentError::Timeout` | Runtime | Program exceeded time limit |
| `TransportError::ConnectionLost` | Bus | Remote agent disconnected |

### Error Recovery Model

```
┌─────────────────────────────────────────────────┐
│  Error occurs during execution                   │
│         │                                        │
│         ▼                                        │
│  ┌─────────────┐                                 │
│  │ Can recover? │───Yes───→ Retry / Skip / Continue│
│  └──────┬──────┘                                 │
│         │ No                                      │
│         ▼                                        │
│  ┌──────────────┐                                │
│  │ Has parent?   │───Yes──→ Escalate to caller    │
│  └──────┬───────┘   (⤊ escalate)                  │
│         │ No                                      │
│         ▼                                        │
│  ┌─────────────────────┐                         │
│  │ Crash: emit error    │                         │
│  │ program, halt VM     │                         │
│  └─────────────────────┘                         │
└─────────────────────────────────────────────────┘
```

### Error Programs

When a program crashes, the VM produces an **error program** — a Σ∞ program that describes what went wrong:

```
⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟤⟨crash⟩ ∷ P:✕ ∷ D:⌴⟨VmError::OutOfMemory⟩⟭
```

This error program can be:
1. **Returned to the caller** — the orchestrator receives the crash report.
2. **Logged** — traced for debugging.
3. **Handled by a supervisor** — a meta-agent that restarts failed programs.

---

## 27. Agent Lifecycle

### States

```
┌──────────┐    ┌──────────┐    ┌──────────┐
│  Idle    │───▶│  Running │───▶│  Idle    │
└──────────┘    └──────────┘    └──────────┘
     │                │               │
     ▼                ▼               ▼
┌──────────┐    ┌──────────┐    ┌──────────┐
│  Error   │◀───│  Error   │    │  Halted  │
└──────────┘    └──────────┘    └──────────┘
     │
     ▼
┌──────────┐
│  Dead    │
└──────────┘
```

```rust
// a2x-agents/src/lifecycle.rs

enum AgentState {
    /// Agent is initialized but idle, waiting for a program.
    Idle,
    /// Agent is executing a program.
    Running {
        program_id: ProgramId,
        started_at: Instant,
        vm: Box<CcsVm>,
    },
    /// Agent encountered a recoverable error.
    Error {
        error: AgentError,
        retry_count: u32,
    },
    /// Agent is permanently stopped.
    Halted,
    /// Agent is terminated (can be restarted).
    Dead,
}

struct AgentLifecycle {
    state: AgentState,
    max_retries: u32,
    heartbeat_interval: Duration,
    last_heartbeat: Instant,
}

impl AgentLifecycle {
    /// Start executing a program.
    fn execute(&mut self, program: SigmaProgram) -> Result<(), AgentError> { /* ... */ }

    /// Handle an error (retry or escalate).
    fn handle_error(&mut self, error: AgentError) -> Result<(), AgentError> {
        if self.state.retry_count < self.max_retries {
            self.state = AgentState::Error { error, retry_count: self.state.retry_count + 1 };
            // Retry after backoff
            Ok(())
        } else {
            Err(error)
        }
    }

    /// Gracefully shut down.
    fn halt(&mut self) { /* save state, disconnect from bus */ }
}
```

### Agent Configuration

```toml
# Example agent config: ~/.a2x/agents/cli-1.toml
[agent]
id = "cli-1"
type = "cli"
label = "primary execution agent"

[agent.capabilities]
exec = true
fs = true
network = ["tcp", "dns"]

[safety]
level = "bounded"
max_instructions = 10000
max_memory_mb = 256
allowed_commands = ["ls", "ps", "netstat", "cat", "grep", "find"]
forbidden_patterns = ["rm", "sudo", "chmod", "dd", "> /dev/*"]

[bus]
transport = "tcp"
listen = "127.0.0.1:0"  # random port
bootstrap = ["127.0.0.1:8777"]  # directory / orchestrator

[storage]
worldgraph = "~/.a2x/data/cli-1/worldgraph.bin"
memory = "~/.a2x/data/cli-1/memory.bin"

[logging]
level = "info"
format = "json"
file = "~/.a2x/logs/cli-1.log"
```

### Monitoring & Heartbeats

- Agents send heartbeats every `heartbeat_interval` (default: 5 seconds).
- If no heartbeat for 3× interval, agent is presumed dead.
- Router marks dead agents offline, redistributes their workload.
- State is stored to disk for crash recovery.

---

## 28. Debug & Probe Protocol

### Probe Interface

Every CCS VM exposes a **probe interface** that allows external tools to inspect its internal state:

```rust
// a2x-ccs/src/probe.rs

/// Requests that can be sent to a running CCS VM.
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

/// Response from a CCS VM to a probe query.
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

### How Probe Tools Connect

```
┌──────────────────┐       ┌──────────────────┐
│  CLI Probe Tool  │──────▶│  CCS VM (Agent)  │
│  a2x-cli          │       │                   │
│  $ probe --agent │       │  ProbeQuery/Response│
│    cli-1         │       │  over transport    │
└──────────────────┘       └──────────────────┘

1. Probe tool discovers agent via bus discovery.
2. Probe tool opens a probe channel (separate stream from program execution).
3. Probe tool sends ProbeQuery messages.
4. VM responds with ProbeSnapshot messages.
5. Probe tool displays or logs the state.
6. Optionally: set breakpoints, single-step through programs.
```

### Visualization (Future)

The `a2x-probe` crate (Phase 5) will provide:
- **WorldGraph visualizer** — graphviz dot output or egui interactive graph
- **StateField heatmap** — visualize tensor regions as color grids
- **Instruction tracer** — step through program execution instruction by instruction
- **MemoryTrace timeline** — scroll through state history, see how concepts evolved

---

## 29. Project Name

**A2X** — Agent-to-Anything.

- The project name is **A2X** (stylized as ***A2X***).
- The runtime is **CCS** (CryoCore Cognitive Substrate).
- The crate prefix is `a2x-*`.
- The directory is `a2x/`.
- The config/data directory is `~/.a2x/`.

This is settled. No further naming discussion needed.

---

## 30. Entity Integration Layer — Connecting Anything to A2X

The "Anything" in A2X isn't just other A2X agents — it's **any external entity**: an existing application, a database, a web API, a human with a terminal, a robot, a Slack bot, a CI/CD pipeline. This section defines how anything with a network connection (or stdin/stdout) can speak A2X.

### What Is an Entity?

An **entity** is any external system or user that communicates with the A2X ecosystem. Entities are **not** native A2X agents — they don't run a CCS VM internally. Instead, they connect through an **entity adapter** that translates between the entity's native protocol and A2X bus messages.

| Entity Type | Native Protocol | Adapter Translation |
|-------------|----------------|---------------------|
| **Human (CLI)** | stdin/stdout | TUI → Σ∞ packet stream |
| **Human (Web)** | HTTP / WebSocket | REST/WS → A2X bus messages |
| **AI (LLM)** | HTTP (OpenAI API format) | API call → Σ∞ program generation |
| **Existing app** | gRPC / REST | Protocol bridge → A2X actions |
| **Database** | SQL / custom wire protocol | Query → WorldGraph lookup program |
| **Robot** | ROS / serial / custom | Sensor stream → Σ∞ perception pipeline |
| **CI/CD** | Webhook / REST API | Event → A2X trigger program |
| **Other A2X network** | A2X bus protocol | Router-to-router bridge (A2X meso-net) |

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    A2X BUS                                │
│  (internal agent-to-agent message routing)                │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │    A2X GATEWAY         │
         │  (entity bridge)       │
         └───────────┬───────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
    ▼                ▼                ▼
┌────────┐     ┌──────────┐     ┌──────────┐
│ Entity │     │ Entity   │     │ Entity   │
│ (CLI)  │     │ (HTTP)   │     │ (DB)     │
└────────┘     └──────────┘     └──────────┘
```

### The Entity Trait

Every entity connector implements the `Entity` trait:

```rust
// a2x-gateway/src/entity.rs

/// Represents an external entity connected to the A2X ecosystem.
#[async_trait]
pub trait Entity: Send + Sync {
    /// Unique entity ID.
    fn entity_id(&self) -> EntityId;

    /// Entity type tag ("human", "llm", "database", etc.).
    fn entity_type(&self) -> EntityType;

    /// Human-readable name (for display/probe).
    fn display_name(&self) -> String;

    /// Send a Σ∞ or Ω program to this entity.
    /// The entity adapter handles translation to the entity's native format.
    async fn send(&self, program: &Packet) -> Result<(), EntityError>;

    /// Receive the next program from this entity (blocking/async).
    async fn recv(&self) -> Result<Packet, EntityError>;

    /// Check if the entity is still connected.
    fn is_alive(&self) -> bool;

    /// Entity capabilities (what this entity can do).
    fn capabilities(&self) -> Vec<Capability>;
}
```

### A2X Gateway (`a2x-gateway` crate)

The **gateway** is a service that:
1. Listens for incoming entity connections on multiple protocols.
2. Creates an `Entity` adapter for each connection.
3. Registers the entity with the A2X bus (via discovery).
4. Translates between entity-native protocols and A2X packets.
5. Routes A2X responses back to the entity.

```rust
// a2x-gateway/src/lib.rs

/// A2X Gateway — bridges external entities to the A2X bus.
pub struct Gateway {
    /// Entity adapters indexed by EntityId.
    entities: HashMap<EntityId, Box<dyn Entity>>,
    /// Connection to the A2X bus.
    bus: BusConnection,
    /// Protocol listeners (HTTP, WS, TCP, etc.).
    listeners: Vec<Box<dyn ProtocolListener>>,
}

impl Gateway {
    /// Start the gateway — begin listening for entity connections.
    pub async fn start(&mut self) -> Result<(), GatewayError> {
        for listener in &self.listeners {
            listener.start().await?;
        }
        // Main loop: accept entities, route programs
        loop {
            tokio::select! {
                // Handle new entity connections
                entity = accept_entity() => {
                    self.register_entity(entity).await?;
                },
                // Route programs between bus and entities
                Some((entity_id, program)) = self.bus.recv_for_entity() => {
                    if let Some(entity) = self.entities.get(&entity_id) {
                        entity.send(&program).await?;
                    }
                },
            }
        }
    }

    /// Register a new entity and announce it on the bus.
    async fn register_entity(&mut self, entity: Box<dyn Entity>) -> Result<(), GatewayError> {
        let id = entity.entity_id();
        self.entities.insert(id, entity);
        self.bus.announce(EntityJoined { id, capabilities }).await?;
        Ok(())
    }
}

/// A protocol listener accepts connections from a specific protocol.
#[async_trait]
pub trait ProtocolListener: Send + Sync {
    /// Start listening on the given address.
    async fn start(&mut self) -> Result<(), GatewayError>;
    /// Accept a new entity connection.
    async fn accept(&self) -> Result<Box<dyn Entity>, GatewayError>;
}
```

### Built-in Protocol Listeners

#### 1. HTTP / REST Listener

Allows any HTTP client to interact with A2X:

```
POST /a2x/execute
Content-Type: application/json

{
  "program": "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭",
  "format": "sigma",
  "timeout_ms": 5000,
  "metadata": {
    "source": "my-web-app",
    "user": "josh"
  }
}

Response 200:
{
  "result": "⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟚⟤⟨22,8080⟩ ∷ P:⤊⥂ ∷ D:⌯⌴⟭",
  "execution_time_ms": 234,
  "status": "completed"
}
```

**Endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/a2x/execute` | Execute a Σ∞/Ω program and return the result |
| `GET` | `/a2x/entities` | List connected entities and agents |
| `GET` | `/a2x/entities/{id}` | Get entity/agent details |
| `GET` | `/a2x/probe/{agent_id}` | Probe an agent's state (if authorized) |
| `POST` | `/a2x/stream` | WebSocket upgrade: streaming bidirectional A2X communication |
| `POST` | `/a2x/webhook` | Register a webhook URL for A2X callbacks |

#### 2. WebSocket Listener

For streaming, bidirectional A2X communication:

```
Client → Server: Raw Σ∞ or Ω packets (text or binary frames)
Server → Client: Result packets, streaming data, events

Example:
  Client sends:   ⟦Σ∞⟧⟬I:⚡✦ ∷ C:⟨log⟩ ∷ P:⥂ ∷ D:⌲⟭   // stream logs
  Server streams: ⟦Σ∞⟧⟬I:⟁ ∷ C:⟨log⟩ ∷ D:∂⟨[log line 1]⟩⟭
  Server streams: ⟦Σ∞⟧⟬I:⟁ ∷ C:⟨log⟩ ∷ D:∂⟨[log line 2]⟩⟭
  Server streams: ⟦Σ∞⟧⟬I:⤑ ∷ C:⟨log⟩ ∷ D:⌳⟨summary⟩⟭   // merged result
```

#### 3. TCP Listener

Raw TCP socket, each message is a length-prefixed serialized packet:

```
[4-byte length][serialized packet bytes]
```

#### 4. stdin/stdout Listener

For CLI/pipe integration:

```bash
# Send a program via stdin, get result on stdout
echo "⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭" | a2x-gateway --listen stdio
```

#### 5. Webhook Callback

Instead of polling for results, entities can register a webhook URL. When a result is ready, A2X POSTs it back:

```
POST /my-service/a2x-callback
Content-Type: application/json

{
  "correlation_id": "abc-123",
  "result": "⟦Σ∞⟧⟬I:⚠⟁ ∷ C:⟚⟤⟨22,8080⟩...",
  "status": "completed"
}
```

### Entity Authentication & Authorization

Entities must authenticate before they can send programs:

```rust
// a2x-gateway/src/auth.rs

/// How an entity authenticates.
pub enum AuthMethod {
    /// API key in HTTP header (X-A2X-Key).
    ApiKey(String),
    /// Bearer token (JWT).
    BearerToken(String),
    /// TLS client certificate.
    ClientCert(Vec<u8>),
    /// No auth (local connections only, e.g. Unix socket).
    Local,
}

/// What an entity is allowed to do.
pub struct EntityPermissions {
    /// Entity ID this permission belongs to.
    pub entity_id: EntityId,
    /// Maximum instruction count per program.
    pub max_instructions: u64,
    /// Allowed opcodes (None = all allowed).
    pub allowed_opcodes: Option<Vec<Opcode>>,
    /// Allowed addressing modes.
    pub allowed_modes: Vec<AddressingMode>,
    /// Can this entity probe agent state?
    pub can_probe: bool,
    /// Can this entity access external network?
    pub can_network: bool,
    /// Per-minute rate limit.
    pub rate_limit: u32,
}
```

### Client SDKs for Entities

To make it easy for any entity to speak A2X, we provide **client SDKs** in multiple languages. Each SDK wraps the protocol details and exposes a simple API:

#### Rust SDK (`a2x-client`)

```rust
// a2x-client/src/lib.rs
use a2x_core::prelude::*;

/// High-level client for connecting to A2X from any application.
pub struct A2xClient {
    gateway_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl A2xClient {
    /// Connect to an A2X gateway.
    pub fn new(gateway_url: &str, api_key: &str) -> Self { /* ... */ }

    /// Execute a Σ∞ program and wait for the result.
    pub async fn execute(&self, program: SigmaProgram) -> Result<SigmaProgram, ClientError> {
        // POST to gateway's /a2x/execute endpoint
        let response = self.client
            .post(format!("{}/a2x/execute", self.gateway_url))
            .header("X-A2X-Key", &self.api_key)
            .json(&program)
            .send().await?;
        Ok(response.json().await?)
    }

    /// Open a streaming WebSocket connection.
    pub async fn stream(&self) -> Result<A2xStream, ClientError> {
        // WebSocket upgrade
    }

    /// Register a webhook for async callbacks.
    pub async fn register_webhook(&self, url: &str) -> Result<(), ClientError> {
        // POST to /a2x/webhook
    }

    /// List all connected agents/entities.
    pub async fn list_entities(&self) -> Result<Vec<EntityInfo>, ClientError> {
        // GET /a2x/entities
    }
}
```

#### Python SDK (Third-party / Future)

```python
# a2x-client-python/a2x/__init__.py

class A2X:
    def __init__(self, gateway_url: str, api_key: str):
        self.gateway_url = gateway_url
        self.api_key = api_key

    def execute(self, program: str, timeout: int = 5000) -> str:
        """Execute a Σ∞ program and return the result as a string."""
        resp = requests.post(f"{self.gateway_url}/a2x/execute",
            json={"program": program, "timeout_ms": timeout},
            headers={"X-A2X-Key": self.api_key})
        return resp.json()["result"]

    def stream(self) -> WebSocket:
        """Open a streaming WebSocket connection."""
        ws = websocket.create_connection(
            f"ws://{self.gateway_url}/a2x/stream",
            header=[f"X-A2X-Key: {self.api_key}"])
        return ws
```

#### JavaScript SDK (Third-party / Future)

```javascript
// a2x-client-js/src/index.js

class A2X {
    constructor(gatewayUrl, apiKey) {
        this.gatewayUrl = gatewayUrl;
        this.apiKey = apiKey;
    }

    async execute(program, timeoutMs = 5000) {
        const resp = await fetch(`${this.gatewayUrl}/a2x/execute`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-A2X-Key': this.apiKey
            },
            body: JSON.stringify({ program, timeout_ms: timeoutMs })
        });
        return (await resp.json()).result;
    }

    stream() {
        const ws = new WebSocket(`${this.gatewayUrl}/a2x/stream`);
        ws.onopen = () => ws.send(JSON.stringify({ auth: this.apiKey }));
        return ws;
    }
}
```

### Gateway Configuration

```toml
# ~/.a2x/gateway.toml
[gateway]
bind_address = "0.0.0.0:8777"

[http]
enabled = true
port = 8778
cors_origin = "*"

[websocket]
enabled = true
port = 8779

[tcp]
enabled = true
port = 8780

[stdio]
enabled = true  # for pipe / CLI integration

[auth]
mode = "api_key"  # or "jwt", "cert", "local"
api_keys = [
    { key = "sk-a2x-abc123", permissions = "admin" },
    { key = "sk-a2x-def456", permissions = "limited" },
]

[webhook]
enabled = true
timeout_ms = 10000
max_retries = 3
```

### Official Entity Adapter Crates

These are official crates in the workspace that provide built-in entity integrations:

| Crate | Purpose |
|-------|---------|
| `a2x-gateway` | Core gateway: entity registry, protocol listeners, auth, routing |
| `a2x-client` | Rust client SDK for entities |
| `a2x-entity-http` | HTTP/REST protocol listener + webhook handler |
| `a2x-entity-ws` | WebSocket protocol listener |
| `a2x-entity-tcp` | Raw TCP protocol listener |
| `a2x-entity-stdio` | stdin/stdout protocol listener (for CLI/pipe) |

Third-party entity crates (examples):
| Crate | Purpose |
|-------|---------|
| `a2x-entity-slack` | Connect Slack bots to A2X |
| `a2x-entity-discord` | Connect Discord bots to A2X |
| `a2x-entity-github` | GitHub Actions / webhook integration |
| `a2x-entity-python` | Python client SDK |
| `a2x-entity-js` | JavaScript/TypeScript client SDK |
| `a2x-entity-postgres` | Execute SQL queries via A2X programs |

### Example: Connecting a Web App to A2X

1. Start the A2X gateway:
   ```bash
   a2x-gateway --config ~/.a2x/gateway.toml
   ```

2. From a web app (JavaScript):
   ```javascript
   const a2x = new A2X('http://localhost:8778', 'sk-a2x-abc123');

   // Execute a program to scan the system
   const result = await a2x.execute('⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭');
   console.log('Scan result:', result);
   ```

3. The gateway:
   - Receives the HTTP POST.
   - Parses the Σ∞ program string into a `SigmaPacket`.
   - Sends it to the A2X bus.
   - The bus routes it to the CLI agent.
   - The CLI agent executes it, returns a result program.
   - The gateway sends the result back as HTTP response.

### Phase Addition to Roadmap

A new phase should be added to the implementation roadmap:

**Phase 6: Entity Integration (Weeks 17-20)**

- [ ] `a2x-gateway` crate: core gateway, Entity trait, entity registry.
- [ ] Entity authentication (API key, JWT, local).
- [ ] HTTP listener (`/a2x/execute`, `/a2x/entities`, `/a2x/probe`).
- [ ] WebSocket listener (streaming Σ∞/Ω packets).
- [ ] TCP listener (length-prefixed binary packets).
- [ ] stdin/stdout listener (pipe/CLI integration).
- [ ] Webhook callback system.
- [ ] `a2x-client` crate: Rust client SDK.
- [ ] Python client SDK (third-party starter).
- [ ] JavaScript client SDK (third-party starter).
- [ ] Gateway configuration (TOML).
- [ ] End-to-end demo: web app → HTTP → gateway → bus → CLI agent → result.
