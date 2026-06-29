// See plans/03-ccs-vm.md §8 and plans/07-probe.md (Probe interface)
//
// Phase 5: extended with BreakpointType, TracerMode, and ProbeEvent.
// The probe channel sits alongside the execution loop — checked between
// each instruction so probe queries never block program execution.
//
// Phase 5 gap-fill: added AccessType, Condition, advanced breakpoint types
// (NodeAccess, RegionAccess, Conditional), and TracerMode execution wiring.

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

/// What kind of access triggers a breakpoint.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccessType {
    /// Break on read access.
    Read,
    /// Break on write access.
    Write,
    /// Break on either read or write.
    Both,
}

impl fmt::Display for AccessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessType::Read => write!(f, "read"),
            AccessType::Write => write!(f, "write"),
            AccessType::Both => write!(f, "both"),
        }
    }
}

/// A condition that can be evaluated against VM state for conditional breakpoints.
///
/// Note: `Custom` is represented as a string DSL for now. A future Phase could
/// use WASM plugins or closures behind a `Box<dyn Fn>` for richer predicates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Condition {
    /// IP == N (equivalent to instruction breakpoint).
    AtInstruction(usize),
    /// Program has executed for > N instructions.
    AfterSteps(u64),
    /// Custom predicate identified by a string key (DSL or named rule).
    Custom(String),
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::AtInstruction(n) => write!(f, "ip=={}", n),
            Condition::AfterSteps(n) => write!(f, "steps>{}", n),
            Condition::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

/// A breakpoint type — what triggers a pause.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BreakpointType {
    /// Stop at a specific instruction index.
    Instruction(usize),
    /// Stop when a specific opcode is about to execute.
    Opcode(Opcode),
    /// Stop after N instructions (watchdog).
    AfterSteps(u64),
    /// Stop when a WorldGraph node matching a label is accessed.
    NodeAccess {
        /// The label to watch.
        label: String,
        /// What kind of access triggers the breakpoint.
        access_type: AccessType,
    },
    /// Stop when a StateField region is read/written.
    RegionAccess {
        /// The region name to watch.
        region: String,
        /// What kind of access triggers the breakpoint.
        access_type: AccessType,
    },
    /// Stop when a condition evaluates to true.
    Conditional {
        /// The condition to evaluate.
        condition: Condition,
    },
}

impl BreakpointType {
    /// Return the instruction index for instruction-type breakpoints, or None.
    pub fn instruction_ip(&self) -> Option<usize> {
        match self {
            BreakpointType::Instruction(ip) => Some(*ip),
            _ => None,
        }
    }

    /// Check if this breakpoint matches the given opcode.
    pub fn matches_opcode(&self, opcode: Opcode) -> bool {
        match self {
            BreakpointType::Opcode(op) => *op == opcode,
            _ => false,
        }
    }

    /// Check if this breakpoint matches the given label access.
    pub fn matches_node_access(&self, label: &str, _access: AccessType) -> bool {
        match self {
            BreakpointType::NodeAccess {
                label: bp_label, ..
            } => bp_label == label,
            _ => false,
        }
    }

    /// Check if this breakpoint matches the given region access.
    pub fn matches_region_access(&self, region: &str, _access: AccessType) -> bool {
        match self {
            BreakpointType::RegionAccess {
                region: bp_region, ..
            } => bp_region == region,
            _ => false,
        }
    }

    /// Evaluate a conditional breakpoint against the given step count.
    pub fn eval_condition(&self, ip: usize, steps: u64) -> bool {
        match self {
            BreakpointType::Conditional { condition } => match condition {
                Condition::AtInstruction(target) => ip == *target,
                Condition::AfterSteps(n) => steps > *n,
                Condition::Custom(_) => false, // Can't evaluate without runtime context
            },
            _ => false,
        }
    }
}

impl fmt::Display for BreakpointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakpointType::Instruction(ip) => write!(f, "instruction@{}", ip),
            BreakpointType::Opcode(op) => write!(f, "opcode={:?}", op),
            BreakpointType::AfterSteps(n) => write!(f, "after_steps({})", n),
            BreakpointType::NodeAccess { label, access_type } => {
                write!(f, "node_{}({})", access_type, label)
            }
            BreakpointType::RegionAccess {
                region,
                access_type,
            } => write!(f, "region_{}({})", access_type, region),
            BreakpointType::Conditional { condition } => write!(f, "cond({})", condition),
        }
    }
}

