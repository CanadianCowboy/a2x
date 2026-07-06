// See plans/03-ccs-vm.md §4 and plans/12-security.md

use a2x_core::opcode::Opcode;

/// Safety classification for an instruction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SafetyClassification {
    /// Does this instruction execute system commands?
    pub requires_exec: bool,
    /// Does this instruction read files?
    pub requires_fs_read: bool,
    /// Does this instruction write files?
    pub requires_fs_write: bool,
    /// Does this instruction make network requests?
    pub requires_network: bool,
    /// Bounds on memory allocation (number of WorldGraph nodes).
    pub max_allocation: Option<u64>,
    /// Bounds on execution time (number of VM steps).
    pub max_steps: Option<u64>,
}

/// Safety level for VM execution.
#[derive(Clone, Debug, PartialEq)]
pub enum SafetyLevel {
    /// No restrictions (dev only).
    Unrestricted,
    /// Bounded execution: limits on loops, memory, side effects.
    Bounded {
        max_instructions: u64,
        max_memory_bytes: u64,
        max_side_effects: u32,
        allowed_opcodes: Vec<Opcode>,
    },
    /// Sandboxed: all side effects filtered through allowlist.
    Sandboxed { allowed_opcodes: Vec<Opcode> },
    /// Full isolation: no side effects at all.
    Isolated,
}

impl Default for SafetyLevel {
    fn default() -> Self {
        SafetyLevel::Bounded {
            max_instructions: 10_000,
            max_memory_bytes: 256 * 1024 * 1024,
            max_side_effects: 100,
            allowed_opcodes: vec![
                Opcode::Nop,
                Opcode::Bind,
                Opcode::Differentiate,
                Opcode::Ground,
                Opcode::Evolve,
                Opcode::Reflect,
                Opcode::Plan,
                Opcode::Jump,
                Opcode::Branch,
                Opcode::Call,
                Opcode::Return,
                Opcode::Fork,
                Opcode::Merge,
                Opcode::Halt,
            ],
        }
    }
}

/// Safety constraints evaluator for the CCS VM.
#[derive(Clone, Debug)]
pub struct SafetyConstraints {
    /// Current safety level.
    pub level: SafetyLevel,
    /// Number of instructions executed so far.
    pub steps_executed: u64,
    /// Number of side effects emitted.
    pub side_effects_emitted: u32,
    /// Number of WorldGraph nodes allocated this run.
    pub nodes_allocated: u64,
}

impl SafetyConstraints {
    /// Create new constraints with the given level.
    pub fn new(level: SafetyLevel) -> Self {
        SafetyConstraints {
            level,
            steps_executed: 0,
            side_effects_emitted: 0,
            nodes_allocated: 0,
        }
    }

