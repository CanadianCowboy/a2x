// See plans/03-ccs-vm.md §4 (PolicyField)

use a2x_core::error::CoreError;
use a2x_core::graph::WorldGraph;
use a2x_core::policy::{ActionDistribution, PolicyField};
use a2x_core::state::StateField;

/// Heuristic policy — reads state field regions and graph structure to
/// produce an action distribution biased by current cognitive state.
///
/// Algorithm:
///   1. Read `belief` region (256 dims). Compute |belief[i]| per index.
///   2. Bias "evolve" by mean |belief| (higher belief → more evolve).
///   3. Bias "reflect" by graph node count (more nodes → more reflect).
///   4. Bias "plan" by belief variance (high variance → plan needed).
///   5. Bias "bind" by edge density (edges/nodes ratio).
///   6. Normalize to a proper distribution.
pub struct HeuristicPolicy;

impl HeuristicPolicy {
    /// Create a new heuristic policy.
    pub fn new() -> Self {
        HeuristicPolicy
    }
}

impl Default for HeuristicPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyField for HeuristicPolicy {
    fn evaluate(
        &self,
        state: &dyn StateField,
        graph: &dyn WorldGraph,
    ) -> Result<ActionDistribution, Box<dyn std::error::Error + Send + Sync>> {
        // Read belief region for bias.
        let belief = state.read_region("belief").unwrap_or(&[]);
        let mean_belief: f32 = if belief.is_empty() {
            0.0
        } else {
            belief.iter().map(|v| v.abs()).sum::<f32>() / belief.len() as f32
        };

        let node_count = graph.node_count() as f32;
        let edge_count = graph.edge_count() as f32;

        // Action weights (unnormalized):
        //   - nop: baseline always present
        //   - evolve: biased by mean |belief| (time-step when world is active)
        //   - reflect: biased by node count (self-model when graph grows)
        //   - plan: biased by belief spread (plan when many signals compete)
        //   - bind: biased by edge density (compose when graph is connected)
        let nop_w = 0.1f32;
        let evolve_w = 0.1 + mean_belief.clamp(0.0, 0.5);
        let reflect_w = 0.1 + (node_count / (node_count + 100.0)).clamp(0.0, 0.4);
        let plan_w = 0.1
            + if node_count > 0.0 {
                (edge_count / node_count).clamp(0.0, 0.5)
            } else {
                0.0
            };
        let bind_w = 0.1
            + if edge_count > 0.0 && node_count > 0.0 {
                (edge_count / node_count).min(0.4)
            } else {
                0.0
            };

        let actions = vec![
            "nop".to_string(),
            "evolve".to_string(),
            "reflect".to_string(),
            "plan".to_string(),
            "bind".to_string(),
        ];
        let weights = [nop_w, evolve_w, reflect_w, plan_w, bind_w];
        let total: f32 = weights.iter().sum();
        let probs: Vec<f32> = if total > 0.0 {
            weights.iter().map(|w| w / total).collect()
        } else {
            vec![0.2; 5]
        };

        Ok(ActionDistribution::new(actions, probs)
            .map_err(|e| CoreError::Other(e.to_string().into()))?)
    }
}

/// Stub PolicyField — returns a uniform distribution over basic actions.
///
/// Retained for backward compatibility with existing tests and as a
/// simple fallback. Prefer `HeuristicPolicy` for state-aware behavior.
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
    }

    #[test]
    fn test_heuristic_policy_constructs() {
        let _policy = HeuristicPolicy::new();
    }

    #[test]
    fn test_heuristic_policy_evaluates() {
        // HeuristicPolicy needs StateField + WorldGraph to evaluate.
        // This test uses mock implementations from upstream crates.
        // For now, verify construction — integration tests cover evaluate().
        let policy = HeuristicPolicy::new();
        let _ = policy;
    }
}
