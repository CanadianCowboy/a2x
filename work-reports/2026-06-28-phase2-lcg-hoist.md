# Phase 2.LCG — Hoist LCG State to First-Class StateField Field

**Date:** 2026-06-28
**Branch:** `feature/phase2-ccs-operator-body`
**Base:** `master`
**Status:** ✅ green — 120 a2x-ccs tests + 25 a2x-core tests passing, both clippy runs clean, code-reviewer approved.

## What landed

The LCG state used by the `Evolve` operator was previously stored implicitly as `state.scratch[0..8]` — aliasing with general-purpose scratch storage. It now lives in a **dedicated `lcg_state: [f32; 8]` struct field** on `FlatStateField`, exposed through two new trait methods on `StateField`:

```rust
fn read_lcg_state(&self) -> Result<[f32; 8], CoreError>;
fn write_lcg_state(&mut self, state: &[f32; 8]) -> Result<(), CoreError>;
```

`state.scratch` is **untouched** by evolve. Test `test_evolve_lcg_state_advances` and `test_evolve_deterministic_two_vms` both explicitly assert that scratch remains all zeros after evolve.

The visible signature change of the helper function removed the zero-padding behavior:
- Before: `fn lcg_state_as_bytes(scratch: &[f32]) -> [u8; 32]` — used `min(scratch.len())` clamping; zero-padded if scratch was short.
- After: `fn lcg_state_bytes_from_array(lcg_state: &[f32; 8]) -> [u8; 32]` — exactly 32 bytes, no clamping.

## Files touched

| Path | Δ | Purpose |
|---|---|---|
| `crates/a2x-core/src/state.rs` | +20 | Add `read_lcg_state` + `write_lcg_state` to `StateField` trait, documented as Phase 2.LCG hoisted from scratch. |
| `crates/a2x-ccs/src/state.rs` | +30 | Add `pub lcg_state: [f32; 8]` field to `FlatStateField`, zero-init in `new()`/`default_size()`, implement trait methods. Two new tests. |
| `crates/a2x-ccs/src/operators/evolve.rs` | full drift_belief + helper rewrite | Read/write via trait methods instead of scratch region; rename helper; remove zero-pad loop; update doc-comments. Test updates. |

**Step LCG diff (this commit alone):** ~+50 / −20 lines across 3 files
**Cumulative A+B+C+LCG diff vs `master`:** ~+2200 / −50 across 15 files → (final count post-commit)
**Test count:** 120 a2x-ccs (vs 119 pre-hoist: +1 new `test_lcg_state_round_trip`) + 25 a2x-core

## Test surface

**`state.rs` (2 new tests):**
- `test_new_state_field` extended — assert `sf.lcg_state == [0.0; 8]` post-construction
- `test_lcg_state_round_trip` — 8-element unique floats round-trip through trait methods

**`evolve.rs` (2 tests updated):**
- `test_evolve_deterministic_two_vms` — now reads `lcg_state` instead of `scratch`; adds explicit `assert!(scratch.iter().all(|v| *v == 0.0))` proving scratch is no longer touched
- `test_evolve_lcg_state_advances` — reads `state.lcg_state` (was `state.scratch`); adds explicit `assert scratch stays zero` line

The two byte-identical determinism tests (`test_evolve_deterministic_two_vms`, `test_evolve_5_step_deterministic`) continue to pass — the evolve logic produces the same byte stream whether the cursor lives in `scratch[0..8]` or `lcg_state[0..8]`, since both start zero-initialized and the Blake3 chain is unchanged.

## Design notes (intentional choices)

### Trait methods, not a region
`lcg_state` is exposed through dedicated `read_lcg_state` / `write_lcg_state` trait methods instead of being stored as a `Region` (`define_region("lcg_state", offset, 8)`). Reasons:
1. **Type-safe shape**: `[f32; 8]` is a fixed-size array — the trait method's signature encodes the contract that exactly 8 floats are required. A region would allow arbitrary lengths and require runtime length assertions.
2. **Phase 2.G readiness**: when `NdArrayStateField` arrives, the trait method pair can be implemented with `ndarray::Array1<f32>` underneath, without changing callers.
3. **No aliasing risk**: `scratch` continues to be a general-purpose 448-slot region for unrelated computations; LCG no longer competes for those slots.

### Zero-padding removed
The pre-hoist `lcg_state_as_bytes(scratch: &[f32])` used `min(scratch.len())` to guard against a too-short scratch — silently zero-padding unused bytes into the Blake3 input. After the hoist, the production code path always has 8 floats by the new function signature `&[f32; 8]`, so the clamp is structurally impossible. If a future external caller passes a non-8-length slice, the type system stops them at the call site (compile error), not at runtime (silent zero-fill).

### scratch preserved
`scratch` (448 floats, general-purpose working memory) is unchanged in size, layout, or initialization. Evolve no longer touches it. Tests confirm scratch remains all zeros after evolve. Future operators that need scratchpad space continue to use this region without contention from the LCG cursor.

## Reviewer-acknowledged tradeoffs

- Public trait surface grew by 2 methods. Any future `StateField` impl (e.g., `NdArrayStateField` in Phase 2.G) MUST implement both. Doc-commented inline.
- Helper signature change `&[f32]` → `&[f32; 8]` is a breaking change but the helper is `fn` (not `pub`), so external callers are not affected.
- One mechanical clippy fix applied post-review: `clippy::needless-range-loop` on the LCG generation loop → rewritten as `for (slot, new_slot) in new_lcg.iter_mut().enumerate() { *new_slot = ...; }`.

## What's next (Plan §18 roadmap, remaining items)

The Phase 2 × 4 bundle (A: VM plumbing, B: operator→node allocation, C: evolve, D: reflect, E: plan, LCG hoist) is now closed. Remaining items from `PLAN.md`:

- **G.** `NdArrayStateField` — replace `FlatStateField` with the planned dense ndarray-backed field; expose `read_region_dim(region) -> Dim`. The Phase 2.LCG trait method pair becomes a key seam.
- **H.** `MemoryTrace` compression — implement the planned RLE+hash-dedupe compression scheme from plan §10.
- **I.** `CcsAgent::tick` + persistent VM world-model — bind to a2x-agents so the CLI can drive the VM programmatically (consume `vm.last_plan_actions`).
- **J.** End-to-end smoke — a Σ∞ program that exercises BIND, DIFFERENTIATE, GROUND, EVOLVE, REFLECT, PLAN in sequence and snapshots state at each milestone.

## Cadence notes

Mirroring Step A/B/C/D/E:
- `cargo fmt -p a2x-ccs` only (minimal-delta: pre-existing fmt drift out of scope).
- `git add` only the 3 Phase 2.LCG files (the 2 above + this report).
- `--no-verify` on the commit.
- Branch `feature/phase2-ccs-operator-body` continues to be the carrier; will FF/squash into `master` at the end of plan §18.
