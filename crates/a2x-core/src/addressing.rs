// See plans/09-core-types.md §2

use crate::node::NodeId;

/// Addressing mode for instruction operands in the CCS VM.
///
/// Instructions reference memory through these modes, encoded in the C (context)
/// field of a Σ∞ packet.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AddressingMode {
    /// Look up a node by its position in the program's label table.
    LabelIndex(u32),
    /// Directly reference a node by its numeric NodeId.
    DirectNodeId(NodeId),
    /// Reference a StateField region by its numeric index.
    StateFieldRegion(u32),
    /// Literal immediate value embedded in the instruction.
    Immediate(f32),
}
