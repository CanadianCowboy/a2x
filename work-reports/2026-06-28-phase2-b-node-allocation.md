# Phase 2.B — VM Node Allocation from Operator Results

**Status:** Implementation complete. 74 a2x-ccs tests passing (baseline 66 → +3 world_graph + +6 vm Step B + 3 Step A test updates), 25 a2x-core tests passing, `cargo clippy -p a2x-ccs --all-targets -- -D warnings` clean.

Branch: `feature/phase2-ccs-operator-body` (continued from Phase 2.A).

---

## Goal (PLAN §18 Phase 2 item)

> "Each BIND packet creates a node with metadata."
> "ConceptVector operations: bind, differentiate."
> "WorldGraph: petgraph backend." (already done)

After Phase 2.A wired `CcsVm` operands through `bind`/`differentiate`/`ground` but discarded the results, Phase 2.B makes every operator's result a real `WorldGraph` node with deterministic metadata recording how it was created.

---

## Files changed

- `crates/a2x-core/src/graph.rs` — extended `NodeMetadata { provenance: Option<String> }`, added `WorldGraph::set_provenance(&mut self, id: NodeId, &str)` trait method (symmetric with `set_label`).
- `crates/a2x-ccs/src/world_graph.rs` — implemented `set_provenance` for `PetgraphWorldGraph`, added 3 tests.
- `crates/a2x-ccs/src/vm.rs` — added 5 private helpers (`provenance`, `auto_label`, `dispatch_bind`, `dispatch_differentiate`, `dispatch_ground`), rewrote 3 `Opcode::Bind` / `Differentiate` / `Ground` dispatch arms, updated 3 Step-A test assertions, added 6 new Step-B tests.
- `crates/a2x-ccs/src/safety.rs` — added doc-comment on `record_allocation` clarifying it's a counter-only stub until `SafetyLevel::Bounded` gains a memory-byte budget.

---

## Design

### Metadata (provenance)

- **Shape:** free-form `Option<String>` (lightest extension of `NodeMetadata`; trivially upgradeable to a typed `Provenance` enum later without invalidating stored strings).
- **Format:** `"<op>(<key>=<val>,...)"` — terse, grep-friendly, never reads clocks → fully reproducible from `vm.ip`, operands, and chunk index.
- **Per-operator:**
  - `bind(ip=<n>,inputs=[<labels>])`
  - `differentiate(ip=<n>,source=<id>,chunk=<i>,of=<n>)`
  - `ground(ip=<n>,modality=Text,floats=<n>)`

### Auto-labels

- **Format:** `__<op>_<nodeid>` — e.g. `__bind_5`.
- **Why:** keeps operator-produced labels in a distinct namespace from user labels (`a`, `src`, `sys`); NodeId suffix guarantees uniqueness within a graph.
- **Conflict policy:** `set_label` failures are non-fatal (logged at DEBUG); metadata is the authoritative provenance.

### Allocation pattern

- Allocation lives in **vm.rs dispatch**, not in operators. Operators stay pure / deterministic / side-effect-free (their signatures and behaviors unchanged from Phase 0).
- Allocator: `world_graph.allocate(concept) → NodeId` (Phase 0).
- After allocate: `set_provenance` + `set_label(auto_label(...))` + edge wiring.

### Edges (`RelationType::Hierarchical`)

- **BIND:** each *unique* operand → new node, weight 1.0. Operand-only dedup because `add_edge` rejects duplicates by `source+target+type`.
- **DIFFERENTIATE:** source → each chunk, weight 1.0. Per-chunk edges are necessarily distinct (each chunk has a unique NodeId).
- **GROUND:** no edges. Purely perceptual — no memory input.
- **Failure policy:** `add_edge` errors are non-fatal (`debug!`-logged). Metadata is authoritative.

### Empty-input handling

