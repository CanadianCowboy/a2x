// See plans/09-core-types.md §2

/// CCS VM instruction opcodes.
///
/// These are the primitive operations the CCS runtime executes.
/// Σ∞ programs compile down to sequences of these opcodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Opcode {
    /// No operation (padding).
    Nop = 0x0,
    /// Merge concepts into composite (constructing a struct).
    Bind = 0x1,
    /// Split a concept into sub-concepts (destructuring).
    Differentiate = 0x2,
    /// Attach raw perception into a ConceptVector (I/O operation).
    Ground = 0x3,
    /// Time-step the VM: advance world state (clock cycle).
    Evolve = 0x4,
    /// Meta-learning from history (profiler-guided optimization).
    Reflect = 0x5,
    /// Generate action sequence (compiler generating instructions).
    Plan = 0x6,
    /// Emit an external side effect (syscall / I/O).
    Actuate = 0x7,
    /// Unconditional jump.
    Jump = 0x8,
    /// Conditional branch.
    Branch = 0x9,
    /// Call sub-program.
    Call = 0xA,
    /// Return from sub-program.
    Return = 0xB,
    /// Fork parallel execution.
    Fork = 0xC,
    /// Merge parallel branches.
    Merge = 0xD,
    /// Halt execution.
    Halt = 0xE,
    /// Reserved for user-defined custom instructions (opcode 0xF).
    Custom(u8),
}

impl Opcode {
    /// Get the numeric opcode value (0–14 for standard, 0xF for Custom).
    pub fn as_u8(&self) -> u8 {
        match self {
            Opcode::Nop => 0x0,
            Opcode::Bind => 0x1,
            Opcode::Differentiate => 0x2,
            Opcode::Ground => 0x3,
            Opcode::Evolve => 0x4,
            Opcode::Reflect => 0x5,
            Opcode::Plan => 0x6,
            Opcode::Actuate => 0x7,
            Opcode::Jump => 0x8,
            Opcode::Branch => 0x9,
            Opcode::Call => 0xA,
            Opcode::Return => 0xB,
            Opcode::Fork => 0xC,
            Opcode::Merge => 0xD,
            Opcode::Halt => 0xE,
            Opcode::Custom(_) => 0xF,
        }
    }

    /// Returns true if this opcode transfers control flow (jump, branch, call, return, fork, merge).
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Opcode::Jump
                | Opcode::Branch
                | Opcode::Call
                | Opcode::Return
                | Opcode::Fork
                | Opcode::Merge
        )
    }

    /// Returns true if this opcode has side effects outside the VM (Ground, Actuate).
    pub fn has_side_effects(&self) -> bool {
        matches!(self, Opcode::Ground | Opcode::Actuate)
    }
}
