// evolve operator: Time-step the VM — advance world state.
// See plans/03-ccs-vm.md §4
//
// Signature: (&mut WorldGraph, &mut StateField, Duration) -> Result<(), EvolveError>
//
// Phase 2 deterministic semantics:
// - attention *= 0.95 (element-wise)
// - temporal: roll right by 1; temporal[0] decremented by dt.as_secs_f32()
// - belief: Blake3-seeded LCG over scratch[0..8] perturbs each belief[i]
// - WorldGraph metadata.access_count += 1 for every node
//
// Determinism: no wall-clock reads. All randomness in belief-drift derives
// from `dt` and the current `scratch[0..8]` LCG state. Two runs of the same
// VM with identical step sequences + dt values produce byte-identical state.

use std::time::Duration;

use a2x_core::graph::WorldGraph;
use a2x_core::state::StateField;
use blake3::Hasher;

/// Per-step decay factor applied to the `attention` region.
pub const ATTENTION_DECAY: f32 = 0.95;
/// Drift rate applied to `belief` from the Blake3-seeded LCG state.
pub const BELIEF_DRIFT_EPSILON: f32 = 0.001;
/// Number of f32 slots in `scratch` that carry the 32-byte LCG state.
const LCG_STATE_SLOTS: usize = 8;
const F32_BYTES: usize = 4;

/// Time-step the VM.
///
/// Pipeline (in order):
///   1. Decay attention (every f32 *= 0.95).
///   2. Shift temporal (roll right + decrement slot[0] by `dt`).
///   3. Drift belief via Blake3-seeded LCG state in `scratch[0..8]`.
///   4. Bump `metadata.access_count` for every node in the WorldGraph.
pub fn evolve(
    graph: &mut dyn WorldGraph,
    state: &mut dyn StateField,
    dt: Duration,
) -> Result<(), EvolveError> {
    decay_attention(state)?;
    shift_temporal(state, dt)?;
    drift_belief(state)?;
    bump_all_access_counts(graph)?;
    Ok(())
}

/// Multiply every f32 in `attention` by `ATTENTION_DECAY`.
fn decay_attention(state: &mut dyn StateField) -> Result<(), EvolveError> {
    let cur = read(state, "attention")?;
    let next: Vec<f32> = cur.iter().map(|v| v * ATTENTION_DECAY).collect();
    write(state, "attention", &next)
}

/// Roll `temporal` right by 1; decrement slot[0] by `dt.as_secs_f32()`.
///
/// After evolve: `next[1] = cur[0]`, `next[2] = cur[1]`, ..., `next[63] = cur[62]`.
/// `next[0] = cur[0] - dt_secs`. The freshest slot tracks accumulated dt;
/// the rolling window carries the prior 63 slot values.
fn shift_temporal(state: &mut dyn StateField, dt: Duration) -> Result<(), EvolveError> {
    let cur = read(state, "temporal")?;
    let mut next = vec![0.0f32; cur.len()];
    if !cur.is_empty() {
        next[1..].copy_from_slice(&cur[..cur.len() - 1]);
        next[0] = cur[0] - dt.as_secs_f32();
    }
    write(state, "temporal", &next)
}

/// Drive `belief` via a Blake3-seeded LCG state stored in `scratch[0..8]`.
///
/// Per call:
///   digest = blake3(lcg_state_bytes)            // 32-byte output
///   new_scratch[slot] = safe_f32_from_bits(digest[slot*4..slot*4+4])
///     (NaN-safe: bit-pattern that decodes NaN is replaced with 0)
///   belief[i] += new_scratch[i % 8] * BELIEF_DRIFT_EPSILON
fn drift_belief(state: &mut dyn StateField) -> Result<(), EvolveError> {
    let scratch = read(state, "scratch")?;
    let lcg_state_bytes = lcg_state_as_bytes(&scratch);
    let digest = {
        let mut hasher = Hasher::new();
        hasher.update(&lcg_state_bytes);
        hasher.finalize().as_bytes().to_vec()
    };

    let mut new_scratch = scratch.clone();
    for slot in 0..LCG_STATE_SLOTS.min(new_scratch.len()) {
        let bytes_idx = slot * F32_BYTES;
        let bits = u32::from_le_bytes([
            digest[bytes_idx],
            digest[bytes_idx + 1],
            digest[bytes_idx + 2],
            digest[bytes_idx + 3],
        ]);
        new_scratch[slot] = safe_f32_from_bits(bits);
    }
    write(state, "scratch", &new_scratch)?;

    let belief = read(state, "belief")?;
    let mut new_belief = belief.clone();
    for (i, v) in new_belief.iter_mut().enumerate() {
        let slot = i % LCG_STATE_SLOTS;
        *v += new_scratch[slot] * BELIEF_DRIFT_EPSILON;
    }
    write(state, "belief", &new_belief)
}

