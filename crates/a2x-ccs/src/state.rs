// See plans/03-ccs-vm.md §5 (StateField)

use std::collections::HashMap;

use a2x_core::error::CoreError;
use a2x_core::state::StateField;

/// A named region within the StateField.
#[derive(Clone, Debug, PartialEq)]
pub struct StateRegion {
    pub name: String,
    pub offset: usize,
    pub len: usize,
}

/// `Vec<f32>`-backed implementation of the StateField trait.
///
/// The StateField is the agent's high-dimensional working memory — analogous
/// to CPU registers + stack. It holds named regions backed by a flat `Vec<f32>`.
///
/// Default total size: 1024 f32 values.
pub struct FlatStateField {
    data: Vec<f32>,
    regions: HashMap<String, StateRegion>,
}

impl FlatStateField {
    /// Create a new StateField with the given total size.
    pub fn new(total_len: usize) -> Self {
        FlatStateField {
            data: vec![0.0; total_len],
            regions: HashMap::new(),
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

impl Default for FlatStateField {
    fn default() -> Self {
        Self::default_size()
    }
}

impl StateField for FlatStateField {
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
        Ok(&self.data[region.offset..region.offset + region.len])
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
        self.data[region.offset..region.offset + region.len].copy_from_slice(data);
        Ok(())
    }

    fn total_len(&self) -> usize {
        self.data.len()
    }

    fn raw_data(&self) -> &[f32] {
        &self.data
    }
}

/// Initialize a StateField with the standard default regions.
///
/// Default regions per plans/03-ccs-vm.md §5:
/// | Region | Offset | Shape | Purpose |
/// |--------|--------|-------|---------|
/// | goal | 0 | `[64]` | Current goal embedding |
/// | belief | 64 | `[256]` | Current belief state |
/// | uncertainty | 320 | `[64]` | Uncertainty estimates |
/// | attention | 384 | `[128]` | Attention focus |
/// | temporal | 512 | `[64]` | Temporal context |
/// | scratch | 576 | `[448]` | General-purpose working memory |
pub fn init_default_regions(state: &mut dyn StateField) -> Result<(), CoreError> {
    state.define_region("goal", 0, 64)?;
    state.define_region("belief", 64, 256)?;
    state.define_region("uncertainty", 320, 64)?;
    state.define_region("attention", 384, 128)?;
    state.define_region("temporal", 512, 64)?;
    state.define_region("scratch", 576, 448)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_field() {
        let sf = FlatStateField::default_size();
        assert_eq!(sf.total_len(), 1024);
        assert_eq!(sf.raw_data().len(), 1024);
    }

    #[test]
    fn test_define_and_read_region() {
        let mut sf = FlatStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        let data = sf.read_region("test").unwrap();
        assert_eq!(data, &[0.0; 4]);
    }

    #[test]
    fn test_write_and_read_region() {
        let mut sf = FlatStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        sf.write_region("test", &[1.0, 2.0, 3.0, 4.0]).unwrap();
        let data = sf.read_region("test").unwrap();
        assert_eq!(data, &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_region_out_of_bounds() {
        let mut sf = FlatStateField::new(100);
        assert!(sf.define_region("big", 90, 20).is_err());
    }

    #[test]
    fn test_duplicate_region() {
        let mut sf = FlatStateField::new(256);
        sf.define_region("dup", 0, 4).unwrap();
        assert!(sf.define_region("dup", 10, 4).is_err());
    }

    #[test]
    fn test_size_mismatch() {
        let mut sf = FlatStateField::new(256);
        sf.define_region("test", 0, 4).unwrap();
        assert!(sf.write_region("test", &[1.0, 2.0]).is_err());
    }

    #[test]
    fn test_region_not_found() {
        let sf = FlatStateField::new(256);
        assert!(sf.read_region("nope").is_err());
    }

    #[test]
    fn test_init_default_regions() {
        let mut sf = FlatStateField::default_size();
        init_default_regions(&mut sf).unwrap();
        assert!(sf.has_region("goal"));
        assert!(sf.has_region("belief"));
        assert!(sf.has_region("uncertainty"));
        assert!(sf.has_region("attention"));
        assert!(sf.has_region("temporal"));
        assert!(sf.has_region("scratch"));
    }
}
