// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

/// Reorder instructions for better cache locality in the VM's instruction cache.
pub fn layout_optimization(_ir: &mut IrGraph) {
    // Stub: no optimization performed in Phase 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_does_nothing() {
        let mut ir = IrGraph::new();
        let count = ir.node_count();
        layout_optimization(&mut ir);
        assert_eq!(ir.node_count(), count);
    }
}
