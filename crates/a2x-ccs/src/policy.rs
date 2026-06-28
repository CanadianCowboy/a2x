// See plans/03-ccs-vm.md §4 (PolicyField)

use a2x_core::error::CoreError;
use a2x_core::graph::WorldGraph;
use a2x_core::policy::{ActionDistribution, PolicyField};
use a2x_core::state::StateField;

/// Stub PolicyField — returns a uniform distribution over basic actions.
///
/// Phase 0: Always returns a distribution that favors Nop (no action).
/// Phase 2+: Replace with learned neural network.
pub struct StubPolicy;

impl StubPolicy {
    /// Create a new stub policy.
    pub fn new() -> Self {
        StubPolicy
    }
}

impl Default for StubPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyField for StubPolicy {
    fn evaluate(
        &self,
        _state: &dyn StateField,
        _graph: &dyn WorldGraph,
    ) -> Result<ActionDistribution, Box<dyn std::error::Error + Send + Sync>> {
        // Stub: uniform over basic CCS operations
        let actions = vec![
            "nop".to_string(),
            "evolve".to_string(),
            "reflect".to_string(),
            "plan".to_string(),
        ];
        let probs = vec![0.25, 0.25, 0.25, 0.25];
        Ok(ActionDistribution::new(actions, probs)
            .map_err(|e| CoreError::Other(e.to_string().into()))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_policy_constructs() {
        let _policy = StubPolicy::new();
        // Phase 0: evaluate method tested via VM integration tests
    }
}
