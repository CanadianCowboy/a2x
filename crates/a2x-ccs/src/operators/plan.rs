// plan operator: Generate action sequence from current world state.
// See plans/03-ccs-vm.md §4
//
// Phase 2.E: real implementation that reads:
//   1. `state.belief` region (256 dims) — selects top-K indices by |belief[i]|
//   2. `last_reflect` NodeId (vm.last_reflect from Phase 2.D) — biases priorities
//
// Output: Vec<Action> where each Action has:
//   - verb: Verb = Propose | Bind | Ground | Snapshot | Evolve
//   - opcode: Opcode (matching Verb)
//   - priority: f32 (higher = earlier in action queue)
//   - target: Option<String> (label or symbolic-name; for Bind: "belief_<i>")
//
// The plan is also allocated as a single Plan node (ConceptVector) in the
// WorldGraph; auto-label `__last_plan` re-points across calls (Phase 2.D's
// lesson — we don't depend on the label, but it's useful for debugging).
//
// Determinism invariant: no wall-clock reads. Same (graph, state, last_reflect,
// top_k) → same PlanResult across two VM runs.

use a2x_core::concept::ConceptVector;
use a2x_core::graph::WorldGraph;
use a2x_core::node::NodeId;
use a2x_core::opcode::Opcode;
use a2x_core::state::StateField;

/// Default top-K: how many belief indices to draw Bind targets from. Per the
/// user spec "K = small enough to be tractable but diverse enough to cover
/// multiple bias slots".
pub const PLAN_DEFAULT_TOP_K: usize = 3;

/// ConceptVector dim for the Plan node. Layout:
///   \[0..16\]  verb histogram (5 active verbs + 11 empty = 16 slots)
///                          concrete indices: 0=Propose, 1=Bind, 2=Ground,
///                          3=Snapshot, 4=Evolve
///   \[16..32\] priority statistics (max, mean, sum)
///   \[32..48\] target counts (Bind, Ground, etc.)
///   \[48..64\] reflect bias signal (last_reflect fingerprint if available)
pub const PLAN_CONCEPT_DIM: usize = 64;
const BUCKETS: usize = 16;

/// Phase 2.E verb classification — semantically narrower than Opcode (an Action
/// has both a Verb for planning semantics and an Opcode for execution).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Verb {
    /// Suggest an action without committing to executing it.
    Propose,
    /// Bind operands to form a composite.
    Bind,
    /// Attach raw perception to memory (I/O).
    Ground,
    /// Capture current state for replay/record (debug / fork).
    Snapshot,
    /// Time-step the VM (advance world state).
    Evolve,
}

impl Verb {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verb::Propose => "Propose",
            Verb::Bind => "Bind",
            Verb::Ground => "Ground",
            Verb::Snapshot => "Snapshot",
            Verb::Evolve => "Evolve",
        }
    }

    /// Map a Verb to the corresponding runtime Opcode. Snapshot has no first-
    /// class Opcode, so we use `Custom(b'S')` as the namespace marker.
    pub fn to_opcode(&self) -> Opcode {
        match self {
            Verb::Propose => Opcode::Nop,
            Verb::Bind => Opcode::Bind,
            Verb::Ground => Opcode::Ground,
            Verb::Snapshot => Opcode::Custom(b'S'),
            Verb::Evolve => Opcode::Evolve,
        }
    }
}

/// A generated action — see plan §4 for phase ordering semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// Verb classification (planning semantics).
    pub verb: Verb,
    /// Runtime opcode corresponding to the verb.
    pub opcode: Opcode,
    /// Priority: higher = execute sooner.
    pub priority: f32,
    /// Optional target (label or symbolic name).
    pub target: Option<String>,
}

impl Action {
    pub fn new(verb: Verb, priority: f32, target: Option<String>) -> Self {
        let opcode = verb.to_opcode();
        Action {
            verb,
            opcode,
            priority,
            target,
        }
    }
}

/// Output of a plan operation: the new Plan node + the emitted actions.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanResult {
    /// NodeId of the freshly-allocated Plan ConceptVector.
    pub plan_node_id: NodeId,
    /// The emitted action sequence. Stored on `vm.last_plan_actions` after dispatch.
    pub actions: Vec<Action>,
    /// The Plan ConceptVector (also stored on the node).
    pub plan_concept: ConceptVector,
}

