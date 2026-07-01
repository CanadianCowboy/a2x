// See plans/05-agents.md §3 (CCS Agent)
//
// Phase 2.I: add `tick` method that drives the VM through one cognitive cycle
// (EVOLVE → REFLECT → PLAN) and exposes the resulting plan via `TickResult`.
// This is what "CCS agent that maintains a world-model" (PLAN §18) actually
// looks like — the agent's persistent VM is the world, and tick() advances
// it.

use std::sync::{Arc, Mutex};

use a2x_ccs::operators::plan::Action;
use a2x_ccs::CcsVm;
use a2x_ccs::VmStatus;
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::graph::{GraphQuery, WorldGraph};
use a2x_core::memory::MemoryTrace;
use a2x_core::node::NodeId;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_sigma::intent::IntentOp;
use a2x_sigma::packet::SigmaPacket;
use a2x_sigma::program::SigmaProgram;

/// Snapshot of one cognitive-loop tick's outcome.
///
/// Returned by `CcsAgent::tick()`. The plan actions are cloned so the caller
/// can hold them after the VM lock is released (avoids holding the mutex
/// across user code).
#[derive(Clone, Debug, PartialEq)]
pub struct TickResult {
    /// VM step count after the tick.
    pub steps_executed: usize,
    /// WorldGraph node count after the tick.
    pub world_graph_size: usize,
    /// MemoryTrace length after the tick.
    pub memory_trace_length: usize,
    /// Whether the REFLECT opcode set `vm.last_reflect` (a self-model node
    /// was created and visible to subsequent PLANs).
    pub last_reflect_set: bool,
    /// Cloned `vm.last_plan_actions` at tick-end. May be empty if no
    /// significant belief signal was present.
    pub plan_actions: Vec<Action>,
}

impl TickResult {
    /// Convenience: did this tick produce any high-priority plan actions?
    pub fn has_actions(&self) -> bool {
        !self.plan_actions.is_empty()
    }
}

/// The CCS agent — long-running cognitive agent.
///
/// Maintains a persistent WorldGraph, continuously executing Evolve/Reflect
/// cycles. Builds up a rich world-model over time and responds to queries.
pub struct CcsAgent {
    /// Agent identity.
    id: AgentId,
    /// Persistent CCS VM (long-running).
    vm: Arc<Mutex<CcsVm>>,
    /// Whether the agent is running its cognitive loop.
    running: Arc<Mutex<bool>>,
}

