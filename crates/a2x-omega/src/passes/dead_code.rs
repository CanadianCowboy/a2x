// See plans/02-omega-compiler.md §3 (Stage 5)

use std::collections::HashSet;

use crate::ir::{IrGraph, IrNodeId};

/// Remove instructions whose results are never used.
///
/// A node is considered "used" if its `IrNodeId` appears in any other node's
/// `control_flow` list, or if it is the `entry`/`exit` node. When the graph
/// has no `entry` set and no `control_flow` edges, it is treated as a flat
/// sequential program and all nodes are retained (nothing is dead).
pub fn dead_code_elimination(ir: &mut IrGraph) {
    if ir.nodes.is_empty() {
        return;
    }

    let has_control_flow = ir.nodes.iter().any(|n| !n.control_flow.is_empty());

    // Flat sequential program (no entry/exit, no control_flow edges):
    // all nodes are live — nothing to eliminate.
    if ir.entry.is_none() && ir.exit.is_none() && !has_control_flow {
        return;
    }

    // Collect all referenced node IDs.
    let mut referenced: HashSet<IrNodeId> = HashSet::new();

    // Always keep entry and exit.
    if let Some(entry) = ir.entry {
        referenced.insert(entry);
    }
    if let Some(exit) = ir.exit {
        referenced.insert(exit);
    }

    // A node is live if it appears as a control_flow TARGET (consumed by
    // another node) or as a control_flow SOURCE (has outgoing edges —
    // part of the execution chain).
    for node in &ir.nodes {
        if !node.control_flow.is_empty() {
            referenced.insert(node.id); // source of control_flow → live
        }
        for &target in &node.control_flow {
            referenced.insert(target); // target of control_flow → live
        }
    }

    // Remove unreferenced nodes (preserving order).
    ir.nodes.retain(|node| referenced.contains(&node.id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, IrNode, IrNodeId};
    use a2x_core::Opcode;

    fn make_node(id: u32, control_flow: Vec<IrNodeId>) -> IrNode {
        IrNode {
            id: IrNodeId(id),
            opcode: Opcode::Bind,
            operands: vec![],
            control_flow,
            metadata: IrMetadata::default(),
        }
    }

    #[test]
    fn test_dead_code_flat_sequential_keeps_all() {
        // Flat sequential program (no entry, no control_flow): all nodes retained.
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, vec![]));
        ir.add_node(make_node(1, vec![]));
        ir.add_node(make_node(2, vec![]));

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 3);
    }

    #[test]
    fn test_dead_code_single_node_flat_keeps_it() {
        // Single node, no entry/exit/control_flow: flat sequential → keep it.
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, vec![]));

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 1);
    }

    #[test]
    fn test_dead_code_with_control_flow_removes_orphan() {
        // Graph with control_flow edges: node 0 → node 1, node 2 is orphaned.
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, vec![IrNodeId(1)]));
        ir.add_node(make_node(1, vec![]));
        ir.add_node(make_node(2, vec![]));

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 2);
        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert!(ids.contains(&IrNodeId(0)));
        assert!(ids.contains(&IrNodeId(1)));
    }

    #[test]
    fn test_dead_code_with_entry_removes_orphan() {
        // entry set: node 0 is entry, node 1 is orphaned, node 2 is referenced by 0.
        let mut ir = IrGraph::new();
        ir.entry = Some(IrNodeId(0));
        ir.add_node(make_node(0, vec![IrNodeId(2)]));
        ir.add_node(make_node(1, vec![])); // orphan
        ir.add_node(make_node(2, vec![]));

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 2);
        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert!(ids.contains(&IrNodeId(0)));
        assert!(ids.contains(&IrNodeId(2)));
    }

    #[test]
    fn test_dead_code_retains_entry_and_exit() {
        let mut ir = IrGraph::new();
        ir.entry = Some(IrNodeId(0));
        ir.exit = Some(IrNodeId(2));
        ir.add_node(make_node(0, vec![])); // entry — no control_flow refs
        ir.add_node(make_node(1, vec![])); // middle — orphaned
        ir.add_node(make_node(2, vec![])); // exit — no control_flow refs

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 2);
        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert!(ids.contains(&IrNodeId(0)));
        assert!(ids.contains(&IrNodeId(2)));
    }

    #[test]
    fn test_dead_code_empty_graph() {
        let mut ir = IrGraph::new();
        dead_code_elimination(&mut ir);
        assert_eq!(ir.node_count(), 0);
    }

    #[test]
    fn test_dead_code_removes_unreferenced_intermediate() {
        // Graph with control_flow: node 0 → node 2, node 1 is orphaned.
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, vec![IrNodeId(2)]));
        ir.add_node(make_node(1, vec![])); // orphan
        ir.add_node(make_node(2, vec![]));

        dead_code_elimination(&mut ir);

        assert_eq!(ir.node_count(), 2);
        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert!(ids.contains(&IrNodeId(0)));
        assert!(ids.contains(&IrNodeId(2)));
    }
}
