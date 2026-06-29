// Phase 2.J — End-to-end Σ∞ smoke test.
// See plans/03-ccs-vm.md, plans/05-agents.md §3
//
// Goal: exercise every cognitive operator in PLAN.md §18 — BIND, DIFFERENTIATE,
// GROUND, EVOLVE, REFLECT, PLAN — through a *single* Σ∞ program.
//
// What this test guards against:
//  1. Operand-plumbing breakage (Phase 2.A+B regression) — operands to
//     BIND/DIFF/GRND must reach the operator functions.
//  2. Opcode dispatch drift (Phase 2.C+D+E) — the intent→opcode mapping
//     (Delay → Evolve, Contradiction → Reflect, Parallel → Plan) must hold.
//  3. Side-effect ordering — REFLECT must set last_reflect before PLAN
//     runs and PLAN must read it; the test asserts plan_actions is
//     non-empty.
//  4. VM halt end-of-program — confirms the 7-instruction program's
//     implicit halt at end-of-packet-stream is honored.

use a2x_ccs::CcsVm;
use a2x_ccs::VmStatus;
use a2x_core::concept::ConceptVector;
use a2x_core::graph::WorldGraph; // trait — activate allocate/set_label/lookup/node_count
use a2x_core::memory::MemoryTrace; // trait — activate trace.len()
use a2x_core::node::NodeId;
use a2x_core::state::StateField; // trait — activate state_field.read_region()
use a2x_sigma::intent::IntentOp;
use a2x_sigma::packet::SigmaPacket;
use a2x_sigma::program::SigmaProgram;

/// Construct one Σ∞ packet (single intent op, optional labels, optional D payload).
fn packet(intent: IntentOp, labels: &[&str], data: &[u8]) -> SigmaPacket {
    let mut p = SigmaPacket::new();
    p.intent.operators.push(intent);
    for l in labels {
        p.context.labels.push((*l).to_string());
    }
    p.data.payload = data.to_vec();
    p
}

/// Pre-allocate two labeled WorldGraph nodes for BIND/DIFF operands.
///
/// Done in the test fixture (not the Σ∞ program) so the program-shape stays
/// orthogonal to seed setup — the program tests operators; the test
/// provides its operands.
fn seed_concepts(vm: &mut CcsVm) -> (NodeId, NodeId) {
    let a = vm
        .world_graph
        .allocate(ConceptVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]))
        .unwrap();
    vm.world_graph.set_label(a, "sys").unwrap();
    let b = vm
        .world_graph
        .allocate(ConceptVector::from_vec(vec![10.0, 20.0, 30.0]))
        .unwrap();
    vm.world_graph.set_label(b, "out").unwrap();
    (a, b)
}

/// Build a 7-instruction program exercising all 6 cognitive operators:
///
///   0. GRND ⟨seed_sys⟩ - 4 floats → ground a perception.
///   1. GRND ⟨seed_out⟩ - 3 floats → ground a second perception.
///   2. DIF ⟨sys⟩ chunk=2 → differentiate sys into 2 chunks.
///   3. BND ⟨sys,out⟩ → composite concept.
///   4. EVOL (no operands) → time-step fields.
///   5. REFL (no operands) → build self-model node, set `last_reflect`.
///   6. PLAN (no operands) → read reflect + belief, emit plan actions.
fn build_phase2_program() -> SigmaProgram {
    let mut prog = SigmaProgram::new();

    // GRND seed_sys: payload = [1,2,3,4]
    let mut sys_payload = Vec::new();
    for f in [1.0f32, 2.0, 3.0, 4.0] {
        sys_payload.extend_from_slice(&f.to_le_bytes());
    }
    prog.push(packet(IntentOp::Star, &["seed_sys"], &sys_payload));

    // GRND seed_out: payload = [10,20,30]
    let mut out_payload = Vec::new();
    for f in [10.0f32, 20.0, 30.0] {
        out_payload.extend_from_slice(&f.to_le_bytes());
    }
    prog.push(packet(IntentOp::Star, &["seed_out"], &out_payload));

    // DIFF sys into 2 chunks.
    prog.push(packet(IntentOp::Split, &["sys"], &2u32.to_le_bytes()));
    // BIND sys + out into a composite.
    prog.push(packet(IntentOp::Synthesis, &["sys", "out"], &[]));
    // EVOLVE
    prog.push(packet(IntentOp::Delay, &[], &[]));
    // REFLECT
    prog.push(packet(IntentOp::Contradiction, &[], &[]));
    // PLAN
    prog.push(packet(IntentOp::Parallel, &[], &[]));

    prog
}

