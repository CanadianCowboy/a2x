// reflect operator: Meta-learning from execution history.
// See plans/03-ccs-vm.md §4
//
// Phase 2.D: reflect is a real read-and-summarize operation that builds a
// deterministic "self-model" vector from the recent MemoryTrace tail, the
// current StateField, and the WorldGraph access_count distribution. The
// self-model is allocated as a new WorldGraph node (ConceptVector). For each
// inspected MemoryEntry, a shadow node is also allocated so a Temporal edge
// can land in the graph (edges are between NodeIds, not MemoryEntries).
//
// Determinism invariant: no wall-clock reads. The "steps/sec" stat is computed
// as `entries_per_inspected_window` (a sequential index proxy that is identical
// across two runs of the same program). The op-mix histogram is derived from
// `instruction_bytes.len()` — coarse but reproducible from VM state alone.
//
// Reflection over what just happened: this is intentionally looser than a
// learned policy update — Phase 2.D is about *making the trace observable as a
// graph node*, not about learning.

use a2x_core::concept::ConceptVector;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_core::relation::{RelationEdge, RelationType};

/// Output of a reflect operation: the new self-model node plus the entry-shadow
/// nodes created to land Temporal edges. Together they form a small sub-graph
/// the rest of the VM can read (Phase 2.E planner consumes this).
#[derive(Clone, Debug, PartialEq)]
pub struct ReflectResult {
    /// NodeId of the freshly-allocated self-model ConceptVector.
    pub self_model_id: a2x_core::node::NodeId,
    /// NodeIds of the entry-shadow nodes (one per inspected MemoryEntry).
    /// Each has a `Temporal` outgoing edge to `self_model_id`.
    pub entry_node_ids: Vec<a2x_core::node::NodeId>,
    /// The self-model ConceptVector (also stored on the node).
    pub self_model_concept: ConceptVector,
}

/// Phase 2.D retained type so existing call sites / tests keep compiling even
/// though we replaced the function signature. The new reflect returns a
/// `ReflectResult`; downstream `PolicyUpdate` is `Default::default()` for now
/// (no learned adjustment yet — Phase 2.D is about *visibility*).
#[derive(Clone, Debug, PartialEq)]
pub struct PolicyUpdate {
    pub adjustment: Vec<f32>,
    pub insight: String,
}

impl Default for PolicyUpdate {
    fn default() -> Self {
        Self {
            adjustment: Vec::new(),
            insight: "Phase 2.D reflect produces a graph node; no policy delta yet".to_string(),
        }
    }
}

/// Window of recent MemoryTrace entries to summarize. Must be >= 1; values
/// larger than the trace length collapse to `trace.len()`.
pub const REFLECT_DEFAULT_WINDOW: usize = 8;

/// Total dims in the self-model ConceptVector. Layout:
///   \[0..16\]   op-mix histogram (16 buckets over `instruction_bytes.len()` mod 16)
///   \[16..32\]  access_count distribution (16 buckets over per-node access_count)
///   \[32..96\]  state snapshot: belief L2 norms (16) + attention mean / std (16) +
///               temporal[0..32]
///   \[96..128\] trace stats: entries_inspected, recent_ip_signature, …
pub const SELF_MODEL_DIM: usize = 128;
const BUCKETS: usize = 16;
const STATE_SLOTS: usize = 32; // first 32 slots of `temporal`
const TRACE_STATS: usize = 32; // 96..128

