// See plans/09-core-types.md §2

/// Content-addressed identifier for a program (Blake3 hash, 32 bytes).
///
/// ProgramIds are computed by hashing a program's contents. They enable
/// program caching, deduplication, and versioning within the A2X ecosystem.
///
/// # Note
/// The hash computation (Blake3) happens at a higher layer (a2x-sigma/a2x-bus).
/// At the core layer, ProgramId is a plain wrapper around bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProgramId([u8; 32]);

impl ProgramId {
    /// Create a new ProgramId from raw hash bytes.
    pub fn new(hash: [u8; 32]) -> Self {
        ProgramId(hash)
    }

    /// Get the raw hash bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create a zero ProgramId (used as placeholder/default).
    pub fn zero() -> Self {
        ProgramId([0u8; 32])
    }
}

impl std::fmt::Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.iter().take(8) {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "..")?;
        Ok(())
    }
}

impl Default for ProgramId {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_as_bytes() {
        let bytes = [42u8; 32];
        let pid = ProgramId::new(bytes);
        assert_eq!(pid.as_bytes(), &bytes);
    }

    #[test]
    fn test_zero() {
        let pid = ProgramId::zero();
        assert_eq!(pid.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_equality() {
        let a = ProgramId::new([1u8; 32]);
        let b = ProgramId::new([1u8; 32]);
        let c = ProgramId::new([2u8; 32]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
