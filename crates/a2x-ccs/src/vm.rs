// See plans/03-ccs-vm.md §3-4 (CcsVm, execution loop)
//
// Phase 1: Added tracing instrumentation — logs Σ∞ packets as structured events.

use std::time::Duration;

use a2x_core::concept::ConceptVector;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::{MemoryEntry, MemoryTrace};
use a2x_core::node::NodeId;
use a2x_core::opcode::Opcode;
use a2x_core::relation::{RelationEdge, RelationType};
use a2x_core::state::StateField;
use a2x_sigma::program::SigmaProgram;
use tracing::{debug, info, trace, warn};

use crate::error::VmError;
use crate::memory::VecMemoryTrace;
use crate::operators::{bind, differentiate, ground, plan, reflect};
use crate::safety::{SafetyConstraints, SafetyLevel};
use crate::state::{init_default_regions, FlatStateField};
use crate::world_graph::PetgraphWorldGraph;

/// Log a Σ∞ packet as a structured tracing event.
fn trace_packet(ip: usize, opcode: Opcode, packet_text: &str) {
    trace!(
        ip = ip,
        opcode = ?opcode,
        packet = %packet_text,
        "Σ∞ execute"
    );
}

/// VM execution status.
#[derive(Clone, Debug, PartialEq)]
pub enum VmStatus {
    Running,
    Halted,
    Yield,
    Fault(VmError),
}

/// Configurable VM limits.
#[derive(Clone, Debug)]
pub struct VmLimits {
    pub max_stack_depth: usize,
    pub max_steps: usize,
    pub evolve_dt: Duration,
    /// Phase 2.D: how many recent MemoryTrace entries the REFLECT operator
    /// summarizes when it builds a self-model node.
    pub reflect_window: usize,
}

impl Default for VmLimits {
    fn default() -> Self {
        VmLimits {
            max_stack_depth: 256,
            max_steps: 10_000,
            evolve_dt: Duration::from_millis(10),
            reflect_window: crate::operators::reflect::REFLECT_DEFAULT_WINDOW,
        }
    }
}

/// The CCS VM — the cognitive substrate runtime.
pub struct CcsVm {
    pub world_graph: PetgraphWorldGraph,
    pub state_field: FlatStateField,
    pub memory_trace: VecMemoryTrace,
    program: Option<SigmaProgram>,
    pub ip: usize,
    call_stack: Vec<usize>,
    steps_executed: usize,
    pub safety: SafetyConstraints,
    pub limits: VmLimits,
    started_at: Option<std::time::Instant>,
    /// Phase 2.D: NodeId of the most recent reflect self-model node (the one
    /// carrying the `__last_reflect` label). Phase 2.E's plan operator reads
    /// this to bias action selection toward what reflect just summarized.
    pub last_reflect: Option<NodeId>,
}

/// Bundle of decoded instruction data, extracted without borrowing `self`.
struct DecodedInstruction {
    opcode: Opcode,
    /// Label for jump/call targets (from C: field).
    jump_label: Option<String>,
    /// All C-field labels — operands for BIND / DIFFERENTIATE (Phase 2.A plumbing).
    operand_labels: Vec<String>,
    /// Raw D-field bytes — operand payload for GROUND (f32 chunks)
    /// and DIFFERENTIATE (chunk count as u32 LE).
    data_payload: Vec<u8>,
    /// Serialized instruction bytes for MemoryTrace.
    bytes: Vec<u8>,
}

impl CcsVm {
    pub fn new() -> Self {
        let mut state = FlatStateField::default_size();
        let _ = init_default_regions(&mut state);

        CcsVm {
            world_graph: PetgraphWorldGraph::new(),
            state_field: state,
            memory_trace: VecMemoryTrace::default_capacity(),
            program: None,
            ip: 0,
            call_stack: Vec::new(),
            steps_executed: 0,
            safety: SafetyConstraints::default(),
            limits: VmLimits::default(),
            started_at: None,
            last_reflect: None,
        }
    }

    pub fn with_safety(safety_level: SafetyLevel) -> Self {
        CcsVm {
            safety: SafetyConstraints::new(safety_level),
            ..Self::new()
        }
    }

    pub fn load(&mut self, program: SigmaProgram) {
        self.program = Some(program);
        self.ip = 0;
        self.call_stack.clear();
        self.steps_executed = 0;
        self.started_at = Some(std::time::Instant::now());
    }

    pub fn program(&self) -> Option<&SigmaProgram> {
        self.program.as_ref()
    }

