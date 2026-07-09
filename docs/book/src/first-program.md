# Your First Program

A Σ∞ program is a sequence of **packets** — each enclosed in `⟦Σ∞⟧⟬...⟭` brackets
with four fields:

- **I** (Intent) — the cognitive operator to apply
- **C** (Context) — labels and concept references
- **P** (Plan) — control flow
- **D** (Data) — payload

## Hello, World!

The simplest program **grounds** a concept with the label `hello`:

```text
⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭
```

This means: **Ground** (✦) a new concept with the label `hello` (⟨hello⟩),
proceed sequentially (⥂), with a tensor payload (⌬).

Run it:

```bash
a2x run "⟦Σ∞⟧⟬I:✦ ∷ C:⟨hello⟩ ∷ P:⥂ ∷ D:⌬⟭"
```

## Synthesizing Ideas

The **Synthesis** (✣) operator combines concepts:

```text
⟦Σ∞⟧⟬I:✣ ∷ C:⟨happy⟩⟨sad⟩ ∷ P:⥂ ∷ D:⌬⟭
```

This creates a new concept by binding `happy` and `sad` together.

## Branching Control Flow

**Branch** (⤐) allows conditional execution:

```text
⟦Σ∞⟧⟬I:✦ ∷ C:⟨check⟩ ∷ P:⤐ ∷ D:⌬⟭
⟦Σ∞⟧⟬I:✣ ∷ C:⟨check⟩⟨result⟩ ∷ P:⥂ ∷ D:⌬⟭
```

## The Shell

For interactive exploration, use the shell:

```bash
a2x shell

# Type a Σ∞ program directly:
> ⟦Σ∞⟧⟬I:✦ ∷ C:⟨world⟩ ∷ P:⥂ ∷ D:⌬⟭

# Special commands:
> :parse ⟦Σ∞⟧⟬I:✣ ∷ C:⟨a⟩⟨b⟩ ∷ P:⥂ ∷ D:⌬⟭
> :agents
> :help
> :exit
```

## Next Steps

- [Σ∞ Language Reference](sigma.md) — all 40+ operators
- [CCS VM](ccs.md) — how programs execute
- [Ω Compiler](omega.md) — how Σ∞ becomes tensors