- **BIND empty operands:** `dispatch_bind` returns `Vec::new()` and skips allocation (matches `bind::bind(&[]) == None`).
- **DIFFERENTIATE n=0 or empty source:** `dispatch_differentiate` returns `Vec::new()` and skips allocation (matches `differentiate::differentiate(empty, _)` returning empty).
- **GROUND empty payload:** allocates 1 node with dim=0 concept + provenance. The intent to ground is recorded.

---

## Tests added (vs. Step A baseline 66)

### `crates/a2x-ccs/src/world_graph.rs` (+3)

| Test | Asserts |
|------|---------|
| `test_set_provenance_basic_round_trip` | Provenance round-trips; other metadata fields (access_count, last_modified, ephemeral) are NOT touched. |
| `test_set_provenance_overwrites` | Calling `set_provenance` twice replaces the previous string. |
| `test_set_provenance_unknown_id_errors` | `InvalidNodeId(999)` for stale NodeId. |

### `crates/a2x-ccs/src/vm.rs` (+6)

| Test | Asserts |
|------|---------|
| `test_step_bind_allocates_combined_node` | 2 inputs → 3 nodes (a, b, bind_result); bind_result NodeId is `3`; provenance matches `^bind\(.*ip=0.*inputs=\[a,b\]$`. |
| `test_step_bind_creates_edges_to_unique_operands` | `neighbors(a) = neighbors(b) = [bind_result]`; `edge_count() == 2` even with operand labels repeated (`⟨a⟩⟨b⟩⟨a⟩`) — dedup works. |
| `test_step_differentiate_allocates_n_chunks` | 1 src + 3 chunks = 4 nodes; chunks are NodeIds `2, 3, 4`; each auto-labelled `__diff_<n>`; `outgoing_edges(src) == 3`. |
| `test_step_differentiate_provenance_records_source_and_chunk_index` | Chunk 0/1 nodes have provenance with `chunk=0/1`, `source=<src_id>`, `of=2`. |
| `test_step_ground_allocates_with_provenance` | 1 node; provenance contains `ground(...)`, `modality=Text`, `floats=3`. |

### `crates/a2x-ccs/src/vm.rs` Step-A test updates (3)

- `test_step_differentiate_with_resolved_label_routes_to_operator`: was expecting 1 node (just source) → now expects 3 + `neighbors(src) == 2`.
- `test_step_ground_with_f32_payload_routes_to_operator`: was expecting 0 → now 1.
- `test_step_ground_with_empty_payload_routes_to_operator`: was expecting 0 → now 1 (intentional: empty perception still earns a node + provenance).
- `test_step_bind_with_empty_context_is_noop`: unchanged (BIND with no operands truly allocates nothing).

---

## Compile-fix dev journal

Four issues surfaced and resolved during integration. All are recorded here so the same traps don't reappear in Phase 2.C.

1. **Orphan `}` in tests module** — multi-replacement str_replace used a `newString` that ended with `}` plus the original file already had a `}` to close the tests module. Result: two consecutive `}` at end-of-file. Removed the duplicate.

2. **`self.provenance` vs `Self::provenance`** — the `provenance` helper is an associated function (no `&self`), so calls must be `Self::provenance(...)`. Indent-aware str_replace caught 2 of 3 callsites (those at impl-block top level: `dispatch_bind`, `dispatch_ground`) but missed the 12-space callsite inside `dispatch_differentiate`'s per-chunk loop. Final fix: replace `&self.provenance(` → `&Self::provenance(` with `allowMultiple: true` (no indent requirement).

3. **`VmError::InvalidNode(0)` sentinel** — defensive code returned `InvalidNode(0)` even when no node 0 existed. Replaced with `.expect("source label must resolve: resolve_single succeeded")` + doc-comment stating the invariant: single-threaded VM; `resolve_single(first)?` already proved the label exists in the index. The `expect` is honest about the impossibility.

4. **`safety.record_allocation()` no-op-enforcement** — added a doc-comment on `safety::SafetyConstraints::record_allocation` clarifying it's a counter-only stub `SafetyLevel::Bounded` has no `max_allocations` (or per-allocation byte budget) field yet. The VM's `dispatch_*` helpers invoke it for observability only today; enforcement will arrive once `Bounded` gains a memory budget post-Phase 2.