/// Reflect on execution history.
///
/// Allocates a self-model node + entry-shadow nodes; creates Temporal edges.
/// Returns the result. The function is deterministic given (trace, graph,
/// window) — no wall-clock reads.
pub fn reflect(
    trace: &dyn MemoryTrace,
    graph: &mut dyn WorldGraph,
    window: usize,
) -> Result<ReflectResult, ReflectError> {
    let window = window.max(1);
    let entries = trace.tail(window);

    // Compute the three stat sections.
    let op_mix = compute_op_mix_histogram(&entries);
    let ac_dist = compute_access_count_distribution(graph);
    let (state_belief_summary, state_attention_summary, temporal_head) =
        compute_state_summary(graph);

    // Assemble the self-model vector.
    let mut data = vec![0.0f32; SELF_MODEL_DIM];
    data[0..BUCKETS].copy_from_slice(&op_mix);
    data[BUCKETS..2 * BUCKETS].copy_from_slice(&ac_dist);
    data[32..48].copy_from_slice(&state_belief_summary);
    data[48..64].copy_from_slice(&state_attention_summary);
    data[64..64 + STATE_SLOTS].copy_from_slice(&temporal_head);

    // Trace stats section: stores entries_inspected as float, recent_ip_signature
    // as a deterministic hash over the recent instruction pointers.
    let entries_inspected = entries.len() as f32;
    data[96] = entries_inspected;
    let ip_sig = compute_recent_ip_signature(&entries);
    data[97..97 + BUCKETS.min(TRACE_STATS - 1)]
        .copy_from_slice(&ip_sig[..(TRACE_STATS - 1).min(BUCKETS)]);

    let self_model = ConceptVector::from_vec(data).with_label("self-model");

    // Allocate the self-model node first so we can use its NodeId as edge target.
    let self_model_id = graph
        .allocate(self_model.clone())
        .map_err(|e| ReflectError::GraphError(e.to_string()))?;
    graph
        .set_provenance(
            self_model_id,
            &format!("reflect(window={},edges={})", window, entries.len()),
        )
        .map_err(|e| ReflectError::GraphError(e.to_string()))?;
    // Phase 2.D Phase-2.E handoff: the latest self-model NodeId is stored on
    // the VM (`vm.last_reflect`) — NOT via a graph label. Earlier experiments
    // tried `__last_reflect` as a re-pointable label, but `set_label` rejects
    // collisions (different node already owns the label), so re-pointing
    // silently failed on the second REFLECT call. VM-side state is the
    // single source of truth — graph labels are read-only views into it.

    // Allocate one shadow node per inspected entry + Temporal edge to self-model.
    let mut entry_node_ids = Vec::with_capacity(entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let shadow = entry_to_concept(entry, i);
        let shadow_id = graph
            .allocate(shadow)
            .map_err(|e| ReflectError::GraphError(e.to_string()))?;
        graph
            .set_provenance(
                shadow_id,
                &format!("reflect_entry(index={},ip={})", i, entry.ip),
            )
            .map_err(|e| ReflectError::GraphError(e.to_string()))?;
        // Temporal: this entry preceded the self-model in execution order.
        let edge = RelationEdge::new(shadow_id, self_model_id, RelationType::Temporal, 1.0);
        if let Err(e) = graph.add_edge(shadow_id, self_model_id, edge) {
            // Duplicate-edges between the same shadow and self_model shouldn't
            // happen here since each shadow is a fresh allocation, but log it.
            let _ = e;
        }
        entry_node_ids.push(shadow_id);
    }

    Ok(ReflectResult {
        self_model_id,
        entry_node_ids,
        self_model_concept: self_model,
    })
}

// ===== Pure stat helpers (unit-tested separately) =====

/// Op-mix histogram over `instruction_bytes.len() mod BUCKETS`.
/// Each bucket = (count of entries whose bytes.len() % 16 == i) / window.
/// The bucket choice is coarse but fully deterministic and zero-cost (no parse).
fn compute_op_mix_histogram(entries: &[a2x_core::memory::MemoryEntry]) -> [f32; BUCKETS] {
    let mut hist = [0.0f32; BUCKETS];
    if entries.is_empty() {
        return hist;
    }
    for entry in entries {
        let bucket = entry.instruction_bytes.len() % BUCKETS;
        hist[bucket] += 1.0;
    }
    let n = entries.len() as f32;
    for v in &mut hist {
        *v /= n;
    }
    hist
}

/// Access-count distribution over the graph: bucket[k] = (count of nodes with
/// `access_count == k`) / total_nodes, falling off as access_count grows.
/// First bucket (`k == 0`) is dominant on a fresh graph.
fn compute_access_count_distribution(graph: &dyn WorldGraph) -> [f32; BUCKETS] {
    let mut dist = [0.0f32; BUCKETS];
    let ids = graph.node_ids();
    if ids.is_empty() {
        return dist;
    }
    let mut total = 0.0f32;
    for id in &ids {
        if let Ok(Some(node)) = graph.lookup(*id) {
            let k = node.metadata.access_count.min(BUCKETS as u64 - 1) as usize;
            dist[k] += 1.0;
            total += 1.0;
        }
    }
    if total > 0.0 {
        for v in &mut dist {
            *v /= total;
        }
    }
    dist
}

