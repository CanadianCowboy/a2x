// a2x-probe — Debug/probe tools for inspecting CCS VM internals
// See plans/07-probe.md
//
// Phase 5: implements the external-facing ProbeTool that connects to a
// CcsVm's probe channel, sends queries, and collects responses.
//
// Architecture:
//   ProbeTool ──mpsc::Sender──▶ CcsVm (probe_rx) ──mpsc::Sender──▶ ProbeTool (event_rx)
//                                                                      │
//                                                                      ▼
//                                                               Visualization
//                                                              (graphviz, heatmap)

use a2x_ccs::probe::{ProbeEvent, ProbeQuery, ProbeSnapshot, TracerMode};

// ─── ProbeTool ──────────────────────────────────────────────────────

/// A probe tool that connects to a CCS VM's probe channel.
///
/// `ProbeTool` sends `ProbeQuery` messages and receives `ProbeEvent`
/// notifications through the mpsc channels established by `CcsVm::attach_probe()`.
///
/// # Example (conceptual)
/// ```ignore
/// let mut vm = CcsVm::new();
/// let mut tool = ProbeTool::from_vm(&mut vm);
/// tool.set_breakpoint(5);
/// let snap = tool.snapshot();
/// ```
pub struct ProbeTool {
    /// Sender for probe queries to the VM.
    query_tx: std::sync::mpsc::Sender<ProbeQuery>,
    /// Receiver for probe events from the VM.
    event_rx: std::sync::mpsc::Receiver<ProbeEvent>,
}

impl ProbeTool {
    /// Create a probe tool from an existing channel pair.
    pub fn new(
        query_tx: std::sync::mpsc::Sender<ProbeQuery>,
        event_rx: std::sync::mpsc::Receiver<ProbeEvent>,
    ) -> Self {
        ProbeTool { query_tx, event_rx }
    }

    /// Send a query and return Ok(()) if the channel is alive.
    fn send(&self, query: ProbeQuery) -> Result<(), ProbeError> {
        self.query_tx
            .send(query)
            .map_err(|_| ProbeError::ChannelClosed)
    }

    /// Try to receive a pending event (non-blocking).
    pub fn try_recv_event(&self) -> Result<Option<ProbeEvent>, ProbeError> {
        match self.event_rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(ProbeError::ChannelClosed),
        }
    }

    /// Block until the next event arrives.
    pub fn recv_event(&self) -> Result<ProbeEvent, ProbeError> {
        self.event_rx.recv().map_err(|_| ProbeError::ChannelClosed)
    }

    // ── Convenience methods ──────────────────────────────────────────

    /// Request a VM state snapshot.
    pub fn snapshot(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::Snapshot)
    }

    /// Set a breakpoint at the given instruction index.
    pub fn set_breakpoint(&self, ip: usize) -> Result<(), ProbeError> {
        self.send(ProbeQuery::SetBreakpoint(ip))
    }

    /// Clear a breakpoint at the given instruction index.
    pub fn clear_breakpoint(&self, ip: usize) -> Result<(), ProbeError> {
        self.send(ProbeQuery::ClearBreakpoint(ip))
    }

    /// Clear all breakpoints.
    pub fn clear_all_breakpoints(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::ClearAllBreakpoints)
    }

    /// Step one instruction (when paused).
    pub fn step(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::Step)
    }

    /// Continue execution (when paused).
    pub fn r#continue(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::Continue)
    }

    /// Set the tracer mode.
    pub fn set_tracer_mode(&self, mode: TracerMode) -> Result<(), ProbeError> {
        self.send(ProbeQuery::SetTracerMode(mode))
    }

    /// Get the current instruction pointer.
    pub fn get_ip(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::GetIp)
    }

    /// Get the last N trace entries.
    pub fn get_trace_tail(&self, n: usize) -> Result<(), ProbeError> {
        self.send(ProbeQuery::GetTraceTail(n))
    }

    /// List all StateField region names.
    pub fn list_regions(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::ListRegions)
    }

    /// Get a summary of the WorldGraph (node/edge counts).
    pub fn graph_summary(&self) -> Result<(), ProbeError> {
        self.send(ProbeQuery::GraphSummary)
    }
}

