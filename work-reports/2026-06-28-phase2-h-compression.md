# Phase 2.H — MemoryTrace RLE + Hash-Dedupe Compression

**Date:** 2026-06-28
**Branch:** `master`
**Status:** ✅ green — 144 a2x-ccs tests on default + 144 on ndarray (11 new) + 25 a2x-core + 38 a2x-agents; clippy `-D warnings` clean across both feature combinations; `cargo fmt --check` clean. Reviewer approved ("Ship it otherwise").

## What landed

PLAN §23 MemoryModel called for proper RLE + hash-dedupe compression; the prior
VecMemoryTrace::compress was subsample-only (lossy, no real savings signal).
Phase 2.H replaces it with two real algorithms, preserving the `MemoryTrace`
trait surface.

### Hash-dedupe snapshot pool

A Blake3-keyed dedup table rebuilt at every `compress()` call:
- `snapshot_pool: Vec<Vec<u8>>` — unique `state_snapshot_bytes` payloads.
- `snapshot_lookup: HashMap<[u8; 32], usize>` — content-hash → pool index.

The `MemoryEntry` struct in `a2x-core` (zero-dep) stays unchanged; the pool is
metadata used for observability today (`distinct_snapshots()`, `snapshot_pool()`).
A follow-up could replace MemoryEntry's inline `state_snapshot_bytes` with a
`snapshot_id`, but that's a `a2x-core` schema change and out of Phase 2.H scope.

### Run-length encoding

Consecutive identical entries collapsed to one representative + one `RleRun` record:
- "Identical" matches on `{ program_id, ip, instruction_bytes, snapshot_id }`.
- Timestamp NOT compared — `SystemTime` monotonicity would prevent any run from
  forming if it were checked.
- For each run of length N: store entry at index 0 of the run; pair with
  `RleRun { entry_index, count: N }`. Length-1 entries stay raw (no RLE record).
- After compress: `entries.len()` drops; `entries[]` holds only representatives.

## Semantic decisions

- `len()` returns physical storage size (`entries.len()`). Satisfies the existing
  `test_compress` invariant `after < before`.
- `tail()` returns physical entries (no RLE expansion) — preserves existing test
  contract; logical reconstruction is available via `logical_len()`.
- `compress()` is idempotent: a second call with no intermediate push no-ops.
- `push()` after compress drops the compression mark — new entries invalidate
  any RLE structure; the trace is back in logical mode until next explicit
  compress().

## Files touched

| Path | Δ | Purpose |
|---|---|---|
| `crates/a2x-ccs/src/memory.rs` | full rewrite (~440 lines) | RLE + hash-dedupe impl + 11 new tests |

## New public API (trait surface unchanged)

```rust
impl VecMemoryTrace {
    pub fn logical_len(&self) -> usize;
    pub fn entries_physical_len(&self) -> usize;
    pub fn distinct_snapshots(&self) -> usize;
    pub fn snapshot_pool(&self) -> &[Vec<u8>];
    pub fn rle_runs(&self) -> &[RleRun];
    pub fn compression_stats(&self) -> Option<&CompressionStats>;
}

pub struct RleRun { entry_index: usize, count: u32 }
pub struct CompressionStats {
    distinct_snapshots: usize,
    snapshot_bytes_total: usize,
    rle_runs: usize,
    physical_reduction: usize,
}
```

## Test coverage

Backward-compat (5 prior tests preserved unchanged):
- `test_push_and_len`, `test_tail`, `test_tail_more_than_len`
- `test_auto_compress_on_overflow`
- `test_compress` (now passes because `len()` = physical storage post-compress)

11 new tests added:
- `test_compress_dedups_identical_state_snapshots` (50 same → 1 distinct)
- `test_compress_pool_records_distinct_payload_by_content` (3 distinct → 3)
- `test_compress_rle_collapses_identical_runs` (100 identical → 1 + RLE count=100)
- `test_compress_no_rle_when_entries_differ` (10 distinct → 0 RLE)
- `test_compress_rle_breaks_on_instruction_change` (5/1/5 → 3 + 2 RLE)
- `test_compress_rle_breaks_on_snapshot_change` (5/5 → 2 RLE)
- `test_compression_stats_idempotent` (compress twice = compress once)
- `test_compression_stats_reports_real_numbers` (1/64/1/99)
- `test_compression_on_empty_trace` (no panic)
- `test_auto_overflow_clears_compression_state` (post-compress push resets)
- `test_logical_len_preserved_across_compress` (50 → 50 logical)

## Reviewer concerns acknowledged

**Snapshot pool staleness** across post-compress pushes: `snapshot_pool` /
`snapshot_lookup` don't get cleared when `compressed = false`. Not user-visible
today (no method exposes the staleness), but a future Phase should either clear
the pool alongside `compressed = false` or recompute it lazily on the next
compress(). Out of scope for Phase 2.H (minimal-delta constraint).

## Cumulative Phase 2 commit list (master post-Phase 2.H)

1. `d851aec` Phase 2.A — VM operand plumbing
2. `cde7b3a` Phase 2.B — operator WorldGraph allocation
3. `08c2600` Phase 2.C — evolve time-step world
4. `18ceb56` Phase 2.D — reflect deterministic self-model
5. `926e9a5` Phase 2.E — plan real Action sequences
6. `077e699` Phase 2.LCG — hoist LCG state to dedicated field
7. `a32ddb1` Phase 2.G — NdArrayStateField (feature-gated)
8. `d7975e0` Phase 2.I — CcsAgent::tick + persistent VM world-model
9. `d4e5845` Phase 2.J — end-to-end Σ∞ smoke test
10. `47f2636` Phase 2 — CCS Cognitive Substrate complete
11. `672c9d1` chore fmt cargo fmt -p a2x-agents
12. `f5ebb49` cleanup: remove .commit_msgs scratch directory
13. **(this commit)** Phase 2.H — RLE + hash-dedupe compression
