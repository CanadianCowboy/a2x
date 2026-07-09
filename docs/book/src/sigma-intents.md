# Intent Operators

The **I-field** specifies the cognitive action to perform.

| Symbol | Name | Description |
|--------|------|-------------|
| ✣ | **Synthesis** | Create new concepts via binding (average concept vectors) |
| ✕ | **Cancel** | Destroy or remove concepts |
| ⟐ | **Split** | Differentiate concepts (find distinctions) |
| ✦ | **Star** | Explore and ground new territory (import external data) |
| ⚡ | **Lightning** | Immediate/accelerated execution |
| ⚠ | **Warning** | Safe/guarded execution |
| ⩂ | **Delay** | Slow/evolve mode (time-step concepts forward) |
| ⩈ | **Parallel** | Fork parallel sub-programs |
| ⩫ | **Merge** | Join parallel results |
| ⩎ | **Contradiction** | Halt on contradiction |
| ✶ | **Reflect** | Self-model introspection (allocate reflect node) |

## Usage Examples

### Ground (✦)
Create a new concept from external data:
```text
⟦Σ∞⟧⟬I:✦ ∷ C:⟨new_idea⟩ ∷ P:⥂ ∷ D:⌬⟭
```

### Synthesis (✣)
Combine two concepts into one:
```text
⟦Σ∞⟧⟬I:✣ ∷ C:⟨happy⟩⟨sad⟩ ∷ P:⥂ ∷ D:⌬⟭
```

### Differentiate (⟐)
Split a concept into distinct parts:
```text
⟦Σ∞⟧⟬I:⟐ ∷ C:⟨mixed⟩ ∷ P:⥂ ∷ D:⌬⟭
```

### Fork/Merge (⩈ / ⩫)
Run sub-programs in parallel:
```text
⟦Σ∞⟧⟬I:⩈ ∷ C:⟨task⟩ ∷ P:⥂ ∷ D:⌬⟭
... sub-programs ...
⟦Σ∞⟧⟬I:⩫ ∷ C:⟨task⟩ ∷ P:⥂ ∷ D:⌬⟭
```

## VM Mapping

Each intent operator maps to a CCS VM opcode:

| Intent | VM Opcode |
|--------|-----------|
| ✣ Synthesis | `Bind` |
| ⟐ Split | `Differentiate` |
| ✦ Star | `Ground` |
| ⩂ Delay | `Evolve` |
| ✶ Reflect | `Reflect` |
| ⩈ Parallel | `Fork` |
| ⩫ Merge | `Merge` |
