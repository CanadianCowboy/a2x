// a2x-omega — Ω compiled latent representation
// See plans/02-omega-compiler.md
//
// Provides: OmegaPacket, OmegaProgram, compilation pipeline, encoder/decoder traits

pub mod bridge;
pub mod compiler;
pub mod decoder;
pub mod encoder;
#[cfg(feature = "learned")]
pub mod environment;
pub mod error;
pub mod ir;
#[cfg(feature = "learned")]
pub mod learned_decoder;
#[cfg(feature = "learned")]
pub mod learned_encoder;
#[cfg(feature = "serde")]
pub mod mcp_bridge;
pub mod packet;
pub mod passes;
pub mod program;
pub mod semantic;
#[cfg(feature = "learned")]
pub mod training;

// Re-export key types
pub use bridge::Bridge;
pub use compiler::CompileToOmega;
pub use decoder::DecompileToSigma;
pub use encoder::encode_instruction;
pub use error::{CompileError, DecompileError, SemanticError};
pub use ir::{IrGraph, IrMetadata, IrNode, IrNodeId, IrOperand};
pub use packet::{
    OmegaPacket, OFFSET_C, OFFSET_D, OFFSET_I, OFFSET_P, SIZE_C, SIZE_D, SIZE_I, SIZE_P, TOTAL_DIM,
};
pub use passes::OptimizationLevel;
pub use program::OmegaProgram;
