// actuate operator: Emit an external side effect (syscall / I/O).
// See plans/03-ccs-vm.md §4
//
// Signature: (&ActionDistribution, &Embodiment) → ExternalCommand

use crate::operators::plan::Action;

/// A command to execute outside the VM.
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalCommand {
    /// The command name (e.g., "shell_exec", "http_request").
    pub command: String,
    /// The command payload.
    pub payload: Vec<u8>,
}

/// Generate a side-effect command from the current plan actions.
///
/// Reads the VM's last plan actions (sorted by priority) and emits the
/// highest-priority actionable command. This bridges the cognitive loop
/// (EVOLVE → REFLECT → PLAN) to the external world.
///
/// Priority mapping:
///   - Bind → no external command (internal graph operation)
///   - Ground → no external command (internal perception)
///   - Evolve → no external command (internal time-step)
///   - Snapshot → serialize current state (returns state bytes)
///   - Propose → emit proposal as text
pub fn actuate() -> ExternalCommand {
    actuate_from_actions(&[])
}

/// Generate a side-effect command from a specific set of plan actions.
///
/// Returns the highest-priority actionable command. If no actionable actions
/// exist, returns a NOP command.
pub fn actuate_from_actions(actions: &[Action]) -> ExternalCommand {
    // Filter to only externally-actionable verbs.
    for action in actions {
        match action.verb {
            crate::operators::plan::Verb::Propose => {
                return ExternalCommand {
                    command: "propose".to_string(),
                    payload: action
                        .target
                        .as_ref()
                        .map(|t| t.as_bytes().to_vec())
                        .unwrap_or_default(),
                };
            }
            crate::operators::plan::Verb::Snapshot => {
                return ExternalCommand {
                    command: "snapshot".to_string(),
                    payload: action
                        .target
                        .as_ref()
                        .map(|t| t.as_bytes().to_vec())
                        .unwrap_or_default(),
                };
            }
            // Bind, Ground, Evolve are internal operations — no external command.
            _ => continue,
        }
    }

    ExternalCommand {
        command: "nop".to_string(),
        payload: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::plan::{Action, Verb};

    #[test]
    fn test_actuate_stub() {
        let cmd = actuate();
        assert_eq!(cmd.command, "nop");
        assert!(cmd.payload.is_empty());
    }

    #[test]
    fn test_actuate_propose_action() {
        let actions = [
            Action::new(Verb::Bind, 0.5, Some("belief_5".into())),
            Action::new(Verb::Propose, 0.8, Some("scan_network".into())),
        ];
        let cmd = actuate_from_actions(&actions);
        assert_eq!(cmd.command, "propose");
        assert_eq!(cmd.payload, b"scan_network");
    }

    #[test]
    fn test_actuate_snapshot_action() {
        let actions = [Action::new(Verb::Snapshot, 0.0, Some("__last_plan".into()))];
        let cmd = actuate_from_actions(&actions);
        assert_eq!(cmd.command, "snapshot");
        assert_eq!(cmd.payload, b"__last_plan");
    }

    #[test]
    fn test_actuate_no_actionable_verbs_returns_nop() {
        let actions = [
            Action::new(Verb::Bind, 0.5, Some("x".into())),
            Action::new(Verb::Evolve, 0.3, Some("__reflect_1".into())),
        ];
        let cmd = actuate_from_actions(&actions);
        assert_eq!(cmd.command, "nop");
    }

    #[test]
    fn test_actuate_empty_actions_returns_nop() {
        let cmd = actuate_from_actions(&[]);
        assert_eq!(cmd.command, "nop");
    }
}