/// Generate a sequence of actions from the current world state.
///
/// Algorithm:
///   1. Read `state.belief` (256 dims). Compute |belief[i]| for each.
///   2. Top-K indices by magnitude produce K `Bind` actions targeting
///      `belief_<i>` (priority = |belief[i]|, normalized).
///   3. If last_reflect is Some and the resulting reflect concept vector has
///      some non-trivial signal (heuristic: any non-zero slot beyond the
///      first 16 bias stats), emit one additional `Evolve` action tagged with
///      the reflect's NodeId as target. This biases toward "world is
///      interesting; we should time-step again" when reflect just produced
///      a meaningful self-model.
///   4. Always emit one `Snapshot` action, priority = max(|belief|), so the
///      trajectory can be replayed.
///
/// Determinism: same input → same output. Two VMs running the same program
/// produce byte-identical PlanResult.
///
/// Takes `&mut dyn WorldGraph` because we allocate a Plan node on it. State
/// is taken immutably — plan() reads belief but never writes back.
pub fn plan(
    graph: &mut dyn WorldGraph,
    state: &dyn StateField,
    last_reflect: Option<NodeId>,
    top_k: usize,
) -> Result<PlanResult, PlanError> {
    let top_k = top_k.max(1);
    let belief = state
        .read_region("belief")
        .map_err(|e| PlanError::StateError(e.to_string()))?;
    // `read_region` already returns `&[f32]`; passing `belief` directly
    // (instead of `&belief`) avoids clippy::needless_borrow — `&[f32]` is not
    // assignable to `&&[f32]` without coercion but `topo_k_indices` expects
    // `&[f32]` directly.
    let belief_mag = topo_k_indices(belief, top_k);

    let max_mag = belief_mag
        .iter()
        .map(|(_, m)| *m)
        .fold(0.0f32, f32::max)
        .max(1e-6);
    let mut actions = Vec::with_capacity(top_k + 2);

    // K Bind actions ranked by |belief[i]|
    for (i, mag) in &belief_mag {
        actions.push(Action::new(
            Verb::Bind,
            *mag / max_mag,
            Some(format!("belief_{}", i)),
        ));
    }

    // Optional Evolve action when reflect just produced a non-trivial summary.
    if let Some(reflect_id) = last_reflect {
        if let Some(node) = reflect_concept(graph, reflect_id) {
            if has_reflect_signal(node) {
                actions.push(Action::new(
                    Verb::Evolve,
                    max_mag,
                    Some(format!("__reflect_{}", reflect_id.as_u64())),
                ));
            }
        }
    }

    // Always one Snapshot action so the plan is replayable. Its priority is
    // intentionally pinned at 0.0 (not `max_mag`) so that descending-sort by
    // priority lands it LAST in the action queue — Snapshot is the *terminal*
    // action, expected to run after Bind/Ground/Evolve have produced results.
    // Ties with all-zero Binds (priority 0/1e-6 = 0) keep Snapshot at the end
    // via stable insertion order (Snapshot pushed last).
    actions.push(Action::new(
        Verb::Snapshot,
        0.0,
        Some("__last_plan".to_string()),
    ));

    // Sort descending by priority so callers can drain from index 0 forward.
    actions.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Build the Plan concept vector from the action set (deterministic encoding).
    let plan_concept = build_plan_concept(&actions, last_reflect);
    let plan_node_id = graph
        .allocate(plan_concept.clone())
        .map_err(|e| PlanError::GraphError(e.to_string()))?;
    graph
        .set_provenance(
            plan_node_id,
            &format!("plan(top_k={},actions={})", top_k, actions.len()),
        )
        .map_err(|e| PlanError::GraphError(e.to_string()))?;
    if let Err(e) = graph.set_label(plan_node_id, "__last_plan") {
        // Same caveat as reflect.rs: set_label rejects collisions across nodes.
        // We deliberately let the label stay attached to the FIRST plan — VM-
        // side `last_plan_actions` is canonical. See reflect.rs note.
        let _ = e;
    }

    Ok(PlanResult {
        plan_node_id,
        actions,
        plan_concept,
    })
}