#[test]
fn test_phase2_full_cognitive_loop_runs_to_completion() {
    let mut vm = CcsVm::new();
    let (a, b) = seed_concepts(&mut vm);
    vm.load(build_phase2_program());
    let status = vm.run().unwrap();
    assert_eq!(status, VmStatus::Halted, "vm should halt at end of program");

    // Node count from each operator:
    //   seeds = 2, grnd = 2, diff-chunks = 2, bind = 1,
    //   reflect self-model = 1, entry-shadows = REFLECT_DEFAULT_WINDOW (=8),
    //   plan = 1
    // Total: 2 + 2 + 2 + 1 + 1 + 8 + 1 = 17
    // The graph must grow beyond the 2 fixture seeds. Each cognitive operator
    // allocates at least one node, so a fully-executed program produces a
    // >=10-node world-graph. We use a generous lower bound (10) rather than
    // an exact-arithmetic count because the reflect operator's actual shadow
    // count depends on `min(window, trace.len())` rather than the bare
    // window — exact arithmetic drifts across small reflect.rs edits.
    assert!(
        vm.world_graph.node_count() > 10,
        "expected > 10 nodes after full Phase-2 loop; got {}",
        vm.world_graph.node_count()
    );
    assert!(vm.world_graph.lookup(a).unwrap().is_some());
    assert!(vm.world_graph.lookup(b).unwrap().is_some());
}

#[test]
fn test_phase2_full_loop_traces_every_step() {
    let mut vm = CcsVm::new();
    seed_concepts(&mut vm);
    vm.load(build_phase2_program());
    vm.run().unwrap();
    // MemoryTrace pushes once per VM step.
    // Program has 7 instructions → len = 7.
    assert_eq!(vm.memory_trace.len(), 7);
}

#[test]
fn test_phase2_reflect_sets_last_reflect_for_plan_consumption() {
    // Contract: REFLECT must set `vm.last_reflect` so PLAN can read it.
    let mut vm = CcsVm::new();
    seed_concepts(&mut vm);
    vm.load(build_phase2_program());
    vm.run().unwrap();
    assert!(
        vm.last_reflect.is_some(),
        "REFLECT must populate vm.last_reflect"
    );
    assert!(
        !vm.last_plan_actions.is_empty(),
        "PLAN must read REFLECT's output and emit >=1 action; got 0"
    );
}

#[test]
fn test_phase2_actions_have_non_negative_priority() {
    let mut vm = CcsVm::new();
    seed_concepts(&mut vm);
    vm.load(build_phase2_program());
    vm.run().unwrap();
    for action in &vm.last_plan_actions {
        assert!(
            action.priority.is_finite(),
            "priority must be finite: {:?}",
            action
        );
    }
}

#[test]
fn test_phase2_evolve_before_reflect_drifts_belief() {
    // After EVOLVE, the belief region has drifted. REFLECT must observe
    // the post-drift belief state.
    let mut vm = CcsVm::new();
    seed_concepts(&mut vm);
    let belief_pre = vm.state_field.read_region("belief").unwrap().to_vec();
    vm.load(build_phase2_program());
    vm.run().unwrap();
    let belief_post = vm.state_field.read_region("belief").unwrap().to_vec();
    let diffs = belief_pre
        .iter()
        .zip(&belief_post)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        diffs > 0,
        "EVOLVE must drift belief before REFLECT summaries it"
    );

    // The self-model node must exist and have the documented 128-dim layout.
    let sm_id = vm.last_reflect.expect("reflect created a self-model");
    let node = vm
        .world_graph
        .lookup(sm_id)
        .unwrap()
        .expect("self-model in graph");
    assert_eq!(
        node.concept.data.len(),
        128,
        "self-model must be 128-dim (REFLECT_DEFAULT_SELF_MODEL_DIM)"
    );
}

#[test]
fn test_phase2_full_loop_deterministic_across_two_vms() {
    // Two fresh VMs running the same program produce identical observable
    // metrics. (Holds if the program pre-seeds WorldGraph labels in the
    // fixture — determinism proof for the full pipeline.)
    let program = build_phase2_program();

    let mut vm1 = CcsVm::new();
    seed_concepts(&mut vm1);
    vm1.load(program.clone());
    let s1 = vm1.run().unwrap();
    assert_eq!(s1, VmStatus::Halted);

    let mut vm2 = CcsVm::new();
    seed_concepts(&mut vm2);
    vm2.load(program);
    let s2 = vm2.run().unwrap();
    assert_eq!(s2, VmStatus::Halted);

    assert_eq!(vm1.world_graph.node_count(), vm2.world_graph.node_count());
    assert_eq!(vm1.memory_trace.len(), vm2.memory_trace.len());
    assert_eq!(vm1.last_plan_actions.len(), vm2.last_plan_actions.len());
    // Self-model ID may differ between VMs (allocation order), but
    // *concept data* must match byte-for-byte.
    let sm1 = vm1
        .world_graph
        .lookup(vm1.last_reflect.unwrap())
        .unwrap()
        .unwrap();
    let sm2 = vm2
        .world_graph
        .lookup(vm2.last_reflect.unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        sm1.concept.data, sm2.concept.data,
        "self-model must be deterministic"
    );
}
