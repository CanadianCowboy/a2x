// See plans/01-sigma-language.md §8

use crate::context::ContextOp;
use crate::data::DataOp;
use crate::intent::IntentOp;
use crate::plan::PlanOp;

/// Boundary marker kind for Σ∞ packets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    /// ⟦ or ⟬ — opens an instruction or context section.
    Open,
    /// ⟧ or ⟭ — closes an instruction or context section.
    Close,
}

/// A single token produced by the lexer from Σ∞ source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// ⟦ or ⟬ (open) or ⟧ or ⟭ (close).
    Boundary(BoundaryKind),
    /// ∷ — separates I, C, P, D fields.
    FieldSeparator,
    /// An intent operator (I field).
    IntentOp(IntentOp),
    /// A context operator (C field).
    ContextOp(ContextOp),
    /// A plan operator (P field).
    PlanOp(PlanOp),
    /// A data operator (D field).
    DataOp(DataOp),
    /// A label — ⟨text⟩ extracts the inner text.
    Label(String),
    /// Σ∞ or Ω protocol identifier.
    ProtocolId,
}
