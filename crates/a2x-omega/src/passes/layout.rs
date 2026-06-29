// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

/// Reorder instructions for better cache locality in the VM's instruction
/// cache. Since the IR is a sequential instruction stream, we sort nodes by
/// their `source_index` metadata (stable sort preserves relative order of
/// nodes without source info).
///
/// This pass is **idempotent**: running it twice produces the same result.
pub fn layout_optimization(ir: &mut IrGraph) {
    if ir.nodes.len() < 2 {
        return;
    }

    ir.nodes
        .sort_by_key(|n| n.metadata.source_index.unwrap_or(usize::MAX));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, IrNode, IrNodeId};
    use a2x_core::Opcode;

    fn make_node(id: u32, source_index: Option<usize>) -> IrNode {
        IrNode {
            id: IrNodeId(id),
            opcode: Opcode::Bind,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata {
                source_index,
                ..IrMetadata::default()
            },
        }
    }

    #[test]
    fn test_layout_reorders_by_source_index() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(2, Some(2)));
        ir.add_node(make_node(0, Some(0)));
        ir.add_node(make_node(1, Some(1)));

        layout_optimization(&mut ir);

        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![IrNodeId(0), IrNodeId(1), IrNodeId(2)]);
    }

    #[test]
    fn test_layout_optimization_is_idempotent() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(2, Some(2)));
        ir.add_node(make_node(0, Some(0)));
        ir.add_node(make_node(1, Some(1)));

        layout_optimization(&mut ir);
        let after_first: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();

        layout_optimization(&mut ir);
        let after_second: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();

        assert_eq!(after_first, after_second);
    }

    #[test]
    fn test_layout_empty_graph() {
        let mut ir = IrGraph::new();
        layout_optimization(&mut ir);
        assert_eq!(ir.node_count(), 0);
    }

    #[test]
    fn test_layout_single_node() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Some(5)));
        layout_optimization(&mut ir);
        assert_eq!(ir.node_count(), 1);
        assert_eq!(ir.nodes[0].id, IrNodeId(0));
    }

    #[test]
    fn test_layout_nodes_without_source_index_go_last() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, None));
        ir.add_node(make_node(1, Some(0)));
        ir.add_node(make_node(2, None));

        layout_optimization(&mut ir);

        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        // Node 1 (source_index=0) should be first.
        assert_eq!(ids[0], IrNodeId(1));
        // Nodes 0 and 2 (no source_index) go after, preserving relative order.
    }

    #[test]
    fn test_layout_preserves_stable_order_for_equal_keys() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Some(1)));
        ir.add_node(make_node(1, Some(1)));
        ir.add_node(make_node(2, Some(1)));

        layout_optimization(&mut ir);

        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        // Stable sort preserves original order for equal keys.
        assert_eq!(ids, vec![IrNodeId(0), IrNodeId(1), IrNodeId(2)]);
    }
}
