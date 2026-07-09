# CCS Cognitive Operators

Each operator transforms the WorldGraph and StateField in a specific way.

## 1. Bind — ✣ Synthesis

Merges concepts by averaging their vectors:

```rust
// Bind concept A and B into C
vm.bind(&["a", "b"], "c")?;
```

**Effect:** Allocates a new node `c` whose concept vector is the element-wise
average of `a` and `b`. Creates Hierarchical edges from `a` → `c` and `b` → `c`.

## 2. Differentiate — ⟐ Split

Splits a concept into distinct sub-concepts:

```rust
vm.differentiate(&["mixed_concept"], "sub_a")?;
```

**Effect:** Computes the directional difference between the source concept and
the mean of all concepts, creating a new node that captures the distinction.

## 3. Ground — ✦ Star

Imports external data into the concept space:

```rust
vm.ground(&[], "new_concept")?;
```

**Effect:** Allocates a new node with a fresh concept vector, ready to be
populated with external data. This is how the WorldGraph grows.

## 4. Evolve — ⩂ Delay

Time-steps all concepts forward:

```rust
vm.evolve()?;
```

**Effect:** Updates access counts, applies attention decay to all concepts.
Concepts that haven't been accessed attenuate over time, simulating forgetting.

## 5. Reflect — ✶ Self

Creates a self-model node:

```rust
vm.reflect()?;
```

**Effect:** Allocates a special `self` concept that represents the VM's own
state. Used by agents for introspection.

## 6. Plan — ⤒ Escalate

Generates an action sequence:

```rust
vm.plan(&["goal"], &state)?;
```

**Effect:** Produces a prioritized list of actions based on current StateField
and the goal region. Outputs ExternalCommands.

## 7. Actuate

Produces external commands:

```rust
let commands = vm.actuate(&actions)?;
// Execute commands in the real world
```

**Effect:** Converts planned actions into executable commands (bus messages,
HTTP calls, file operations).
