// See plans/02-omega-compiler.md §6

use crate::compiler::CompileToOmega;
use crate::decoder::DecompileToSigma;
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
    ) -> Result<crate::program::OmegaProgram<29796>, crate::compiler::CompileError> {
        program.compile(level)
    }

    /// Decompile an Ω packet back to Σ∞.
    ///
    /// Returns None if decompilation fails (Phase 0: always fails, stub).
    pub fn decompile(packet: &crate::packet::OmegaPacket) -> Option<SigmaPacket> {
        SigmaPacket::decompile(packet).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_sigma::parse_program;

    #[test]
    fn test_bridge_compile_decompile() {
        let input = "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭";
        let prog = parse_program(input).unwrap();
        let omega = Bridge::compile(&prog, OptimizationLevel::default()).unwrap();
        // Decompilation is a stub — always returns None in Phase 0
        let decompiled = Bridge::decompile(&omega.instructions[0]);
        assert!(decompiled.is_none());
    }
}
