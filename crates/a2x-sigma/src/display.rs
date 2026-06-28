// See plans/01-sigma-language.md §9

use crate::packet::SigmaPacket;
use std::fmt;

impl fmt::Display for SigmaPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "⟦Σ∞⟧⟬")?;

        let mut has_prev = false;

        // I: field
        if !self.intent.is_empty() {
            if has_prev {
                write!(f, " ∷ ")?;
            }
            write!(f, "I:")?;
            for op in &self.intent.operators {
                write!(f, "{}", op.to_char())?;
            }
            has_prev = true;
        }

        // C: field
        if !self.context.is_empty() {
            if has_prev {
                write!(f, " ∷ ")?;
            }
            write!(f, "C:")?;
            for op in &self.context.operators {
                write!(f, "{}", op.to_char())?;
            }
            for label in &self.context.labels {
                write!(f, "⟨{}⟩", label)?;
            }
            has_prev = true;
        }

        // P: field
        if !self.plan.is_empty() {
            if has_prev {
                write!(f, " ∷ ")?;
            }
            write!(f, "P:")?;
            for op in &self.plan.operators {
                write!(f, "{}", op.to_char())?;
            }
            has_prev = true;
        }

        // D: field
        if !self.data.is_empty() {
            if has_prev {
                write!(f, " ∷ ")?;
            }
            write!(f, "D:")?;
            for op in &self.data.operators {
                write!(f, "{}", op.to_char())?;
            }
            if !self.data.payload.is_empty() {
                write!(f, "⟨...⟩")?;
            }
        }

        write!(f, "⟭")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextOp;
    use crate::data::DataOp;
    use crate::intent::IntentOp;
    use crate::packet::{ContextField, DataField, IntentField, PlanField};
    use crate::plan::PlanOp;

    #[test]
    fn test_display_roundtrip_simple() {
        let mut pkt = SigmaPacket::new();
        pkt.intent = IntentField {
            operators: vec![IntentOp::Lightning, IntentOp::Synthesis, IntentOp::Parallel],
        };
        pkt.context = ContextField {
            operators: vec![ContextOp::Compression, ContextOp::Uncertainty],
            labels: vec!["sys".into()],
        };
        pkt.plan = PlanField {
            operators: vec![PlanOp::Swarm, PlanOp::Enforce, PlanOp::Descend],
        };
        pkt.data = DataField {
            operators: vec![DataOp::GraphDelta, DataOp::Summary, DataOp::Fusion],
            payload: vec![],
        };

        let text = pkt.to_string();
        // Expected: ⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭
        assert!(text.starts_with("⟦Σ∞⟧⟬"));
        assert!(text.contains("⚡✣⩫"));
        assert!(text.contains("⟨sys⟩"));
        assert!(text.contains("⥁⤒⤈"));
        assert!(text.contains("⌮⌳⌱"));
        assert!(text.ends_with("⟭"));
    }

    #[test]
    fn test_display_empty_packet() {
        let pkt = SigmaPacket::new();
        let text = pkt.to_string();
        assert_eq!(text, "⟦Σ∞⟧⟬⟭");
    }
}
