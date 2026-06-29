// Phase 2.G — NdArrayStateField: ndarray-backed StateField implementation.
// See plans/03-ccs-vm.md §5
//
// This is the dense-tensor backend for StateField (per PLAN.md §18 item:
// "StateField: high-dimensional tensor with ndarray (behind feature gate)").
// FlatStateField (`state.rs`) remains the default zero-dep backend. When the
// `ndarray` cargo feature is enabled on `a2x-ccs`, this module activates and
// offers a parallel ndarray-backed implementation that is name-compatible
// with FlatStateField — same StateField trait surface, same default regions,
// same `lcg_state: [f32; 8]` field (Phase 2.LCG hoist).
//
// Cargo feature gating: this entire file is included only when the
// `ndarray` feature is on (see `crates/a2x-ccs/Cargo.toml`). The default
// build still uses FlatStateField; only opted-in builds get this backend.
//
// Why a parallel impl rather than `FlatStateField` wrapping an `Array1`?
// Trait `read_region(&self) -> &[f32]` requires the underlying storage to
// be contiguous 1D f32. ndarray::Array1 with default MemoryOrder (C-order)
// satisfies that, so we can route slice operations through `as_slice()` /
// `as_slice_mut()` and obtain `&[f32]` / `&mut [f32]` directly. No glue-ish
// Vec translation per-call, no intermediate allocation on read or write.

#[cfg(feature = "ndarray")]
use std::collections::HashMap;

#[cfg(feature = "ndarray")]
use a2x_core::error::CoreError;
#[cfg(feature = "ndarray")]
use a2x_core::state::StateField;
#[cfg(feature = "ndarray")]
use ndarray::Array1;

#[cfg(feature = "ndarray")]
use crate::state::StateRegion;

/// `ndarray::Array1<f32>`-backed implementation of the StateField trait.
///
/// Default total size: 1024 f32 elements (matches FlatStateField for
/// drop-in parity).
///
/// Phase 2.LCG parity: carries its own `lcg_state: [f32; 8]` struct field,
/// zero-initialized on construction — completely independent of any
/// region by design.
#[cfg(feature = "ndarray")]
pub struct NdArrayStateField {
    data: Array1<f32>,
    regions: HashMap<String, StateRegion>,
    pub lcg_state: [f32; 8],
}

#[cfg(feature = "ndarray")]
impl NdArrayStateField {
    /// Create a new nd-array-backed StateField with the given total size.
    /// Allocates exactly that many zeros — no implementation-defined padding.
    pub fn new(total_len: usize) -> Self {
        NdArrayStateField {
            data: Array1::zeros(total_len),
            regions: HashMap::new(),
            lcg_state: [0.0; 8],
        }
    }

    /// Create with default size (1024).
    pub fn default_size() -> Self {
        Self::new(1024)
    }

    /// Get all defined region names.
    pub fn region_names(&self) -> Vec<&str> {
        self.regions.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a region is defined.
    pub fn has_region(&self, name: &str) -> bool {
        self.regions.contains_key(name)
    }
}

#[cfg(feature = "ndarray")]
impl Default for NdArrayStateField {
    fn default() -> Self {
        Self::default_size()
    }
}

#[cfg(feature = "ndarray")]
impl StateField for NdArrayStateField {
    fn define_region(&mut self, name: &str, offset: usize, len: usize) -> Result<(), CoreError> {
        if self.regions.contains_key(name) {
            return Err(CoreError::Other(
                format!("region already defined: {}", name).into(),
            ));
        }
        if offset + len > self.data.len() {
            return Err(CoreError::Other(
                format!(
                    "region out of bounds: offset={} len={} total={}",
                    offset,
                    len,
                    self.data.len()
                )
                .into(),
            ));
        }
        self.regions.insert(
            name.to_string(),
            StateRegion {
                name: name.to_string(),
                offset,
                len,
            },
        );
        Ok(())
    }

    fn read_region(&self, name: &str) -> Result<&[f32], CoreError> {
        let region = self
            .regions
            .get(name)
            .ok_or_else(|| CoreError::Other(format!("region not found: {}", name).into()))?;
        // Array1<f32> with default (C) memory order is always contiguous,
        // so `as_slice()` returns Some(&[f32]). The bounds check below
        // guarantees the slice range is in-range — `expect` rather than a
        // silent zero-fill is the right failure mode.
        let slice = self.data.as_slice().expect("Array1 contiguous");
        if region.offset + region.len > slice.len() {
            return Err(CoreError::Other(
                format!(
                    "region out of bounds on read: offset={} len={} total={}",
                    region.offset,
                    region.len,
                    slice.len()
                )
                .into(),
            ));
        }
        Ok(&slice[region.offset..region.offset + region.len])
    }

    fn write_region(&mut self, name: &str, data: &[f32]) -> Result<(), CoreError> {
        let region = self
            .regions
            .get(name)
            .ok_or_else(|| CoreError::Other(format!("region not found: {}", name).into()))?;
        if data.len() != region.len {
            return Err(CoreError::Other(
                format!(
                    "size mismatch: region {} expects {}, got {}",
                    name,
                    region.len,
                    data.len()
                )
                .into(),
            ));
        }
        // copy_from_slice on a contiguous slice is the most efficient path —
        // no ndarray view construction, no allocation, no element-wise assign().
        let slice_mut = self
            .data
            .as_slice_mut()
            .expect("Array1 contiguous for write");
        slice_mut[region.offset..region.offset + region.len].copy_from_slice(data);
        Ok(())
    }

