// See plans/03-ccs-vm.md §3-4 (CcsVm, execution loop)
//
// Phase 1: Added tracing instrumentation — logs Σ∞ packets as structured events.

use std::time::Duration;

use a2x_core::concept::ConceptVector;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::{MemoryEntry, MemoryTrace};
use a2x_core::opcode::Opcode;
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
}

impl Default for VmLimits {
    fn default() -> Self {
        VmLimits {
            max_stack_depth: 256,
            max_steps: 10_000,
            evolve_dt: Duration::from_millis(10),
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
                // Phase 2.A plumbing: resolve C-field labels into concepts and call
                // the bind operator. The composite result is discarded here; Step B
                // will allocate the resulting `ConceptVector` as a new WorldGraph node.
                if !decoded.operand_labels.is_empty() {
                    let concepts = self.resolve_concepts(&decoded.operand_labels)?;
                    let _ = bind::bind(&concepts);
                }
            }
            Opcode::Differentiate => {
                debug!("DIFF operator");
                // Phase 2.A plumbing: resolve the first C-field label as the source
                // concept, parse chunk count from D-field payload, call differentiate.
                // Step B will allocate the resulting chunks.
                if let Some(first) = decoded.operand_labels.first() {
                    let concept = self.resolve_single(first)?;
                    let n = Self::parse_chunk_count(&decoded.data_payload);
                    let _ = differentiate::differentiate(&concept, n);
                }
            }
            Opcode::Ground => {
                debug!("GRND operator");
                // Phase 2.A plumbing: parse D-field payload bytes as an `f32` chunk
                // and call ground. Step B will allocate the resulting concept node.
                let floats = Self::parse_f32_payload(&decoded.data_payload);
                let _ = ground::ground(&floats, &a2x_core::modality::Modality::Text);
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
                let _update = reflect::reflect(self.memory_trace.len());
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
        // Step A: no allocation yet — source node still alone.
        assert_eq!(vm.world_graph.node_count(), 1);
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
        // Step A: no allocation yet (ground's result is discarded here).
        assert_eq!(vm.world_graph.node_count(), 0);
    }

    #[test]
    fn test_step_ground_with_empty_payload_routes_to_operator() {
        let mut vm = CcsVm::new();
        let mut prog = SigmaProgram::new();
        prog.push(synth_ground_packet(&[]));
        vm.load(prog);
        let status = vm.step().unwrap();
        assert_eq!(status, VmStatus::Running);
        assert_eq!(vm.world_graph.node_count(), 0);
    }
}
