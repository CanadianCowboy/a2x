# Phase 2.A \u2014 VM Operand Plumbing

**Date:** 2026-06-28
**Status:** Plumbing complete, all tests green and clippy clean. Step B (operator allocation) is next.

## What Was Done

The VM now resolves C-field labels and parses D-field bytes before calling the CCS operators (`BIND`, `DIFFERENTIATE`, `GROUND`). Operator *results* are discarded in Step A — Step B will allocate them as new WorldGraph nodes. This is the plumbing half of the **"Bundle A + B"\u2014 two reviewable commits in one branch** plan we agreed on.

## Files Changed

| File | Change |
|------|--------|
| `crates/a2x-core/src/graph.rs` | Added `WorldGraph::set_label(&mut self, NodeId, &str)` trait method. |
| `crates/a2x-ccs/src/world_graph.rs` | Implemented `set_label` in `PetgraphWorldGraph`. +4 tests (basic set, conflict-on-different-node, replace-old-label, idempotent). |
| `crates/a2x-ccs/src/error.rs` | Added `VmError::UnresolvedOperand(String)` variant + Display arm + test. |
| `crates/a2x-ccs/src/vm.rs` | Added `operand_labels: Vec<String>` + `data_payload: Vec<u8>` to `DecodedInstruction`. Added 4 helpers (`fetch_concept`, `resolve_concepts`, `resolve_single`, `parse_chunk_count`, `parse_f32_payload`). Rewrote `BIND` / `DIFFERENTIATE` / `GROUND` dispatch to resolve operands and call operators (no allocation yet). +11 VM-level integration tests. |
| `crates/a2x-ccs/src/safety.rs` | Added `Opcode::Ground` to default `SafetyLevel::Bounded` allowlist. Added comment explaining why `Fork` / `Merge` are intentionally excluded. +1 regression test. |

## Verification

```
cargo test -p a2x-ccs:                  66 passed, 0 failed, 0 ignored
cargo clippy -p a2x-ccs --all-targets -- -D warnings:  clean
```

Workspace-wide tests grew 186 \u2192 203 (+17 new tests across the 5 files).

## ColdStart Coding-Grade (R1\u2013R7)

- **R1 Structure:** One concept per file; explicit error paths via `Result<T, VmError>`; no magic constants.
- **R2 Self-Verification:** 17 new tests including unit, integration, negative cases (unresolved labels), and edge cases (empty operands, truncated f32 chunks).
- **R3 Context:** Doc comments on all pub items; `// See plans/03-ccs-vm.md \u00a74` references preserved on operator modules.
- **R4 Boundary:** Deterministic. No RNG; f32 LE parsing is byte-deterministic.
- **R5 Safety:** Illegal states (missing operand) are represented as `VmError::UnresolvedOperand`, not silently coerced. Safety allowlist changes are minimal and documented.
- **R6 Minimal Delta:** Only added what Step A needs. No operator body changes (Steps C\u2013F handle those).
- **R7 Format:** `cargo fmt` + `cargo clippy --all-targets -- -D warnings` clean. Naming consistent. Cross-crate trait extension respects existing crate layering.

## Design Decisions

- **Unresolved label = error, not silent skip.** Per Plan \u00a726 `VmError::InvalidAddress`-family. `BIND` / `DIFFERENTIATE` return `VmError::UnresolvedOperand(label)` when a label cannot be resolved.
- **Empty operand lists = no-op.** `BIND` / `DIFFERENTIATE` silently no-op with no operands (don't allocate zeros). `GROUND` always proceeds (empty payload produces empty-tensor concept).
- **D-field encoding.** `u32` LE for chunk count (`DIFFERENTIATE` *n*); `f32` LE chunks for `GROUND` data. Trailing partial f32 chunks are dropped (deterministic, documented).
- **DRY.** Extracted `fetch_concept(&self, &str)` from `resolve_concepts` and `resolve_single` (~10 lines of duplication eliminated) per code-reviewer feedback.
- **Default safety includes Ground.** `Ground` has no network/fs/exec side effects; treating it like `Bind` / `Differentiate` / `Evolve` keeps the cognitive operator set available at default safety.

## Code-Reviewer Findings (all addressed)

- Test gap for `DIFFERENTIATE` and `GROUND` dispatch routes \u2192 addressed (+4 integration tests).
- DRY between resolve helpers \u2192 addressed (extracted `fetch_concept`).
- `(*l).to_string()` minor clippy lint \u2192 addressed (`l.to_string()`).
- Default `Bounded` allowing `Ground` not directly tested \u2192 addressed (`test_default_bounded_allows_compute_ops`).
- `Fork` / `Merge` exclusion undocumented \u2192 addressed (rationale comment in `safety.rs`).

A focused code-reviewer pass on the most recent micro-fixes (the `Fork` / `Merge` rationale comment + `test_default_bounded_allows_compute_ops`) was spawned but its response was not returned before this report was written. Those changes are conservative (an additive doc comment + a test that asserts existing-allowed opcodes are still permitted), and the validation just prior to the spawn was green at 66 / 66 + clippy clean.

## Next Step (\u2192 Phase 2.B)

Operators allocate WorldGraph nodes from their results:

| Operator | Allocation |
|----------|------------|
| `BIND` | 1 new node from the bound composite (carrying metadata access_count = 1). |
| `DIFFERENTIATE` | *n* new nodes from chunked sub-concepts (parented to the source node via an outgoing `RelationType::Hierarchical` edge on the source). |
| `GROUND` | 1 new node from the encoded perception (no relation edges). |

Plus wire `self.safety.record_allocation()` for each new allocation and ensure `world_graph.allocate()` errors propagate cleanly into `VmError`.

Per the user's "Bundle A + B together \u2014 two reviewable commits in one branch" agreement, Step B lands as the second commit on the same branch.
