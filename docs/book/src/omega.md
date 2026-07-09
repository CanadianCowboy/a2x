# Ω Compilation Pipeline

The Ω (Omega) compiler transforms human-readable Σ∞ programs into executable
latent tensor representations.

## Pipeline Stages

| Stage | Description |
|-------|-------------|
| 1. **Lexer** | Unicode-aware tokenizer → `Vec<Token>` |
| 2. **Parser** | Token stream → `SigmaProgram` with label table |
| 3. **Semantic Analyzer** | Validates jumps, contradictory ops, data types |
| 4. **IR Generator** | `SigmaProgram` → `IrGraph` with dataflow edges |
| 5. **Optimizer** | 4 passes: constant folding, dead code, fusion, layout |
| 6. **Code Generator** | `IrGraph` → `OmegaProgram` via topological sort |
| 7. **Serializer** | `OmegaProgram` → binary blob (bincode) |

## Usage

```rust
use a2x_omega::CompileToOmega;
use a2x_sigma::parse_program;

let source = r#"⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭"#;
let sigma = parse_program(source)?;
let omega = sigma.compile(a2x_omega::OptimizationLevel::default())?;
```

## Optimization Levels

| Level | Passes | Description |
|-------|--------|-------------|
| `None` | 0 passes | No optimization, fastest compile |
| `Default` | 4 passes | Balanced — constant folding + dead code + fusion + layout |
| `Aggressive` | 4 passes (extra iterations) | Maximum optimization for production |

## Validation

The semantic analyzer checks:

- **Jump targets** — all branch labels exist and are reachable
- **Contradictory operators** — no Cancel+Ground on same label in sequence
- **Data types** — payload types match operator requirements
- **Empty intents** — every instruction has a valid operator

Errors are reported with source location, making debugging Σ∞ programs
straightforward.
