# WorldGraph & Concepts

The WorldGraph is A2X's knowledge representation system — a directed graph where
**nodes are concepts** and **edges are relations** between them.

## Concept Vectors

Each concept is represented by a `ConceptVector` — an f32 array that captures
the concept's meaning in a high-dimensional latent space.

```rust
use a2x_core::concept::ConceptVector;

// A concept vector is a fixed-size f32 array
let concept = ConceptVector::default(); // all zeros initially
```

Operations on concept vectors:

- **Average** (BIND) — merge two concepts by averaging their vectors
- **Difference** (DIFFERENTIATE) — split by computing directional differences
- **Ground** (GROUND) — assign external data to a concept

## Relation Types

Edges in the WorldGraph have typed relations:

| Relation | Symbol | Description |
|----------|--------|-------------|
| `CausalChain` | → | A causes B |
| `SpatialChain` | ↔ | A is near B |
| `TemporalChain` | ↻ | A precedes B in time |
| `LogicalChain` | ⊨ | A entails B |
| `Hierarchical` | ⊂ | A is a child of B |

## WorldGraph API

```rust
use a2x_core::graph::WorldGraph;
use a2x_core::relation::{RelationEdge, RelationType};

// Allocate a new concept node
let node_id = graph.allocate(ConceptVector::default());

// Label it for lookups
graph.set_label(node_id, "my_concept")?;

// Create a relation
let edge = RelationEdge::new(
    source_id,
    target_id,
    RelationType::CausalChain,
);
graph.add_edge(edge)?;

// Query by label
let found = graph.lookup_by_label("my_concept")?;

// Traverse neighbors
let neighbors = graph.neighbors_of(node_id, 1)?;

// Query by relation type
let causal = graph.by_relation(RelationType::CausalChain)?;
```

## Bootstrap WorldGraph

On startup, the gateway seeds the WorldGraph with foundational concepts:

- **System concepts:** `sys`, `orch`, `bus`, `agent`, `concept`, `relation`
- **Operation concepts:** `ground`, `bind`, `differentiate`, `evolve`, `reflect`
- **Meta concept:** `self`

These 12 nodes are connected by 10 hierarchical edges, forming the initial
knowledge graph that agents build upon.

## Performance

- Backed by `petgraph` for efficient graph operations
- O(1) label lookups via HashMap
- O(degree) neighbor traversal
- Configurable adjacency storage (dense or sparse)
