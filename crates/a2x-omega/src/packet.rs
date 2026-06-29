// See plans/02-omega-compiler.md §2

/// A single Ω latent tensor packet — a compiled Σ∞ instruction.
///
/// Each packet is a high-dimensional tensor segmented into 4 regions:
/// - Ω_I (intent): bytes 0..1023
/// - Ω_C (context): bytes 1024..5119
/// - Ω_P (plan): bytes 5120..13311
/// - Ω_D (data): bytes 13312..N-1
///
/// The total dimension N defaults to 29,796 and is configurable via const generics.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OmegaPacket<const N: usize = 29796> {
    /// Flat tensor storage.
    pub data: [f32; N],
}

/// Slice offsets within the Ω tensor (compile-time constants).
pub const OFFSET_I: usize = 0;
pub const OFFSET_C: usize = 1024;
pub const OFFSET_P: usize = 1024 + 4096; // 5120
pub const OFFSET_D: usize = 1024 + 4096 + 8192; // 13312

pub const SIZE_I: usize = 1024;
pub const SIZE_C: usize = 4096;
pub const SIZE_P: usize = 8192;
pub const SIZE_D: usize = 16484;
pub const TOTAL_DIM: usize = OFFSET_D + SIZE_D; // 29796

impl<const N: usize> OmegaPacket<N> {
    /// Create a zero-initialized packet.
    pub fn zeros() -> Self {
        OmegaPacket { data: [0.0; N] }
    }

    /// Access the intent region (Ω_I).
    pub fn intent_slice(&self) -> &[f32] {
        &self.data[OFFSET_I..OFFSET_C]
    }

    /// Mutable access to the intent region.
    pub fn intent_slice_mut(&mut self) -> &mut [f32] {
        &mut self.data[OFFSET_I..OFFSET_C]
    }

    /// Access the context region (Ω_C).
    pub fn context_slice(&self) -> &[f32] {
        &self.data[OFFSET_C..OFFSET_P]
    }

    /// Mutable access to the context region.
    pub fn context_slice_mut(&mut self) -> &mut [f32] {
        &mut self.data[OFFSET_C..OFFSET_P]
    }

    /// Access the plan region (Ω_P).
    pub fn plan_slice(&self) -> &[f32] {
        &self.data[OFFSET_P..OFFSET_D]
    }

    /// Mutable access to the plan region.
    pub fn plan_slice_mut(&mut self) -> &mut [f32] {
        &mut self.data[OFFSET_P..OFFSET_D]
    }

    /// Access the data region (Ω_D).
    pub fn data_slice(&self) -> &[f32] {
        &self.data[OFFSET_D..N]
    }

    /// Mutable access to the data region.
    pub fn data_slice_mut(&mut self) -> &mut [f32] {
        &mut self.data[OFFSET_D..N]
    }

    /// Create from a raw f32 array.
    pub fn from_raw(data: [f32; N]) -> Self {
        OmegaPacket { data }
    }
}

impl<const N: usize> Default for OmegaPacket<N> {
    fn default() -> Self {
        Self::zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let pkt = OmegaPacket::<TOTAL_DIM>::zeros();
        assert_eq!(pkt.data.len(), TOTAL_DIM);
        assert!(pkt.data.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_slice_sizes() {
        let pkt = OmegaPacket::<TOTAL_DIM>::zeros();
        assert_eq!(pkt.intent_slice().len(), SIZE_I);
        assert_eq!(pkt.context_slice().len(), SIZE_C);
        assert_eq!(pkt.plan_slice().len(), SIZE_P);
        assert_eq!(pkt.data_slice().len(), SIZE_D);
    }

    #[test]
    fn test_slice_mutation() {
        let mut pkt = OmegaPacket::<TOTAL_DIM>::zeros();
        pkt.intent_slice_mut()[0] = 1.0;
        assert_eq!(pkt.intent_slice()[0], 1.0);
        // Verify other regions still zero
        assert_eq!(pkt.context_slice()[0], 0.0);
    }
}
