// See plans/03-ccs-vm.md §8 and plans/07-probe.md (Probe interface)
//
// Phase 5: extended with BreakpointType, TracerMode, and ProbeEvent.
// The probe channel sits alongside the execution loop — checked between
// each instruction so probe queries never block program execution.

use std::fmt;

use a2x_core::node::NodeId;
use a2x_core::opcode::Opcode;
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
    /// Set the tracer mode.
    SetTracerMode(TracerMode),
    /// List all defined breakpoints.
    ListBreakpoints,
    /// Clear all breakpoints.
    ClearAllBreakpoints,
    /// List all StateField region names.
    ListRegions,
    /// Get WorldGraph summary (node count, edge count).
    GraphSummary,
}

/// A breakpoint type — what triggers a pause.
#[derive(Clone, Debug, PartialEq)]
pub enum BreakpointType {
    /// Stop at a specific instruction index.
    Instruction(usize),
    /// Stop when a specific opcode is about to execute.
    Opcode(Opcode),
    /// Stop after N instructions (watchdog).
    AfterSteps(u64),
}

/// Tracer verbosity level.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, Default)]
pub enum TracerMode {
    /// No tracing (fastest).
    Off,
    /// Log instruction IP + opcode only.
    Light,
    /// Log full instruction + state delta.
    Full,
    /// Log everything including MemoryTrace entries.
    #[default]
    Verbose,
}

impl fmt::Display for TracerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TracerMode::Off => write!(f, "off"),
            TracerMode::Light => write!(f, "light"),
            TracerMode::Full => write!(f, "full"),
            TracerMode::Verbose => write!(f, "verbose"),
        }
    }
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

/// Events emitted by the VM towards the probe channel.
///
/// Sent asynchronously whenever something interesting happens during
/// execution — breakpoint hits, step completions, program halts.
#[derive(Clone, Debug)]
pub enum ProbeEvent {
    /// A breakpoint was hit; VM is now paused.
    BreakpointHit {
        /// The breakpoint type that triggered.
        breakpoint: BreakpointType,
        /// The instruction pointer when the breakpoint fired.
        ip: usize,
        /// The opcode about to execute.
        opcode: Opcode,
    },
    /// A single step completed; VM is paused again.
    Stepped { ip: usize, opcode: Opcode },
    /// The program halted.
    Halted { ip: usize, steps_executed: usize },
    /// The program faulted.
    Faulted { ip: usize, error: String },
}

impl fmt::Display for ProbeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeEvent::BreakpointHit {
                breakpoint,
                ip,
                opcode,
            } => {
                write!(
                    f,
                    "breakpoint {:?} hit at ip={} opcode={:?}",
                    breakpoint, ip, opcode
                )
            }
            ProbeEvent::Stepped { ip, opcode } => {
                write!(f, "stepped to ip={} opcode={:?}", ip, opcode)
            }
            ProbeEvent::Halted { ip, steps_executed } => {
                write!(f, "halted at ip={} after {} steps", ip, steps_executed)
            }
            ProbeEvent::Faulted { ip, error } => {
                write!(f, "faulted at ip={}: {}", ip, error)
            }
        }
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