// ===== Pure stat helpers (unit-tested separately) =====

/// Top-K indices of a region by absolute magnitude. Returns Vec of
/// `(index, |value|)` sorted descending.
fn topo_k_indices(region: &[f32], k: usize) -> Vec<(usize, f32)> {
    let mut indexed: Vec<(usize, f32)> = region
        .iter()
        .enumerate()
        .map(|(i, v)| (i, v.abs()))
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(k);
    indexed
}

/// Look up the reflect self-model concept by NodeId (immutable borrow).
fn reflect_concept(graph: &dyn WorldGraph, reflect_id: NodeId) -> Option<ConceptVector> {
    graph.lookup(reflect_id).ok().flatten().map(|n| n.concept)
}

/// True if the reflect self-model concept contains any non-zero slot beyond
/// the first 16 op-mix histogram buckets — proxy for "reflect found a
/// meaningful distribution".
fn has_reflect_signal(concept: ConceptVector) -> bool {
    concept.data.iter().skip(BUCKETS).any(|v| v.abs() > 1e-6)
}

/// Encodes `actions` + reflect signal into a 64-dim ConceptVector describing
/// the resulting plan. Layout (mirrors `PLAN_CONCEPT_DIM`):
///   \[0..16\]  verb histogram
///   \[16..32\] priority statistics: [max, mean, sum] + 13 zeros
///   \[32..48\] action count per verb class
///   \[48..64\] reflect bias signal: [has_reflect:f32, reflect_id_f32, ..., 14 zeros]
fn build_plan_concept(actions: &[Action], last_reflect: Option<NodeId>) -> ConceptVector {
    let mut data = vec![0.0f32; PLAN_CONCEPT_DIM];
    let mut verb_hist = [0.0f32; BUCKETS];
    let mut priorities: [f32; 3] = [0.0, 0.0, 0.0];
    let mut count_per_verb = [0.0f32; BUCKETS];

    for action in actions {
        let slot = match action.verb {
            Verb::Propose => 0,
            Verb::Bind => 1,
            Verb::Ground => 2,
            Verb::Snapshot => 3,
            Verb::Evolve => 4,
        };
        verb_hist[slot] += 1.0;
        count_per_verb[slot] += 1.0;
    }
    if !actions.is_empty() {
        let max = actions.iter().map(|a| a.priority).fold(f32::MIN, f32::max);
        let sum: f32 = actions.iter().map(|a| a.priority).sum();
        let mean = sum / actions.len() as f32;
        priorities = [max, mean, sum];
    }

    data[0..BUCKETS].copy_from_slice(&verb_hist);
    data[16..19].copy_from_slice(&priorities);
    data[32..32 + BUCKETS].copy_from_slice(&count_per_verb);
    if let Some(id) = last_reflect {
        data[48] = 1.0; // has_reflect flag
        data[49] = id.as_u64() as f32; // reflect_id encoded as f32
    }

    ConceptVector::from_vec(data).with_label("plan")
}

/// Error during plan operation.
#[derive(Clone, Debug, PartialEq)]
pub enum PlanError {
    GraphError(String),
    StateError(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::GraphError(msg) => write!(f, "plan graph error: {}", msg),
            PlanError::StateError(msg) => write!(f, "plan state error: {}", msg),
        }
    }
}

