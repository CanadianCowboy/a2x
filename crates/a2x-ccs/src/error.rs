// See plans/03-ccs-vm.md and plans/14-resilience.md

use std::fmt;

/// VM-level runtime error.
#[derive(Clone, Debug, PartialEq)]
pub enum VmError {
    /// No program loaded into the VM.
    NoProgram,
    /// Instruction pointer out of bounds.
    InvalidInstructionPointer { ip: usize, length: usize },
    /// Referenced a label that doesn't exist in the program.
    UndefinedLabel(String),
    /// Referenced a sub-program that doesn't exist.
    UndefinedSubProgram(String),
    /// WorldGraph allocation limit exceeded (out of memory).
    OutOfMemory,
    /// Safety constraint violated.
    SafetyViolation(String),
    /// Operand references a non-existent WorldGraph node.
    InvalidNode(u64),
    /// Operand references a non-existent StateField region.
    UndefinedRegion(String),
    /// Operand references a label that does not exist in the WorldGraph.
    /// Returned by `CcsVm` when a C-field label cannot be resolved.
    UnresolvedOperand(String),
    /// Parallel fork results conflict and can't be merged.
    ParallelMergeConflict,
    /// Program exceeded its maximum instruction count.
    MaxStepsExceeded { max: usize, actual: usize },
    /// Stack overflow (too many nested sub-program calls).
    StackOverflow { max_depth: usize },
    /// Stack underflow (return with empty call stack).
    StackUnderflow,
    /// Division by zero or other math error.
    ArithmeticError(String),
    /// Catch-all for other VM errors.
    Other(String),
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::NoProgram => write!(f, "no program loaded"),
            VmError::InvalidInstructionPointer { ip, length } => {
                write!(f, "IP {} out of bounds (program length {})", ip, length)
            }
            VmError::UndefinedLabel(l) => write!(f, "undefined label: {}", l),
            VmError::UndefinedSubProgram(n) => write!(f, "undefined sub-program: {}", n),
            VmError::OutOfMemory => write!(f, "out of memory: graph allocation limit exceeded"),
            VmError::SafetyViolation(msg) => write!(f, "safety violation: {}", msg),
            VmError::InvalidNode(id) => write!(f, "invalid node: {}", id),
            VmError::UndefinedRegion(r) => write!(f, "undefined StateField region: {}", r),
            VmError::UnresolvedOperand(s) => write!(f, "unresolved operand: {}", s),
            VmError::ParallelMergeConflict => write!(f, "parallel merge conflict"),
            VmError::MaxStepsExceeded { max, actual } => {
                write!(f, "max steps {} exceeded (executed {})", max, actual)
            }
            VmError::StackOverflow { max_depth } => {
                write!(f, "stack overflow: max depth {} exceeded", max_depth)
            }
            VmError::StackUnderflow => write!(f, "stack underflow: return with empty call stack"),
            VmError::ArithmeticError(msg) => write!(f, "arithmetic error: {}", msg),
            VmError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for VmError {}

/// WorldGraph operation error.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldGraphError {
    /// Referenced a NodeId that doesn't exist in the graph.
    NodeNotFound(u64),
    /// Attempted to allocate a node but reached the capacity limit.
    AtCapacity { max_nodes: usize },
    /// Label already exists in the index.
    LabelConflict(String),
    /// Edge already exists between source and target.
    EdgeAlreadyExists { source: u64, target: u64 },
    /// Edge does not exist.
    EdgeNotFound { source: u64, target: u64 },
    /// Self-loops are not allowed.
    SelfLoop(u64),
}

impl fmt::Display for WorldGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldGraphError::NodeNotFound(id) => write!(f, "node {} not found in graph", id),
            WorldGraphError::AtCapacity { max_nodes } => {
                write!(f, "graph at capacity (max {} nodes)", max_nodes)
            }
            WorldGraphError::LabelConflict(label) => write!(f, "label conflict: {}", label),
            WorldGraphError::EdgeAlreadyExists { source, target } => {
                write!(f, "edge {} -> {} already exists", source, target)
            }
            WorldGraphError::EdgeNotFound { source, target } => {
                write!(f, "edge {} -> {} not found", source, target)
            }
            WorldGraphError::SelfLoop(id) => write!(f, "self-loop on node {} not allowed", id),
        }
    }
}

impl std::error::Error for WorldGraphError {}

/// StateField operation error.
#[derive(Clone, Debug, PartialEq)]
pub enum StateError {
    /// Region name already exists.
    RegionAlreadyDefined(String),
    /// Referenced a region that doesn't exist.
    RegionNotFound(String),
    /// Data size doesn't match region size.
    SizeMismatch { expected: usize, actual: usize },
    /// Offset + length exceeds StateField total size.
    RegionOutOfBounds {
        offset: usize,
        len: usize,
        total: usize,
    },
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::RegionAlreadyDefined(n) => write!(f, "region already defined: {}", n),
            StateError::RegionNotFound(n) => write!(f, "region not found: {}", n),
            StateError::SizeMismatch { expected, actual } => {
                write!(f, "size mismatch: expected {}, got {}", expected, actual)
            }
            StateError::RegionOutOfBounds { offset, len, total } => {
                write!(
                    f,
                    "region out of bounds: offset={} len={} total={}",
                    offset, len, total
                )
            }
        }
    }
}

impl std::error::Error for StateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_error_display() {
        let err = VmError::InvalidInstructionPointer { ip: 5, length: 3 };
        assert!(format!("{}", err).contains("out of bounds"));
    }

    #[test]
    fn test_world_graph_error_display() {
        let err = WorldGraphError::NodeNotFound(42);
        assert!(format!("{}", err).contains("42"));
    }

    #[test]
    fn test_state_error_display() {
        let err = StateError::RegionNotFound("goal".into());
        assert!(format!("{}", err).contains("goal"));
    }

    #[test]
    fn test_unresolved_operand_display() {
        let err = VmError::UnresolvedOperand("\u{27e8}missing\u{27e9}".into());
        let s = format!("{}", err);
        assert!(s.contains("unresolved operand"));
        assert!(s.contains("missing"));
    }
}