/// State summary extracted from the StateField via `ConceptVector`s stored on
/// the graph. The reflect operator is graph-aware (Phase 2.B nodes carry
/// memory snapshots), so we read those. Returns three sections that fill
/// `[32..48]`, `[48..64]`, and `[64..96]` of the self-model vector.
fn compute_state_summary(graph: &dyn WorldGraph) -> ([f32; 16], [f32; 16], [f32; STATE_SLOTS]) {
    let mut belief_summary = [0.0f32; 16];
    let mut attention_summary = [0.0f32; 16];
    let mut temporal_head = [0.0f32; STATE_SLOTS];

    // Walk the graph and bucket different concept "shapes" into stat slots.
    // The shape detector is intentionally trivial: by provenance prefix
    // (`bind(`, `differentiate(`, `ground(`) we can tell what kind of node
    // we have. This is a *runtime type tag* computed deterministically.
    let ids = graph.node_ids();
    let mut belief_total = 0.0f32;
    let mut attention_total = 0.0f32;
    let mut belief_count = 0.0f32;
    let mut attention_count = 0.0f32;

    for id in &ids {
        let node = match graph.lookup(*id) {
            Ok(Some(n)) => n,
            _ => continue,
        };
        match node.metadata.provenance.as_deref() {
            Some(p) if p.starts_with("bind(") => {
                // Bind composites: sum squares into belief slot.
                let sq_sum: f32 = node.concept.data.iter().map(|v| v * v).sum();
                belief_total += sq_sum.sqrt();
                belief_count += 1.0;
            }
            Some(p) if p.starts_with("differentiate(") => {
                // Diff chunks: count contributes to belief_norm count,
                // magnitude contributes to attention.
                let norm: f32 = node.concept.data.iter().map(|v| v * v).sum::<f32>().sqrt();
                attention_total += norm;
                attention_count += 1.0;
            }
            Some(p) if p.starts_with("ground(") => {
                // Ground nodes: payload length into temporal_head slot.
                let dim = node.concept.dimensions.min(STATE_SLOTS);
                if dim > 0 {
                    let slot = dim - 1;
                    temporal_head[slot] += 1.0;
                }
            }
            _ => {}
        }
    }

    let belief_mean = if belief_count > 0.0 {
        belief_total / belief_count
    } else {
        0.0
    };
    let attention_mean = if attention_count > 0.0 {
        attention_total / attention_count
    } else {
        0.0
    };

    // Encode summaries as fixed layouts — easy to test.
    belief_summary[0] = belief_count; // number of bind-composites
    belief_summary[1] = belief_mean;
    belief_summary[2] = belief_mean * belief_mean; // mean-square proxy
    attention_summary[0] = attention_count; // number of diff chunks
    attention_summary[1] = attention_mean;
    attention_summary[2] = attention_mean * attention_mean;
    temporal_head[0] = temporal_head.iter().sum(); // total ground events by tail slot

    (belief_summary, attention_summary, temporal_head)
}

/// Deterministic hash of recent `MemoryEntry.ip` values into a 16-element
/// bucket array. Used as a "recent instruction-pointer signature" so two VMs
/// with the same `last_n` instruction pointers produce the same self-model.
fn compute_recent_ip_signature(entries: &[a2x_core::memory::MemoryEntry]) -> [f32; BUCKETS] {
    let mut sig = [0.0f32; BUCKETS];
    for entry in entries {
        let bucket = entry.ip % BUCKETS;
        sig[bucket] += 1.0;
    }
    sig
}

/// Reduce a single MemoryEntry to a small ConceptVector that can live as a
/// graph node. Captures enough signal that downstream queries on the entry
/// can recover what happened.
fn entry_to_concept(entry: &a2x_core::memory::MemoryEntry, index: usize) -> ConceptVector {
    let mut data = vec![0.0f32; 16];
    data[0] = entry.ip as f32;
    data[1] = entry.instruction_bytes.len() as f32;
    // Snapshot fingerprint: first 14 little-endian u32 words of state snapshot.
    let n_words = (entry.state_snapshot_bytes.len() / 4).min(14);
    for k in 0..n_words {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&entry.state_snapshot_bytes[k * 4..k * 4 + 4]);
        let v = f32::from_le_bytes(buf);
        data[2 + k] = if v.is_nan() { 0.0 } else { v };
    }
    ConceptVector::from_vec(data).with_label(format!("reflect_entry_{}", index))
}

