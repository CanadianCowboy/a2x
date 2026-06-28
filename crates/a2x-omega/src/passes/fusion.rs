// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

/// Merge adjacent instructions that operate on the same memory region
/// into a single fused operation.
pub fn instruction_fusion(_ir: &mut IrGraph) {
    // Stub: no optimization performed in Phase 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_does_nothing() {
        let mut ir = IrGraph::new();
        let count = ir.node_count();
        instruction_fusion(&mut ir);
        assert_eq!(ir.node_count(), count);
    }
}