impl CcsAgent {
    /// Create a new CCS agent with a persistent WorldGraph.
    pub fn new(id: AgentId) -> Self {
        CcsAgent {
            id,
            vm: Arc::new(Mutex::new(CcsVm::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the continuous cognitive loop (Evolve + Reflect).
    ///
    /// Spawns a background thread that continuously calls `tick()` with a
    /// configurable sleep interval between ticks. The loop runs until
    /// `stop_cognitive_loop()` is called.
    pub fn start_cognitive_loop(&self) {
        {
            let mut running = self.running.lock().expect("cognitive loop lock poisoned");
            if *running {
                return; // Already running.
            }
            *running = true;
        }

        // Clone Arcs for the background thread.
        let vm = Arc::clone(&self.vm);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            let tick_interval = std::time::Duration::from_millis(100);
            loop {
                // Check if we should keep running.
                {
                    let guard = running.lock().expect("cognitive loop lock poisoned");
                    if !*guard {
                        break;
                    }
                }

                // Build and execute a cognitive-loop program on the VM.
                {
                    let mut vm_guard = vm.lock().expect("vm lock poisoned");
                    let prog = build_cognitive_loop_program();
                    vm_guard.load(prog);
                    match vm_guard.run() {
                        Ok(VmStatus::Halted) | Ok(VmStatus::Yield) | Ok(VmStatus::Suspended) => {
                            // Tick completed successfully.
                        }
                        Ok(VmStatus::Running) => {
                            tracing::warn!("CCS cognitive loop: program did not halt");
                        }
                        Ok(VmStatus::Fault(err)) => {
                            tracing::error!("CCS cognitive loop fault: {}", err);
                        }
                        Err(err) => {
                            tracing::error!("CCS cognitive loop error: {}", err);
                        }
                    }
                }

                std::thread::sleep(tick_interval);
            }
        });
    }

    /// Stop the cognitive loop.
    pub fn stop_cognitive_loop(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }
    }

    /// Check if the cognitive loop is running.
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Drive one cognitive-loop tick:
    ///   1. Build a 3-instruction Σ∞ program: EVOLVE → REFLECT → PLAN.
    ///   2. Load + run it on the persistent VM.
    ///   3. Snapshot `vm.last_plan_actions` + observability fields
    ///      into a `TickResult` and return.
    ///
    /// Locking: holds the VM mutex for the load+run only; clones
    /// observations before releasing so callers can use the result
    /// without blocking the VM.
    ///
    /// Determinism: the constructed program contains no wall-clock reads
    /// and uses fixed opcode symbols (intent::Delay / Contradiction /
    /// Parallel), so two ticks on the same VM produce identical
    /// TickResults modulo state that intentionally evolved on prior ticks.
    pub fn tick(&self) -> Result<TickResult, AgentError> {
        let program = build_cognitive_loop_program();

        let observations = {
            let mut vm = self
                .vm
                .lock()
                .map_err(|e| AgentError::VmError(format!("vm mutex poisoned: {}", e)))?;
            vm.load(program);
            match vm.run() {
                Ok(VmStatus::Halted) => {}
                Ok(VmStatus::Yield) => {
                    // Yield is a soft-pause, not a fault, so we proceed to
                    // snapshot anyway — the program may resume on a later
                    // tick. But for now, callers see Halted semantics
                    // since we built a fixed-length 3-instruction program
                    // with an implicit HALT at end-of-packet-stream.
                }
                Ok(VmStatus::Suspended) => {
                    // T1-3: suspended VM can be resumed later.
                    // For tick(), treat as complete.
                }
                Ok(VmStatus::Running) => {
                    return Err(AgentError::VmError(
                        "tick program did not halt after run()".into(),
                    ));
                }
                Ok(VmStatus::Fault(err)) => {
                    return Err(AgentError::VmError(format!("tick fault: {}", err)));
                }
                Err(err) => {
                    return Err(AgentError::VmError(format!("tick error: {}", err)));
                }
            }

            TickResult {
                steps_executed: vm.steps_executed(),
                world_graph_size: vm.world_graph.node_count(),
                memory_trace_length: vm.memory_trace.len(),
                last_reflect_set: vm.last_reflect.is_some(),
                plan_actions: vm.last_plan_actions.clone(),
            }
        };

        Ok(observations)
    }

    /// Run a Σ∞ program on the persistent VM and return the tick summary.
    ///
    /// Distinct from `tick()` which hard-codes the 3-instruction cognitive
    /// loop. This variant lets callers run their own Σ∞ program (e.g.
    /// parsed from text) and still get the same observability surface.
    pub fn run_program(&self, program: SigmaProgram) -> Result<TickResult, AgentError> {
        let observations = {
            let mut vm = self
                .vm
                .lock()
                .map_err(|e| AgentError::VmError(format!("vm mutex poisoned: {}", e)))?;
            vm.load(program);
            match vm.run() {
                Ok(VmStatus::Halted) | Ok(VmStatus::Yield) | Ok(VmStatus::Suspended) => {}
                Ok(VmStatus::Running) => {
                    return Err(AgentError::VmError(
                        "program did not halt after run()".into(),
                    ));
                }
                Ok(VmStatus::Fault(err)) => {
                    return Err(AgentError::VmError(format!("program fault: {}", err)));
                }
                Err(err) => {
                    return Err(AgentError::VmError(format!("program error: {}", err)));
                }
            }

            TickResult {
                steps_executed: vm.steps_executed(),
                world_graph_size: vm.world_graph.node_count(),
                memory_trace_length: vm.memory_trace.len(),
                last_reflect_set: vm.last_reflect.is_some(),
                plan_actions: vm.last_plan_actions.clone(),
            }
        };

        Ok(observations)
    }

    /// Query the agent's WorldGraph.
    ///
    /// Supports structured query syntax:
    ///   - "label:<name>" — lookup by label
    ///   - "neighbors:<id>[:<max_hops>]" — neighbor traversal (default 1 hop)
    ///   - "similar:<id>[:<threshold>]" — cosine-similarity search (default 0.5)
    ///   - "relation:<type>" — all nodes with a given relation type
    ///   - "summary" — node/edge count summary
    ///   - Anything else: treated as a label query.
    pub fn query(&self, query_str: &str) -> Result<SigmaProgram, AgentError> {
        let vm = self
            .vm
            .lock()
            .map_err(|e| AgentError::VmError(format!("vm mutex poisoned: {}", e)))?;

        let result_ids = if let Some(rest) = query_str.strip_prefix("label:") {
            match vm
                .world_graph
                .lookup_label(rest)
                .map_err(|e| AgentError::VmError(format!("graph error: {}", e)))?
            {
                Some(id) => vec![id],
                None => Vec::new(),
            }
        } else if let Some(rest) = query_str.strip_prefix("neighbors:") {
            let parts: Vec<&str> = rest.split(':').collect();
            let src_id = parts[0]
                .parse::<u64>()
                .map_err(|_| AgentError::VmError(format!("invalid node id: {}", parts[0])))?;
            let max_hops = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1usize);
            vm.world_graph
                .query(&GraphQuery::Neighbors {
                    node: NodeId::new(src_id),
                    max_hops,
                })
                .map_err(|e| AgentError::VmError(format!("query error: {}", e)))?
        } else if let Some(rest) = query_str.strip_prefix("similar:") {
            let parts: Vec<&str> = rest.split(':').collect();
            let src_id = parts[0]
                .parse::<u64>()
                .map_err(|_| AgentError::VmError(format!("invalid node id: {}", parts[0])))?;
            let threshold = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.5f32);
            let src_node = vm
                .world_graph
                .lookup(NodeId::new(src_id))
                .map_err(|e| AgentError::VmError(format!("lookup error: {}", e)))?
                .ok_or_else(|| AgentError::VmError(format!("node {} not found", src_id)))?;
            vm.world_graph
                .query(&GraphQuery::BySimilarity {
                    concept: src_node.concept,
                    threshold,
                })
                .map_err(|e| AgentError::VmError(format!("query error: {}", e)))?
        } else if let Some(rel_type_str) = query_str.strip_prefix("relation:") {
            let rel_type = match rel_type_str {
                "causal" => a2x_core::relation::RelationType::Causal,
                "spatial" => a2x_core::relation::RelationType::Spatial,
                "temporal" => a2x_core::relation::RelationType::Temporal,
                "logical" => a2x_core::relation::RelationType::Logical,
                "hierarchical" => a2x_core::relation::RelationType::Hierarchical,
                _ => {
                    return Err(AgentError::VmError(format!(
                        "unknown relation type: {}",
                        rel_type_str
                    )))
                }
            };
            vm.world_graph
                .query(&GraphQuery::ByRelation(rel_type))
                .map_err(|e| AgentError::VmError(format!("query error: {}", e)))?
        } else if query_str == "summary" {
            // Return a serialized summary as a program with one data packet.
            let mut prog = SigmaProgram::new();
            let mut p = SigmaPacket::new();
            let summary = format!(
                "nodes={} edges={}",
                vm.world_graph.node_count(),
                vm.world_graph.edge_count()
            );
            p.data.payload = summary.into_bytes();
            prog.push(p);
            return Ok(prog);
        } else {
            // Default: treat as label query.
            match vm
                .world_graph
                .lookup_label(query_str)
                .map_err(|e| AgentError::VmError(format!("graph error: {}", e)))?
            {
                Some(id) => vec![id],
                None => Vec::new(),
            }
        };