/// Increment `access_count` for every node currently in the graph.
///
/// Semantics choice: **global tick heartbeat** — every node in the graph gets
/// `+1` per `evolve` step, regardless of whether the node "actively" did
/// anything this tick. This treats `access_count` as a "how many evolve ticks
/// has this node survived" counter (monotonic with VM uptime).
///
/// Alternative semantics considered (rejected for Phase 2 simplicity): bump
/// only the operands of the just-executed operator + the freshly-allocated
/// result. That would measure "operational participation" instead, which is
/// noisier (depends on operator mix) and harder to compare across runs. The
/// heartbeat reading is monotone with `evolve` count — simpler invariant.
fn bump_all_access_counts(graph: &mut dyn WorldGraph) -> Result<(), EvolveError> {
    let ids = graph.node_ids();
    for id in ids {
        graph
            .bump_access_count(id)
            .map_err(|e| EvolveError::GraphError(e.to_string()))?;
    }
    Ok(())
}

fn read(state: &mut dyn StateField, region: &str) -> Result<Vec<f32>, EvolveError> {
    state
        .read_region(region)
        .map(|s| s.to_vec())
        .map_err(|e| EvolveError::StateError(e.to_string()))
}

fn write(state: &mut dyn StateField, region: &str, data: &[f32]) -> Result<(), EvolveError> {
    state
        .write_region(region, data)
        .map_err(|e| EvolveError::StateError(e.to_string()))
}

/// Reinterpret the first `LCG_STATE_SLOTS` f32 slots of `scratch` as a
/// 32-byte Blake3 input buffer. Returns a fixed-size array regardless of the
/// scratch length — excess slots become zero-padding if scratch is short.
fn lcg_state_as_bytes(scratch: &[f32]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let slots_used = LCG_STATE_SLOTS.min(scratch.len());
    for (slot, slot_val) in scratch.iter().enumerate().take(slots_used) {
        let slot_bytes = slot_val.to_le_bytes();
        let start = slot * F32_BYTES;
        buf[start..start + F32_BYTES].copy_from_slice(&slot_bytes);
    }
    buf
}

/// Convert a u32 bit pattern to f32; if NaN, return 0 to keep downstream
/// math well-defined. (Blake3 output bits treated as f32 can occasionally
/// produce NaN; this prevents belief from being silently poisoned.)
fn safe_f32_from_bits(bits: u32) -> f32 {
    let f = f32::from_bits(bits);
    if f.is_nan() {
        0.0
    } else {
        f
    }
}

/// Error during evolve operation.
#[derive(Clone, Debug, PartialEq)]
pub enum EvolveError {
    /// WorldGraph error during evolution.
    GraphError(String),
    /// StateField error during evolution.
    StateError(String),
}

impl std::fmt::Display for EvolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvolveError::GraphError(msg) => write!(f, "evolve graph error: {}", msg),
            EvolveError::StateError(msg) => write!(f, "evolve state error: {}", msg),
        }
    }
}

