# Phase 2.G — NdArrayStateField (ndarray backend, feature-gated)

**Date:** 2026-06-28
**Branch:** `feature/phase2-ccs-operator-body`
**Status:** ✅ green — 130 a2x-ccs tests + 25 a2x-core + 38 a2x-agents passing on default, **+140 tests on ndarray feature** (10 new). All 4 clippy runs clean.

## What landed

PLAN.md §18 item "StateField: high-dimensional tensor with ndarray (behind feature gate)" now has an implementation. The `ndarray` cargo feature already existed (`Cargo.toml`: `ndarray = ["dep:ndarray"]`, dep already optional) but the impl was missing. Phase 2.G closes that gap.

New: `crates/a2x-ccs/src/state_ndarray.rs` — `NdArrayStateField` struct backed by `ndarray::Array1<f32>`, name-compatible with `FlatStateField`:

| Field | Type | Note |
|---|---|---|
| `data` | `Array1<f32>` | backing tensor, contiguous (C-order) |
| `regions` | `HashMap<String, StateRegion>` | region index |
| `lcg_state` | `[f32; 8]` | dedicated Phase 2.LCG-curated field |

Trait surface is identical to FlatStateField — `define_region`, `read_region`, `write_region`, `total_len`, `raw_data`, `read_lcg_state`, `write_lcg_state`. Slice ops route through `as_slice()` / `as_slice_mut()` (no glue Vec intermediate). `init_ndarray_default_regions` is a one-liner delegate to the canonical `crate::state::init_default_regions` (no duplication).

10 new unit tests in `state_ndarray.rs`: zero-init check, lcg_state round-trip, define+read region, write+read region, out-of-bounds, duplicate-name, size-mismatch, not-found, default-regions round-trip, independence of two NdArrayStateField instances.

## Files touched

| Path | Δ | Purpose |
|---|---|---|
| `crates/a2x-ccs/src/state_ndarray.rs` | +250 (new) | `NdArrayStateField` struct + impl + 10 tests |
| `crates/a2x-ccs/src/lib.rs` | +3 | gated `pub mod state_ndarray;` + re-exports |

## Reviewer acknowledgements

- Public API surface: `NdArrayStateField` + `init_ndarray_default_regions` both gated by `#[cfg(feature = "ndarray")]`. Default build unaffected.
- Mirrors FlatStateField semantics exactly — same default 1024-f32 size, same region layout.
- `lcg_state` is a struct field (NOT a region) — independent of any state region by Phase 2.LCG hoist contract.