    pub fn steps_executed(&self) -> usize {
        self.steps_executed
    }

    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    pub fn uptime(&self) -> Duration {
        self.started_at
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// Look up a single context label in the WorldGraph and return its stored
    /// `ConceptVector`. Returns `VmError::UnresolvedOperand` if the label is
    /// not in the index, or `VmError::InvalidNode` if the index is stale
    /// (node deallocated between label lookup and node lookup).
    fn fetch_concept(&self, label: &str) -> Result<ConceptVector, VmError> {
        let nid = self
            .world_graph
            .lookup_label(label)
            .map_err(|e| VmError::Other(e.to_string()))?
            .ok_or_else(|| VmError::UnresolvedOperand(label.to_string()))?;
        let node = self
            .world_graph
            .lookup(nid)
            .map_err(|e| VmError::Other(e.to_string()))?
            .ok_or(VmError::InvalidNode(nid.as_u64()))?;
        Ok(node.concept)
    }

    /// Resolve a list of context labels into `ConceptVector`s by looking them up
    /// in the WorldGraph. Used by the `BIND` operator (multiple operands).
    /// Returns the first `VmError::UnresolvedOperand` encountered.
    fn resolve_concepts(&self, labels: &[String]) -> Result<Vec<ConceptVector>, VmError> {
        labels.iter().map(|l| self.fetch_concept(l)).collect()
    }

    /// Resolve a single context label into a `ConceptVector`. Used by the
    /// `DIFFERENTIATE` operator (single source operand).
    fn resolve_single(&self, label: &str) -> Result<ConceptVector, VmError> {
        self.fetch_concept(label)
    }

    /// Parse chunk count from a D-field payload as `u32` little-endian.
    /// Returns 1 when the payload is shorter than 4 bytes or encodes zero.
    /// Used by the `DIFFERENTIATE` operator.
    fn parse_chunk_count(payload: &[u8]) -> usize {
        if payload.len() < 4 {
            return 1;
        }
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&payload[..4]);
        let v = u32::from_le_bytes(buf);
        if v == 0 {
            1
        } else {
            v as usize
        }
    }

