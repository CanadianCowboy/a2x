// See plans/02-omega-compiler.md §5

use crate::error::DecompileError;
use crate::packet::OmegaPacket;
use a2x_sigma::SigmaPacket;

/// Trait for decompiling Ω tensor packets back into Σ∞ symbolic form.
///
/// Used for debugging, logging, and inspection of compiled programs.
pub trait DecompileToSigma: Sized {
    type Error;

    /// Attempt to reconstruct a Σ∞ packet from an Ω tensor.
    fn decompile(packet: &OmegaPacket) -> Result<Self, Self::Error>;
}

impl DecompileToSigma for SigmaPacket {
    type Error = DecompileError;

    fn decompile(_packet: &OmegaPacket) -> Result<Self, Self::Error> {
        // Phase 0 stub: decompilation not supported yet
        // Future: project tensor regions back to nearest symbolic operators
        // using a learned decoder or nearest-neighbor lookup
        Err(DecompileError::NoMatchingOperator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::OmegaPacket;

    #[test]
    fn test_decompile_stub_returns_error() {
        let pkt: OmegaPacket = OmegaPacket::zeros();
        let result = SigmaPacket::decompile(&pkt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DecompileError::NoMatchingOperator
        ));
    }
}
