// See plans/09-core-types.md §2-3 (ActionDistribution, PolicyField trait)

use crate::error::CoreError;
use crate::graph::WorldGraph;
use crate::state::StateField;

/// Probability distribution over available actions.
///
/// The PolicyField produces this distribution, and the CCS VM samples from it
/// (or takes the argmax for deterministic execution).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionDistribution {
    /// Action labels (for symbolic mapping to intent operators).
    pub actions: Vec<String>,
    /// Probability for each action. Must sum to 1.0.
    pub probabilities: Vec<f32>,
}

impl ActionDistribution {
    /// Create a new ActionDistribution. Validates that probabilities and
    /// actions have the same length and probabilities sum to ~1.0.
    pub fn new(actions: Vec<String>, probabilities: Vec<f32>) -> Result<Self, CoreError> {
        if actions.len() != probabilities.len() {
            return Err(CoreError::Other(
                "actions and probabilities must have same length".into(),
            ));
        }
        let sum: f32 = probabilities.iter().sum();
        if (sum - 1.0).abs() > 0.01 {
            return Err(CoreError::Other("probabilities must sum to 1.0".into()));
        }
        Ok(ActionDistribution {
            actions,
            probabilities,
        })
    }

    /// Get the most likely action.
    pub fn argmax(&self) -> Option<&str> {
        let idx = self
            .probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
        self.actions.get(idx.0).map(|s| s.as_str())
    }
}

/// Trait for the policy (JIT compiler + optimizer).
///
/// The PolicyField is responsible for mapping the agent's current state
/// (WorldGraph + StateField) into a distribution over actions. Think of it
/// as the "brain" of the CCS VM — it decides what to do next.
///
/// The actual implementation (neural network, symbolic planner, etc.)
/// lives in `a2x-ccs`.
pub trait PolicyField: Send + Sync {
    /// Evaluate the policy given current world state.
    fn evaluate(
        &self,
        state: &dyn StateField,
        graph: &dyn WorldGraph,
    ) -> Result<ActionDistribution, Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let dist = ActionDistribution::new(vec!["a".into(), "b".into()], vec![0.3, 0.7]).unwrap();
        assert_eq!(dist.actions.len(), 2);
    }

    #[test]
    fn test_new_mismatched_lengths() {
        let result = ActionDistribution::new(vec!["a".into()], vec![0.3, 0.7]);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_sum_not_one() {
        let result = ActionDistribution::new(vec!["a".into(), "b".into()], vec![0.3, 0.3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_argmax() {
        let dist =
            ActionDistribution::new(vec!["low".into(), "high".into()], vec![0.1, 0.9]).unwrap();
        assert_eq!(dist.argmax(), Some("high"));
    }
}