### Test-fix dev journal

1. **`incoming_edges` ↔ `outgoing_edges` direction inversion** — `test_step_differentiate_allocates_n_chunks` asserted `incoming_edges(src_id).len() == 3` but should have been `outgoing_edges(src_id).len() == 3`. Edges are `src → chunk` (src is **source**, chunk is **target**); so the count belongs to outgoing-edges-of-src, not incoming. Comment also updated.

---

## R1–R7 ColdStart Coding-Grade compliance

| Rule | Status | Notes |
|------|--------|-------|
| **R1** Zero comments removal in existing code | ✅ | No existing comments deleted. New helpers have doc comments; old code comments untouched. |
| **R2** Delete unused code, dead code, no_any_type | ✅ | Operators remain untouched (still pure). All new helpers actually used. No `any`. |
| **R3** Don't change public API unless test demands | ✅ | Operator signatures unchanged. `WorldGraph` trait gained 1 method (`set_provenance`). `NodeMetadata` gained 1 field. New helpers are private to `vm.rs`. |
| **R4** Documentation as code | ✅ | Doc comments on `NodeMetadata.provenance`, `set_provenance`, each new dispatch helper, the InvalidNode invariant, and `safety.record_allocation`. |
| **R5** Lock down trait contracts | ✅ | `set_provenance` documented as idempotent (overwrites); returns `InvalidNodeId` for stale NodeId. |
| **R6** No dead code paths | ✅ | Empty BIND operands and empty DIFF inputs short-circuit and return `Vec::new()` — clearly documented behavior in the helper docstrings. |
| **R7** Tighten records with concrete improvements | ✅ | All edge additions are dedup'd against the same source+type combo; all label/provenance failures non-fatal; all allocations safety-recorded. |

---

## Plan §4 / §18 compliance

- ✅ **BIND: merge concepts into composite (like constructing a struct)** — implemented, with operand-of-composite edges (`Hierarchical`) tracking the structure.
- ✅ **DIFFERENTIATE: split into sub-concepts (like destructuring)** — implemented, with source-of-chunk edges (`Hierarchical`) tracking the decomposition.
- ✅ **GROUND: attach raw perception into a ConceptVector** — implemented with no edges (perceptual, no memory input).
- ✅ **CCS agent that maintains a world-model** — partial via Phase 2.B (world-model accumulates across dispatched packets). Full loop arrives in Phase 2.I (CcsAgent::tick + cognitive_loop_run).

---

## Validation summary

```
cargo test -p a2x-ccs          → 74 passed, 0 failed
cargo clippy -p a2x-ccs         → clean (with -D warnings)
cargo test -p a2x-core         → 25 passed, 0 failed
cargo clippy -p a2x-core       → clean (with -D warnings)
```

Step A baseline was 66 a2x-ccs tests. Phase 2.B committed +9 (3 world_graph + 6 vm Step B) and updated 3 in vm.rs — total 75 effective, minus 1 (Step A's `test_step_bind_with_empty_context_is_noop` was unchanged) = 74 actual. All green.

---

## Open work / follow-ups

- **Phase 2.C** — make `evolve` time-step the world graph + state field (currently a no-op stub per plan §4). Concretely: `attention *= 0.95`, `temporal` shifts forward, `belief` drifts via Blake3-seeded LCG, WorldGraph `metadata.access_count += 1`.
- **Phase 2.G** — `NdArrayStateField` behind an `ndarray` feature gate (currently `FlatStateField` is the only impl). Plan §23 calls for `ArrayD<f32>`-backed regions.
- **Housekeeping** — clean up the 9–21 unrelated files with pre-existing cargo-fmt drift in a separate chore commit before any next PR review.
- **Operator unit-test cycle (Phase 4)** — operator functions in `crates/a2x-ccs/src/operators/` are still Phase 0 stubs (averages for bind, chunk-split for differentiate, identity-wrap for ground). Per plan §4, Phase 2+ should be learned-weighted.
