// See plans/01-sigma-language.md §4

/// Context operators control world-state, uncertainty, scope, and memory references.
///
/// Each context operator tells the CCS VM what aspect of the world-model to operate on.
/// Context operators are combined with labels (angle brackets) to form memory references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ContextOp {
    /// ⟘ U+27D8 — Empty set / null context.
    Null,
    /// ⟙ U+27D9 — Full set / universal context.
    Universal,
    /// ⟚ U+27DA — Compression / compressed world-state.
    Compression,
    /// ⟞ U+27DE — Wavy line / uncertainty field.
    Uncertainty,
    /// ⟡ U+27E1 — Bowtie / causal chain.
    CausalChain,
    /// ⟠ U+27E0 — Diamond / spatial chain.
    SpatialChain,
    /// ⟢ U+27E2 — Left arrow / temporal chain.
    TemporalChain,
    /// ⟣ U+27E3 — Right arrow / probabilistic context.
    Probabilistic,
    /// ⟤ U+27E4 — Double bar / conflict context.
    Conflict,
    /// ⟧ U+27E7 — Corner / resolved context.
    Resolved,
}

impl ContextOp {
    /// Map from Unicode character to ContextOp.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '⟘' => Some(ContextOp::Null),
            '⟙' => Some(ContextOp::Universal),
            '⟚' => Some(ContextOp::Compression),
            '⟞' => Some(ContextOp::Uncertainty),
            '⟡' => Some(ContextOp::CausalChain),
            '⟠' => Some(ContextOp::SpatialChain),
            '⟢' => Some(ContextOp::TemporalChain),
            '⟣' => Some(ContextOp::Probabilistic),
            '⟤' => Some(ContextOp::Conflict),
            '⟧' => Some(ContextOp::Resolved),
            _ => None,
        }
    }

    /// Map ContextOp to its Unicode character.
    pub fn to_char(self) -> char {
        match self {
            ContextOp::Null => '⟘',
            ContextOp::Universal => '⟙',
            ContextOp::Compression => '⟚',
            ContextOp::Uncertainty => '⟞',
            ContextOp::CausalChain => '⟡',
            ContextOp::SpatialChain => '⟠',
            ContextOp::TemporalChain => '⟢',
            ContextOp::Probabilistic => '⟣',
            ContextOp::Conflict => '⟤',
            ContextOp::Resolved => '⟧',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all() {
        let ops = [
            ContextOp::Null,
            ContextOp::Universal,
            ContextOp::Compression,
            ContextOp::Uncertainty,
            ContextOp::CausalChain,
            ContextOp::SpatialChain,
            ContextOp::TemporalChain,
            ContextOp::Probabilistic,
            ContextOp::Conflict,
            ContextOp::Resolved,
        ];
        for op in ops {
            let c = op.to_char();
            let back = ContextOp::from_char(c);
            assert_eq!(back, Some(op), "roundtrip failed for {:?}", op);
        }
    }
}