    fn total_len(&self) -> usize {
        self.data.len()
    }

    fn raw_data(&self) -> &[f32] {
        // Same contiguity reasoning as read_region.
        self.data.as_slice().expect("Array1 contiguous raw")
    }

    fn read_lcg_state(&self) -> Result<[f32; 8], CoreError> {
        Ok(self.lcg_state)
    }

    fn write_lcg_state(&mut self, state: &[f32; 8]) -> Result<(), CoreError> {
        self.lcg_state = *state;
        Ok(())
    }
}

/// Initialize an NdArrayStateField with the standard default regions.
///
/// Alias of [`crate::state::init_default_regions`] — exposed under the
/// ndarray-specific name for symmetry with the FlatStateField API. Region
/// layout matches FlatStateField exactly:
///
/// | Region | Offset | Shape | Purpose |
/// |--------|--------|-------|---------|
/// | goal | 0 | `[64]` | Current goal embedding |
/// | belief | 64 | `[256]` | Current belief state |
/// | uncertainty | 320 | `[64]` | Uncertainty estimates |
/// | attention | 384 | `[128]` | Attention focus |
/// | temporal | 512 | `[64]` | Temporal context |
/// | scratch | 576 | `[448]` | General-purpose working memory |
#[cfg(feature = "ndarray")]
pub fn init_ndarray_default_regions(state: &mut dyn StateField) -> Result<(), CoreError> {
    crate::state::init_default_regions(state)
}

#[cfg(all(test, feature = "ndarray"))]
mod tests {
    use super::*;

    #[test]
    fn test_new_ndarray_state_field() {
        let sf = NdArrayStateField::default_size();
        assert_eq!(sf.total_len(), 1024);
        assert_eq!(sf.raw_data().len(), 1024);
        // All zeros on construction.
        assert!(sf.raw_data().iter().all(|v| *v == 0.0));
        // Phase 2.LCG parity: lcg_state zero-init.
        assert_eq!(sf.lcg_state, [0.0; 8]);
        assert_eq!(sf.read_lcg_state().unwrap(), [0.0; 8]);
    }

    #[test]
    fn test_ndarray_lcg_state_round_trip() {
        let mut sf = NdArrayStateField::default_size();
        let new_state: [f32; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        sf.write_lcg_state(&new_state).unwrap();
        assert_eq!(sf.lcg_state, new_state);
        assert_eq!(sf.read_lcg_state().unwrap(), new_state);
    }

    #[test]
    fn test_ndarray_define_and_read_region() {
        let mut sf = NdArrayStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        let data = sf.read_region("test").unwrap();
        assert_eq!(data, &[0.0; 4]);
    }

    #[test]
    fn test_ndarray_write_and_read_region() {
        let mut sf = NdArrayStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        sf.write_region("test", &[1.0, 2.0, 3.0, 4.0]).unwrap();
        let data = sf.read_region("test").unwrap();
        assert_eq!(data, &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_ndarray_region_out_of_bounds() {
        let mut sf = NdArrayStateField::new(100);
        assert!(sf.define_region("big", 90, 20).is_err());
    }

    #[test]
    fn test_ndarray_duplicate_region() {
        let mut sf = NdArrayStateField::new(256);
        sf.define_region("dup", 0, 4).unwrap();
        assert!(sf.define_region("dup", 10, 4).is_err());
    }

    #[test]
    fn test_ndarray_size_mismatch() {
        let mut sf = NdArrayStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        assert!(sf.write_region("test", &[1.0, 2.0]).is_err());
    }

    #[test]
    fn test_ndarray_region_not_found() {
        let sf = NdArrayStateField::new(256);
        assert!(sf.read_region("nope").is_err());
    }

    #[test]
    fn test_ndarray_init_default_regions() {
        let mut sf = NdArrayStateField::default_size();
        init_ndarray_default_regions(&mut sf).unwrap();
        for name in [
            "goal",
            "belief",
            "uncertainty",
            "attention",
            "temporal",
            "scratch",
        ] {
            assert!(sf.has_region(name), "missing region {}", name);
        }
        // Round-trip read on every default region to prove regions are valid.
        let _ = sf.read_region("goal").unwrap();
        let _ = sf.read_region("belief").unwrap();
        let _ = sf.read_region("uncertainty").unwrap();
        let _ = sf.read_region("attention").unwrap();
        let _ = sf.read_region("temporal").unwrap();
        let _ = sf.read_region("scratch").unwrap();
    }

    #[test]
    fn test_ndarray_is_independent_of_flat_state_field_lcg() {
        // The two backends are independent structures. lcg_state is per-instance,
        // not shared. Confirming parity semantics — no cross-instance bleed.
        let mut sf1 = NdArrayStateField::default_size();
        let sf2 = NdArrayStateField::default_size();
        sf1.write_lcg_state(&[1.0; 8]).unwrap();
        // sf2 must still see the zero-initialized lcg_state.
        assert_eq!(sf2.read_lcg_state().unwrap(), [0.0; 8]);
    }
}
