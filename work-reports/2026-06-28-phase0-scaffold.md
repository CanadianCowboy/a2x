# Work Report — Phase 0: Workspace Scaffold & a2x-core

**Date:** 2026-06-28  
**Agent:** Buffy (DeepSeek v4 Pro)  
**Branch:** `master`  
**Commits:** `b8fd24e` (a2x-core), TBD (full scaffold + work report + README update)

---

## What Was Done

### 1. Cargo Workspace Root (`Cargo.toml`)
- Created virtual workspace manifest with Rust 2021 edition, MSRV 1.75.0
- Configured `serde`, `petgraph`, `ndarray`, `tokio` as workspace dependencies
- Member crates: all 10 official crates

### 2. `a2x-core` — Full Implementation (Zero-Dependency)
The foundation crate — all other crates depend on it. **25 tests passing.**

**Types implemented (20 source files):**
- `ConceptVector` — dense embedding, the atomic value type. Ops: add, subtract, multiply, scale, dot, cosine_similarity, norm
- `NodeId` — WorldGraph node identifier (u64 newtype)
- `ProgramId` — content-addressed program hash ([u8; 32])
- `AgentId` — agent/entity identifier
- `RelationEdge` — typed directed edge between nodes
- `GraphNode` — WorldGraph node with concept + edges + metadata
- `MemoryEntry` — execution history entry (raw bytes at core layer)
- `StateSnapshot` — agent state for probe/debug
- `ActionDistribution` — probability distribution over actions
- `Packet` — unified transport packet (Raw variant at core layer)
- `NodeMetadata` — node bookkeeping

**Enums:**
- `RelationType` — Causal, Spatial, Temporal, Logical, Hierarchical, Custom
- `Opcode` — 16 VM instructions (Nop..Halt, Custom)
- `AgentType` — Orchestrator, Llm, Cli, Ccs, Omega, Entity, Custom
- `EntityType` — HumanCli, HumanWeb, LlmService, Application, Database, Robot, CiCd, A2xNetwork, Custom
- `Capability` — Execute, FileSystem, Network, Shell, Probe, Custom
- `Modality` — Vision, Audio, Text, Proprioception, Custom
- `AddressingMode` — LabelIndex, DirectNodeId, StateFieldRegion, Immediate
- `ProtocolId` — Sigma, Omega, Raw
- `GraphQuery` — ByLabel, ByRelation, Neighbors, BySimilarity, Custom

**Traits:**
- `WorldGraph` — heap / persistent memory interface
- `StateField` — registers / working memory interface
- `PolicyField` — JIT compiler + optimizer interface
- `MemoryTrace` — execution history interface
- `Agent` — execution context interface

**Error types:**
- `CoreError` — DimensionMismatch, InvalidNodeId, LabelConflict, OutOfMemory, Other
- `AgentError` — NotFound, AtCapacity, ProgramCrash, Timeout, VmError, TransportError, SafetyViolation, Core

**Design decisions:**
- All traits use concrete `CoreError` instead of associated types for object safety
- `StateField::snapshot()` removed for object safety (use `StateSnapshot` struct)
- `Agent::execute()` is synchronous in core (async in `a2x-agents` with tokio)
- `Packet` only has `Raw(Vec<u8>)` (typed variants in higher crates)
- `MemoryEntry` uses raw bytes (`instruction_bytes`, `state_snapshot_bytes`)
- `ProgramId::compute()` omitted (needs blake3 dependency)
- `NodeMetadata` derives `Default` (clippy clean)

### 3. All 9 Remaining Crates — Scaffolded as Stubs
Each with `Cargo.toml` + `lib.rs` (or `main.rs` for CLI) + stub test.

| Crate | Status | Dependencies |
|-------|--------|-------------|
| `a2x-sigma` | stub | a2x-core |
| `a2x-omega` | stub | a2x-core, ndarray (opt) |
| `a2x-bus` | stub | a2x-core, a2x-sigma, tokio (opt) |
| `a2x-ccs` | stub | a2x-core, petgraph, ndarray (opt) |
| `a2x-agents` | stub | a2x-core, a2x-sigma, a2x-bus, a2x-ccs |
| `a2x-gateway` | stub | a2x-core, a2x-sigma, a2x-bus |
| `a2x-client` | stub | a2x-core |
| `a2x-cli` | stub binary | a2x-core, a2x-bus, a2x-agents |
| `a2x-probe` | stub | a2x-core, a2x-ccs |

### 4. README.md — Updated
- Added "Document your work" rule to Working Style
- Updated Project Status checklist
- Added "Work Reports" section with link to this file
- Added a2x-gateway and a2x-client to Official Crates table
- Added work report requirement to "If You Are Starting Fresh" list

---

## Verification

```
cargo build --workspace     ✅ all 10 crates compile
cargo test --workspace      ✅ 33 tests pass (25 core + 8 stubs)
cargo fmt --all             ✅ passes
cargo clippy -p a2x-core    ✅ 0 warnings
```

---

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Updated workspace members to all 10 crates; added ndarray, tokio |
| `crates/a2x-core/Cargo.toml` | Created |
| `crates/a2x-core/src/*.rs` | 20 source files |
| `crates/a2x-sigma/*` | Stub crate |
| `crates/a2x-omega/*` | Stub crate |
| `crates/a2x-bus/*` | Stub crate |
| `crates/a2x-ccs/*` | Stub crate |
| `crates/a2x-agents/*` | Stub crate |
| `crates/a2x-gateway/*` | Stub crate |
| `crates/a2x-client/*` | Stub crate |
| `crates/a2x-cli/*` | Stub binary crate |
| `crates/a2x-probe/*` | Stub crate |
| `README.md` | Added work report rule, updated project status, crate table |
| `work-reports/2026-06-28-phase0-scaffold.md` | This file |

---

## Next Steps (Phase 0 remaining)

- [ ] Implement `a2x-sigma`: tokenizer, parser, all 4 operator tables, SigmaPacket
- [ ] Implement `a2x-omega`: OmegaPacket, compilation stubs
- [ ] Implement `a2x-bus`: in-memory message bus
- [ ] Implement `a2x-ccs`: WorldGraph (petgraph), StateField, MemoryTrace
- [ ] Set up CI/CD (GitHub Actions)
