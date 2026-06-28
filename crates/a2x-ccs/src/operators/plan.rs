// plan operator: Generate action sequence from current world state.
// See plans/03-ccs-vm.md §4
//
// Signature: (&WorldGraph, &StateField, &Goal) → Vec<Action>

use a2x_core::opcode::Opcode;

/// A generated action — maps to a CCS VM opcode.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// The opcode to execute.
    pub opcode: Opcode,
    /// Priority: higher = execute sooner.
    pub priority: f32,
    /// Optional target (label or node ID).
    pub target: Option<String>,
}

/// Generate a sequence of actions from the current world state.
///
/// Phase 0 stub: returns a single Nop action.
/// Phase 2+: learned planner that maps WorldGraph + goal → action sequence.
pub fn plan() -> Vec<Action> {
    vec![Action {
        opcode: Opcode::Nop,
        priority: 1.0,
        target: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_stub() {
        let actions = plan();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].opcode, Opcode::Nop);
    }
}
