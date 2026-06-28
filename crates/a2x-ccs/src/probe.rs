// See plans/03-ccs-vm.md §8 and plans/07-probe.md (Probe interface)

use std::fmt;

use a2x_core::node::NodeId;
use a2x_core::program_id::ProgramId;

/// Requests that can be sent to a running CCS VM for inspection.
#[derive(Clone, Debug, PartialEq)]
pub enum ProbeQuery {
    /// Snapshot the entire VM state.
    Snapshot,
    /// Get the current instruction pointer.
    GetIp,
    /// Dump a WorldGraph node by ID.
    GetNode(NodeId),
    /// Dump a WorldGraph node by label.
    GetNodeByLabel(String),
    /// Get a StateField region by name.
    GetRegion(String),
    /// Get the program counter (MemoryTrace position).
    GetPc,
    /// Get the last N MemoryTrace entries.
    GetTraceTail(usize),
    /// Set a breakpoint at instruction index.
    SetBreakpoint(usize),
    /// Clear a breakpoint.
    ClearBreakpoint(usize),
    /// Step one instruction (when paused at breakpoint).
    Step,
    /// Continue execution (when paused at breakpoint).
    Continue,
}

/// Response from a CCS VM to a probe query.
#[derive(Clone, Debug)]
pub enum ProbeSnapshot {
    /// Top-level VM state.
    VmState {
        program_id: Option<ProgramId>,
        ip: usize,
        steps_executed: usize,
        status: String,
    },
    /// A WorldGraph node.
    Node {
        id: NodeId,
        concept: Vec<f32>,
        label: Option<String>,
        edge_count: usize,
    },
    /// A StateField region.
    Region {
        name: String,
        offset: usize,
        len: usize,
        data: Vec<f32>,
    },
    /// A list of node IDs from a query.
    QueryResult(Vec<NodeId>),
    /// MemoryTrace tail entries.
    TraceSegment { entries: Vec<ProbeTraceEntry> },
    /// Breakpoint set confirmation.
    BreakpointSet(usize),
    /// Breakpoint cleared confirmation.
    BreakpointCleared(usize),
    /// VM stepped one instruction.
    Stepped,
    /// VM continued after breakpoint.
    Continued,
}

/// Lightweight trace entry for probe responses.
#[derive(Clone, Debug)]
pub struct ProbeTraceEntry {
    /// Instruction pointer at time of execution.
    pub ip: usize,
    /// When the instruction ran.
    pub timestamp: Option<String>,
    /// First few bytes of the state snapshot.
    pub state_preview: Vec<f32>,
}

impl fmt::Display for ProbeTraceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ip={} ts={:?} state=[{:.3}..]",
            self.ip,
            self.timestamp,
            self.state_preview.first().unwrap_or(&0.0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_query_variants() {
        let q = ProbeQuery::GetIp;
        assert_eq!(q, ProbeQuery::GetIp);

        let q = ProbeQuery::SetBreakpoint(42);
        assert_eq!(q, ProbeQuery::SetBreakpoint(42));
    }

    #[test]
    fn test_probe_snapshot_variants() {
        let snap = ProbeSnapshot::BreakpointSet(7);
        assert!(matches!(snap, ProbeSnapshot::BreakpointSet(7)));
    }
}
