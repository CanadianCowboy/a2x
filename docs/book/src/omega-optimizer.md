# Ω Optimizer Passes

The optimizer transforms the intermediate representation to produce efficient
Ω programs. Four passes run in sequence.

## Pass 1: Constant Folding

Folds operations with all-immediate operands into no-ops:

- BIND with two zero-vectors → NOP
- GROUND with zero data → NOP
- Any instruction where all operands are known at compile time

```
Before: BIND(⟨x⟩, ⟨y⟩) with x = y = [0,0,0,...]
After:  NOP
```

## Pass 2: Dead Code Elimination

Removes instructions that have no effect:

- Orphaned nodes (no control flow references)
- Unreachable code after HALT
- Instructions whose outputs are never read

```
Before:
  GROUND ⟨x⟩
  BIND ⟨x⟩⟨y⟩ → ⟨z⟩
  NOP
  HALT

After:
  GROUND ⟨x⟩
  BIND ⟨x⟩⟨y⟩ → ⟨z⟩
  HALT
```

## Pass 3: Instruction Fusion

Merges adjacent instructions with matching labels:

- BIND + DIFFERENTIATE on same operands → FUSED_BIND_DIFF
- Two consecutive GROUNDs → single GROUND with combined payload

```
Before:
  BIND ⟨a⟩⟨b⟩ → ⟨c⟩
  DIFF ⟨c⟩⟨d⟩ → ⟨e⟩

After:
  FUSED ⟨a⟩⟨b⟩⟨c⟩⟨d⟩ → ⟨e⟩
```

## Pass 4: Layout Optimization

Reorders the instruction sequence for cache-friendly access:

- Sorts instructions by source operands (locality)
- Groups all GROUND operations together
- Places HALT at the end

## Running the Optimizer

```rust
use a2x_omega::passes;

let mut ir = generate_ir(&program)?;
passes::constant_folding::run(&mut ir)?;
passes::dead_code::run(&mut ir)?;
passes::fusion::run(&mut ir)?;
passes::layout::run(&mut ir)?;
```
