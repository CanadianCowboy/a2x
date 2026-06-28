// See plans/09-core-types.md §2 (ConceptVector full definition and operations)

use crate::error::CoreError;

/// A dense embedding representing a concept, object, event, or abstraction.
/// This is the atomic value type in the A2X language.
///
/// ConceptVectors are the "values" that A2X programs operate on — analogous to
/// integers/floats in traditional languages, but high-dimensional and semantic.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConceptVector {
    /// The embedding data.
    pub data: Vec<f32>,
    /// Optional human-readable label (for debug/probe only; not used in execution).
    pub label: Option<String>,
    /// Dimensionality hint (for validation). Must match `data.len()`.
    pub dimensions: usize,
}

impl ConceptVector {
    /// Create a zero-initialized vector of given dimensionality.
    pub fn zeros(dim: usize) -> Self {
        ConceptVector {
            data: vec![0.0; dim],
            label: None,
            dimensions: dim,
        }
    }

    /// Create from raw data. Sets `dimensions` from `data.len()`.
    pub fn from_vec(data: Vec<f32>) -> Self {
        let dim = data.len();
        ConceptVector {
            data,
            label: None,
            dimensions: dim,
        }
    }

    /// Create with a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Euclidean (L2) norm.
    pub fn norm(&self) -> f32 {
        let sum_sq: f32 = self.data.iter().map(|x| x * x).sum();
        sum_sq.sqrt()
    }

    /// Cosine similarity with another ConceptVector.
    /// Returns 0.0 if either vector has zero norm.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        self.check_dims_match(other).ok(); // silently fail for sim
        let dot: f32 = self
            .data
            .iter()
            .zip(other.data.iter())
            .take(self.dimensions.min(other.dimensions))
            .map(|(a, b)| a * b)
            .sum();
        let na = self.norm();
        let nb = other.norm();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }

    /// Element-wise addition. Both vectors must have the same dimensionality.
    pub fn add(&self, other: &Self) -> Result<Self, CoreError> {
        self.check_dims_match(other)?;
        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a + b)
            .collect();
        Ok(ConceptVector {
            dimensions: self.dimensions,
            data,
            label: None,
        })
    }

    /// Element-wise subtraction.
    pub fn subtract(&self, other: &Self) -> Result<Self, CoreError> {
        self.check_dims_match(other)?;
        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a - b)
            .collect();
        Ok(ConceptVector {
            dimensions: self.dimensions,
            data,
            label: None,
        })
    }

    /// Element-wise multiplication (Hadamard product).
    pub fn multiply(&self, other: &Self) -> Result<Self, CoreError> {
        self.check_dims_match(other)?;
        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .collect();
        Ok(ConceptVector {
            dimensions: self.dimensions,
            data,
            label: None,
        })
    }

    /// Scale by a scalar.
    pub fn scale(&self, factor: f32) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x * factor).collect();
        ConceptVector {
            dimensions: self.dimensions,
            data,
            label: self.label.clone(),
        }
    }

    /// Dot product.
    pub fn dot(&self, other: &Self) -> f32 {
        self.data
            .iter()
            .zip(other.data.iter())
            .take(self.dimensions.min(other.dimensions))
            .map(|(a, b)| a * b)
            .sum()
    }

    /// Validate internal consistency.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.data.len() != self.dimensions {
            return Err(CoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: self.data.len(),
            });
        }
        Ok(())
    }

    fn check_dims_match(&self, other: &Self) -> Result<(), CoreError> {
        if self.dimensions != other.dimensions {
            return Err(CoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: other.dimensions,
            });
        }
        Ok(())
    }
}

impl std::fmt::Display for ConceptVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref label) = self.label {
            write!(f, "<{}> (dim={})", label, self.dimensions)
        } else {
            write!(f, "ConceptVector(dim={})", self.dimensions)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeros() {
        let v = ConceptVector::zeros(3);
        assert_eq!(v.dimensions, 3);
        assert_eq!(v.data, vec![0.0; 3]);
    }

    #[test]
    fn test_add() {
        let a = ConceptVector::from_vec(vec![1.0, 2.0, 3.0]);
        let b = ConceptVector::from_vec(vec![4.0, 5.0, 6.0]);
        let c = a.add(&b).unwrap();
        assert_eq!(c.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_add_dimension_mismatch() {
        let a = ConceptVector::from_vec(vec![1.0, 2.0]);
        let b = ConceptVector::from_vec(vec![3.0, 4.0, 5.0]);
        assert!(a.add(&b).is_err());
    }

    #[test]
    fn test_cosine_similarity_same() {
        let a = ConceptVector::from_vec(vec![1.0, 0.0]);
        assert!((a.cosine_similarity(&a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = ConceptVector::from_vec(vec![1.0, 0.0]);
        let b = ConceptVector::from_vec(vec![0.0, 1.0]);
        assert!((a.cosine_similarity(&b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_norm() {
        let a = ConceptVector::zeros(3);
        let b = ConceptVector::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn test_norm() {
        let v = ConceptVector::from_vec(vec![3.0, 4.0]);
        assert!((v.norm() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_scale() {
        let v = ConceptVector::from_vec(vec![1.0, 2.0, 3.0]);
        let s = v.scale(2.0);
        assert_eq!(s.data, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_validate_passes() {
        let v = ConceptVector::from_vec(vec![1.0, 2.0]);
        assert!(v.validate().is_ok());
    }

    #[test]
    fn test_validate_fails() {
        let mut v = ConceptVector::from_vec(vec![1.0, 2.0]);
        v.dimensions = 3;
        assert!(v.validate().is_err());
    }
}