impl std::error::Error for EvolveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{init_default_regions, FlatStateField};
    use crate::world_graph::PetgraphWorldGraph;
    use a2x_core::concept::ConceptVector;

    fn setup() -> (PetgraphWorldGraph, FlatStateField) {
        let wg = PetgraphWorldGraph::new();
        let mut sf = FlatStateField::default_size();
        init_default_regions(&mut sf).unwrap();
        (wg, sf)
    }

    #[test]
    fn test_evolve_attention_decays() {
        let (mut wg, mut sf) = setup();
        let ones = vec![1.0f32; 128];
        sf.write_region("attention", &ones).unwrap();
        evolve(&mut wg, &mut sf, Duration::from_secs(1)).unwrap();
        let after = sf.read_region("attention").unwrap();
        for v in after {
            assert!((v - ATTENTION_DECAY).abs() < 1e-6);
        }
    }

    #[test]
    fn test_evolve_attention_double_decay() {
        // Two evolutions → attention ~= 0.95 * 0.95 = 0.9025
        let (mut wg, mut sf) = setup();
        let ones = vec![1.0f32; 128];
        sf.write_region("attention", &ones).unwrap();
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        let after = sf.read_region("attention").unwrap();
        let expected = ATTENTION_DECAY * ATTENTION_DECAY;
        for v in after {
            assert!((v - expected).abs() < 1e-6, "{} vs {}", v, expected);
        }
    }

    #[test]
    fn test_evolve_temporal_rolls_right() {
        let (mut wg, mut sf) = setup();
        // Set up recognizable temporal: [0, 1, 2, ..., 63]
        let setup_vals: Vec<f32> = (0..64).map(|i| i as f32).collect();
        sf.write_region("temporal", &setup_vals).unwrap();
        // dt = 0 so temporal[0] stays 0 (was 0, dt makes 0)
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        let after = sf.read_region("temporal").unwrap();
        // After evolve:
        //   after[0] = 0 - 0 = 0
        //   after[1] = old[0] = 0
        //   after[2] = old[1] = 1
        //   after[3] = old[2] = 2
        //   after[63] = old[62] = 62
        assert_eq!(after[0], 0.0);
        assert_eq!(after[1], 0.0);
        assert_eq!(after[2], 1.0);
        assert_eq!(after[3], 2.0);
        assert_eq!(after[63], 62.0);
    }

    #[test]
    fn test_evolve_temporal_decrements_top_slot() {
        let (mut wg, mut sf) = setup();
        let mut setup_vals = vec![0.0f32; 64];
        setup_vals[0] = 5.0;
        sf.write_region("temporal", &setup_vals).unwrap();
        evolve(&mut wg, &mut sf, Duration::from_secs(1)).unwrap();
        let after = sf.read_region("temporal").unwrap();
        // temporal[0] = 5.0 - 1.0 = 4.0
        assert!((after[0] - 4.0).abs() < 1e-6, "{} vs 4.0", after[0]);
    }

    #[test]
    fn test_evolve_bumps_access_count_for_every_node() {
        let (mut wg, mut sf) = setup();
        let a = wg.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
        let b = wg.allocate(ConceptVector::from_vec(vec![2.0])).unwrap();
        let c = wg.allocate(ConceptVector::from_vec(vec![3.0])).unwrap();
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        assert_eq!(wg.lookup(a).unwrap().unwrap().metadata.access_count, 1);
        assert_eq!(wg.lookup(b).unwrap().unwrap().metadata.access_count, 1);
        assert_eq!(wg.lookup(c).unwrap().unwrap().metadata.access_count, 1);
    }

    #[test]
    fn test_evolve_bumps_access_count_saturating() {
        // Three evolutions on the same node → access_count = 3 (no overflow)
        let (mut wg, mut sf) = setup();
        let a = wg.allocate(ConceptVector::from_vec(vec![1.0])).unwrap();
        for _ in 0..3 {
            evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        }
        assert_eq!(wg.lookup(a).unwrap().unwrap().metadata.access_count, 3);
    }

    #[test]
    fn test_evolve_deterministic_two_vms() {
        let (mut wg1, mut sf1) = setup();
        let (mut wg2, mut sf2) = setup();
        evolve(&mut wg1, &mut sf1, Duration::from_millis(10)).unwrap();
        evolve(&mut wg2, &mut sf2, Duration::from_millis(10)).unwrap();
        let b1 = sf1.read_region("belief").unwrap().to_vec();
        let b2 = sf2.read_region("belief").unwrap().to_vec();
        assert_eq!(
            b1, b2,
            "two fresh VMs running evolve should produce identical belief"
        );
        let s1 = sf1.read_region("scratch").unwrap().to_vec();
        let s2 = sf2.read_region("scratch").unwrap().to_vec();
        assert_eq!(
            s1, s2,
            "scratch LCG state should be deterministic across separate runs"
        );
    }

    #[test]
    fn test_evolve_lcg_state_advances() {
        let (mut wg, mut sf) = setup();
        // Initial scratch is all zeros
        let before: Vec<f32> = sf.read_region("scratch").unwrap().to_vec();
        assert!(before.iter().all(|v| *v == 0.0));
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        let after = sf.read_region("scratch").unwrap().to_vec();
        // After first evolve, scratch[0..8] should be blake3(zero-bytes) reinterpreted.
        // Very high probability that some slot became non-zero (deterministic bits).
        let non_zero_slots = after[..LCG_STATE_SLOTS]
            .iter()
            .filter(|v| **v != 0.0)
            .count();
        assert!(
            non_zero_slots >= 1,
            "expected at least one LCG slot to be non-zero after blake3(zeros); got {} of {}",
            non_zero_slots,
            LCG_STATE_SLOTS
        );
    }

    #[test]
    fn test_evolve_belief_initial_drift() {
        let (mut wg, mut sf) = setup();
        let before_belief = sf.read_region("belief").unwrap().to_vec();
        evolve(&mut wg, &mut sf, Duration::ZERO).unwrap();
        let after_belief = sf.read_region("belief").unwrap().to_vec();
        let diffs = before_belief
            .iter()
            .zip(&after_belief)
            .filter(|(a, b)| a != b)
            .count();
        assert!(diffs > 0, "belief should drift on first evolve");
    }

    #[test]
    fn test_evolve_5_step_deterministic() {
        // Five evolutions on two fresh VMs must produce identical belief.
        let (mut wg1, mut sf1) = setup();
        let (mut wg2, mut sf2) = setup();
        for _ in 0..5 {
            evolve(&mut wg1, &mut sf1, Duration::from_millis(10)).unwrap();
            evolve(&mut wg2, &mut sf2, Duration::from_millis(10)).unwrap();
        }
        assert_eq!(
            sf1.read_region("belief").unwrap(),
            sf2.read_region("belief").unwrap(),
        );
        assert_eq!(
            sf1.read_region("attention").unwrap(),
            sf2.read_region("attention").unwrap(),
        );
        assert_eq!(
            sf1.read_region("temporal").unwrap(),
            sf2.read_region("temporal").unwrap(),
        );
    }
}
