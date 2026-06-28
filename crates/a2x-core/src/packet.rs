// See plans/09-core-types.md §2

/// Unified packet enum for transport within the A2X ecosystem.
///
/// At the core layer, only the `Raw` variant is available (since `a2x-core`
/// is zero-dependency). Typed `Sigma` and `Omega` variants are added at
/// higher layers (a2x-sigma and a2x-omega respectively).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Packet {
    /// Raw binary packet (serialized Σ∞ or Ω, or future protocol).
    Raw(Vec<u8>),
}

impl Packet {
    /// Create a new raw packet from bytes.
    pub fn raw(bytes: impl Into<Vec<u8>>) -> Self {
        Packet::Raw(bytes.into())
    }

    /// Get the raw bytes, if this is a raw packet.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Packet::Raw(bytes) => Some(bytes),
        }
    }

    /// Returns the number of bytes in this packet.
    pub fn len(&self) -> usize {
        match self {
            Packet::Raw(bytes) => bytes.len(),
        }
    }

    /// Returns true if the packet has no data.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
