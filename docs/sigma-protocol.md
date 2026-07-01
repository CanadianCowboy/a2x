# A2X Σ∞ Protocol Reference

> Language specification for the Σ∞ (Sigma-Infinity) protocol — the AI-native
> programming language used by agents to express cognitive operations.

## Overview

Σ∞ is a Unicode-based instruction format where each instruction (packet) encodes:

- **I (Intent):** What to do — cognitive operators like Synthesis, Split, Star
- **C (Context):** Where/on what — labels, regions, concept references
- **P (Plan):** Control flow — sequential, branch, descend, swarm
- **D (Data):** Payload — raw tensors, graph deltas, diff patches

### Packet Syntax

```
⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟨sys⟩ ∷ P:⤐ ∷ D:⌬⟭
```

Each instruction is enclosed in `⟦Σ∞⟧⟬...⟭` brackets with 4 sections separated by `∷`.

## Intent Operators (I-field)

| Symbol | Operator | Description |
|--------|----------|-------------|
| ✣ | Synthesis | Create new concepts via binding |
| ✕ | Cancel | Destroy/remove concepts |
| ⟐ | Split | Differentiate concepts |
| ✦ | Star | Explore/ground new territory |
| ⚡ | Lightning | Immediate/accelerated execution |
| ⚠ | Warning | Safe/guarded execution |
| ⩂ | Delay | Slow/evolve mode |
| ⩈ | Parallel | Fork parallel sub-programs |
| ⩫ | Merge | Join parallel results |
| ⩎ | Contradiction | Halt on contradiction |

## Context Operators (C-field)

| Symbol | Operator | Description |
|--------|----------|-------------|
| ⟘ | Null | No context |
| ⧖ | Universal | All concepts |
| ⟑ | Compression | Compressed view |
| ⩕ | Uncertainty | Probabilistic context |
| → | CausalChain | Cause-effect links |
| ↔ | SpatialChain | Spatial relations |
| ↻ | TemporalChain | Time-ordered links |

## Plan Operators (P-field)

| Symbol | Operator | Description |
|--------|----------|-------------|
| ⥂ | Sequential | Execute next instruction |
| ⤐ | Branch | Conditional jump |
| ⤈ | Descend | Enter sub-program |
| ⤉ | Ascend | Return from sub-program |
| ⥁ | Swarm | Parallel execution |
| ⤑ | Merge | Join branches |
| ⤒ | Escalate | Raise to orchestrator |
| ⤓ | Recursive | Self-referential call |

## Data Operators (D-field)

| Symbol | Operator | Description |
|--------|----------|-------------|
| ⌬ | RawTensor | Raw tensor data |
| ⌭ | LatentVector | Latent space vector |
| ⌮ | GraphDelta | WorldGraph delta |
| ⌯ | DiffPatch | Differentiation patch |
| ⌰ | Binary | Binary payload |
| ⌱ | Fusion | Fused operator data |
| ⌲ | Streaming | Stream marker |

## Protocol Identifiers

| Protocol | Description |
|----------|-------------|
| Σ∞ | Sigma-Infinity (text form) |
| Ω | Omega (latent tensor form) |
| Raw | Raw binary (ISA encoding) |

## See Also

- [Ω Compilation Pipeline](omega-compilation.md)
- [CCS Virtual Machine](ccs-vm.md)
- `plans/01-sigma-language.md` — Full language design spec
