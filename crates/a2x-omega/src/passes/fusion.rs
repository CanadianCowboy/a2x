// See plans/02-omega-compiler.md §3 (Stage 5)

use a2x_core::Opcode;

use crate::ir::{IrGraph, IrNodeId, IrOperand};

/// Merge adjacent `Bind` + `Differentiate` pairs that share the same label
/// set into a single fused `IrNode` with `metadata.fused = true`.
///
/// This eliminates the round-trip of merging concepts just to split them
/// again — a common pattern in exploratory Σ∞ programs where the orchestrator
/// Bind-scopes a context then immediately Differentiate to inspect sub-parts.
pub fn instruction_fusion(ir: &mut IrGraph) {
    if ir.nodes.len() < 2 {
        return;
    }

    // Find adjacent Bind → Differentiate pairs with matching label sets.
    // Store (bind_id, diff_id) so we can look up by ID after removals.
    let mut pairs_to_fuse: Vec<(IrNodeId, IrNodeId)> = Vec::new();
    for window in ir.nodes.windows(2) {
        let (a, b) = (&window[0], &window[1]);
        if a.opcode == Opcode::Bind && b.opcode == Opcode::Differentiate {
            let a_labels: Vec<_> = a
                .operands
                .iter()
                .filter_map(|op| match op {
                    IrOperand::Label(l) => Some(l.clone()),
                    _ => None,
                })
                .collect();
            let b_labels: Vec<_> = b
                .operands
                .iter()
                .filter_map(|op| match op {
                    IrOperand::Label(l) => Some(l.clone()),
                    _ => None,
                })
                .collect();
            if a_labels == b_labels && !a_labels.is_empty() {
                pairs_to_fuse.push((a.id, b.id));
            }
        }
    }

    // Merge the Differentiate into the Bind: keep Bind's opcode, mark as fused,
    // and remove the Differentiate node.
    for (bind_id, diff_id) in pairs_to_fuse {
        // Remove the Differentiate node by ID (safe: position() handles index shifts).
        if let Some(diff_idx) = ir.nodes.iter().position(|n| n.id == diff_id) {
            let diff_node = ir.nodes.remove(diff_idx);
            // Find THE correct Bind node by ID (not just any Bind).
            if let Some(bind) = ir.nodes.iter_mut().find(|n| n.id == bind_id) {
                bind.metadata.fused = true;
                // Inherit control flow from the Differentiate node.
                bind.control_flow = diff_node.control_flow;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, IrNode};

    fn make_node(id: u32, opcode: Opcode, labels: Vec<&str>) -> IrNode {
        IrNode {
            id: IrNodeId(id),
            opcode,
            operands: labels
                .into_iter()
                .map(|l| IrOperand::Label(l.to_string()))
                .collect(),
            control_flow: vec![],
            metadata: IrMetadata::default(),
        }
    }

    #[test]
    fn test_fusion_merges_adjacent_bind_diff() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        ir.add_node(make_node(1, Opcode::Differentiate, vec!["sys"]));

        instruction_fusion(&mut ir);

        // Differentiate should be removed, Bind marked as fused.
        assert_eq!(ir.node_count(), 1);
        assert_eq!(ir.nodes[0].opcode, Opcode::Bind);
        assert!(ir.nodes[0].metadata.fused);
    }

    #[test]
    fn test_fusion_skips_different_label_sets() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        ir.add_node(make_node(1, Opcode::Differentiate, vec!["other"]));

        instruction_fusion(&mut ir);

        // Different labels — no fusion.
        assert_eq!(ir.node_count(), 2);
    }

    #[test]
    fn test_fusion_skips_empty_labels() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec![]));
        ir.add_node(make_node(1, Opcode::Differentiate, vec![]));

        instruction_fusion(&mut ir);

        // Empty labels — no fusion (would be a degenerate case).
        assert_eq!(ir.node_count(), 2);
    }

    #[test]
    fn test_fusion_skips_non_adjacent() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        ir.add_node(make_node(1, Opcode::Plan, vec![])); // interrupting node
        ir.add_node(make_node(2, Opcode::Differentiate, vec!["sys"]));

        instruction_fusion(&mut ir);

        // Not adjacent — no fusion.
        assert_eq!(ir.node_count(), 3);
    }

    #[test]
    fn test_fusion_empty_graph() {
        let mut ir = IrGraph::new();
        instruction_fusion(&mut ir);
        assert_eq!(ir.node_count(), 0);
    }

    #[test]
    fn test_fusion_single_node() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        instruction_fusion(&mut ir);
        assert_eq!(ir.node_count(), 1);
    }

    #[test]
    fn test_fusion_preserves_control_flow() {
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        let mut diff = make_node(1, Opcode::Differentiate, vec!["sys"]);
        diff.control_flow = vec![IrNodeId(2)];
        ir.add_node(diff);
        ir.add_node(IrNode {
            id: IrNodeId(2),
            opcode: Opcode::Halt,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        });

        instruction_fusion(&mut ir);

        assert_eq!(ir.node_count(), 2);
        let bind = &ir.nodes[0];
        assert!(bind.metadata.fused);
        assert_eq!(bind.control_flow, vec![IrNodeId(2)]);
    }

    #[test]
    fn test_fusion_multiple_pairs_fuse_correctly() {
        // Bind(0)+Diff(1) on "sys", Bind(2)+Diff(3) on "scope"
        let mut ir = IrGraph::new();
        ir.add_node(make_node(0, Opcode::Bind, vec!["sys"]));
        ir.add_node(make_node(1, Opcode::Differentiate, vec!["sys"]));
        ir.add_node(make_node(2, Opcode::Bind, vec!["scope"]));
        ir.add_node(make_node(3, Opcode::Differentiate, vec!["scope"]));

        instruction_fusion(&mut ir);

        // Both Differentiates removed, both Binds marked fused.
        assert_eq!(ir.node_count(), 2);
        assert!(ir.nodes.iter().all(|n| n.metadata.fused));
        let ids: Vec<IrNodeId> = ir.nodes.iter().map(|n| n.id).collect();
        assert!(ids.contains(&IrNodeId(0)));
        assert!(ids.contains(&IrNodeId(2)));
    }
}
