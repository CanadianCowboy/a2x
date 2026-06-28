// See plans/02-omega-compiler.md §5

use crate::error::CompileError;
use crate::ir::IrGraph;
use crate::packet::{OmegaPacket, SIZE_C, SIZE_D, SIZE_I, SIZE_P};
use crate::passes::{optimize, OptimizationLevel};
use crate::program::OmegaProgram;
use a2x_sigma::SigmaProgram;

/// Trait for compiling a Σ∞ program into Ω latent tensors.
///
/// Phase 0 uses deterministic hash-based projection as a placeholder.
/// Future phases will use learned neural encoders and optimizing compilers.
pub trait CompileToOmega {
    type Error;

    /// Compile this program into an Ω program.
    fn compile(&self, level: OptimizationLevel) -> Result<OmegaProgram<29796>, Self::Error>;
}

impl CompileToOmega for SigmaProgram {
    type Error = CompileError;

    fn compile(&self, level: OptimizationLevel) -> Result<OmegaProgram<29796>, Self::Error> {
        if self.is_empty() {
            return Err(CompileError::EmptyProgram);
        }

        // Stage 3: Semantic analysis (stub — validate basic structure)
        // Stage 4: IR generation
        let mut ir = build_ir(self)?;

        // Stage 5: Optimization
        optimize(&mut ir, level);

        // Stage 6: Code generation
        let mut program = codegen(&ir);
        program.source_id = Some(self.id);

        Ok(program)
    }
}

/// Build the IR graph from a Σ∞ program (Stage 4).
fn build_ir(program: &SigmaProgram) -> Result<IrGraph, CompileError> {
    use crate::ir::{IrMetadata, IrNode, IrNodeId, IrOperand};
    use a2x_core::Opcode;

    let mut graph = IrGraph::new();
    if program.is_empty() {
        return Err(CompileError::EmptyProgram);
    }

    for (i, packet) in program.instructions.iter().enumerate() {
        // Map intent operators to VM opcode (Phase 0: simplified mapping)
        let opcode = if packet.intent.is_empty() {
            Opcode::Nop
        } else {
            match packet.intent.operators[0] {
                a2x_sigma::IntentOp::Synthesis => Opcode::Bind,
                a2x_sigma::IntentOp::Split => Opcode::Differentiate,
                a2x_sigma::IntentOp::Star => Opcode::Ground,
                a2x_sigma::IntentOp::Cancel => Opcode::Halt,
                a2x_sigma::IntentOp::Lightning => Opcode::Plan,
                a2x_sigma::IntentOp::Warning => Opcode::Actuate,
                _ => Opcode::Nop,
            }
        };

        let mut operands = Vec::new();
        for label in &packet.context.labels {
            operands.push(IrOperand::Label(label.clone()));
        }

        let node = IrNode {
            id: IrNodeId(i as u32),
            opcode,
            operands,
            control_flow: Vec::new(),
            metadata: IrMetadata {
                source_index: Some(i),
                source_position: Some(i),
            },
        };

        graph.add_node(node);
    }

    Ok(graph)
}

/// Generate Ω tensors from the IR graph (Stage 6).
fn codegen(ir: &IrGraph) -> OmegaProgram<29796> {
    let mut program = OmegaProgram::new();
    for node in &ir.nodes {
        program.push(encode_instruction(node));
    }
    program
}

/// Encode a single IR node as an Ω tensor packet.
///
/// Phase 0: deterministic Blake3-based projection of the opcode, operands,
/// and control flow into the 4 tensor regions.
fn encode_instruction(node: &crate::ir::IrNode) -> OmegaPacket<29796> {
    let mut packet = OmegaPacket::<29796>::zeros();

    // Project opcode → intent region (I)
    let hash = blake3::hash(&node.opcode.as_u8().to_le_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_I) {
        packet.intent_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project operands → context region (C)
    let op_str = format!("{:?}", &node.operands);
    let hash = blake3::hash(op_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_C) {
        packet.context_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project control flow → plan region (P)
    let cf_str = format!("{:?}", &node.control_flow);
    let hash = blake3::hash(cf_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_P) {
        packet.plan_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project metadata → data region (D)
    let meta_str = format!("{:?}", &node.metadata.source_index);
    let hash = blake3::hash(meta_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_D) {
        packet.data_slice_mut()[j] = byte as f32 / 255.0;
    }

    packet
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_sigma::parse_program;

    #[test]
    fn test_compile_empty() {
        let prog = SigmaProgram::new();
        let result = prog.compile(OptimizationLevel::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_simple_program() {
        let input = "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭";
        let prog = parse_program(input).unwrap();
        let result = prog.compile(OptimizationLevel::Light);
        assert!(result.is_ok());
        let omega = result.unwrap();
        assert_eq!(omega.len(), 1);
    }

    #[test]
    fn test_compile_multi_instruction() {
        let input = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨scope⟩ ∷ P:⥂ ∷ D:⌵⟭⟦Σ∞⟧⟬I:✕ ∷ C:⟘ ∷ P:⤉ ∷ D:⟘⟭";
        let prog = parse_program(input).unwrap();
        let result = prog.compile(OptimizationLevel::Light);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
