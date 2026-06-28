// See plans/02-omega-compiler.md §3

use crate::packet::OmegaPacket;
use a2x_core::ProgramId;
use a2x_sigma::program::ProgramMetadata;

/// A compiled Ω program — a sequence of latent tensor packets.
///
/// This is the compiled form of a Σ∞ program. The CCS VM executes Ω programs
/// natively at maximum speed without any symbolic parsing overhead.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OmegaProgram<const N: usize> {
    /// The instruction stream as Ω latent tensors.
    pub instructions: Vec<OmegaPacket<N>>,
    /// Program metadata (author, version, description).
    pub metadata: ProgramMetadata,
    /// Optional source program ID (if compiled from a known Σ∞ program).
    pub source_id: Option<ProgramId>,
}

impl<const N: usize> OmegaProgram<N> {
    /// Create an empty Ω program.
    pub fn new() -> Self {
        OmegaProgram {
            instructions: Vec::new(),
            metadata: ProgramMetadata::default(),
            source_id: None,
        }
    }

    /// Push a packet to the instruction stream.
    pub fn push(&mut self, packet: OmegaPacket<N>) {
        self.instructions.push(packet);
    }

    /// Number of instructions.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Returns true if the program has no instructions.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

impl<const N: usize> Default for OmegaProgram<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_program_is_empty() {
        let prog: OmegaProgram<29796> = OmegaProgram::new();
        assert!(prog.is_empty());
        assert_eq!(prog.len(), 0);
    }

    #[test]
    fn test_push() {
        let mut prog: OmegaProgram<29796> = OmegaProgram::new();
        prog.push(OmegaPacket::zeros());
        assert_eq!(prog.len(), 1);
    }
}
