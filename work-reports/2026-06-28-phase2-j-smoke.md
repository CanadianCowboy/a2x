# Phase 2.J — End-to-end Σ∞ smoke test

**Date:** 2026-06-28
**Branch:** `feature/phase2-ccs-operator-body`
**Status:** ✅ green — all 6 smoke tests pass; clippy clean across both default + ndarray.

## What landed

Cross-feature integration test at `crates/a2x-ccs/tests/phase2_smoke.rs` that exercises every cognitive operator in a single Σ∞ program.

The 7-instruction program runs on a fresh `CcsVm` with 2 pre-allocated seeds (`sys`, `out`) labeled in the WorldGraph:

| # | Op | Rationale |
|---|----|-----------|
| 0 | GRND seed_sys (4 f32 payload) | ground a perception |
| 1 | GRND seed_out (3 f32 payload) | ground a second perception |
| 2 | DIF sys into 2 chunks | split the source concept |
| 3 | BND sys + out | composite concat |
| 4 | EVOL (no operands) | time-step fields (drift belief) |
| 5 | REFLECT (no operands) | build self-model node, set `last_reflect` |
| 6 | PLAN (no operands) | read last_reflect, emit plan actions |

6 cross-feature tests guard the pipeline:

1. **`test_phase2_full_cognitive_loop_runs_to_completion`** — `vm.run()` returns Halted; world-graph node count > 10 (loose lower bound; exact arithmetic drifts across reflect.rs edits because `min(window, trace.len())` shadows).
2. **`test_phase2_full_loop_traces_every_step`** — MemoryTrace length == 7.
3. **`test_phase2_reflect_sets_last_reflect_for_plan_consumption`** — REFLECT populates `vm.last_reflect` AND plan_actions is non-empty (cross-operator contract).
4. **`test_phase2_actions_have_non_negative_priority`** — All emitted actions have finite priority.
5. **`test_phase2_evolve_before_reflect_drifts_belief`** — Belief differs post-EVOLVE; self-model node exists with 128-dim layout.
6. **`test_phase2_full_loop_deterministic_across_two_vms`** — Two fresh VMs running same program produce identical world-graph node count, trace length, plan actions count, byte-equal self-model concept data.

## Files touched

| Path | Δ | Purpose |
|---|---|----|
| `crates/a2x-ccs/tests/phase2_smoke.rs` | +180 (new) | 6 integration tests + helpers |

## Design notes

- **Trait imports needed at top of test:**
  - `a2x_core::graph::WorldGraph` — activates `allocate`, `set_label`, `lookup`, `node_count`
  - `a2x_core::memory::MemoryTrace` — activates `trace.len()`
  - `a2x_core::state::StateField` — activates `state_field.read_region()`
- **Determinism invariant:** smoke test asserts byte-equal self-model concepts across two fresh VMs — proves no wall-clock reads leak into the reflect computation.
