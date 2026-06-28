// See plans/09-core-types.md §2

/// Protocol identifier for A2X layers.
///
/// Tags messages as Σ∞ (symbolic), Ω (compiled latent), or raw bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolId {
    /// Σ∞ — the symbolic programming language / ISA.
    Sigma,
    /// Ω — the compiled latent tensor representation.
    Omega,
    /// Raw binary (for future/unknown protocols).
    Raw,
}

impl ProtocolId {
    /// Returns the 2-bit wire encoding for this protocol.
    /// Σ∞ = 0b00, Ω = 0b01, Raw = 0b11
    pub fn as_bits(&self) -> u8 {
        match self {
            ProtocolId::Sigma => 0b00,
            ProtocolId::Omega => 0b01,
            ProtocolId::Raw => 0b11,
        }
    }
}
