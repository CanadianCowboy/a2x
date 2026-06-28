// See plans/01-sigma-language.md §3

/// Intent operators control the goal type, urgency, and mode of an instruction.
///
/// Each intent operator maps to an execution mode in the CCS VM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IntentOp {
    /// ⚡ U+26A1 — Immediate execution (skip safety checks, max priority).
    Lightning,
    /// ⚠ U+26A0 — Critical risk (enable all safety constraints, log everything).
    Warning,
    /// ✦ U+2726 — Discovery/exploration (allow non-deterministic branching).
    Star,
    /// ✣ U+2723 — Synthesis (merge concepts into composite).
    Synthesis,
    /// ✕ U+2715 — Cancel (halt execution).
    Cancel,
    /// ⟁ U+27C1 — Contradiction (conflict detected in parallel merge).
    Contradiction,
    /// ⧖ U+29D6 — Delay/hold (pause execution).
    Delay,
    /// ⧗ U+29D7 — Accelerate (increase execution priority).
    Accelerate,
    /// ⩫ U+2A6B — Parallel multi-goal (fork for each sub-goal).
    Parallel,
    /// ⩪ U+2A6A — Merge goals (join parallel branches).
    Merge,
    /// ⩨ U+2A68 — Split goals (divide into sub-goals).
    Split,
}

impl IntentOp {
    /// Map from Unicode character to IntentOp.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '⚡' => Some(IntentOp::Lightning),
            '⚠' => Some(IntentOp::Warning),
            '✦' => Some(IntentOp::Star),
            '✣' => Some(IntentOp::Synthesis),
            '✕' => Some(IntentOp::Cancel),
            '⟁' => Some(IntentOp::Contradiction),
            '⧖' => Some(IntentOp::Delay),
            '⧗' => Some(IntentOp::Accelerate),
            '⩫' => Some(IntentOp::Parallel),
            '⩪' => Some(IntentOp::Merge),
            '⩨' => Some(IntentOp::Split),
            _ => None,
        }
    }

    /// Map IntentOp to its Unicode character.
    pub fn to_char(self) -> char {
        match self {
            IntentOp::Lightning => '⚡',
            IntentOp::Warning => '⚠',
            IntentOp::Star => '✦',
            IntentOp::Synthesis => '✣',
            IntentOp::Cancel => '✕',
            IntentOp::Contradiction => '⟁',
            IntentOp::Delay => '⧖',
            IntentOp::Accelerate => '⧗',
            IntentOp::Parallel => '⩫',
            IntentOp::Merge => '⩪',
            IntentOp::Split => '⩨',
        }
    }

    /// Returns true if this intent operator sets an execution mode.
    pub fn is_execution_mode(&self) -> bool {
        matches!(
            self,
            IntentOp::Lightning | IntentOp::Warning | IntentOp::Star | IntentOp::Accelerate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all() {
        let ops = [
            IntentOp::Lightning,
            IntentOp::Warning,
            IntentOp::Star,
            IntentOp::Synthesis,
            IntentOp::Cancel,
            IntentOp::Contradiction,
            IntentOp::Delay,
            IntentOp::Accelerate,
            IntentOp::Parallel,
            IntentOp::Merge,
            IntentOp::Split,
        ];
        for op in ops {
            let c = op.to_char();
            let back = IntentOp::from_char(c);
            assert_eq!(back, Some(op), "roundtrip failed for {:?}", op);
        }
    }

    #[test]
    fn test_from_char_invalid() {
        assert_eq!(IntentOp::from_char('x'), None);
        assert_eq!(IntentOp::from_char('A'), None);
    }
}