/// Tracer verbosity level.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, Default)]
pub enum TracerMode {
    /// No tracing (fastest).
    Off,
    /// Log instruction IP + opcode only (~50ns/inst).
    Light,
    /// Log full instruction + state delta (~200ns/inst).
    Full,
    /// Log everything including MemoryTrace entries (~500ns/inst).
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

/// A single tracer log entry emitted between instructions.
///
/// The `Tracer` collects these and provides formatted output for the
/// instruction tracer display (§9 of plans/07-probe.md).
#[derive(Clone, Debug)]
pub struct TraceLogEntry {
    /// Instruction pointer when this entry was recorded.
    pub ip: usize,
    /// The opcode that was executed.
    pub opcode: Opcode,
    /// Steps executed so far (monotonic).
    pub steps: usize,
    /// State snapshot summary (first 4 bytes as f32, for heatmap preview).
    pub state_summary: Vec<f32>,
    /// Optional memory trace entry count at this point.
    pub trace_len: Option<usize>,
}

impl fmt::Display for TraceLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:4}] {:?} (step {:4})",
            self.ip, self.opcode, self.steps
        )
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
    /// List of breakpoints.
    BreakpointList(Vec<(usize, String)>),
    /// List of StateField regions.
    RegionList(Vec<(String, usize, usize)>),
    /// WorldGraph summary.
    GraphSummary {
        node_count: usize,
        edge_count: usize,
    },
    /// Instruction trace log entries.
    TraceLog(Vec<TraceLogEntry>),
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
                    "breakpoint {} hit at ip={} opcode={:?}",
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

    #[test]
    fn test_breakpoint_type_display() {
        assert_eq!(BreakpointType::Instruction(5).to_string(), "instruction@5");
        assert_eq!(
            BreakpointType::Opcode(Opcode::Bind).to_string(),
            "opcode=Bind"
        );
        assert_eq!(
            BreakpointType::AfterSteps(100).to_string(),
            "after_steps(100)"
        );
        assert_eq!(
            BreakpointType::NodeAccess {
                label: "sys".to_string(),
                access_type: AccessType::Read,
            }
            .to_string(),
            "node_read(sys)"
        );
        assert_eq!(
            BreakpointType::RegionAccess {
                region: "goal".to_string(),
                access_type: AccessType::Write,
            }
            .to_string(),
            "region_write(goal)"
        );
    }

    #[test]
    fn test_breakpoint_type_matches_opcode() {
        let bp = BreakpointType::Opcode(Opcode::Bind);
        assert!(bp.matches_opcode(Opcode::Bind));
        assert!(!bp.matches_opcode(Opcode::Ground));
    }

    #[test]
    fn test_breakpoint_type_matches_node_access() {
        let bp = BreakpointType::NodeAccess {
            label: "sys".to_string(),
            access_type: AccessType::Read,
        };
        assert!(bp.matches_node_access("sys", AccessType::Read));
        assert!(!bp.matches_node_access("other", AccessType::Read));
    }

    #[test]
    fn test_breakpoint_type_matches_region_access() {
        let bp = BreakpointType::RegionAccess {
            region: "goal".to_string(),
            access_type: AccessType::Both,
        };
        assert!(bp.matches_region_access("goal", AccessType::Write));
        assert!(!bp.matches_region_access("belief", AccessType::Read));
    }

    #[test]
    fn test_breakpoint_type_eval_condition() {
        let bp = BreakpointType::Conditional {
            condition: Condition::AtInstruction(5),
        };
        assert!(bp.eval_condition(5, 10));
        assert!(!bp.eval_condition(4, 10));

        let bp = BreakpointType::Conditional {
            condition: Condition::AfterSteps(100),
        };
        assert!(bp.eval_condition(0, 101));
        assert!(!bp.eval_condition(0, 99));
    }

    #[test]
    fn test_access_type_display() {
        assert_eq!(AccessType::Read.to_string(), "read");
        assert_eq!(AccessType::Write.to_string(), "write");
        assert_eq!(AccessType::Both.to_string(), "both");
    }

    #[test]
    fn test_condition_display() {
        assert_eq!(Condition::AtInstruction(3).to_string(), "ip==3");
        assert_eq!(Condition::AfterSteps(50).to_string(), "steps>50");
        assert_eq!(
            Condition::Custom("my_rule".to_string()).to_string(),
            "custom(my_rule)"
        );
    }

    #[test]
    fn test_tracer_mode_display() {
        assert_eq!(TracerMode::Off.to_string(), "off");
        assert_eq!(TracerMode::Light.to_string(), "light");
        assert_eq!(TracerMode::Full.to_string(), "full");
        assert_eq!(TracerMode::Verbose.to_string(), "verbose");
    }

    #[test]
    fn test_trace_log_entry_display() {
        let entry = TraceLogEntry {
            ip: 42,
            opcode: Opcode::Bind,
            steps: 100,
            state_summary: vec![0.5],
            trace_len: Some(50),
        };
        let text = entry.to_string();
        assert!(text.contains("42"));
        assert!(text.contains("Bind"));
    }

    #[test]
    fn test_breakpoint_instruction_ip() {
        assert_eq!(BreakpointType::Instruction(10).instruction_ip(), Some(10));
        assert_eq!(BreakpointType::Opcode(Opcode::Bind).instruction_ip(), None);
    }

    #[test]
    fn test_probe_event_display() {
        let event = ProbeEvent::BreakpointHit {
            breakpoint: BreakpointType::Instruction(5),
            ip: 5,
            opcode: Opcode::Bind,
        };
        let text = event.to_string();
        assert!(text.contains("breakpoint"));
        assert!(text.contains("ip=5"));

        let event = ProbeEvent::Halted {
            ip: 100,
            steps_executed: 500,
        };
        let text = event.to_string();
        assert!(text.contains("halted"));
        assert!(text.contains("500"));
    }
}
