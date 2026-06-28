// See plans/02-omega-compiler.md §3 (Stage 5)

use crate::ir::IrGraph;

mod constant_folding;
mod dead_code;
mod fusion;
mod layout;

pub use constant_folding::*;
pub use dead_code::*;
pub use fusion::*;
pub use layout::*;

/// Run all optimizer passes on the IR graph.
pub fn optimize(ir: &mut IrGraph, level: OptimizationLevel) {
    match level {
        OptimizationLevel::None => {}
        OptimizationLevel::Light => {
            constant_folding(ir);
            dead_code_elimination(ir);
        }
        OptimizationLevel::Balanced | OptimizationLevel::Aggressive | OptimizationLevel::Size => {
            constant_folding(ir);
            dead_code_elimination(ir);
            instruction_fusion(ir);
            layout_optimization(ir);
        }
    }
}

/// Compiler optimization level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimization passes (debug/development).
    None,
    /// Constant folding + dead code elimination (default).
    #[default]
    Light,
    /// All standard passes (production).
    Balanced,
    /// All passes + speculative optimization (hot paths).
    Aggressive,
    /// All passes, optimize for tensor size (bandwidth-constrained).
    Size,
}
