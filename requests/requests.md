# A2X Integration Requests — Soong Path AGI

> These are requested changes/additions to the A2X (ailang) crates to better support
> integration with the Soong Path AGI system (`soong-path/`). All crates are used as
> **path dependencies** from `D:/projects/ailang/crates/`.

## Status

| Request | Crate | Priority | Status |
|---------|-------|----------|--------|
| [R-001](#r-001-edition-2024-compatibility) | workspace | High | Pending |
| [R-002](#r-002-serde-default-enabled) | a2x-core, a2x-ccs | High | Pending |
| [R-003](#r-003-trait-object-safety) | a2x-core | Medium | Pending |
| [R-004](#r-004-async-vm-tokio-default) | a2x-ccs | Medium | Pending |
| [R-005](#r-005-bus-channel-api) | a2x-bus | Medium | Pending |
| [R-006](#r-006-worldgraph-adapter) | a2x-core | Low | Pending |
| [R-007](#r-007-dependency-version-alignment) | workspace | Low | Pending |

---

## R-001: Edition 2024 Compatibility

**Crate:** Workspace (all crates)
**Priority:** High

### Problem
ailang uses Rust edition **2021** while soong-path uses **2024**. While Cargo handles mixed
editions in path deps, certain patterns differ (RPIT lifetime capture, `gen` keyword, etc.).

### Request
- Upgrade workspace edition to `2024`
- Bump MSRV to `1.85.0`
- Audit public APIs for RPIT lifetime capture changes

---

## R-002: Serde Default Enabled

**Crate:** `a2x-core`, `a2x-ccs`
**Priority:** High

### Problem
ailang gates serde behind optional features. soong-path needs serialization of A2X types
(ConceptVector, GraphNode, MemoryEntry) but the feature is off by default.

### Request
- Add `serde` to `default` features for `a2x-core` and `a2x-ccs`
- OR: Document that consumers must enable `features = ["serde"]`

---

## R-003: Trait Object Safety

**Crate:** `a2x-core`
**Priority:** Medium

### Problem
WorldGraph, StateField, MemoryTrace traits may not be object-safe. soong-path uses
`Arc<RwLock<dyn Trait>>` patterns extensively.

### Request
- Ensure traits are object-safe (move `Self: Sized` bounds to individual methods)
- OR: Provide `DynWorldGraph` / adapter wrapper types

---

## R-004: Async VM Tokio Default

**Crate:** `a2x-ccs`
**Priority:** Medium

### Problem
The `tokio` feature (enabling `run_async`, `ProgramScheduler`, `ParallelSwarm`) is behind
a feature gate. soong-path needs the async VM for its tokio-based daemon.

### Request
- Add `tokio` to `default` features for `a2x-ccs`

---

## R-005: Bus Channel API

**Crate:** `a2x-bus`
**Priority:** Medium

### Problem
soong-path uses `async-channel = "2"` for its EventStream. a2x-bus has its own
`InMemoryAsyncBus`. These should share channel infrastructure.

### Request
- Expose channel creation from `a2x-bus` so soong can create bus instances on its own channels
- OR: Provide a `From` adapter between `async_channel::Sender` and bus internals

---

## R-006: WorldGraph Adapter

**Crate:** `a2x-core`
**Priority:** Low

### Problem
soong-world has its own graph-like state (Predictor, PatternDetector, ObjectTracker).
The A2X WorldGraph uses ConceptVector (Vec<f32>) as node values, which doesn't map
cleanly to soong-world's TrackedObject/Pattern types.

### Request
- Add a generic node value type to WorldGraph
- OR: Add a WorldGraphExt trait for custom implementations

---

## R-007: Dependency Version Alignment

**Crate:** workspace
**Priority:** Low

### Problem
Both workspaces pin different versions of shared crates:
- ailang: `axum = "0.7"`, `tower-http = "0.5"`, `reqwest = "0.12"`
- soong: `axum = "0.8"`, `tower-http = "0.6"`

This creates duplicate deps at link time.

### Request
- Align axum/tower-http/reqwest versions where possible
- Document known version splits

---

## Integration Map

| soong crate | ailang crates | Purpose |
|-------------|---------------|----------|
| soong-core | a2x-core | Foundational types |
| soong-cognition | a2x-ccs, a2x-sigma, a2x-omega | Cognitive substrate VM |
| soong-world | a2x-core | WorldGraph predictive modeling |
| soong-perception | a2x-core, a2x-sigma | Perception + symbolic encoding |
| soong-platform | a2x-core | Platform abstractions |
| soong-daemon | a2x-bus, a2x-agents, a2x-gateway, a2x-client, a2x-probe | Full integration hub |
| soong-shell | a2x-cli, a2x-core | Interactive shell |
| soong-gui | a2x-gateway, a2x-core | Entity connections |
| soong-service | a2x-bus, a2x-core | Service messaging |
| soong-init | a2x-startup, a2x-core | System initialization |
