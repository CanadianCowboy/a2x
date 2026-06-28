# A2X Ω Compiler — Σ∞ → Ω Compilation Pipeline

> **The compiler that transforms Σ∞ source programs into Ω latent tensor programs for fast neural execution.**

---

## 1. Overview

Ω is the **compiled form** of a Σ∞ program. Once a Σ∞ program is stable and debugged, it can be JIT-compiled into Ω tensors for direct execution by the CCS runtime — no symbolic parsing needed.

| Concept | Analogy |
|---------|---------|
| Σ∞ | Source code (human-peekable, symbolic) |
| Ω | Compiled binary (latent, tensor, fast) |
| CCS | The CPU that executes it |

- **Crate:** `a2x-omega`
- **Depends on:** `a2x-core`, `ndarray` (optional)
- **Key files:** `compiler.rs`, `packet.rs`, `program.rs`, `decoder.rs`, `bridge.rs`, `passes/`

---

## 2. Ω Packet Shape

Each Ω instruction is a single high-dimensional tensor:

```
Ω ∈ ℝ^N   (single high-dimensional tensor)

Segmented into slices:
  - Ω_I  ∈ ℝ^1024   → intent
  - Ω_C  ∈ ℝ^4096   → context
  - Ω_P  ∈ ℝ^8192   → plan
  - Ω_D  ∈ ℝ^16384  → data
```

Total dimension: 29,796 (configurable via const generics).

```rust
#[derive(Clone, Debug)]
pub struct OmegaPacket<const N: usize = 29796> {
    data: [f32; N],
}

// Slice offsets (compile-time constants)
const OFFSET_I: usize = 0;
const OFFSET_C: usize = 1024;
const OFFSET_P: usize = 1024 + 4096;       // 5120
const OFFSET_D: usize = 1024 + 4096 + 8192; // 13312
```

---

## 3. Compilation Pipeline (7 Stages)

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

### Stage 1: Lexer
- Input: Raw string (Σ∞ text)
- Output: `Vec<Token>`
- Matches Unicode operators using trie-based matcher
- See [01-sigma-language.md](01-sigma-language.md#8-tokenizer-design)

### Stage 2: Parser
- Input: `Vec<Token>`
- Output: `SigmaProgram` (instruction stream + label table)
- See [01-sigma-language.md](01-sigma-language.md#9-parser-design)

### Stage 3: Semantic Analyzer
Validates:
- All jump targets reference valid labels or sub-programs
- Sub-program definitions are well-formed
- Data types in D field match expected types for the opcode
- No contradictory operators in the same instruction

### Stage 4: IR Generator

Produces an **Intermediate Representation** — a dataflow graph:

```rust
struct IrNode {
    id: NodeId,
    opcode: VmOpcode,          // CCS VM instruction
    operands: Vec<Operand>,    // WorldGraph refs, StateField regions
    control_flow: Vec<NodeId>, // Next nodes
    metadata: IrMetadata,
}

enum VmOpcode {
    Bind, Differentiate, Ground, Evolve, Reflect, Plan, Actuate,
    Jump, Branch, Call, Return, Fork, Merge, Halt,
}
```

### Stage 5: Optimizer Passes

| Pass | Description |
|------|-------------|
| **Constant folding** | Evaluate constant `BIND` operations at compile time |
| **Dead instruction elimination** | Remove unused results |
| **Instruction fusion** | Merge adjacent instructions on same memory region |
| **Layout optimization** | Reorder for cache locality in VM instruction cache |

```rust
fn optimize(ir: &mut IrGraph) {
    constant_folding(ir);
    dead_code_elimination(ir);
    instruction_fusion(ir);
    layout_optimization(ir);
}
```

### Stage 6: Code Generator

IR graph → Ω program:

```rust
fn codegen(ir: &IrGraph) -> Result<OmegaProgram, CompileError> {
    let mut omega = OmegaProgram::new();
    for node in topological_sort(ir) {
        let packet = encode_instruction(node)?;
        omega.push(packet);
    }
    Ok(omega)
}
```

Each IR node is encoded into the four Ω tensor regions:
- Opcode → intent region
- Operands → context region
- Control flow → plan region
- Metadata → data region

### Stage 7: Serializer

Ω program → binary blob (bincode or raw bytes):

```rust
#[derive(Serialize, Deserialize)]
pub struct OmegaProgram {
    pub instructions: Vec<OmegaPacket>,
    pub metadata: ProgramMetadata,
}
```

---

## 4. Compiler Optimization Levels

| Level | Name | Passes | Use Case |
|-------|------|--------|----------|
| `-O0` | None | None | Debugging, development |
| `-O1` | Light | Constant folding + dead code elimination | Default |
| `-O2` | Balanced | All passes | Production |
| `-O3` | Aggressive | All passes + speculative optimization | Hot paths |
| `-Os` | Size | All passes, optimize for tensor size | Bandwidth-constrained |

---

## 5. Encoder / Decoder Traits

### Compilation (Encoder)

```rust
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

---

## 6. Σ∞ ↔ Ω Bridge

```
Σ∞ source  ──compile──→ Ω latent  ──execute──→ CCS runtime
Ω latent   ──decompile──→ Σ∞ source  ──log/debug──→ human peek
```

The bridge serves as the **compiler toolchain**:
1. Write/debug programs in Σ∞ (symbolic, inspectable)
2. Compile hot paths to Ω (fast, latent, non-symbolic)
3. CCS executes Ω natively at maximum speed
4. Decompile Ω back to Σ∞ for debugging and tracing

---

*This sub-plan maps to phases 0 and 3 of the implementation roadmap.*
