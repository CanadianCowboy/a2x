// See plans/01-sigma-language.md §5

/// Plan operators control how execution proceeds — sequencing, branching, parallelism.
///
/// Each plan operator tells the CCS VM how to update the instruction pointer
/// and manage the call stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlanOp {
    /// ⤈ U+2908 — Descend into sub-plan (push IP+1, jump to sub-program).
    Descend,
    /// ⤉ U+2909 — Ascend to meta-plan (pop return address, jump there).
    Ascend,
    /// ⤊ U+290A — Escalate (propagate to parent/caller).
    Escalate,
    /// ⤋ U+290B — De-escalate (reduce severity, handle locally).
    DeEscalate,
    /// ⤐ U+2910 — Branch (conditional jump to target).
    Branch,
    /// ⤑ U+2911 — Merge (pop from call stack, resume parent).
    Merge,
    /// ⤒ U+2912 — Enforce (apply constraints, strict mode).
    Enforce,
    /// ⤓ U+2913 — Relax (loosen constraints, permissive mode).
    Relax,
    /// ⥁ U+2941 — Parallel swarm (fork N VM instances).
    Swarm,
    /// ⥂ U+2942 — Sequential chain (IP += 1, normal flow).
    Sequential,
    /// ⥃ U+2943 — Recursive (push IP, jump to program start).
    Recursive,
    /// ⥄ U+2944 — Self-modifying (modify instruction stream, continue).
    SelfModifying,
}

impl PlanOp {
    /// Map from Unicode character to PlanOp.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '⤈' => Some(PlanOp::Descend),
            '⤉' => Some(PlanOp::Ascend),
            '⤊' => Some(PlanOp::Escalate),
            '⤋' => Some(PlanOp::DeEscalate),
            '⤐' => Some(PlanOp::Branch),
            '⤑' => Some(PlanOp::Merge),
            '⤒' => Some(PlanOp::Enforce),
            '⤓' => Some(PlanOp::Relax),
            '⥁' => Some(PlanOp::Swarm),
            '⥂' => Some(PlanOp::Sequential),
            '⥃' => Some(PlanOp::Recursive),
            '⥄' => Some(PlanOp::SelfModifying),
            _ => None,
        }
    }

    /// Map PlanOp to its Unicode character.
    pub fn to_char(self) -> char {
        match self {
            PlanOp::Descend => '⤈',
            PlanOp::Ascend => '⤉',
            PlanOp::Escalate => '⤊',
            PlanOp::DeEscalate => '⤋',
            PlanOp::Branch => '⤐',
            PlanOp::Merge => '⤑',
            PlanOp::Enforce => '⤒',
            PlanOp::Relax => '⤓',
            PlanOp::Swarm => '⥁',
            PlanOp::Sequential => '⥂',
            PlanOp::Recursive => '⥃',
            PlanOp::SelfModifying => '⥄',
        }
    }

    /// Returns true if this plan op changes control flow (jumps, calls, returns).
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            PlanOp::Branch
                | PlanOp::Merge
                | PlanOp::Descend
                | PlanOp::Ascend
                | PlanOp::Swarm
                | PlanOp::Recursive
                | PlanOp::Escalate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all() {
        let ops = [
            PlanOp::Descend,
            PlanOp::Ascend,
            PlanOp::Escalate,
            PlanOp::DeEscalate,
            PlanOp::Branch,
            PlanOp::Merge,
            PlanOp::Enforce,
            PlanOp::Relax,
            PlanOp::Swarm,
            PlanOp::Sequential,
            PlanOp::Recursive,
            PlanOp::SelfModifying,
        ];
        for op in ops {
            let c = op.to_char();
            let back = PlanOp::from_char(c);
            assert_eq!(back, Some(op), "roundtrip failed for {:?}", op);
        }
    }
}
