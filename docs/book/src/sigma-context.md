# Context Operators

The **C-field** specifies what concepts or regions to operate on.

## Context Scoping

| Symbol | Name | Description |
|--------|------|-------------|
| ⟘ | **Null** | No context (empty scope) |
| ⧖ | **Universal** | All concepts in the WorldGraph |
| ⟑ | **Compression** | Compressed/quantized view |
| ⩕ | **Uncertainty** | Probabilistic context |

## Labels

Concepts are referenced by labels enclosed in `⟨⟩`:

```text
C:⟨sys⟩          — single concept
C:⟨a⟩⟨b⟩⟨c⟩      — multiple concepts
C:⟘              — no context
C:⧖              — all concepts
```

## Relation Chains

| Symbol | Name | Description |
|--------|------|-------------|
| → | **CausalChain** | Follow cause-effect links |
| ↔ | **SpatialChain** | Follow spatial relations |
| ↻ | **TemporalChain** | Follow time-ordered links |
| ⊨ | **LogicalChain** | Follow logical entailments |
| ⊂ | **Hierarchical** | Follow parent-child relations |

### Chained Context Example

```text
C:⟨sys⟩→          — concepts causally linked from sys
C:⟨agent⟩↔        — concepts spatially related to agent
C:⟨root⟩⊂         — children of root in hierarchy
```

## Context Resolution

When the VM resolves context, it follows this order:

1. **Direct labels** — `C:⟨sys⟩` looks up the node labeled `sys`
2. **Chain operators** — `C:⟨sys⟩→` traverses causal edges from `sys`
3. **Special scopes** — `C:⧖` (universal) or `C:⟘` (null)
