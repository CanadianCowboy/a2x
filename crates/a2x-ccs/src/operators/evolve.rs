// evolve operator: Time-step the VM — advance world state.
// See plans/03-ccs-vm.md §4
//
// Signature: (&WorldGraph, &StateField, Duration) → (WorldGraph, StateField)

use std::time::Duration;

use a2x_core::graph::WorldGraph;
use a2x_core::state::StateField;

/// Evolve the world state by one time step.
///
/// Phase 0 stub: returns success without modifying state.
/// Phase 2+: applies learned transition dynamics to WorldGraph and StateField.
pub fn evolve(
    _graph: &mut dyn WorldGraph,
    _state: &mut dyn StateField,
    _dt: Duration,
) -> Result<(), EvolveError> {
    // Phase 0: no-op
    Ok(())
}

/// Error during evolve operation.
#[derive(Clone, Debug, PartialEq)]
pub enum EvolveError {
    /// WorldGraph error during evolution.
    GraphError(String),
    /// StateField error during evolution.
    StateError(String),
}

impl std::fmt::Display for EvolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvolveError::GraphError(msg) => write!(f, "evolve graph error: {}", msg),
            EvolveError::StateError(msg) => write!(f, "evolve state error: {}", msg),
        }
    }
}

impl std::error::Error for EvolveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolve_stub_ok() {
        // Phase 0: evolve is a no-op, tested via VM integration
    }
}
