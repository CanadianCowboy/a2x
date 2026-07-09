# Plan Operators

The **P-field** controls execution flow.

## Control Flow

| Symbol | Name | Description |
|--------|------|-------------|
| ⥂ | **Sequential** | Execute next instruction in order |
| ⤐ | **Branch** | Conditional jump to a labeled target |
| ⤈ | **Descend** | Enter a sub-program |
| ⤉ | **Ascend** | Return from a sub-program |
| ⥁ | **Swarm** | Parallel execution across multiple agents |
| ⤑ | **Merge** | Join parallel branches |
| ⤒ | **Escalate** | Raise execution to the orchestrator |
| ⤓ | **Recursive** | Self-referential call |
| ⩈ | **Fork** | Split into parallel sub-programs |
| ⩫ | **Join** | Merge parallel results |

## Plan Primitives

### Sequential (⥂)
The default — instructions execute one after another:
```text
P:⥂
```

### Branch (⤐)
Conditionally jump to another instruction:
```text
P:⤐⟨target_label⟩
```

### Descend/Ascend (⤈ / ⤉)
Enter and exit sub-programs:
```text
⟦Σ∞⟧⟬I:⤈ ∷ C:⟨sub⟩ ∷ P:⥂ ∷ D:⌬⟭
... sub-program instructions ...
⟦Σ∞⟧⟬I:⤉ ∷ C:⟘ ∷ P:⥂ ∷ D:⌬⟭
```

### Swarm (⥁)
Execute across multiple agents simultaneously:
```text
P:⥁⟨agent_group⟩
```

### Escalate (⤒)
Send execution to the orchestrator for coordination:
```text
P:⤒
```

## Plan Resolution

The Omega compiler resolves plan operators during IR generation:

1. Labels are resolved to instruction indices
2. Branch targets are validated
3. Recursive calls are detected (depth limit enforced)
4. Fork/Merge pairs are matched