        // Encode result NodeIds into a Σ∞ program (one GROUND packet per result).
        let mut prog = SigmaProgram::new();
        for id in result_ids {
            if let Some(node) = vm
                .world_graph
                .lookup(id)
                .map_err(|e| AgentError::VmError(format!("lookup error: {}", e)))?
            {
                let mut p = SigmaPacket::new();
                p.intent.operators.push(IntentOp::Star); // GROUND
                                                         // Encode concept data as f32 LE bytes.
                let mut bytes = Vec::new();
                for f in &node.concept.data {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                // Also append NodeId as a label for identifiability.
                p.context.labels.push(format!("n{}", id.as_u64()));
                if let Some(ref label) = node.label {
                    p.context.labels.push(label.clone());
                }
                p.data.payload = bytes;
                prog.push(p);
            }
        }
        Ok(prog)
    }

    /// Borrow the persistent VM's observable state (safe read-only peek).
    /// Returns None if the VM mutex is poisoned.
    pub fn vm_snapshot(&self) -> Option<VmSnapshot> {
        let vm = self.vm.lock().ok()?;
        Some(VmSnapshot {
            ip: vm.ip,
            steps_executed: vm.steps_executed(),
            world_graph_size: vm.world_graph.node_count(),
            memory_trace_length: vm.memory_trace.len(),
            last_reflect_set: vm.last_reflect.is_some(),
            plan_actions: vm.last_plan_actions.clone(),
            uptime: vm.uptime(),
        })
    }
}

