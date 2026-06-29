// See plans/02-omega-compiler.md §3 (Stage 5)

use a2x_core::Opcode;

use crate::ir::{IrGraph, IrNodeId, IrOperand};

/// Evaluate constant `BIND` operations at compile time.
///
/// If all inputs to a `Bind` node are `IrOperand::Immediate`, merge them
/// into a single immediate and replace the opcode with `Nop` (the result
/// is already folded into the operand). This eliminates redundant runtime
/// concept-merging for programs with known constant payloads.
pub fn constant_folding(ir: &mut IrGraph) {
    let node_ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();

    for id in node_ids {
        let node = match ir.nodes.iter_mut().find(|n| n.id == id) {
            Some(n) => n,
            None => continue,
        };

        if node.opcode != Opcode::Bind || node.operands.is_empty() {
            continue;
        }

        let all_immediate = node
            .operands
            .iter()
            .all(|op| matches!(op, IrOperand::Immediate(_)));

        if !all_immediate {
            continue;
        }

        // Merge all immediate byte vectors into one and replace with Nop.
        let merged: Vec<u8> = node
            .operands
            .iter()
            .flat_map(|op| match op {
                IrOperand::Immediate(bytes) => bytes.clone(),
                _ => unreachable!("checked above"),
            })
            .collect();
        node.opcode = Opcode::Nop;
        node.operands = vec![IrOperand::Immediate(merged)];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, IrNode};

    fn make_bind_node(id: u32, operands: Vec<IrOperand>) -> IrNode {
        IrNode {
            id: IrNodeId(id),
            opcode: Opcode::Bind,
            operands,
            control_flow: vec![],
            metadata: IrMetadata::default(),
        }
    }

    #[test]
    fn test_constant_folding_folds_all_immediate_bind() {
        let mut ir = IrGraph::new();
        ir.add_node(make_bind_node(
            0,
            vec![
                IrOperand::Immediate(vec![1, 2]),
                IrOperand::Immediate(vec![3, 4]),
            ],
        ));
        assert_eq!(ir.node_count(), 1);

        constant_folding(&mut ir);

        // Node should still exist but opcode changed to Nop with merged immediate.
        assert_eq!(ir.node_count(), 1);
        let node = &ir.nodes[0];
        assert_eq!(node.opcode, Opcode::Nop);
        assert_eq!(node.operands, vec![IrOperand::Immediate(vec![1, 2, 3, 4])]);
    }

    #[test]
    fn test_constant_folding_skips_non_immediate_operands() {
        let mut ir = IrGraph::new();
        ir.add_node(make_bind_node(
            0,
            vec![
                IrOperand::Immediate(vec![1]),
                IrOperand::Label("sys".to_string()),
            ],
        ));

        constant_folding(&mut ir);

        // Opcode should remain Bind — not all operands are Immediate.
        assert_eq!(ir.nodes[0].opcode, Opcode::Bind);
    }

    #[test]
    fn test_constant_folding_skips_non_bind_opcodes() {
        let mut ir = IrGraph::new();
        ir.add_node(IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Differentiate,
            operands: vec![IrOperand::Immediate(vec![1])],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        });

        constant_folding(&mut ir);

        assert_eq!(ir.nodes[0].opcode, Opcode::Differentiate);
    }

    #[test]
    fn test_constant_folding_handles_empty_operands() {
        let mut ir = IrGraph::new();
        ir.add_node(make_bind_node(0, vec![]));

        constant_folding(&mut ir);

        // Empty operands: the node.operands.is_empty() check prevents folding.
        assert_eq!(ir.nodes[0].opcode, Opcode::Bind);
    }

    #[test]
    fn test_constant_folding_empty_graph() {
        let mut ir = IrGraph::new();
        constant_folding(&mut ir);
        assert_eq!(ir.node_count(), 0);
    }
}