// ─── Errors ─────────────────────────────────────────────────────────

/// Errors that can occur during probing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeError {
    /// The probe channel is closed (VM shut down or disconnected).
    ChannelClosed,
    /// The probe tool is not connected to any VM.
    NotConnected,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::ChannelClosed => write!(f, "probe channel closed"),
            ProbeError::NotConnected => write!(f, "probe tool not connected to any VM"),
        }
    }
}

impl std::error::Error for ProbeError {}

// ─── Visualization helpers ──────────────────────────────────────────

/// Format a ProbeSnapshot as a human-readable string.
pub fn format_snapshot(snap: &ProbeSnapshot) -> String {
    match snap {
        ProbeSnapshot::VmState {
            program_id,
            ip,
            steps_executed,
            status,
        } => {
            let pid = program_id
                .map(|id| format!("{:?}", id))
                .unwrap_or_else(|| "<none>".to_string());
            format!(
                "VM [{}] ip={} steps={} status={}",
                &pid[..8.min(pid.len())],
                ip,
                steps_executed,
                status
            )
        }
        ProbeSnapshot::Node {
            id,
            concept,
            label,
            edge_count,
        } => {
            let lbl = label.as_deref().unwrap_or("<unlabelled>");
            let dim = concept.len();
            let preview: Vec<String> = concept
                .iter()
                .take(4)
                .map(|v| format!("{:.3}", v))
                .collect();
            format!(
                "Node #{} \"{}\" dim={} edges=[{}] vals=[{}..]",
                id.as_u64(),
                lbl,
                dim,
                edge_count,
                preview.join(", ")
            )
        }
        ProbeSnapshot::Region {
            name,
            offset,
            len,
            data,
        } => {
            let preview: Vec<String> = data.iter().take(8).map(|v| format!("{:.3}", v)).collect();
            format!(
                "Region \"{}\" [{}..{}] len={} vals=[{}..]",
                name,
                offset,
                offset + len,
                len,
                preview.join(", ")
            )
        }
        ProbeSnapshot::QueryResult(ids) => {
            let strs: Vec<String> = ids.iter().map(|id| id.as_u64().to_string()).collect();
            format!("QueryResult [{}]", strs.join(", "))
        }
        ProbeSnapshot::TraceSegment { entries } => {
            let lines: Vec<String> = entries.iter().map(|e| format!("  {}", e)).collect();
            format!("Trace ({} entries):\n{}", entries.len(), lines.join("\n"))
        }
        ProbeSnapshot::BreakpointSet(ip) => format!("Breakpoint set at ip={}", ip),
        ProbeSnapshot::BreakpointCleared(ip) => format!("Breakpoint cleared at ip={}", ip),
        ProbeSnapshot::Stepped => "Stepped one instruction".to_string(),
        ProbeSnapshot::Continued => "Continued execution".to_string(),
    }
}

/// Generate a graphviz dot string from a WorldGraph snapshot.
///
/// This is a simplified visualization — it shows nodes with their labels
/// and concept vector first value, connected by edges.
pub fn world_graph_to_dot(
    nodes: &[(u64, Option<String>, f32)], // (id, label, first_concept_value)
    edges: &[(u64, u64, String)],         // (source, target, edge_type)
) -> String {
    let mut dot = String::from("digraph WorldGraph {\n");
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=ellipse, style=filled, fillcolor=lightblue];\n\n");

    for (id, label, val) in nodes {
        let fallback = format!("#{}", id);
        let name = label.as_deref().unwrap_or(&fallback);
        dot.push_str(&format!("  n{} [label=\"{}\\n{:.3}\"];\n", id, name, val));
    }

    dot.push('\n');
    for (src, tgt, etype) in edges {
        dot.push_str(&format!(
            "  n{} -> n{} [label=\"{}\", color=gray];\n",
            src, tgt, etype
        ));
    }

    dot.push_str("}\n");
    dot
}

