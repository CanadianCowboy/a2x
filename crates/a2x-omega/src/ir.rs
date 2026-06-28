// See plans/02-omega-compiler.md §3 (Stages 4-5)

use a2x_core::Opcode;

/// A node in the IR graph — represents a single VM operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrNode {
    /// Unique node identifier.
    pub id: IrNodeId,
    /// The CCS VM opcode this node represents.
    pub opcode: Opcode,
    /// Operand references (WorldGraph node refs, StateField regions).
    pub operands: Vec<IrOperand>,
    /// Control flow targets (next nodes, branch targets).
    pub control_flow: Vec<IrNodeId>,
    /// Source-level metadata for debugging.
    pub metadata: IrMetadata,
}

/// Unique identifier for an IR node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IrNodeId(pub u32);

/// An operand in the IR — either a memory reference or an immediate value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrOperand {
    /// Reference to a WorldGraph node by label.
    Label(String),
    /// Reference to a WorldGraph node by numeric ID.
    NodeId(u64),
    /// Reference to a StateField region by name.
    Region(String),
    /// An immediate literal value.
    Immediate(Vec<u8>),
}

/// Debug metadata attached to each IR node.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct IrMetadata {
    /// Source instruction index (for mapping back to Σ∞).
    pub source_index: Option<usize>,
    /// Source line/position info.
    pub source_position: Option<usize>,
}

/// The IR graph — a dataflow representation of a program.
///
/// Nodes are VM operations; edges are data and control dependencies.
#[derive(Clone, Debug, Default)]
pub struct IrGraph {
    pub nodes: Vec<IrNode>,
    pub entry: Option<IrNodeId>,
    pub exit: Option<IrNodeId>,
}

impl IrGraph {
    pub fn new() -> Self {
        IrGraph::default()
    }

    pub fn add_node(&mut self, node: IrNode) -> IrNodeId {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
