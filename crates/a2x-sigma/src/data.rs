// See plans/01-sigma-language.md §6

/// Data operators control the payload type and structure of an instruction.
///
/// Each data operator tells the CCS VM what kind of immediate data the
/// instruction carries and how to interpret it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DataOp {
    /// ⌬ U+232C — Raw tensor block.
    RawTensor,
    /// ⌭ U+232D — Latent vector block.
    LatentVector,
    /// ⌮ U+232E — Graph delta (WorldGraph changes).
    GraphDelta,
    /// ⌯ U+232F — Diff patch.
    DiffPatch,
    /// ⌰ U+2330 — Binary payload.
    Binary,
    /// ⌱ U+2331 — Multimodal fusion.
    Fusion,
    /// ⌲ U+2332 — Streaming block.
    Streaming,
    /// ⌳ U+2333 — Compressed summary.
    Summary,
    /// ⌴ U+2334 — Anomaly payload.
    Anomaly,
    /// ⌵ U+2335 — Structured schema.
    Schema,
    /// ⌶ U+2336 — Self-describing payload.
    SelfDescribing,
}

impl DataOp {
    /// Map from Unicode character to DataOp.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '⌬' => Some(DataOp::RawTensor),
            '⌭' => Some(DataOp::LatentVector),
            '⌮' => Some(DataOp::GraphDelta),
            '⌯' => Some(DataOp::DiffPatch),
            '⌰' => Some(DataOp::Binary),
            '⌱' => Some(DataOp::Fusion),
            '⌲' => Some(DataOp::Streaming),
            '⌳' => Some(DataOp::Summary),
            '⌴' => Some(DataOp::Anomaly),
            '⌵' => Some(DataOp::Schema),
            '⌶' => Some(DataOp::SelfDescribing),
            _ => None,
        }
    }

    /// Map DataOp to its Unicode character.
    pub fn to_char(self) -> char {
        match self {
            DataOp::RawTensor => '⌬',
            DataOp::LatentVector => '⌭',
            DataOp::GraphDelta => '⌮',
            DataOp::DiffPatch => '⌯',
            DataOp::Binary => '⌰',
            DataOp::Fusion => '⌱',
            DataOp::Streaming => '⌲',
            DataOp::Summary => '⌳',
            DataOp::Anomaly => '⌴',
            DataOp::Schema => '⌵',
            DataOp::SelfDescribing => '⌶',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all() {
        let ops = [
            DataOp::RawTensor,
            DataOp::LatentVector,
            DataOp::GraphDelta,
            DataOp::DiffPatch,
            DataOp::Binary,
            DataOp::Fusion,
            DataOp::Streaming,
            DataOp::Summary,
            DataOp::Anomaly,
            DataOp::Schema,
            DataOp::SelfDescribing,
        ];
        for op in ops {
            let c = op.to_char();
            let back = DataOp::from_char(c);
            assert_eq!(back, Some(op), "roundtrip failed for {:?}", op);
        }
    }
}