/// Read-only snapshot of the persistent VM's observable state.
#[derive(Clone, Debug)]
pub struct VmSnapshot {
    pub ip: usize,
    pub steps_executed: usize,
    pub world_graph_size: usize,
    pub memory_trace_length: usize,
    pub last_reflect_set: bool,
    pub plan_actions: Vec<Action>,
    pub uptime: std::time::Duration,
}

/// Construct the canonical 3-instruction cognitive-loop program:
///   [EVOLVE, REFLECT, PLAN]
///
/// The VM's execute loop naturally halts at end-of-packet-stream, so no
/// explicit HALT is required.
fn build_cognitive_loop_program() -> SigmaProgram {
    let mut prog = SigmaProgram::new();
    prog.push(make_packet(IntentOp::Delay)); // → Opcode::Evolve
    prog.push(make_packet(IntentOp::Contradiction)); // → Opcode::Reflect
    prog.push(make_packet(IntentOp::Parallel)); // → Opcode::Plan
    prog
}

fn make_packet(intent: IntentOp) -> SigmaPacket {
    let mut p = SigmaPacket::new();
    p.intent.operators.push(intent);
    p
}

impl Agent for CcsAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Ccs
    }

    fn execute(&self, program: Packet) -> Result<Packet, AgentError> {
        // Try to parse the packet as a Σ∞ program text, then run through
        // the persistent VM. Returns the result as a raw Σ∞ text packet.
        let text = match &program {
            Packet::Raw(bytes) => String::from_utf8_lossy(bytes).to_string(),
        };

        let sigma_prog = if text.trim().is_empty() {
            SigmaProgram::new()
        } else {
            a2x_sigma::parse_program(&text).unwrap_or_else(|_| SigmaProgram::new())
        };

        let result = self.run_program(sigma_prog)?;

        // Serialize the result actions as text
        let output = if result.has_actions() {
            result
                .plan_actions
                .iter()
                .map(|a| {
                    format!(
                        "{}:{:?} priority={:.2}",
                        a.verb.as_str(),
                        a.opcode,
                        a.priority
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            format!(
                "steps={} graph_size={} trace_len={}",
                result.steps_executed, result.world_graph_size, result.memory_trace_length
            )
        };

        Ok(Packet::Raw(output.into_bytes()))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let vm = self.vm.lock().ok()?;
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: if self.is_running() {
                "running".into()
            } else {
                "idle".into()
            },
            current_program: None,
            ip: Some(vm.ip),
            world_graph_size: vm.world_graph.node_count(),
            memory_trace_length: vm.memory_trace.len(),
            uptime: vm.uptime(),
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Execute,
            Capability::Custom("plan".into()),
            Capability::Custom("schedule".into()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ccs_agent() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        assert_eq!(agent.id(), AgentId::new("ccs-1"));
        assert_eq!(agent.agent_type(), AgentType::Ccs);
        assert!(!agent.is_running());
    }

    #[test]
    fn test_start_stop_loop() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        assert!(!agent.is_running());
        agent.start_cognitive_loop();
        assert!(agent.is_running());
        agent.stop_cognitive_loop();
        assert!(!agent.is_running());
    }

    #[test]
    fn test_query_stub() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.query("find: anomaly");
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_summary() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let summary = agent.state_summary();
        assert!(summary.is_some());
    }

    // === Phase 2.I: tick ===

    #[test]
    fn test_tick_returns_three_steps_executed() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.tick().unwrap();
        // 3 instructions: EVOLVE, REFLECT, PLAN.
        assert_eq!(result.steps_executed, 3);
    }

    #[test]
    fn test_tick_appends_trace_entries() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.tick().unwrap();
        // MemoryTrace pushes once per step → len == 3 after tick.
        assert_eq!(result.memory_trace_length, 3);
    }

    #[test]
    fn test_tick_sets_last_reflect() {
        // REFLECT allocates a self-model node → vm.last_reflect is Some.
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.tick().unwrap();
        assert!(
            result.last_reflect_set,
            "reflect should populate last_reflect"
        );
    }

    #[test]
    fn test_tick_records_plan_actions() {
        // PLAN with a non-zero belief signal should produce 1+ action.
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let result = agent.tick().unwrap();
        assert!(
            !result.plan_actions.is_empty(),
            "first tick should produce at least 1 plan action; got 0"
        );
    }

    #[test]
    fn test_tick_grows_world_graph() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let before = agent.vm_snapshot().map(|s| s.world_graph_size).unwrap_or(0);
        agent.tick().unwrap();
        let after = agent.vm_snapshot().unwrap().world_graph_size;
        // REFLECT allocates a self-model node (1 new) plus PLAN allocates
        // a Plan node (1 new) → +2 nodes minimum.
        assert!(
            after >= before + 2,
            "expected graph to grow by >=2; was {} now {}",
            before,
            after
        );
    }

    #[test]
    fn test_two_ticks_increments_steps_by_three_each() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let r1 = agent.tick().unwrap();
        let r2 = agent.tick().unwrap();
        assert_eq!(r1.steps_executed, 3);
        assert_eq!(r2.steps_executed, 3);
        // Second tick should grow the graph further (more refs + plans).
        assert!(r2.world_graph_size >= r1.world_graph_size);
    }

    #[test]
    fn test_tick_when_not_running_still_works() {
        // tick() is independent of start_cognitive_loop. The flag tracks
        // explicit cognitive loop state; tick() always advances the VM.
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        assert!(!agent.is_running());
        let r = agent.tick().unwrap();
        assert_eq!(r.steps_executed, 3);
        // Loop status unchanged by tick.
        assert!(!agent.is_running());
    }

    #[test]
    fn test_run_program_with_halted_program_returns_tick() {
        // Empty program → VM halts immediately with no steps.
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let r = agent.run_program(SigmaProgram::new()).unwrap();
        assert_eq!(r.steps_executed, 0);
        assert!(
            r.plan_actions.is_empty(),
            "empty program must not produce plan actions; got {} actions",
            r.plan_actions.len()
        );
        assert!(!r.last_reflect_set);
    }

    #[test]
    fn test_run_program_with_single_evolve_advances_state() {
        let agent = CcsAgent::new(AgentId::new("ccs-1"));
        let mut prog = SigmaProgram::new();
        prog.push(make_packet(IntentOp::Delay)); // EVOLVE
        let r = agent.run_program(prog).unwrap();
        assert_eq!(r.steps_executed, 1);
        // No REFLECT was issued, so plan_actions may be empty (no signal).
        assert!(!r.last_reflect_set);
    }
}