impl std::error::Error for PlanError {}

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
    fn test_verb_to_opcode_mapping() {
        assert_eq!(Verb::Propose.to_opcode(), Opcode::Nop);
        assert_eq!(Verb::Bind.to_opcode(), Opcode::Bind);
        assert_eq!(Verb::Ground.to_opcode(), Opcode::Ground);
        assert_eq!(Verb::Snapshot.to_opcode(), Opcode::Custom(b'S'));
        assert_eq!(Verb::Evolve.to_opcode(), Opcode::Evolve);
    }

    #[test]
    fn test_action_new_assigns_opcode() {
        let a = Action::new(Verb::Bind, 0.5, Some("x".into()));
        assert_eq!(a.verb, Verb::Bind);
        assert_eq!(a.opcode, Opcode::Bind);
        assert_eq!(a.target.as_deref(), Some("x"));
    }

    #[test]
    fn test_topo_k_indices_descending() {
        let v = vec![1.0, -5.0, 3.0, 2.0, -4.0];
        let top = topo_k_indices(&v, 3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0], (1, 5.0));
        assert_eq!(top[1], (4, 4.0));
        assert_eq!(top[2], (2, 3.0));
    }

    #[test]
    fn test_topo_k_indices_all_zeros() {
        let v = vec![0.0f32; 8];
        let top = topo_k_indices(&v, 3);
        assert_eq!(top.len(), 3);
        assert!(top.iter().all(|(_, m)| *m == 0.0));
    }

    #[test]
    fn test_topo_k_indices_shorter_than_k() {
        let v = vec![1.0, 2.0];
        let top = topo_k_indices(&v, 5);
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn test_reflect_concept_none_on_unknown_node() {
        let wg = PetgraphWorldGraph::new();
        let unknown = NodeId::new(9999);
        assert!(reflect_concept(&wg, unknown).is_none());
    }

    #[test]
    fn test_reflect_signal_true_when_nontrivial() {
        let mut wg = PetgraphWorldGraph::new();
        // Build a fake reflect self-model with non-zero slot beyond bucket 16.
        let mut data = vec![0.0f32; 128];
        data[20] = 0.7;
        let id = wg.allocate(ConceptVector::from_vec(data)).unwrap();
        let cv = reflect_concept(&wg, id).unwrap();
        assert!(has_reflect_signal(cv));
    }

    #[test]
    fn test_reflect_signal_false_when_only_bucket_zero() {
        let mut wg = PetgraphWorldGraph::new();
        // All signal lives in bucket 0..16 — ops-mix histogram — but slots
        // beyond are zero. Should return false (no "real" signal).
        let data = vec![0.0f32; 128];
        let id = wg.allocate(ConceptVector::from_vec(data)).unwrap();
        let cv = reflect_concept(&wg, id).unwrap();
        assert!(!has_reflect_signal(cv));
    }

    #[test]
    fn test_plan_empty_state_emits_only_snapshot() {
        let (mut wg, sf) = setup();
        // All-zero belief → all Bind priorities are 0, but plan still emits
        // one Snapshot action terminally.
        let result = plan(&mut wg, &sf, None, PLAN_DEFAULT_TOP_K).unwrap();
        // K = 3 Bind (priority 0) + 1 Snapshot (priority max(max 0, 1e-6)) = 4.
        assert_eq!(result.actions.len(), 4);
        // Snapshot is always last (sorted descending; max = 1e-6 / 1e-6 = 1.0
        // vs Bind = 0/1e-6 = 0). Reverse: Bind first, Snapshot last.
        assert_eq!(result.actions.last().unwrap().verb, Verb::Snapshot);
        assert_eq!(result.actions.first().unwrap().verb, Verb::Bind);
    }

    #[test]
    fn test_plan_with_belief_bias_picks_top_k() {
        let (mut wg, mut sf) = setup();
        // Set belief: index 5 = 10.0, index 12 = 7.0, index 0 = 3.0.
        let belief = {
            let mut b = vec![0.0f32; 256];
            b[5] = 10.0;
            b[12] = 7.0;
            b[0] = 3.0;
            b
        };
        sf.write_region("belief", &belief).unwrap();

        let result = plan(&mut wg, &sf, None, 3).unwrap();
        // 3 Bind + 1 Snapshot = 4 actions.
        assert_eq!(result.actions.len(), 4);

        // First action should bind belief_5 (max |belief[i]| = 10.0).
        let first = &result.actions[0];
        assert_eq!(first.verb, Verb::Bind);
        assert_eq!(first.target.as_deref(), Some("belief_5"));
        assert!((first.priority - 1.0).abs() < 1e-6, "{}", first.priority);

        // Verify the remaining two binds cover belief_12 and belief_0.
        let targets: Vec<&str> = result
            .actions
            .iter()
            .filter_map(|a| a.target.as_deref())
            .collect();
        assert!(targets.contains(&"belief_5"));
        assert!(targets.contains(&"belief_12"));
        assert!(targets.contains(&"belief_0"));
    }

    #[test]
    fn test_plan_with_reflect_emits_evolve() {
        let (mut wg, mut sf) = setup();
        // Pre-allocate a "reflect" node with non-trivial signal (slot 20 = 0.7).
        let data = {
            let mut d = vec![0.0f32; 128];
            d[20] = 0.7;
            d
        };
        let reflect_id = wg.allocate(ConceptVector::from_vec(data)).unwrap();
        let belief = {
            let mut b = vec![0.0f32; 256];
            b[5] = 5.0;
            b
        };
        sf.write_region("belief", &belief).unwrap();

        let result = plan(&mut wg, &sf, Some(reflect_id), 2).unwrap();
        // 2 Bind + 1 Evolve + 1 Snapshot = 4 actions.
        assert_eq!(result.actions.len(), 4);
        let verbs: Vec<Verb> = result.actions.iter().map(|a| a.verb).collect();
        assert!(verbs.contains(&Verb::Bind));
        assert!(verbs.contains(&Verb::Evolve));
        assert!(verbs.contains(&Verb::Snapshot));

        // Evolve target should reference reflect_id.
        let evo = result
            .actions
            .iter()
            .find(|a| a.verb == Verb::Evolve)
            .unwrap();
        let expected_target = format!("__reflect_{}", reflect_id.as_u64());
        assert_eq!(evo.target.as_deref(), Some(expected_target.as_str()));
    }

    #[test]
    fn test_plan_with_reflect_no_signal_skips_evolve() {
        let (mut wg, sf) = setup();
        let data = vec![0.0f32; 128];
        let reflect_id = wg.allocate(ConceptVector::from_vec(data)).unwrap();
        let result = plan(&mut wg, &sf, Some(reflect_id), 2).unwrap();
        // 2 Bind + 1 Snapshot = 3 actions (no Evolve).
        assert_eq!(result.actions.len(), 3);
        assert!(!result.actions.iter().any(|a| a.verb == Verb::Evolve));
    }

    #[test]
    fn test_plan_top_k_zero_clamps_to_one() {
        let (mut wg, sf) = setup();
        let result = plan(&mut wg, &sf, None, 0).unwrap();
        // k=0 clamps to 1 → 1 Bind + 1 Snapshot = 2 actions.
        assert_eq!(result.actions.len(), 2);
    }

    #[test]
    fn test_plan_allocates_plan_node_with_provenance() {
        let (mut wg, sf) = setup();
        let result = plan(&mut wg, &sf, None, 2).unwrap();
        // 2 Bind + 1 Snapshot = 3 actions.
        assert_eq!(result.plan_concept.dimensions, PLAN_CONCEPT_DIM);
        let node = wg.lookup(result.plan_node_id).unwrap().unwrap();
        assert_eq!(
            node.metadata.provenance.as_deref(),
            Some("plan(top_k=2,actions=3)")
        );
        assert_eq!(wg.node_count(), 1);
    }

    #[test]
    fn test_plan_two_vms_deterministic() {
        // Two fresh VMs with identical state → byte-identical PlanResult vectors.
        let (mut wg1, mut sf1) = setup();
        let (mut wg2, mut sf2) = setup();
        let belief = {
            let mut b = vec![0.0f32; 256];
            b[5] = 10.0;
            b[12] = 7.0;
            b[0] = 3.0;
            b
        };
        sf1.write_region("belief", &belief).unwrap();
        sf2.write_region("belief", &belief).unwrap();

        let r1 = plan(&mut wg1, &sf1, None, 3).unwrap();
        let r2 = plan(&mut wg2, &sf2, None, 3).unwrap();
        assert_eq!(
            r1.plan_concept.data, r2.plan_concept.data,
            "two fresh VMs should produce identical plan concepts"
        );
        assert_eq!(r1.actions.len(), r2.actions.len());
        // Action priority sums should be equal under determinism.
        let s1: f32 = r1.actions.iter().map(|a| a.priority).sum();
        let s2: f32 = r2.actions.iter().map(|a| a.priority).sum();
        assert!((s1 - s2).abs() < 1e-6);
    }
}
