// See plans/03-ccs-vm.md §5 (MemoryTrace) and plans/23 §23 compression design.
//
// Phase 2.H adds proper RLE + hash-dedupe compression to VecMemoryTrace:
//
//   * **Hash-dedupe**: identical `state_snapshot_bytes` blocks across many
//     entries are stored once in a `snapshot_pool` keyed by Blake3 content
//     hash. MemoryEntry's `state_snapshot_bytes` field is untouched — the
//     dedup table is metadata used for size accounting and observability.
//     If the table is enabled for backing-store optimization in a follow-up,
//     the snapshot pool becomes the canonical storage and MemoryEntry's
//     inline `state_snapshot_bytes` is replaced with a `snapshot_id`.
//
//   * **RLE**: consecutive identical MemoryEntries (program_id, ip,
//     instruction_bytes, snapshot_hash all equal — timestamps may differ
//     because SystemTime walks forward but the algorithm collapses them
//     to a single representative per run) are collapsed to one
//     representative entry + one `RleRun` record carrying the count.
//
// Trait surface (`MemoryTrace` from `a2x-core/src/memory.rs`) is preserved:
// `push`, `tail`, `len`, `is_empty`, `compress` keep their original
// signatures and observable behaviors. The compression extension uses
// additional public methods (`logical_len`, `compression_stats`,
// `entries_physical_len`, `distinct_snapshots`) for observability.
//
// `len()` semantics (per Phase 2.H):
//   * Pre-compress: `entries.len() == len() == logical_len()` (no RLE).
//   * Post-compress: `len() == entries.len()` (storage count = physical
//     size). This is what `test_compress` asserts (`after < before`).
//   * `logical_len()` always returns the total logical entries that
//     were pushed + auto-trimmed, regardless of RLE collapsing.

use std::collections::HashMap;

use a2x_core::error::CoreError;
use a2x_core::memory::{MemoryEntry, MemoryTrace};
use blake3::Hasher;

/// Vec-backed implementation of the MemoryTrace trait.
///
/// Phase 2.H: adds hash-deduped snapshot storage + RLE collapse on
/// `compress()`. The `MemoryTrace` trait surface is preserved.
pub struct VecMemoryTrace {
    entries: Vec<MemoryEntry>,
    /// Optional: maximum capacity before auto-compression triggers.
    max_capacity: usize,

    // === Phase 2.H additions ===
    /// Dedup'd snapshot storage. `snapshot_pool[i]` is the i-th unique
    /// `state_snapshot_bytes` payload observed during the last compress()
    /// pass. Index 0 holds the first observed unique payload (well-formed
    /// after `compress()` even if entries[] is empty).
    snapshot_pool: Vec<Vec<u8>>,

    /// Blake3-hash → snapshot_pool index. 32-byte content hash; cheap to
    /// compute and stable across runs with identical state_snapshot_bytes.
    snapshot_lookup: HashMap<[u8; 32], usize>,

    /// RLE runs. Each run collapses a contiguous identical-entry sequence
    /// in `entries[]`. The first entry of the run is kept in `entries[]`;
    /// the remaining `count - 1` are accounted for via the run record.
    rle_runs: Vec<RleRun>,

    /// Stats accumulated during the last `compress()` call. Useful for
    /// observability without re-running compress.
    last_compression: Option<CompressionStats>,

    /// Set to true after compress() so `len()` reports physical storage
    /// (drops with compression). False while entries[] is canonical logical
    /// storage (no RLE applied).
    compressed: bool,
}

/// One RLE run: a collapsed range of identical entries.
///
/// `entry_index` is the position in `VecMemoryTrace::entries[]` of the
/// representative entry. `count` is the total logical entries the
/// representative stands for (always >= 2 — runs of size 1 stay in the
/// entries[] raw form without a paired RleRun record).
#[derive(Clone, Debug, PartialEq)]
pub struct RleRun {
    pub entry_index: usize,
    pub count: u32,
}

