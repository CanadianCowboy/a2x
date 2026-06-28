// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

/// Remove instructions whose results are never used.
pub fn dead_code_elimination(_ir: &mut IrGraph) {
    // Stub: no optimization performed in Phase 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_does_nothing() {
        let mut ir = IrGraph::new();
        let count = ir.node_count();
        dead_code_elimination(&mut ir);
        assert_eq!(ir.node_count(), count);
    }
}
