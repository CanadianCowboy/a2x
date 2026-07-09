# Σ∞ Language Reference

> **Σ∞** (Sigma-Infinity) — the AI-native programming language used by agents
> to express cognitive operations.

## Packet Syntax

```
⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭
```

Each instruction is enclosed in `⟦Σ∞⟧⟬...⟭` brackets with 4 fields separated
by `∷` (Unicode PROPORTION, U+2237).

| Field | Meaning | Example |
|-------|---------|---------|
| **I** (Intent) | What to do | `✦` (Ground), `✣` (Synthesize) |
| **C** (Context) | Where/on what | `⟨sys⟩` (system context) |
| **P** (Plan) | Control flow | `⥂` (sequential), `⤐` (branch) |
| **D** (Data) | Payload | `⌬` (raw tensor), `⌭` (latent vector) |

## Protocol Identifiers

| Protocol | Encoding | Use |
|----------|----------|-----|
| **Σ∞** | Unicode text | Human-readable, editable |
| **Ω** | Float tensors | Machine-executable |
| **Raw** | Binary ISA | Maximum performance |

## Operator Categories

- [Intent Operators](sigma-intents.md) — 11 cognitive actions
- [Context Operators](sigma-context.md) — 11 ways to reference concepts
- [Plan Operators](sigma-plan.md) — 12 control flow directives
- [Data Operators](sigma-data.md) — 11 payload encodings

## Multi-Packet Programs

Programs are sequences of packets executed in order:

```text
⟦Σ∞⟧⟬I:✦ ∷ C:⟨x⟩ ∷ P:⥂ ∷ D:⌬⟭
⟦Σ∞⟧⟬I:✦ ∷ C:⟨y⟩ ∷ P:⥂ ∷ D:⌬⟭
⟦Σ∞⟧⟬I:✣ ∷ C:⟨x⟩⟨y⟩ ∷ P:⥂ ∷ D:⌬⟭
```

This program:
1. Grounds concept `x`
2. Grounds concept `y`
3. Synthesizes a new concept from `x` and `y`
