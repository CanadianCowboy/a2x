// actuate operator: Emit an external side effect (syscall / I/O).
// See plans/03-ccs-vm.md §4
//
// Signature: (&ActionDistribution, &Embodiment) → ExternalCommand

/// A command to execute outside the VM.
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalCommand {
    /// The command name (e.g., "shell_exec", "http_request").
    pub command: String,
    /// The command payload.
    pub payload: Vec<u8>,
}

/// Actuate (emit) a side effect to the external world.
///
/// Phase 0 stub: returns a no-op command.
/// Phase 2+: maps ActionDistribution through safety constraints to real commands.
pub fn actuate() -> ExternalCommand {
    ExternalCommand {
        command: "nop".to_string(),
        payload: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actuate_stub() {
        let cmd = actuate();
        assert_eq!(cmd.command, "nop");
        assert!(cmd.payload.is_empty());
    }
}