    /// Check if an opcode is allowed under the current safety level.
    pub fn check_opcode(&self, opcode: Opcode) -> Result<(), String> {
        match &self.level {
            SafetyLevel::Unrestricted => Ok(()),
            SafetyLevel::Bounded {
                allowed_opcodes, ..
            }
            | SafetyLevel::Sandboxed { allowed_opcodes } => {
                if allowed_opcodes.contains(&opcode) {
                    Ok(())
                } else {
                    Err(format!(
                        "opcode {:?} not allowed under safety level",
                        opcode
                    ))
                }
            }
            SafetyLevel::Isolated => {
                if opcode.has_side_effects() {
                    Err(format!(
                        "opcode {:?} has side effects; not allowed in Isolated mode",
                        opcode
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Increment instruction counter and check bounds.
    pub fn step(&mut self) -> Result<(), String> {
        self.steps_executed += 1;
        match &self.level {
            SafetyLevel::Bounded {
                max_instructions, ..
            } => {
                if self.steps_executed > *max_instructions {
                    return Err(format!("max instructions {} exceeded", max_instructions));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Record a side effect emission.
    pub fn record_side_effect(&mut self) -> Result<(), String> {
        self.side_effects_emitted += 1;
        match &self.level {
            SafetyLevel::Bounded {
                max_side_effects, ..
            } => {
                if self.side_effects_emitted > *max_side_effects {
                    return Err(format!("max side effects {} exceeded", max_side_effects));
                }
                Ok(())
            }
            SafetyLevel::Isolated => Err("no side effects allowed in Isolated mode".into()),
            _ => Ok(()),
        }
    }

    /// Record a node allocation and enforce memory budget.
    ///
    /// Tracks the number of nodes allocated and checks against
    /// `max_memory_bytes` when the safety level is `Bounded`.
    /// The VM calls this after each WorldGraph allocation.
    pub fn record_allocation(&mut self) -> Result<(), String> {
        self.nodes_allocated += 1;

        // Enforce max_memory_bytes: estimate ~4KB per node (ConceptVector
        // data + metadata + edges). This is a heuristic — precise byte
        // tracking would require the actual allocation size.
        match &self.level {
            SafetyLevel::Bounded {
                max_memory_bytes, ..
            } => {
                let estimated_bytes = self.nodes_allocated.saturating_mul(4096);
                if estimated_bytes > *max_memory_bytes {
                    return Err(format!(
                        "memory limit exceeded: estimated {} bytes > {} max",
                        estimated_bytes, max_memory_bytes
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Check if an operation is safe to perform.
    pub fn check_classification(&self, class: &SafetyClassification) -> Result<(), String> {
        match &self.level {
            SafetyLevel::Unrestricted => Ok(()),
            SafetyLevel::Isolated => {
                if class.requires_exec
                    || class.requires_fs_read
                    || class.requires_fs_write
                    || class.requires_network
                {
                    return Err("side effects not allowed in Isolated mode".into());
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl Default for SafetyConstraints {
    fn default() -> Self {
        Self::new(SafetyLevel::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unrestricted_allows_all() {
        let safety = SafetyConstraints::new(SafetyLevel::Unrestricted);
        assert!(safety.check_opcode(Opcode::Actuate).is_ok());
        assert!(safety.check_opcode(Opcode::Ground).is_ok());
    }

    #[test]
    fn test_isolated_blocks_side_effects() {
        let safety = SafetyConstraints::new(SafetyLevel::Isolated);
        assert!(safety.check_opcode(Opcode::Actuate).is_err());
        assert!(safety.check_opcode(Opcode::Nop).is_ok());
    }

    #[test]
    fn test_bounded_allows_safe_ops() {
        let safety = SafetyConstraints::new(SafetyLevel::Bounded {
            max_instructions: 100,
            max_memory_bytes: 1024,
            max_side_effects: 10,
            allowed_opcodes: vec![Opcode::Nop, Opcode::Bind],
        });
        assert!(safety.check_opcode(Opcode::Nop).is_ok());
        assert!(safety.check_opcode(Opcode::Bind).is_ok());
        assert!(safety.check_opcode(Opcode::Actuate).is_err());
    }

    #[test]
    fn test_step_counting() {
        let mut safety = SafetyConstraints::new(SafetyLevel::Bounded {
            max_instructions: 3,
            max_memory_bytes: 1024,
            max_side_effects: 10,
            allowed_opcodes: vec![Opcode::Nop],
        });
        assert!(safety.step().is_ok());
        assert!(safety.step().is_ok());
        assert!(safety.step().is_ok());
        assert!(safety.step().is_err()); // 4th step exceeds max
    }

    #[test]
    fn test_default_bounded_allows_compute_ops() {
        // Locks the default SafetyLevel::Bounded allowlist so a future edit can't
        // accidentally drop `Ground` (or any of the other CCS compute operators)
        // and silently break their VM-level dispatch.
        let safety = SafetyConstraints::default();
        for op in [
            Opcode::Nop,
            Opcode::Bind,
            Opcode::Differentiate,
            Opcode::Ground,
            Opcode::Evolve,
            Opcode::Reflect,
            Opcode::Plan,
        ] {
            assert!(
                safety.check_opcode(op).is_ok(),
                "default Bounded must allow {:?}",
                op
            );
        }
    }
}
