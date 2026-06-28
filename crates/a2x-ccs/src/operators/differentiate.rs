// differentiate operator: Split a concept into sub-concepts.
// See plans/03-ccs-vm.md §4
//
// Signature: (&ConceptVector, usize) → Vec<ConceptVector>

use a2x_core::concept::ConceptVector;

/// Differentiate (split) a concept vector into `n` sub-concept vectors.
///
/// Phase 0 stub: splits the vector evenly into `n` contiguous chunks.
/// Phase 2+: learned decomposition via attention or SVD.
pub fn differentiate(concept: &ConceptVector, n: usize) -> Vec<ConceptVector> {
    if n == 0 || concept.data.is_empty() {
        return Vec::new();
    }

    let chunk_size = concept.dimensions.div_ceil(n);
    let mut results = Vec::with_capacity(n);

    for i in 0..n {
        let start = i * chunk_size;
        let end = concept.dimensions.min(start + chunk_size);
        let mut data = vec![0.0f32; concept.dimensions];
        let slice = &concept.data[start..end];
        for (j, &v) in slice.iter().enumerate() {
            data[start + j] = v;
        }
        results.push(ConceptVector::from_vec(data));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_differentiate_empty() {
        let c = ConceptVector::from_vec(vec![]);
        let results = differentiate(&c, 3);
        assert!(results.is_empty());
    }

    #[test]
    fn test_differentiate_n_zero() {
        let c = ConceptVector::from_vec(vec![1.0, 2.0]);
        let results = differentiate(&c, 0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_differentiate_two() {
        let c = ConceptVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        let results = differentiate(&c, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].data[0], 1.0);
        assert_eq!(results[0].data[1], 2.0);
        assert_eq!(results[1].data[2], 3.0);
        assert_eq!(results[1].data[3], 4.0);
    }
}