/// Format a StateField region summary as an ASCII table.
pub fn state_field_summary(regions: &[(String, usize, usize)], // (name, offset, len)
) -> String {
    let mut lines = vec![
        format!("{:<16} {:>8} {:>8} {:>8}", "Region", "Offset", "Len", "End"),
        "-".repeat(44),
    ];
    for (name, offset, len) in regions {
        lines.push(format!(
            "{:<16} {:>8} {:>8} {:>8}",
            name,
            offset,
            len,
            offset + len
        ));
    }
    lines.join("\n")
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_tool_roundtrip() {
        let (query_tx, query_rx) = std::sync::mpsc::channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();

        let tool = ProbeTool::new(query_tx, event_rx);

        // Send a breakpoint query.
        tool.set_breakpoint(42).unwrap();

        // VM side: receive the query.
        let query = query_rx.try_recv().unwrap();
        assert_eq!(query, ProbeQuery::SetBreakpoint(42));

        // VM side: send an event back.
        event_tx
            .send(ProbeEvent::Stepped {
                ip: 42,
                opcode: a2x_core::opcode::Opcode::Bind,
            })
            .unwrap();

        // Tool side: receive the event.
        let event = tool.try_recv_event().unwrap().unwrap();
        assert!(matches!(event, ProbeEvent::Stepped { ip: 42, .. }));
    }

    #[test]
    fn test_probe_tool_empty_recv() {
        let (dummy_tx, _query_rx) = std::sync::mpsc::channel::<ProbeQuery>();
        let (_event_tx, event_rx) = std::sync::mpsc::channel::<ProbeEvent>();

        let tool = ProbeTool::new(dummy_tx, event_rx);

        let result = tool.try_recv_event().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_format_snapshot_vm_state() {
        let snap = ProbeSnapshot::VmState {
            program_id: None,
            ip: 5,
            steps_executed: 10,
            status: "Running".to_string(),
        };
        let text = format_snapshot(&snap);
        assert!(text.contains("ip=5"));
        assert!(text.contains("steps=10"));
        assert!(text.contains("Running"));
    }

    #[test]
    fn test_format_snapshot_node() {
        let snap = ProbeSnapshot::Node {
            id: a2x_core::node::NodeId::new(1),
            concept: vec![1.0, 2.0, 3.0],
            label: Some("sys".to_string()),
            edge_count: 2,
        };
        let text = format_snapshot(&snap);
        assert!(text.contains("\"sys\""));
        assert!(text.contains("edges=[2]"));
    }

    #[test]
    fn test_world_graph_to_dot() {
        let nodes = vec![
            (1u64, Some("a".to_string()), 0.5),
            (2, Some("b".to_string()), -0.3),
        ];
        let edges = vec![(1u64, 2u64, "Causal".to_string())];
        let dot = world_graph_to_dot(&nodes, &edges);
        assert!(dot.contains("digraph WorldGraph"));
        assert!(dot.contains("n1"));
        assert!(dot.contains("n2"));
        assert!(dot.contains("Causal"));
    }

    #[test]
    fn test_state_field_summary() {
        let regions = vec![("goal".to_string(), 0, 64), ("belief".to_string(), 64, 256)];
        let summary = state_field_summary(&regions);
        assert!(summary.contains("goal"));
        assert!(summary.contains("belief"));
        assert!(summary.contains("Offset"));
    }

    #[test]
    fn test_probe_error_display() {
        assert_eq!(
            ProbeError::ChannelClosed.to_string(),
            "probe channel closed"
        );
        assert_eq!(
            ProbeError::NotConnected.to_string(),
            "probe tool not connected to any VM"
        );
    }
}