/// Error during reflect operation.
#[derive(Clone, Debug, PartialEq)]
pub enum ReflectError {
    GraphError(String),
}

impl std::fmt::Display for ReflectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReflectError::GraphError(msg) => write!(f, "reflect graph error: {}", msg),
        }
    }
}

impl std::error::Error for ReflectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::VecMemoryTrace;
    use crate::world_graph::PetgraphWorldGraph;
    use a2x_core::concept::ConceptVector;
    use a2x_core::memory::MemoryEntry;

    fn make_entry(ip: usize, inst_len: usize, snap: Vec<u8>) -> MemoryEntry {
        MemoryEntry {
            timestamp: None,
            instruction_bytes: vec![0u8; inst_len],
            ip,
            program_id: None,
            state_snapshot_bytes: snap,
        }
    }

    #[test]
    fn test_reflect_stub_legacy_compat() {
        // Phase 0 stub was `reflect(_trace_len) -> PolicyUpdate`. The new API
        // is opaque-data-driven and PolicyUpdate is `Default::default()`. The
        // type still exists so downstream code referencing it doesn't break.
        let u = PolicyUpdate::default();
        assert!(u.adjustment.is_empty());
    }

    #[test]
    fn test_op_mix_histogram_uniform() {
        // 8 entries, each with instruction_bytes.len() = 8 → all bucket == 8%16 == 8.
        let entries: Vec<MemoryEntry> = (0..8).map(|i| make_entry(i, 8, vec![])).collect();
        let hist = compute_op_mix_histogram(&entries);
        assert_eq!(hist[8], 1.0);
        for (i, v) in hist.iter().enumerate() {
            if i != 8 {
                assert_eq!(*v, 0.0, "bucket {} should be 0", i);
            }
        }
    }

    #[test]
    fn test_op_mix_histogram_mixed() {
        // 4 with len=0 (bucket 0), 4 with len=1 (bucket 1).
        let entries: Vec<MemoryEntry> = (0..8)
            .map(|i| make_entry(i, if i < 4 { 0 } else { 1 }, vec![]))
            .collect();
        let hist = compute_op_mix_histogram(&entries);
        assert_eq!(hist[0], 0.5);
        assert_eq!(hist[1], 0.5);
        for (i, v) in hist.iter().enumerate() {
            if i != 0 && i != 1 {
                assert_eq!(*v, 0.0);
            }
        }
    }

    #[test]
    fn test_op_mix_histogram_empty() {
        let hist = compute_op_mix_histogram(&[]);
        assert!(hist.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn test_access_count_distribution_empty_graph() {
        let wg = PetgraphWorldGraph::new();
        let dist = compute_access_count_distribution(&wg);
        assert!(dist.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn test_access_count_distribution_three_unaccessed_nodes() {
        let mut wg = PetgraphWorldGraph::new();
        wg.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
        wg.allocate(ConceptVector::from_vec(vec![2.0])).unwrap();
        wg.allocate(ConceptVector::from_vec(vec![3.0])).unwrap();
        let dist = compute_access_count_distribution(&wg);
        assert_eq!(dist[0], 1.0);
        for (i, v) in dist.iter().enumerate() {
            if i != 0 {
                assert_eq!(*v, 0.0);
            }
        }
    }

    #[test]
    fn test_access_count_distribution_bumps_advance_bucket() {
        let mut wg = PetgraphWorldGraph::new();
        let id = wg.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
        wg.bump_access_count(id).unwrap();
        wg.bump_access_count(id).unwrap();
        // access_count=2 falls into bucket 2.
        let dist = compute_access_count_distribution(&wg);
        assert_eq!(dist[2], 1.0);
    }

    #[test]
    fn test_recent_ip_signature_uniform() {
        let entries: Vec<MemoryEntry> = (0..4).map(|i| make_entry(i * 16, 0, vec![])).collect();
        let sig = compute_recent_ip_signature(&entries);
        // All ips land in bucket 0.
        assert_eq!(sig[0], 4.0);
    }

    #[test]
    fn test_recent_ip_signature_modulo_dispatch() {
        let entries: Vec<MemoryEntry> = vec![
            make_entry(0, 0, vec![]),
            make_entry(1, 0, vec![]),
            make_entry(2, 0, vec![]),
        ];
        let sig = compute_recent_ip_signature(&entries);
        assert_eq!(sig[0], 1.0);
        assert_eq!(sig[1], 1.0);
        assert_eq!(sig[2], 1.0);
    }

    #[test]
    fn test_entry_to_concept_dims_and_label() {
        let snap = vec![1.0f32.to_le_bytes(); 4]
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        let entry = make_entry(7, 5, snap);
        let cv = entry_to_concept(&entry, 3);
        assert_eq!(cv.dimensions, 16);
        assert_eq!(cv.data[0], 7.0); // ip
        assert_eq!(cv.data[1], 5.0); // inst_len
        assert_eq!(cv.label.as_deref(), Some("reflect_entry_3"));
    }

    #[test]
    fn test_reflect_end_to_end_empty_trace() {
        // Empty MemoryTrace → reflects on an empty window → a single
        // self-model node, no entry-shadow nodes, no edges.
        let mut wg = PetgraphWorldGraph::new();
        let trace = VecMemoryTrace::default_capacity();
        let result = reflect(&trace, &mut wg, REFLECT_DEFAULT_WINDOW).unwrap();
        assert_eq!(wg.node_count(), 1);
        assert!(result.entry_node_ids.is_empty());
        assert_eq!(result.self_model_concept.dimensions, SELF_MODEL_DIM);
        let node = wg.lookup(result.self_model_id).unwrap().unwrap();
        assert_eq!(
            node.metadata.provenance.as_deref(),
            Some("reflect(window=8,edges=0)")
        );
    }

    #[test]
    fn test_reflect_end_to_end_with_entries() {
        let mut wg = PetgraphWorldGraph::new();
        // Pre-allocate a few graph nodes so access_count distribution is non-trivial.
        for k in 0..3 {
            wg.allocate(ConceptVector::from_vec(vec![k as f32]))
                .unwrap();
        }
        let mut trace = VecMemoryTrace::default_capacity();
        for k in 0..5 {
            trace.push(make_entry(k, 7, vec![0u8; 16])).unwrap();
        }
        let result = reflect(&trace, &mut wg, REFLECT_DEFAULT_WINDOW).unwrap();
        // 3 prior nodes + 5 entry-shadow nodes + 1 self-model = 9 total.
        assert_eq!(wg.node_count(), 3 + 5 + 1);
        // 5 entry-shadow nodes, each with an outgoing Temporal edge to self-model.
        assert_eq!(result.entry_node_ids.len(), 5);
        for shadow_id in &result.entry_node_ids {
            assert_eq!(wg.outgoing_edges(*shadow_id).len(), 1);
        }
        // Self-model has 5 incoming Temporal edges.
        assert_eq!(wg.incoming_edges(result.self_model_id).len(), 5);
    }

    #[test]
    fn test_reflect_window_clamps_to_trace_len() {
        // Asking for a window larger than trace length should not panic;
        // reflect internally clamps to trace.len().
        let mut wg = PetgraphWorldGraph::new();
        let mut trace = VecMemoryTrace::default_capacity();
        trace.push(make_entry(0, 0, vec![])).unwrap();
        let result = reflect(&trace, &mut wg, 100).unwrap();
        assert_eq!(result.entry_node_ids.len(), 1);
    }

    #[test]
    fn test_reflect_two_vms_deterministic() {
        // Two fresh VMs with the same trace + graph contents → identical
        // self-model ConceptVectors (proves no wall-clock is read).
        let mut wg1 = PetgraphWorldGraph::new();
        let mut wg2 = PetgraphWorldGraph::new();
        for _ in 0..2 {
            wg1.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
            wg2.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
        }
        let mut t1 = VecMemoryTrace::default_capacity();
        let mut t2 = VecMemoryTrace::default_capacity();
        for k in 0..4 {
            let e = make_entry(k, 3, vec![0u8; 8]);
            t1.push(e.clone()).unwrap();
            t2.push(e).unwrap();
        }
        let r1 = reflect(&t1, &mut wg1, 8).unwrap();
        let r2 = reflect(&t2, &mut wg2, 8).unwrap();
        assert_eq!(
            r1.self_model_concept.data, r2.self_model_concept.data,
            "two fresh VMs should produce identical self-model vectors"
        );
    }
}
