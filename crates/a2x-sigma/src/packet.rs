// See plans/01-sigma-language.md §9

use crate::context::ContextOp;
use crate::data::DataOp;
use crate::intent::IntentOp;
use crate::plan::PlanOp;
use a2x_core::ProtocolId;

/// The intent field of a Σ∞ packet (I: field).
///
/// Contains one or more intent operators that set the instruction's
/// goal type, urgency, and execution mode.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntentField {
    pub operators: Vec<IntentOp>,
}

impl IntentField {
    pub fn new() -> Self {
        IntentField {
            operators: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty()
    }
}

impl Default for IntentField {
    fn default() -> Self {
        Self::new()
    }
}

/// The context field of a Σ∞ packet (C: field).
///
/// Contains context operators and optional label references that tell
/// the CCS VM what part of the WorldGraph to operate on.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContextField {
    pub operators: Vec<ContextOp>,
    pub labels: Vec<String>,
}

impl ContextField {
    pub fn new() -> Self {
        ContextField {
            operators: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty() && self.labels.is_empty()
    }
}

impl Default for ContextField {
    fn default() -> Self {
        Self::new()
    }
}

/// The plan field of a Σ∞ packet (P: field).
///
/// Contains plan operators that control how execution proceeds —
/// sequencing, branching, parallelism, sub-programs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanField {
    pub operators: Vec<PlanOp>,
}

impl PlanField {
    pub fn new() -> Self {
        PlanField {
            operators: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty()
    }
}

impl Default for PlanField {
    fn default() -> Self {
        Self::new()
    }
}

/// The data field of a Σ∞ packet (D: field).
///
/// Contains data operators that specify the payload type and structure
/// of the instruction's immediate data.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataField {
    pub operators: Vec<DataOp>,
    /// Raw payload bytes (for binary/structured data operators).
    pub payload: Vec<u8>,
}

impl DataField {
    pub fn new() -> Self {
        DataField {
            operators: Vec::new(),
            payload: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty() && self.payload.is_empty()
    }
}

impl Default for DataField {
    fn default() -> Self {
        Self::new()
    }
}

/// A single Σ∞ instruction — the fundamental unit of the A2X ISA.
///
/// Each packet corresponds to one instruction that the CCS VM executes.
/// A sequence of packets forms a Σ∞ program.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SigmaPacket {
    /// Protocol identifier (Σ∞).
    pub protocol: ProtocolId,
    /// I: field — intent operators (execution mode, goal type).
    pub intent: IntentField,
    /// C: field — context operators + label references (memory operands).
    pub context: ContextField,
    /// P: field — plan operators (control flow).
    pub plan: PlanField,
    /// D: field — data operators + raw payload (immediate data).
    pub data: DataField,
}

impl SigmaPacket {
    /// Create a new empty Σ∞ packet.
    pub fn new() -> Self {
        SigmaPacket {
            protocol: ProtocolId::Sigma,
            intent: IntentField::new(),
            context: ContextField::new(),
            plan: PlanField::new(),
            data: DataField::new(),
        }
    }

    /// Returns true if all fields are empty (no operators or labels).
    pub fn is_empty(&self) -> bool {
        self.intent.is_empty()
            && self.context.is_empty()
            && self.plan.is_empty()
            && self.data.is_empty()
    }
}

impl Default for SigmaPacket {
    fn default() -> Self {
        Self::new()
    }
}
