# A2X Ω Compilation Pipeline

> Reference for the Omega (Ω) compiler — transforms Σ∞ programs into latent
> tensor representations for execution on the CCS virtual machine.

## Pipeline Stages

The Ω compiler has 7 stages (see `plans/02-omega-compiler.md`):

| Stage | Name | Description |
|:-----:|------|-------------|
| 1 | **Lexer** | Unicode-aware tokenizer → `Vec<Token>`. Fuzzed and proptested. |
| 2 | **Parser** | Token stream → `SigmaProgram` with label table. Proptested. |
| 3 | **Semantic Analyzer** | Validates jump targets, contradictory operators, data types, empty intents. |
| 4 | **IR Generator** | `SigmaProgram` → `IrGraph` with dataflow and control flow edges. |
| 5 | **Optimizer** | 4 passes: constant folding, dead code elimination, operator fusion, layout. |
| 6 | **Code Generator** | `IrGraph` → `OmegaProgram` via topological sort + deterministic encoding. |
| 7 | **Serializer** | `OmegaProgram` → binary blob (bincode-ready). |

## Compilation Example

```rust
use a2x_omega::{CompileToOmega, OptimizationLevel};
use a2x_sigma::parse_program;

let source = "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭";
let sigma = parse_program(source)?;
let omega = sigma.compile(OptimizationLevel::default())?;
```

## Optimizer Passes

| Pass | Description |
|------|-------------|
| `constant_folding` | Folds all-immediate Bind operations into Nop |
| `dead_code` | Removes orphaned nodes with no control flow references |
| `fusion` | Merges adjacent Bind+Diff pairs with matching labels |
| `layout` | Sorts nodes by source_index for cache-friendly access |

## Tensor Encoding

Each Σ∞ instruction encodes into a 29,796-element float tensor with 4 regions:

- **I (Intent):** 52 floats — opcode projection
- **C (Context):** 42 floats — operand/label projection
- **P (Plan):** 76 floats — control flow projection
- **D (Data):** 29,626 floats — metadata/payload projection

Phase 0 uses deterministic Blake3 hashing. Future phases use learned neural encoders.

## See Also

- [Σ∞ Protocol Reference](sigma-protocol.md)
- [CCS Virtual Machine](ccs-vm.md)
- `plans/02-omega-compiler.md` — Full compiler design spec
