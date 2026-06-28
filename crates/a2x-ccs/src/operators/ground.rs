// ground operator: Attach raw perception into a ConceptVector.
// See plans/03-ccs-vm.md §4
//
// Signature: (&[f32], Modality) → ConceptVector

use a2x_core::concept::ConceptVector;
use a2x_core::modality::Modality;

/// Ground (attach) raw perception data into a ConceptVector with a modality tag.
///
/// Phase 0 stub: wraps the raw data directly into a ConceptVector.
/// Phase 2+: learned encoder that maps raw data through modality-specific layers.
pub fn ground(data: &[f32], _modality: &Modality) -> ConceptVector {
    ConceptVector::from_vec(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_wraps_data() {
        let cv = ground(&[1.0, 2.0, 3.0], &Modality::Text);
        assert_eq!(cv.data, vec![1.0, 2.0, 3.0]);
        assert_eq!(cv.dimensions, 3);
    }

    #[test]
    fn test_ground_empty() {
        let cv = ground(&[], &Modality::Audio);
        assert!(cv.data.is_empty());
    }
}
