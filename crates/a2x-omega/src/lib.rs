// a2x-omega — Ω compiled latent representation
// See plans/02-omega-compiler.md
//
// Provides: OmegaPacket, OmegaProgram, compilation pipeline, encoder/decoder traits

pub mod bridge;
pub mod compiler;
pub mod decoder;
pub mod ir;
pub mod packet;
pub mod passes;
pub mod program;

// Re-export key types
pub use bridge::Bridge;
pub use compiler::{CompileError, CompileToOmega};
pub use decoder::{DecompileError, DecompileToSigma};
pub use ir::{IrGraph, IrMetadata, IrNode, IrNodeId, IrOperand};
pub use packet::{
    OmegaPacket, OFFSET_C, OFFSET_D, OFFSET_I, OFFSET_P, SIZE_C, SIZE_D, SIZE_I, SIZE_P, TOTAL_DIM,
};
pub use passes::OptimizationLevel;
pub use program::OmegaProgram;
