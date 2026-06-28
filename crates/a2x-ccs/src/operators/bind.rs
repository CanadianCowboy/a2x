// bind operator: Merge concepts into a composite.
// See plans/03-ccs-vm.md §4
//
// Signature: (&[ConceptVector]) → ConceptVector

use a2x_core::concept::ConceptVector;

/// Bind (merge) multiple concept vectors into a single composite vector.
///
/// Phase 0 stub: averages the input vectors element-wise.
/// Phase 2+: learned weighted combination with attention.
pub fn bind(concepts: &[ConceptVector]) -> Option<ConceptVector> {
    if concepts.is_empty() {
        return None;
    }
    if concepts.len() == 1 {
        return Some(concepts[0].clone());
    }

    let dim = concepts[0].dimensions;
    let mut data = vec![0.0f32; dim];
    let n = concepts.len() as f32;

    for concept in concepts {
        for (i, v) in concept.data.iter().enumerate() {
            data[i] += v;
        }
    }

    for v in &mut data {
        *v /= n;
    }

    Some(ConceptVector::from_vec(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_empty() {
        assert!(bind(&[]).is_none());
    }

    #[test]
    fn test_bind_single() {
        let c = ConceptVector::from_vec(vec![1.0, 2.0]);
        let result = bind(std::slice::from_ref(&c)).unwrap();
        assert_eq!(result.data, c.data);
    }

    #[test]
    fn test_bind_multiple() {
        let a = ConceptVector::from_vec(vec![1.0, 0.0]);
        let b = ConceptVector::from_vec(vec![0.0, 1.0]);
        let result = bind(&[a, b]).unwrap();
        assert_eq!(result.data, vec![0.5, 0.5]);
    }
}
