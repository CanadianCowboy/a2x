// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

/// Evaluate constant `BIND` operations at compile time.
///
/// If all inputs to a `BIND` are known constants, compute the result now
/// instead of at runtime.
pub fn constant_folding(_ir: &mut IrGraph) {
    // Stub: no optimization performed in Phase 0
    // Future: walk the IR graph, identify BIND nodes with constant inputs,
    // compute the result ConceptVector and replace the node with the result.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_does_nothing() {
        let mut ir = IrGraph::new();
        let node_count = ir.node_count();
        constant_folding(&mut ir);
        assert_eq!(ir.node_count(), node_count);
    }
}
