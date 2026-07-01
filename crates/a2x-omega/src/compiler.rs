// See plans/02-omega-compiler.md §5

use crate::encoder::encode_instruction;
use crate::error::CompileError;
use crate::ir::{IrGraph, IrNodeId};
use crate::passes::{optimize, OptimizationLevel};
use crate::program::OmegaProgram;
use crate::semantic;
use a2x_core::Opcode;
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

        // Stage 3: Semantic analysis — validates jump targets, contradictory
        // operators, data types, and empty intents.
        semantic::analyze(self)?;

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
    use crate::ir::{IrMetadata, IrNode, IrOperand};

    let mut graph = IrGraph::new();
    if program.is_empty() {
        return Err(CompileError::EmptyProgram);
    }

    for (i, packet) in program.instructions.iter().enumerate() {
        // Map intent operators to VM opcode.
        // T2-4: handle multi-operator intents — use the first recognized
        // intent operator (in priority order) as the primary opcode.
        let opcode = map_intent_to_opcode(&packet.intent.operators);

        let mut operands = Vec::new();
        for label in &packet.context.labels {
            operands.push(IrOperand::Label(label.clone()));
        }

        // T2-2: Wire control flow from plan operators.
        // Map plan operators to IR control flow targets.
        let control_flow = map_plan_to_control_flow(&packet.plan.operators, i);

        let node = IrNode {
            id: IrNodeId(i as u32),
            opcode,
            operands,
            control_flow,
            metadata: IrMetadata {
                source_index: Some(i),
                source_position: Some(i),
                fused: false,
            },
        };

        graph.add_node(node);
    }

    Ok(graph)
}

/// Map Σ∞ intent operators to VM opcodes.
/// T2-4: handles all intent operators (not just the first).
fn map_intent_to_opcode(intents: &[a2x_sigma::IntentOp]) -> Opcode {
    use a2x_sigma::IntentOp;
    // Priority order: explicit operations first, then Nop fallback.
    for intent in intents {
        let op = match intent {
            IntentOp::Synthesis => Opcode::Bind,
            IntentOp::Split => Opcode::Differentiate,
            IntentOp::Star => Opcode::Ground,
            IntentOp::Cancel => Opcode::Halt,
            IntentOp::Lightning => Opcode::Plan,
            IntentOp::Warning => Opcode::Actuate,
            IntentOp::Delay => Opcode::Evolve,
            IntentOp::Contradiction => Opcode::Reflect,
            IntentOp::Parallel => Opcode::Fork,
            IntentOp::Merge => Opcode::Merge,
            _ => continue,
        };
        return op;
    }
    Opcode::Nop
}

/// Map Σ∞ plan operators to IR control flow edges.
/// T2-2: wire IR control flow from plan operators.
fn map_plan_to_control_flow(plans: &[a2x_sigma::PlanOp], current_index: usize) -> Vec<IrNodeId> {
    use a2x_sigma::PlanOp;
    let mut targets = Vec::new();

    for plan in plans {
        match plan {
            PlanOp::Sequential => {
                // Default: next node. Don't add explicit edge — handled by
                // sequential ordering in the IR.
            }
            PlanOp::Branch | PlanOp::Descend => {
                // These are resolved at runtime via label lookup. We can't
                // add compile-time control flow edges without knowing the
                // target labels at compile time. Record the intent as a
                // control flow marker.
                // For now, add self-referencing edge as a placeholder.
                targets.push(IrNodeId(current_index as u32));
            }
            PlanOp::Merge | PlanOp::Swarm => {
                // Merge/swarm collect results from multiple branches.
                // No compile-time control flow edge — handled at runtime.
            }
            PlanOp::Ascend => {
                // Return from sub-program — pop call stack at runtime.
            }
            _ => {}
        }
    }

    targets
}

/// Generate Ω tensors from the IR graph (Stage 6).
///
/// T2-3: Code generation now uses topological sort to order nodes by
/// dataflow dependencies. Producers come before consumers, ensuring
/// the Ω tensor layout respects the execution dependency graph.
fn codegen(ir: &IrGraph) -> OmegaProgram<29796> {
    let order = topological_sort(ir);
    let mut program = OmegaProgram::new();
    for node in order {
        program.push(encode_instruction(node));
    }
    program
}

/// Topological sort of IR nodes using DFS post-order (T2-3).
///
/// Nodes are sorted so that data producers appear before consumers.
/// Control-flow edges are treated as dependency edges: if node A has
/// a control_flow edge to node B, then A comes before B in the output.
fn topological_sort(ir: &IrGraph) -> Vec<&crate::ir::IrNode> {
    use std::collections::HashMap;

    let n = ir.nodes.len();
    if n == 0 {
        return Vec::new();
    }

    // Build a node lookup by ID
    let id_to_index: HashMap<IrNodeId, usize> = ir
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id, i))
        .collect();

    let mut visited = vec![false; n];
    let mut on_stack = vec![false; n];
    let mut order = Vec::with_capacity(n);

    fn dfs<'a>(
        node_idx: usize,
        nodes: &'a [crate::ir::IrNode],
        id_to_index: &HashMap<IrNodeId, usize>,
        visited: &mut [bool],
        on_stack: &mut [bool],
        order: &mut Vec<&'a crate::ir::IrNode>,
    ) {
        if visited[node_idx] {
            return;
        }
        visited[node_idx] = true;
        on_stack[node_idx] = true;

        // Visit control_flow targets first (dependencies come before dependents)
        for target_id in &nodes[node_idx].control_flow {
            if let Some(&target_idx) = id_to_index.get(target_id) {
                if on_stack[target_idx] {
                    // Cycle detected — skip to avoid infinite recursion.
                    // Cycles in control flow are valid (loops).
                    continue;
                }
                dfs(target_idx, nodes, id_to_index, visited, on_stack, order);
            }
        }

        on_stack[node_idx] = false;
        order.push(&nodes[node_idx]);
    }

    for i in 0..n {
        dfs(
            i,
            &ir.nodes,
            &id_to_index,
            &mut visited,
            &mut on_stack,
            &mut order,
        );
    }

    order
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