    /// Reinterpret raw D-field bytes as a `Vec<f32>` (little-endian, one
    /// element per 4 bytes). Trailing bytes that don't form a complete `f32`
    /// are dropped. Used by the `GROUND` operator.
    fn parse_f32_payload(payload: &[u8]) -> Vec<f32> {
        let n = payload.len() / 4;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&payload[i * 4..(i + 1) * 4]);
            out.push(f32::from_le_bytes(buf));
        }
        out
    }

    /// Build a deterministic provenance string for a newly-allocated node.
    /// Format: `<op>(<key>=<val>,...)`. Terse, grep-friendly, never reads
    /// clocks \u2014 fully reproducible from VM state + inputs.
    fn provenance(op: &str, kv: &[(&str, String)]) -> String {
        let mut parts = Vec::with_capacity(kv.len());
        for (k, v) in kv {
            parts.push(format!("{}={}", k, v));
        }
        format!("{}({})", op, parts.join(","))
    }

    /// Auto-label for a freshly-allocated node: `__<op>_<nodeid>`.
    /// The `__<op>_` prefix keeps operator-produced labels in a distinct
    /// namespace from user-visible labels (e.g. `a`, `src`, `sys`), and the
    /// NodeId suffix guarantees uniqueness within a graph.
    fn auto_label(op: &str, id: NodeId) -> String {
        format!("__{}_{}", op, id.as_u64())
    }

    /// BIND: resolve operand labels \u2192 call `bind::bind` \u2192 allocate the
    /// composite as a new WorldGraph node \u2192 set provenance + auto-label
    /// \u2192 create `Hierarchical` edges from each *unique* operand to the new
    /// node. Returns Vec of new NodeIds (empty if no operands or if `bind`
    /// returned `None`).
    fn dispatch_bind(&mut self, operand_labels: &[String]) -> Result<Vec<NodeId>, VmError> {
        if operand_labels.is_empty() {
            return Ok(Vec::new());
        }
        let concepts = self.resolve_concepts(operand_labels)?;
        let composite = match bind::bind(&concepts) {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let new_id = self
            .world_graph
            .allocate(composite)
            .map_err(|e| VmError::Other(e.to_string()))?;
        let inputs_joined = operand_labels.join(",");
        self.world_graph
            .set_provenance(
                new_id,
                &Self::provenance(
                    "bind",
                    &[
                        ("ip", self.ip.to_string()),
                        ("inputs", format!("[{}]", inputs_joined)),
                    ],
                ),
            )
            .map_err(|e| VmError::Other(e.to_string()))?;
        if let Err(e) = self
            .world_graph
            .set_label(new_id, &Self::auto_label("bind", new_id))
        {
            debug!(error = %e, "auto-label for bind node failed");
        }

        // Edges from each unique operand NodeId \u2192 new node. `Hierarchical`
        // captures BIND's plan \u00a74 semantics: operand is *part of* composite.
        // Dedup because `add_edge` rejects duplicates by source+target+type.
        let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for label in operand_labels {
            let src_id = match self.world_graph.lookup_label(label) {
                Ok(Some(id)) => id,
                _ => continue,
            };
            if seen.insert(src_id.as_u64()) {
                let edge = RelationEdge::new(src_id, new_id, RelationType::Hierarchical, 1.0);
                if let Err(e) = self.world_graph.add_edge(src_id, new_id, edge) {
                    debug!(error = %e, label = %label, "bind edge creation failed");
                }
            }
        }

        self.safety
            .record_allocation()
            .map_err(VmError::SafetyViolation)?;
        Ok(vec![new_id])
    }

    /// DIFFERENTIATE: resolve first operand as source \u2192 parse chunk count
    /// \u2192 call `differentiate::differentiate` \u2192 allocate one node per chunk
    /// (skips if operator returns empty) \u2192 set provenance per chunk with
    /// chunk index \u2192 create `Hierarchical` edge from source to each chunk.
    fn dispatch_differentiate(
        &mut self,
        operand_labels: &[String],
        payload: &[u8],
    ) -> Result<Vec<NodeId>, VmError> {
        let first = match operand_labels.first() {
            Some(l) => l,
            None => return Ok(Vec::new()),
        };
        let source_concept = self.resolve_single(first)?;
        // `resolve_single` succeeded, so the label is in the index \u2014 a
        // second lookup here cannot fail without concurrent mutation
        // (single-threaded VM). `expect` rather than the InvalidNode(0)
        // sentinel makes the invariant explicit and surfaces real bugs.
        let source_id = self
            .world_graph
            .lookup_label(first)
            .map_err(|e| VmError::Other(e.to_string()))?
            .expect("source label must resolve: resolve_single succeeded");
        let n = Self::parse_chunk_count(payload);
        let chunks = differentiate::differentiate(&source_concept, n);
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let of = chunks.len().to_string();
        let mut new_ids = Vec::with_capacity(chunks.len());
        for (i, chunk) in chunks.into_iter().enumerate() {
            let new_id = self
                .world_graph
                .allocate(chunk)
                .map_err(|e| VmError::Other(e.to_string()))?;
            self.world_graph
                .set_provenance(
                    new_id,
                    &Self::provenance(
                        "differentiate",
                        &[
                            ("ip", self.ip.to_string()),
                            ("source", source_id.as_u64().to_string()),
                            ("chunk", i.to_string()),
                            ("of", of.clone()),
                        ],
                    ),
                )
                .map_err(|e| VmError::Other(e.to_string()))?;
            if let Err(e) = self
                .world_graph
                .set_label(new_id, &Self::auto_label("diff", new_id))
            {
                debug!(error = %e, chunk = i, "auto-label for diff chunk failed");
            }
            // Hierarchical: each chunk is part of the source concept.
            let edge = RelationEdge::new(source_id, new_id, RelationType::Hierarchical, 1.0);
            if let Err(e) = self.world_graph.add_edge(source_id, new_id, edge) {
                debug!(error = %e, chunk = i, "diff edge creation failed");
            }
            new_ids.push(new_id);
        }

        self.safety
            .record_allocation()
            .map_err(VmError::SafetyViolation)?;
        Ok(new_ids)
    }

    /// GROUND: parse D-field payload as f32 chunks \u2192 call `ground::ground`
    /// \u2192 allocate exactly one node (even for empty payload \u2014 the intent
    /// to ground is recorded) \u2192 set provenance with modality. No edges:
    /// GROUND is purely perceptual (no memory input).
    fn dispatch_ground(&mut self, payload: &[u8]) -> Result<Vec<NodeId>, VmError> {
        let floats = Self::parse_f32_payload(payload);
        let cv = ground::ground(&floats, &a2x_core::modality::Modality::Text);
        let new_id = self
            .world_graph
            .allocate(cv)
            .map_err(|e| VmError::Other(e.to_string()))?;
        self.world_graph
            .set_provenance(
                new_id,
                &Self::provenance(
                    "ground",
                    &[
                        ("ip", self.ip.to_string()),
                        ("modality", "Text".to_string()),
                        ("floats", floats.len().to_string()),
                    ],
                ),
            )
            .map_err(|e| VmError::Other(e.to_string()))?;
        if let Err(e) = self
            .world_graph
            .set_label(new_id, &Self::auto_label("ground", new_id))
        {
            debug!(error = %e, "auto-label for ground node failed");
        }
        self.safety
            .record_allocation()
            .map_err(VmError::SafetyViolation)?;
        Ok(vec![new_id])
    }

    /// Fetch and decode the current instruction. Returns None if halted.
    /// All data is extracted into owned values so the borrow on self is released.
    fn fetch_and_decode(&self) -> Result<DecodedInstruction, VmError> {
        let program = self.program.as_ref().ok_or(VmError::NoProgram)?;
        if self.ip >= program.instructions.len() {
            return Err(VmError::InvalidInstructionPointer {
                ip: self.ip,
                length: program.instructions.len(),
            });
        }
        let instruction = &program.instructions[self.ip];

        let first_intent = instruction.intent.operators.first().copied();
        let first_plan = instruction.plan.operators.first().copied();
        let first_data = instruction.data.operators.first().copied();
        let opcode = Self::map_to_opcode(first_intent, first_plan, first_data);
        let jump_label = instruction.context.labels.first().cloned();
        let operand_labels = instruction.context.labels.clone();
        let data_payload = instruction.data.payload.clone();
        let bytes = instruction.to_string().into_bytes();

        Ok(DecodedInstruction {
            opcode,
            jump_label,
            operand_labels,
            data_payload,
            bytes,
        })
    }

    /// Execute a single fetch-decode-execute step.
    pub fn step(&mut self) -> Result<VmStatus, VmError> {
        // 1. FETCH + DECODE (all data extracted, borrow released)
        let decoded = match self.fetch_and_decode() {
            Ok(d) => d,
            Err(VmError::InvalidInstructionPointer { .. }) => {
                debug!("VM halted — IP past end of program");
                return Ok(VmStatus::Halted);
            }
            Err(e) => {
                warn!(error = %e, "VM fetch/decode error");
                return Err(e);
            }
        };

        let old_ip = self.ip;
        let program = self.program.as_ref();

        // Trace the Σ∞ packet being executed
        if let Some(prog) = program {
            if let Some(inst) = prog.instructions.get(old_ip) {
                trace_packet(old_ip, decoded.opcode, &inst.to_string());
            }
        }

        // 2. SAFETY CHECK
        self.safety
            .check_opcode(decoded.opcode)
            .map_err(VmError::SafetyViolation)?;
        self.safety.step().map_err(VmError::SafetyViolation)?;

        // 3. EXECUTE — dispatch to operator
        match decoded.opcode {
            Opcode::Nop => {}
            Opcode::Bind => {
                debug!("BIND operator");
                // Phase 2.B: route operands to operator, allocate the composite
                // vector as a new WorldGraph node, set provenance + auto-label,
                // create `Hierarchical` edges from each unique operand to the new
                // node. See `dispatch_bind`.
                let _ = self.dispatch_bind(&decoded.operand_labels)?;
            }
            Opcode::Differentiate => {
                debug!("DIFF operator");
                // Phase 2.B: resolve source operand → differentiate into n chunks
                // → allocate one node per chunk → set provenance per chunk with
                // chunk index → create `Hierarchical` edges from source to each.
                let _ =
                    self.dispatch_differentiate(&decoded.operand_labels, &decoded.data_payload)?;
            }
            Opcode::Ground => {
                debug!("GRND operator");
                // Phase 2.B: parse D-field as f32 payload → ground (even for empty
                // input the zero-vector allocation is meaningful) → allocate 1
                // node → set provenance with modality. No edges: GROUND is purely
                // perceptual (no memory input).
                let _ = self.dispatch_ground(&decoded.data_payload)?;
            }
            Opcode::Evolve => {
                debug!("EVOL operator");
                crate::operators::evolve::evolve(
                    &mut self.world_graph,
                    &mut self.state_field,
                    self.limits.evolve_dt,
                )
                .map_err(|e| VmError::Other(e.to_string()))?;
            }
            Opcode::Reflect => {
                debug!("REFL operator");
                // Phase 2.D: route reflect to the real self-model builder.
                // Result is retained on `self.last_reflect` so Phase 2.E's
                // PLAN operator can read it without re-querying the graph.
                let result = reflect::reflect(
                    &self.memory_trace,
                    &mut self.world_graph,
                    self.limits.reflect_window,
                )
                .map_err(|e| VmError::Other(e.to_string()))?;
                self.last_reflect = Some(result.self_model_id);
            }
            Opcode::Plan => {
                debug!("PLAN operator");
                let _actions = plan::plan();
            }
            Opcode::Actuate => {
                debug!("ACT operator");
                let _cmd = crate::operators::actuate::actuate();
                self.safety
                    .record_side_effect()
                    .map_err(VmError::SafetyViolation)?;
            }
            Opcode::Jump => {
                debug!("JMP operator");
                let target = decoded
                    .jump_label
                    .as_deref()
                    .ok_or_else(|| VmError::Other("jump without target label".into()))?;
                self.control_jump(target)?;
                return Ok(VmStatus::Running);
            }
            Opcode::Branch => {
                debug!("BR operator");
                let target = decoded
                    .jump_label
                    .as_deref()
                    .ok_or_else(|| VmError::Other("branch without target label".into()))?;
                self.control_jump(target)?;
                return Ok(VmStatus::Running);
            }
            Opcode::Call => {
                debug!("CALL operator");
                let target = decoded
                    .jump_label
                    .as_deref()
                    .ok_or_else(|| VmError::Other("call without target label".into()))?;
                self.control_call(target)?;
                return Ok(VmStatus::Running);
            }
            Opcode::Return => {
                debug!("RET operator");
                self.control_return()?;
                return Ok(VmStatus::Running);
            }
            Opcode::Fork => {}
            Opcode::Merge => {}
            Opcode::Halt => {
                info!("VM halted normally");
                return Ok(VmStatus::Halted);
            }
            Opcode::Custom(_) => {}
        }

        // 4. CONTROL FLOW — default: sequential (IP += 1)
        self.ip += 1;
        self.steps_executed += 1;

        // 5. TRACE — log to MemoryTrace
        let entry = MemoryEntry {
            timestamp: Some(std::time::SystemTime::now()),
            instruction_bytes: decoded.bytes,
            ip: old_ip,
            program_id: self.program.as_ref().map(|p| p.id),
            state_snapshot_bytes: self
                .state_field
                .raw_data()
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect(),
        };
        self.memory_trace
            .push(entry)
            .map_err(|e| VmError::Other(e.to_string()))?;

        if self.steps_executed > self.limits.max_steps {
            return Err(VmError::MaxStepsExceeded {
                max: self.limits.max_steps,
                actual: self.steps_executed,
            });
        }

        Ok(VmStatus::Running)
    }

    /// Jump to a labeled instruction.
    fn control_jump(&mut self, target: &str) -> Result<(), VmError> {
        let program = self.program.as_ref().ok_or(VmError::NoProgram)?;
        let target_ip = program
            .resolve_label(target)
            .ok_or_else(|| VmError::UndefinedLabel(target.to_string()))?;
        self.control_jump_to(target_ip);
        Ok(())
    }

    fn control_jump_to(&mut self, target_ip: usize) {
        self.ip = target_ip;
        self.steps_executed += 1;
    }

    /// Call a sub-program at label.
    fn control_call(&mut self, target: &str) -> Result<(), VmError> {
        if self.call_stack.len() >= self.limits.max_stack_depth {
            return Err(VmError::StackOverflow {
                max_depth: self.limits.max_stack_depth,
            });
        }
        self.call_stack.push(self.ip + 1);
        self.control_jump(target)?;
        Ok(())
    }

    /// Return from sub-program.
    fn control_return(&mut self) -> Result<(), VmError> {
        match self.call_stack.pop() {
            Some(return_ip) => {
                self.ip = return_ip;
                self.steps_executed += 1;
                Ok(())
            }
            None => Err(VmError::StackUnderflow),
        }
    }

    pub fn run(&mut self) -> Result<VmStatus, VmError> {
        loop {
            match self.step()? {
                VmStatus::Running => continue,
                VmStatus::Halted => return Ok(VmStatus::Halted),
                VmStatus::Yield => return Ok(VmStatus::Yield),
                VmStatus::Fault(err) => return Err(err),
            }
        }
    }

    fn map_to_opcode(
        intent: Option<operators_deps::IntentOp>,
        plan: Option<operators_deps::PlanOp>,
        data: Option<operators_deps::DataOp>,
    ) -> Opcode {
        use operators_deps::{DataOp, IntentOp, PlanOp};

        // Plan (control flow) operators take priority
        match plan {
            Some(PlanOp::Sequential) => return Opcode::Nop,
            Some(PlanOp::Branch) => return Opcode::Branch,
            Some(PlanOp::Descend) => return Opcode::Call,
            Some(PlanOp::Ascend) => return Opcode::Return,
            Some(PlanOp::Swarm) => return Opcode::Fork,
            Some(PlanOp::Merge) => return Opcode::Merge,
            _ => {}
        }

        // Intent operators map to compute operations
        match intent {
            Some(IntentOp::Lightning) => Opcode::Actuate,
            Some(IntentOp::Star) => Opcode::Ground,
            Some(IntentOp::Synthesis) => Opcode::Bind,
            Some(IntentOp::Cancel) => Opcode::Halt,
            Some(IntentOp::Split) => Opcode::Differentiate,
            Some(IntentOp::Contradiction) => Opcode::Reflect,
            Some(IntentOp::Delay) => Opcode::Evolve,
            Some(IntentOp::Parallel) => Opcode::Plan,
            Some(IntentOp::Merge) => Opcode::Merge,
            _ => match data {
                Some(DataOp::GraphDelta) => Opcode::Plan,
                _ => Opcode::Nop,
            },
        }
    }
}

