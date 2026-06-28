// See plans/09-core-types.md §2

use crate::node::NodeId;

/// Semantic type tag for relations between concepts in the WorldGraph.
///
/// Each relation carries a type that determines how it's interpreted by the
/// CCS VM during execution (e.g., causal edges drive prediction, spatial
/// edges organize geometry).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RelationType {
    /// A causes B (or A is caused by B). Used for predictive reasoning.
    Causal,
    /// A is located at B (or A contains B spatially).
    Spatial,
    /// A happens before/after B. Used for temporal ordering.
    Temporal,
    /// A implies B (or A is a prerequisite for B). Used for logical deduction.
    Logical,
    /// A is a part of B (or A is a type of B). Used for hierarchical structure.
    Hierarchical,
    /// Custom user-defined relation type (4-byte namespace for extension).
    Custom([u8; 4]),
}

/// A directed, typed edge between two nodes in the WorldGraph.
///
/// Relations are learned transformations between concepts. Together with
/// ConceptVectors, they form the basis of the WorldGraph memory model.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationEdge {
    /// Source node ID.
    pub source: NodeId,
    /// Target node ID.
    pub target: NodeId,
    /// Type of relation (semantic category).
    pub relation_type: RelationType,
    /// Learned weight matrix stored as flat vector (optional, for neural relations).
    /// Shape is `(source_dim × target_dim)` when present.
    pub weight_matrix: Option<Vec<f32>>,
    /// Strength/confidence of this relation, in range [0.0, 1.0].
    pub strength: f32,
}

impl RelationEdge {
    /// Create a new edge without a weight matrix.
    pub fn new(source: NodeId, target: NodeId, relation_type: RelationType, strength: f32) -> Self {
        RelationEdge {
            source,
            target,
            relation_type,
            weight_matrix: None,
            strength: strength.clamp(0.0, 1.0),
        }
    }

    /// Create a new edge with a weight matrix.
    pub fn with_weights(
        source: NodeId,
        target: NodeId,
        relation_type: RelationType,
        weights: Vec<f32>,
        strength: f32,
    ) -> Self {
        RelationEdge {
            source,
            target,
            relation_type,
            weight_matrix: Some(weights),
            strength: strength.clamp(0.0, 1.0),
        }
    }

    /// Returns true if this edge has a learned weight matrix.
    pub fn has_weights(&self) -> bool {
        self.weight_matrix.is_some()
    }
}

impl std::fmt::Display for RelationEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({} --[{:?}: {:.2}]--> {})",
            self.source, self.relation_type, self.strength, self.target
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_edge() {
        let edge = RelationEdge::new(NodeId::new(1), NodeId::new(2), RelationType::Causal, 0.8);
        assert_eq!(edge.source, NodeId::new(1));
        assert_eq!(edge.target, NodeId::new(2));
        assert_eq!(edge.strength, 0.8);
        assert!(edge.weight_matrix.is_none());
    }

    #[test]
    fn test_strength_clamped() {
        let edge = RelationEdge::new(NodeId::new(1), NodeId::new(2), RelationType::Spatial, 2.5);
        assert_eq!(edge.strength, 1.0);
        assert!((-0.5f32).clamp(0.0, 1.0) == 0.0); // confirm clamp works for negative
    }

    #[test]
    fn test_with_weights() {
        let edge = RelationEdge::with_weights(
            NodeId::new(1),
            NodeId::new(2),
            RelationType::Logical,
            vec![0.5, 0.3],
            0.9,
        );
        assert!(edge.has_weights());
        assert_eq!(edge.weight_matrix.unwrap(), vec![0.5, 0.3]);
    }
}