/// Compression observability struct, returned by `compression_stats()`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompressionStats {
    /// Number of unique state_snapshot_bytes payloads (== snapshot_pool.len()).
    pub distinct_snapshots: usize,
    /// Total snapshot bytes (sum of all snapshot_pool[i].len()).
    pub snapshot_bytes_total: usize,
    /// Number of RLE runs collapsed.
    pub rle_runs: usize,
    /// Total entries absorbed into RLE runs, excluding representatives.
    /// `physical_reduction == rle_runs` summed (count - 1).
    pub physical_reduction: usize,
}

impl VecMemoryTrace {
    /// Create a new empty MemoryTrace with the given max capacity.
    pub fn new(max_capacity: usize) -> Self {
        VecMemoryTrace {
            entries: Vec::new(),
            max_capacity,
            snapshot_pool: Vec::new(),
            snapshot_lookup: HashMap::new(),
            rle_runs: Vec::new(),
            last_compression: None,
            compressed: false,
        }
    }

    /// Create with a default capacity of 10,000 entries.
    pub fn default_capacity() -> Self {
        Self::new(10_000)
    }

    /// Get all entries (for iteration).
    pub fn all_entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    /// Phase 2.H: total logical entry count (entries[] + RLE run absorption,
    /// minus representative count). Equals entries[] count when no
    /// compression applied.
    pub fn logical_len(&self) -> usize {
        let rle_absorbed: usize = self
            .rle_runs
            .iter()
            .map(|r| (r.count as usize).saturating_sub(1))
            .sum();
        self.entries.len() + rle_absorbed
    }

    /// Phase 2.H: number of physically-stored MemoryEntry instances
    /// (entries[] length post any RLE collapse).
    pub fn entries_physical_len(&self) -> usize {
        self.entries.len()
    }

    /// Phase 2.H: number of unique state_snapshot_bytes payloads in the
    /// dedup pool (== snapshot_pool.len()). Zero pre-compress.
    pub fn distinct_snapshots(&self) -> usize {
        self.snapshot_pool.len()
    }

    /// Phase 2.H: snapshot pool accessor for advanced callers.
    pub fn snapshot_pool(&self) -> &[Vec<u8>] {
        &self.snapshot_pool
    }

    /// Phase 2.H: RLE runs accessor.
    pub fn rle_runs(&self) -> &[RleRun] {
        &self.rle_runs
    }

    /// Phase 2.H: stats from the last compress() call. None if compress
    /// was never invoked.
    pub fn compression_stats(&self) -> Option<&CompressionStats> {
        self.last_compression.as_ref()
    }

    /// Phase 2.H: compute Blake3 content hash of an entry's
    /// `state_snapshot_bytes`. Stable across runs with identical content.
    fn snapshot_hash(entry: &MemoryEntry) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&entry.state_snapshot_bytes);
        *hasher.finalize().as_bytes()
    }
}

impl Default for VecMemoryTrace {
    fn default() -> Self {
        Self::default_capacity()
    }
}

impl MemoryTrace for VecMemoryTrace {
    fn push(&mut self, entry: MemoryEntry) -> Result<(), CoreError> {
        if self.entries.len() >= self.max_capacity {
            // Phase 2.H: simple auto-truncate (drop oldest half). Note: we
            // intentionally clear ALL compression metadata — RLE runs,
            // snapshot dedup pool, lookup map, and cached stats — because
            // the truncation invalidates RLE run indices (they pointed at
            // the now-drained positions) AND destroys the snapshot hash
            // basis (snapshot_lookup hashes no longer correspond to any
            // surviving entries[] payloads). The two clear paths in this
            // function (overflow truncate vs post-compress push) are now
            // deliberately symmetric so neither leaves stale compression
            // metadata observable via distinct_snapshots() / snapshot_pool().
            let keep = self.max_capacity / 2;
            let drain_end = self.entries.len() - keep;
            self.entries.drain(0..drain_end);
            self.rle_runs.clear();
            self.snapshot_pool.clear();
            self.snapshot_lookup.clear();
            self.compressed = false;
            self.last_compression = None;
        }
        if self.compressed {
            // New entry invalidates any pre-computed RLE structure (a fresh
            // run may form with neighbours, or a run may split). Drop ALL
            // compression metadata — RLE runs, snapshot dedup pool, and the
            // cached stats — so the trace is in a fully-consistent logical
            // mode until the next explicit compress() rebuilds everything.
            // Snapshot pool staleness was the canonical example of why
            // "compressed: false" alone is insufficient: the pool still
            // reflects pre-push contents even though entries[] has grown.
            self.rle_runs.clear();
            self.snapshot_pool.clear();
            self.snapshot_lookup.clear();
            self.compressed = false;
            self.last_compression = None;
        }
        self.entries.push(entry);
        Ok(())
    }