impl Default for CcsVm {
    fn default() -> Self {
        Self::new()
    }
}

mod operators_deps {
    pub use a2x_sigma::data::DataOp;
    pub use a2x_sigma::intent::IntentOp;
    pub use a2x_sigma::plan::PlanOp;
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::concept::ConceptVector;
    use a2x_sigma::{IntentOp, SigmaPacket, SigmaProgram};

    fn nop_program() -> SigmaProgram {
        let packet = SigmaPacket::default();
        let mut prog = SigmaProgram::new();
        prog.push(packet);
        prog
    }

    fn empty_program() -> SigmaProgram {
        SigmaProgram::new()
    }

    #[test]
    fn test_new_vm() {
        let vm = CcsVm::new();
        assert!(vm.program().is_none());
        assert_eq!(vm.ip, 0);
        assert_eq!(vm.world_graph.node_count(), 0);
    }

    #[test]
    fn test_load_empty_program() {
        let mut vm = CcsVm::new();
        vm.load(empty_program());
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Halted);
    }

    #[test]
    fn test_run_empty_program() {
        let mut vm = CcsVm::new();
        vm.load(empty_program());
        let status = vm.run().unwrap();
        assert_eq!(status, VmStatus::Halted);
    }

    #[test]
    fn test_run_nop_program() {
        let mut vm = CcsVm::new();
        vm.load(nop_program());
        let status = vm.run().unwrap();
        assert_eq!(status, VmStatus::Halted);
        assert_eq!(vm.steps_executed, 1);
        assert_eq!(vm.memory_trace.len(), 1);
    }

    #[test]
    fn test_ip_advances() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(SigmaPacket::default());
        prog.push(SigmaPacket::default());
        vm.load(prog);
        vm.step().unwrap();
        assert_eq!(vm.ip, 1);
        vm.step().unwrap();
        assert_eq!(vm.ip, 2);
    }

    #[test]
    fn test_no_program_error() {
        let mut vm = CcsVm::new();
        assert_eq!(vm.run(), Err(VmError::NoProgram));
    }

    #[test]
    fn test_return_underflow() {
        let mut vm = CcsVm::new();
        vm.ip = 5;
        assert_eq!(vm.control_return(), Err(VmError::StackUnderflow));
    }

    #[test]
    fn test_map_to_opcode_nop() {
        let op = CcsVm::map_to_opcode(None, None, None);
        assert_eq!(op, Opcode::Nop);
    }

    // === Phase 2.A: VM operand plumbing ===

    #[test]
    fn test_resolve_concepts_with_labels() {
        let mut vm = CcsVm::new();
        let id = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0, 2.0]))
            .unwrap();
        vm.world_graph.set_label(id, "a").unwrap();
        let id2 = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![3.0, 4.0]))
            .unwrap();
        vm.world_graph.set_label(id2, "b").unwrap();
        let concepts = vm
            .resolve_concepts(&["a".to_string(), "b".to_string()])
            .unwrap();
        assert_eq!(concepts.len(), 2);
        assert_eq!(concepts[0].data, vec![1.0, 2.0]);
        assert_eq!(concepts[1].data, vec![3.0, 4.0]);
    }

    #[test]
    fn test_resolve_concepts_missing_label() {
        let vm = CcsVm::new();
        let err = vm.resolve_concepts(&["missing".to_string()]).unwrap_err();
        assert_eq!(err, VmError::UnresolvedOperand("missing".into()));
    }

    #[test]
    fn test_parse_chunk_count() {
        assert_eq!(CcsVm::parse_chunk_count(&[]), 1);
        assert_eq!(CcsVm::parse_chunk_count(&[5u8]), 1);
        assert_eq!(CcsVm::parse_chunk_count(&[0, 0, 0, 0]), 1);
        assert_eq!(CcsVm::parse_chunk_count(&[3, 0, 0, 0]), 3);
        let big = (1024u32).to_le_bytes();
        assert_eq!(CcsVm::parse_chunk_count(&big), 1024);
    }

    #[test]
    fn test_parse_f32_payload() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&5.0f32.to_le_bytes());
        bytes.extend_from_slice(&7.0f32.to_le_bytes());
        let floats = CcsVm::parse_f32_payload(&bytes);
        assert_eq!(floats, vec![5.0, 7.0]);
    }

    #[test]
    fn test_parse_f32_payload_truncates_partial_chunk() {
        let bytes = vec![0u8, 0, 0, 0, 1, 2, 3];
        let floats = CcsVm::parse_f32_payload(&bytes);
        assert_eq!(floats, vec![0.0]);
    }

    fn synth_bind_packet(labels: &[&str]) -> SigmaPacket {
        let mut p = SigmaPacket::new();
        p.intent.operators.push(IntentOp::Synthesis);
        for l in labels {
            p.context.labels.push(l.to_string());
        }
        p
    }

    #[test]
    fn test_step_bind_with_unresolved_label_returns_error() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_bind_packet(&["never"]));
        vm.load(prog);
        let result = vm.step();
        assert_eq!(result, Err(VmError::UnresolvedOperand("never".into())));
    }

    #[test]
    fn test_step_bind_with_empty_context_is_noop() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_bind_packet(&[]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        assert_eq!(vm.world_graph.node_count(), 0);
    }

    fn synth_differentiate_packet(label: &str, n: u32) -> SigmaPacket {
        let mut p = SigmaPacket::new();
        p.intent.operators.push(IntentOp::Split);
        p.context.labels.push(label.to_string());
        p.data.payload = n.to_le_bytes().to_vec();
        p
    }

    fn synth_ground_packet(floats: &[f32]) -> SigmaPacket {
        let mut p = SigmaPacket::new();
        p.intent.operators.push(IntentOp::Star);
        let mut bytes = Vec::new();
        for f in floats {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        p.data.payload = bytes;
        p
    }

    #[test]
    fn test_step_differentiate_with_resolved_label_routes_to_operator() {
        let mut vm = CcsVm::new();
        let id = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
        vm.world_graph.set_label(id, "src").unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_differentiate_packet("src", 2));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        // Phase 2.B: source (1) + 2 chunks = 3 nodes total.
        assert_eq!(vm.world_graph.node_count(), 3);
        // Source's outgoing edges now lead to both chunks.
        let source_neighbours = vm.world_graph.neighbors(id).unwrap();
        assert_eq!(source_neighbours.len(), 2);
    }

    #[test]
    fn test_step_differentiate_with_unresolved_label_returns_error() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_differentiate_packet("missing", 3));
        vm.load(prog);
        let result = vm.step();
        assert_eq!(result, Err(VmError::UnresolvedOperand("missing".into())));
    }

    #[test]
    fn test_step_ground_with_f32_payload_routes_to_operator() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_ground_packet(&[1.0, 2.0, 3.0]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        // Phase 2.B: ground always allocates exactly 1 node, even when it
        // wraps a non-empty payload into ConceptVector.
        assert_eq!(vm.world_graph.node_count(), 1);
    }

    #[test]
    fn test_step_ground_with_empty_payload_routes_to_operator() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_ground_packet(&[]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        // Phase 2.B: ground *intends* to ground — even an empty payload earns
        // a node (dim=0) plus provenance recording the attempt.
        assert_eq!(vm.world_graph.node_count(), 1);
    }

    // === Phase 2.B: node allocation ===

    #[test]
    fn test_step_bind_allocates_combined_node() {
        let mut vm = CcsVm::new();
        let aid = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0, 2.0]))
            .unwrap();
        vm.world_graph.set_label(aid, "a").unwrap();
        let bid = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![3.0, 4.0]))
            .unwrap();
        vm.world_graph.set_label(bid, "b").unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_bind_packet(&["a", "b"]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        // 2 input nodes + 1 bind result = 3 total.
        assert_eq!(vm.world_graph.node_count(), 3);
        // The new node is the 3rd allocated (after a=1, b=2); auto-label
        // `__bind_3` should resolve.
        let new_id = vm
            .world_graph
            .lookup_label("__bind_3")
            .unwrap()
            .expect("bind result node labelled");
        assert_eq!(new_id.as_u64(), 3);
        let node = vm.world_graph.lookup(new_id).unwrap().unwrap();
        // Provenance is deterministic + grep-friendly.
        let prov = node
            .metadata
            .provenance
            .as_deref()
            .expect("bind node has provenance");
        assert!(prov.starts_with("bind("));
        assert!(prov.contains("ip=0"));
        assert!(prov.contains("inputs=[a,b]"));
    }

    #[test]
    fn test_step_bind_creates_edges_to_unique_operands() {
        let mut vm = CcsVm::new();
        let aid = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0]))
            .unwrap();
        vm.world_graph.set_label(aid, "a").unwrap();
        let bid = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![2.0]))
            .unwrap();
        vm.world_graph.set_label(bid, "b").unwrap();
        let mut prog = SigmaProgram::new();
        // ⟨a⟩⟨b⟩⟨a⟩ BIND — `a` appears twice; dedup ensures one edge per
        // *unique* operand, not one edge per operand *occurrence*.
        prog.push(synth_bind_packet(&["a", "b", "a"]));
        vm.load(prog);
        vm.step().unwrap();
        let new_id = vm.world_graph.lookup_label("__bind_3").unwrap().unwrap();
        // Each input has exactly one outgoing edge to the bind result.
        assert_eq!(vm.world_graph.neighbors(aid).unwrap(), vec![new_id]);
        assert_eq!(vm.world_graph.neighbors(bid).unwrap(), vec![new_id]);
        // Total edges: 2 (one from a, one from b).
        assert_eq!(vm.world_graph.edge_count(), 2);
    }

    #[test]
    fn test_step_differentiate_allocates_n_chunks() {
        let mut vm = CcsVm::new();
        let src_id = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
        vm.world_graph.set_label(src_id, "src").unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_differentiate_packet("src", 3));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        // 1 source + 3 chunks = 4 nodes. Chunks are NodeIds 2, 3, 4.
        assert_eq!(vm.world_graph.node_count(), 4);
        for i in 0..3 {
            let label = format!("__diff_{}", i + 2);
            assert!(
                vm.world_graph.lookup_label(&label).unwrap().is_some(),
                "chunk {} should be auto-labelled",
                i
            );
        }
        // Each chunk is an outgoing edge from the source.
        assert_eq!(vm.world_graph.outgoing_edges(src_id).len(), 3);
    }

    #[test]
    fn test_step_differentiate_provenance_records_source_and_chunk_index() {
        let mut vm = CcsVm::new();
        let src_id = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
        vm.world_graph.set_label(src_id, "src").unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_differentiate_packet("src", 2));
        vm.load(prog);
        vm.step().unwrap();
        // Chunks are NodeIds 2, 3 (after src=1).
        for (idx, expected_nid) in [0usize, 1].iter().zip([2u64, 3u64]) {
            let node = vm
                .world_graph
                .lookup(a2x_core::node::NodeId::new(expected_nid))
                .unwrap()
                .unwrap();
            let prov = node
                .metadata
                .provenance
                .as_deref()
                .expect("chunk has provenance");
            assert!(prov.starts_with("differentiate("));
            assert!(prov.contains(&format!("source={}", src_id.as_u64())));
            assert!(prov.contains(&format!("chunk={}", idx)));
            assert!(prov.contains("of=2"));
        }
    }

    #[test]
    fn test_step_ground_allocates_with_provenance() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_ground_packet(&[1.0, 2.0, 3.0]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        assert_eq!(vm.world_graph.node_count(), 1);
        let node = vm
            .world_graph
            .lookup(vm.world_graph.lookup_label("__ground_1").unwrap().unwrap())
            .unwrap()
            .unwrap();
        let prov = node
            .metadata
            .provenance
            .as_deref()
            .expect("ground node has provenance");
        assert!(prov.starts_with("ground("));
        assert!(prov.contains("modality=Text"));
        assert!(prov.contains("floats=3"));
    }

    // === Phase 2.C: evolve ===

    fn synth_evolve_packet() -> SigmaPacket {
        let mut p = SigmaPacket::new();
        p.intent.operators.push(IntentOp::Delay);
        p
    }

    #[test]
    fn test_step_evolve_advances_ip_and_bumps_access_counts() {
        let mut vm = CcsVm::new();
        let id_a = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![1.0]))
            .unwrap();
        let id_b = vm
            .world_graph
            .allocate(ConceptVector::from_vec(vec![2.0]))
            .unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_evolve_packet());
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        assert_eq!(vm.ip, 1); // ip advanced after evolve
                              // Every node's access_count should now be 1.
        assert_eq!(
            vm.world_graph
                .lookup(id_a)
                .unwrap()
                .unwrap()
                .metadata
                .access_count,
            1
        );
        assert_eq!(
            vm.world_graph
                .lookup(id_b)
                .unwrap()
                .unwrap()
                .metadata
                .access_count,
            1
        );
    }

    #[test]
    fn test_step_evolve_attention_decays() {
        let mut vm = CcsVm::new();
        let ones = vec![1.0f32; 128];
        vm.state_field.write_region("attention", &ones).unwrap();
        let mut prog = SigmaProgram::new();
        prog.push(synth_evolve_packet());
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        let after = vm.state_field.read_region("attention").unwrap();
        for v in after {
            assert!((v - 0.95).abs() < 1e-6);
        }
    }

    #[test]
    fn test_step_evolve_temporal_decrements() {
        // vm.limits.evolve_dt = Duration::from_millis(10); temporal[0] -= 0.01
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_evolve_packet());
        vm.load(prog);
        vm.step().unwrap();
        let temporal = vm.state_field.read_region("temporal").unwrap();
        assert!((temporal[0] - (-0.01)).abs() < 1e-6);
    }

    #[test]
    fn test_step_evolve_belief_drifts() {
        let mut vm = CcsVm::new();
        let before_belief = vm.state_field.read_region("belief").unwrap().to_vec();
        let mut prog = SigmaProgram::new();
        prog.push(synth_evolve_packet());
        vm.load(prog);
        vm.step().unwrap();
        let after_belief = vm.state_field.read_region("belief").unwrap().to_vec();
        let diffs = before_belief
            .iter()
            .zip(&after_belief)
            .filter(|(a, b)| a != b)
            .count();
        assert!(diffs > 0, "belief should drift after evolve");
    }

    #[test]
    fn test_step_evolve_three_steps_deterministic() {
        // Two fresh VMs running 3 evolutions produce identical StateField
        // snapshots — proves no wall-clock is read.
        let mut prog = SigmaProgram::new();
        for _ in 0..3 {
            prog.push(synth_evolve_packet());
        }

        let mut vm1 = CcsVm::new();
        vm1.load(prog.clone());
        vm1.run().unwrap();

        let mut vm2 = CcsVm::new();
        vm2.load(prog);
        vm2.run().unwrap();

        assert_eq!(
            vm1.state_field.read_region("belief").unwrap(),
            vm2.state_field.read_region("belief").unwrap(),
        );
        assert_eq!(
            vm1.state_field.read_region("attention").unwrap(),
            vm2.state_field.read_region("attention").unwrap(),
        );
        assert_eq!(
            vm1.state_field.read_region("temporal").unwrap(),
            vm2.state_field.read_region("temporal").unwrap(),
        );
    }
}
