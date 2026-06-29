// See plans/02-omega-compiler.md §6

use crate::compiler::CompileToOmega;
use crate::decoder::DecompileToSigma;
use crate::error::CompileError;
use crate::passes::OptimizationLevel;
use a2x_sigma::SigmaPacket;
use a2x_sigma::SigmaProgram;

/// The Σ∞ ↔ Ω bridge — coordinates the compiler toolchain.
///
/// ```text
/// Σ∞ source  ──compile──→ Ω latent  ──execute──→ CCS runtime
/// Ω latent   ──decompile──→ Σ∞ source  ──log/debug──→ human peek
/// ```
pub struct Bridge;

impl Bridge {
    /// Compile a Σ∞ program to Ω with the given optimization level.
    pub fn compile(
        program: &SigmaProgram,
        level: OptimizationLevel,
    ) -> Result<crate::program::OmegaProgram<29796>, CompileError> {
        program.compile(level)
    }

    /// Decompile an Ω packet back to Σ∞.
    ///
    /// Returns `None` if the packet cannot be decoded (e.g. unknown opcode,
    /// control-flow opcode with no canonical IntentOp mapping).
    pub fn decompile(packet: &crate::packet::OmegaPacket<29796>) -> Option<SigmaPacket> {
        SigmaPacket::decompile(packet).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_sigma::intent::IntentOp;
    use a2x_sigma::parse_program;

    #[test]
    fn test_bridge_compile_decompile() {
        // ⚡ (Lightning) encodes to Opcode::Plan; the real Phase 3.1
        // decoder inverts Opcode::Plan → IntentOp::Lightning.
        let input = "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥁ ∷ D:⌬⟭";
        let prog = parse_program(input).unwrap();
        let omega = Bridge::compile(&prog, OptimizationLevel::default()).unwrap();
        let decompiled = Bridge::decompile(&omega.instructions[0]);
        let sigma = decompiled.expect("Phase 3.1 decoder must decode ⚡");
        assert_eq!(sigma.intent.operators, vec![IntentOp::Lightning]);
    }

    #[test]
    fn test_bridge_decompile_unmapped_returns_none() {
        // Opcode::Branch has no canonical IntentOp — decoder errors,
        // so Bridge::decompile returns None.
        use crate::packet::OmegaPacket;
        use a2x_core::Opcode;

        let mut pkt: OmegaPacket = OmegaPacket::zeros();
        let hash = *blake3::hash(&[Opcode::Branch.as_u8()]).as_bytes();
        for (j, &byte) in hash.iter().enumerate().take(32) {
            pkt.intent_slice_mut()[j] = byte as f32 / 255.0;
        }
        assert!(Bridge::decompile(&pkt).is_none());
    }
}