    fn tail(&self, n: usize) -> Vec<MemoryEntry> {
        // Pre-compress: tail is straightforward — last n entries.
        // Post-compress: tail returns the last `n` *physical* entries from
        // entries[]. RLE runs are not expanded (logical reconstruction is
        // a separate API: `expand_logical_tail` if needed by future code).
        // This matches the existing `test_tail` expectation: `tail(n)`
        // returns up to `n` entries based on stored bounds.
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].to_vec()
    }

    fn len(&self) -> usize {
        // After Phase 2.H compress(), len() returns the physical storage
        // size so callers can observe that compression actually reduced
        // storage. Pre-compress, physical == logical.
        self.entries.len()
    }

    fn compress(&mut self) -> Result<(), CoreError> {
        // Phase 2.H: idempotent — if the trace is already compressed and
        // entries haven't changed since the last compress(), no-op.
        // This guarantees `compress(); compress();` produces identical
        // stats (test_compression_stats_idempotent) without rebuilding
        // pool + RLE metadata when nothing has changed.
        if self.compressed {
            return Ok(());
        }
        // Empty trace: nothing to do.
        if self.entries.is_empty() {
            self.snapshot_pool.clear();
            self.snapshot_lookup.clear();
            self.rle_runs.clear();
            self.compressed = true;
            self.last_compression = Some(CompressionStats {
                distinct_snapshots: 0,
                snapshot_bytes_total: 0,
                rle_runs: 0,
                physical_reduction: 0,
            });
            return Ok(());
        }

        // Phase 2.H Step 1: rebuild the snapshot pool from current entries.
        // For each entry, check the dedup table; if missing, insert into
        // pool + lookup map. This pass is linear in entries.len().
        self.snapshot_pool.clear();
        self.snapshot_lookup.clear();
        let mut snapshot_ids: Vec<usize> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let hash = Self::snapshot_hash(entry);
            let pool_id = match self.snapshot_lookup.get(&hash) {
                Some(&id) => id,
                None => {
                    let id = self.snapshot_pool.len();
                    self.snapshot_pool.push(entry.state_snapshot_bytes.clone());
                    self.snapshot_lookup.insert(hash, id);
                    id
                }
            };
            snapshot_ids.push(pool_id);
        }

        // Phase 2.H Step 2: build RLE runs over consecutive identical entries.
        // Two entries are "identical" iff all of {program_id, ip,
        // instruction_bytes, snapshot_id} are equal. Timestamps are NOT
        // compared — SystemTime monotonicity would prevent any run from
        // forming if it were checked, since instruction bytes may be
        // identical (replay) but timestamps will differ.
        let mut new_rle_runs: Vec<RleRun> = Vec::new();
        let mut keep_indices: Vec<usize> = Vec::with_capacity(self.entries.len());

        let mut i = 0;
        while i < self.entries.len() {
            let mut run_count: u32 = 1;
            let mut j = i + 1;
            while j < self.entries.len()
                && self.entries[i].program_id == self.entries[j].program_id
                && self.entries[i].ip == self.entries[j].ip
                && self.entries[i].instruction_bytes == self.entries[j].instruction_bytes
                && snapshot_ids[i] == snapshot_ids[j]
            {
                run_count += 1;
                j += 1;
            }

            keep_indices.push(i); // representative kept
            if run_count >= 2 {
                new_rle_runs.push(RleRun {
                    entry_index: i,
                    count: run_count,
                });
            }
            i = j;
        }

        // Phase 2.H Step 3: collapse entries[] to representatives only.
        // Use a copy-then-swap approach to avoid borrow issues — allocate
        // a new Vec with just the kept representatives, then move-rewrite
        // self.entries. Order is preserved.
        let new_entries: Vec<MemoryEntry> = keep_indices
            .iter()
            .map(|&k| self.entries[k].clone())
            .collect();
        self.entries = new_entries;
        self.rle_runs = new_rle_runs;

        // Compute final stats.
        let snapshot_bytes_total: usize = self.snapshot_pool.iter().map(|v| v.len()).sum();
        let physical_reduction: usize = self
            .rle_runs
            .iter()
            .map(|r| (r.count as usize).saturating_sub(1))
            .sum();
        let stats = CompressionStats {
            distinct_snapshots: self.snapshot_pool.len(),
            snapshot_bytes_total,
            rle_runs: self.rle_runs.len(),
            physical_reduction,
        };
        self.compressed = true;
        self.last_compression = Some(stats);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::memory::MemoryEntry;

    fn make_entry() -> MemoryEntry {
        MemoryEntry {
            timestamp: None,
            instruction_bytes: vec![1u8, 2, 3],
            ip: 0,
            program_id: None,
            state_snapshot_bytes: vec![0u8; 64],
        }
    }

    fn make_entry_with_inst(inst_byte: u8, ip: usize, snap_byte: u8) -> MemoryEntry {
        MemoryEntry {
            timestamp: None,
            instruction_bytes: vec![inst_byte],
            ip,
            program_id: None,
            state_snapshot_bytes: vec![snap_byte; 16],
        }
    }

    // === Backward-compat tests (Phase 0 / existing) ===

    #[test]
    fn test_push_and_len() {
        let mut trace = VecMemoryTrace::new(100);
        trace.push(make_entry()).unwrap();
        trace.push(make_entry()).unwrap();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace.logical_len(), 2);
        assert_eq!(trace.entries_physical_len(), 2);
    }

    #[test]
    fn test_tail() {
        let mut trace = VecMemoryTrace::new(100);
        for _ in 0..5 {
            trace.push(make_entry()).unwrap();
        }
        let tail = trace.tail(3);
        assert_eq!(tail.len(), 3);
    }

    #[test]
    fn test_tail_more_than_len() {
        let mut trace = VecMemoryTrace::new(100);
        trace.push(make_entry()).unwrap();
        let tail = trace.tail(10);
        assert_eq!(tail.len(), 1);
    }

    #[test]
    fn test_auto_compress_on_overflow() {
        let mut trace = VecMemoryTrace::new(10);
        for _ in 0..15 {
            trace.push(make_entry()).unwrap();
        }
        assert!(trace.len() <= 10);
    }

    #[test]
    fn test_compress() {
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..200 {
            trace.push(make_entry()).unwrap();
        }
        let before = trace.len();
        trace.compress().unwrap();
        let after = trace.len();
        assert!(after < before);
    }

    // === Phase 2.H NEW: hash-dedupe ===

    #[test]
    fn test_compress_dedups_identical_state_snapshots() {
        // 50 entries push identical snapshots → 1 unique snapshot post-compress.
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..50 {
            trace.push(make_entry()).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.distinct_snapshots(), 1);
        // Pool holds 1 unique entry; that entry's payload is 64 bytes.
        assert_eq!(trace.snapshot_pool().len(), 1);
        assert_eq!(trace.snapshot_pool()[0].len(), 64);
    }

    #[test]
    fn test_compress_pool_records_distinct_payload_by_content() {
        // 3 distinct payloads across 9 pushes → pool size 3.
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..3 {
            trace.push(make_entry_with_inst(1, 0, 0xAA)).unwrap();
        }
        for _ in 0..3 {
            trace.push(make_entry_with_inst(1, 0, 0xBB)).unwrap();
        }
        for _ in 0..3 {
            trace.push(make_entry_with_inst(1, 0, 0xCC)).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.distinct_snapshots(), 3);
    }

    // === Phase 2.H NEW: RLE ===

    #[test]
    fn test_compress_rle_collapses_identical_runs() {
        // 100 identical entries → 1 representative + 1 RleRun count=100.
        let mut trace = VecMemoryTrace::new(2000);
        for _ in 0..100 {
            trace.push(make_entry()).unwrap();
        }
        assert_eq!(trace.entries_physical_len(), 100);
        trace.compress().unwrap();
        assert_eq!(trace.entries_physical_len(), 1);
        assert_eq!(trace.rle_runs().len(), 1);
        assert_eq!(trace.rle_runs()[0].count, 100);
        assert_eq!(trace.logical_len(), 100);
        assert_eq!(trace.len(), 1);
    }

    #[test]
    fn test_compress_no_rle_when_entries_differ() {
        let mut trace = VecMemoryTrace::new(1000);
        for i in 0..10 {
            trace.push(make_entry_with_inst(i as u8, i, 0)).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.entries_physical_len(), 10);
        assert_eq!(trace.rle_runs().len(), 0);
        assert_eq!(trace.logical_len(), 10);
    }

    #[test]
    fn test_compress_rle_breaks_on_instruction_change() {
        // 5 identical, then 1 different, then 5 identical again.
        // → 3 representatives + 2 RLE runs (first + last of length 5).
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..5 {
            trace.push(make_entry_with_inst(1, 0, 0)).unwrap();
        }
        trace.push(make_entry_with_inst(2, 0, 0)).unwrap();
        for _ in 0..5 {
            trace.push(make_entry_with_inst(1, 0, 0)).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.entries_physical_len(), 3);
        assert_eq!(trace.rle_runs().len(), 2);
        assert_eq!(trace.rle_runs()[0].count, 5);
        assert_eq!(trace.rle_runs()[1].count, 5);
        assert_eq!(trace.logical_len(), 11);
    }

    #[test]
    fn test_compress_rle_breaks_on_snapshot_change() {
        // Same instruction but different snapshot → RLE must break.
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..5 {
            trace.push(make_entry_with_inst(1, 0, 0xAA)).unwrap();
        }
        for _ in 0..5 {
            trace.push(make_entry_with_inst(1, 0, 0xBB)).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.entries_physical_len(), 2);
        assert_eq!(trace.rle_runs().len(), 2);
    }

    // === Phase 2.H NEW: stats ===

    #[test]
    fn test_compression_stats_idempotent() {
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..100 {
            trace.push(make_entry()).unwrap();
        }
        trace.compress().unwrap();
        let stats1 = trace.compression_stats().unwrap().clone();
        trace.compress().unwrap();
        let stats2 = trace.compression_stats().unwrap().clone();
        assert_eq!(stats1, stats2);
    }

    #[test]
    fn test_compression_stats_reports_real_numbers() {
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..100 {
            trace.push(make_entry()).unwrap();
        }
        trace.compress().unwrap();
        let stats = trace.compression_stats().unwrap();
        assert_eq!(stats.distinct_snapshots, 1);
        assert_eq!(stats.snapshot_bytes_total, 64); // snapshot = vec![0u8; 64]
        assert_eq!(stats.rle_runs, 1);
        assert_eq!(stats.physical_reduction, 99); // 100 - 1 representative
    }

    #[test]
    fn test_compression_on_empty_trace() {
        let mut trace = VecMemoryTrace::new(100);
        trace.compress().unwrap();
        let stats = trace.compression_stats().unwrap();
        assert_eq!(stats.distinct_snapshots, 0);
        assert_eq!(stats.snapshot_bytes_total, 0);
        assert_eq!(stats.rle_runs, 0);
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn test_auto_overflow_clears_compression_state() {
        // After compress() ANY subsequent push invalidates RLE structure
        // (a new entry might extend a run, split a run, or start a new
        // one). Push must clear compressed-mark so the next compress()
        // recomputes. This guards: RLE indices stay consistent with
        // entries[].
        let mut trace = VecMemoryTrace::new(10);
        for _ in 0..10 {
            trace.push(make_entry()).unwrap();
        }
        trace.compress().unwrap();
        assert!(trace.compressed);
        assert_eq!(trace.rle_runs().len(), 1);
        assert_eq!(trace.distinct_snapshots(), 1);
        // After any push post-compress, trace is back in logical mode.
        trace.push(make_entry()).unwrap();
        assert!(!trace.compressed);
        assert!(trace.rle_runs().is_empty());
        assert!(trace.compression_stats().is_none());
        // Snapshot pool also cleared so the dedup table doesn't lie about
        // contents that no longer exist for entries that did exist pre-push.
        assert!(trace.distinct_snapshots() == 0);
        assert!(trace.snapshot_pool().is_empty());
    }

    #[test]
    fn test_post_compress_push_clears_pool_then_recompress_rebuilds() {
        // After post-compress push, even if the new entry's snapshot_byte
        // pattern happens to match a previously-pooled payload, the pool
        // is cleared. The next compress() rebuilds it from current
        // entries[] — no caching of stale pool across pushes.
        let mut trace = VecMemoryTrace::new(100);
        for _ in 0..10 {
            trace.push(make_entry()).unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.distinct_snapshots(), 1);
        // Push a new entry whose snapshot matches the old pool's payload.
        let mut e = make_entry();
        e.ip = 99; // different ip ensures RLE doesn't reform across push
        e.state_snapshot_bytes = vec![0u8; 64]; // matches previous pool
        trace.push(e).unwrap();
        // Pool must have been cleared (stale data would otherwise leak).
        assert_eq!(trace.distinct_snapshots(), 0);
        // Re-compress rebuilds pool from current entries[]: 2 physical
        // entries (1 collapsed rep + 1 new push) with differ by ip, so
        // no RLE run forms and pool is rebuilt with the single unique
        // snapshot payload (both share `vec![0u8; 64]`).
        trace.compress().unwrap();
        assert_eq!(trace.distinct_snapshots(), 1);
        assert_eq!(trace.rle_runs().len(), 0);
        assert_eq!(trace.entries_physical_len(), 2);
    }

    #[test]
    fn test_logical_len_preserved_across_compress() {
        // The whole point: logical_len should NOT drop with compression —
        // only entries[] storage drops. tail() expansion would use this.
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..50 {
            trace.push(make_entry()).unwrap();
        }
        let logical_pre = trace.logical_len();
        assert_eq!(logical_pre, 50);
        trace.compress().unwrap();
        assert_eq!(trace.logical_len(), 50); // post-compress logical == pre-compress logical
    }

    #[test]
    fn test_overflow_clears_snapshot_pool() {
        // Regression: when the autoflow-truncate branch fires while the
        // snapshot pool is populated (i.e., entries.len() >= max_capacity
        // AND the trace is in compressed mode with no RLE collapse so
        // entries[] has not been shrunk by compress), the overflow branch
        // MUST clear snapshot_pool/snapshot_lookup, not just rle_runs +
        // compressed + last_compression. Without this, `snapshot_pool()`
        // and `distinct_snapshots()` would expose stale dedup data
        // indexed against entries[] items the overflow branch just
        // drained out of indices [0..drain_end).
        //
        // Construction: max_capacity = 10, 10 distinct snapshot payloads
        // → compress() does NOT trigger RLE collapse (everything differs
        // by snapshot_byte); entries.len() stays at 10, snapshot_pool
        // holds 10 distinct payloads. The 11th push triggers overflow.
        let mut trace = VecMemoryTrace::new(10);
        for i in 0..10 {
            trace
                .push(make_entry_with_inst(i as u8, i as usize, (0xA0 + i) as u8))
                .unwrap();
        }
        trace.compress().unwrap();
        assert_eq!(trace.entries_physical_len(), 10); // no RLE: all distinct
        assert_eq!(trace.distinct_snapshots(), 10); // pool pre-overflow
        assert_eq!(trace.snapshot_pool().len(), 10);

        // 11th push: 10 >= 10 → overflow branch fires first.
        let entry11 = make_entry_with_inst(99, 99, 0xFE);
        trace.push(entry11).unwrap();

        // All compression metadata cleared, including snapshot_pool +
        // snapshot_lookup (the new overflow-branch clear introduced as a
        // sibling to the post-compress-push branch).
        assert!(trace.snapshot_pool().is_empty());
        assert_eq!(trace.distinct_snapshots(), 0);
        assert!(trace.rle_runs().is_empty());
        assert!(!trace.compressed);
        assert!(trace.compression_stats().is_none());
        // Overflow drained entries[0..5] (keep = 10/2 = 5) then pushed
        // entry11: 5 + 1 = 6 physical entries.
        assert_eq!(trace.entries_physical_len(), 6);
    }
}
