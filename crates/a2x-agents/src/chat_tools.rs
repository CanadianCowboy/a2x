// ChatAgent tools — A2X subsystem tool definitions and execution
//
// Each tool wraps an A2X subsystem capability. Tools are defined with
// JSON Schema parameters so the LLM can call them via function calling.

use std::sync::{Arc, Mutex};

use a2x_bus::{AgentFilter, Bus};
use a2x_ccs::CcsVm;
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_sigma::intent::IntentOp;
use a2x_sigma::plan::PlanOp;
use a2x_sigma::program::SigmaProgram;
use a2x_sigma::SigmaPacket;
use serde_json::Value;

use crate::cli_agent::CliAgent;
use crate::llm_backend::ToolDef;

/// Context passed to tool execution functions.
pub struct ToolContext {
    /// The A2X bus for agent discovery and dispatch.
    pub bus: Arc<Mutex<Bus>>,
    /// CLI agent for filesystem/shell operations.
    pub cli_agent: Arc<CliAgent>,
    /// CCS VM for cognitive operations.
    pub ccs_vm: Arc<Mutex<CcsVm>>,
    /// Conversation history (for probe/reflect operations).
    pub conversation: Vec<(String, String)>,
    /// Tool call counter for generating unique call IDs.
    pub tool_call_count: u64,
}

/// Result of executing a tool.
#[derive(Clone, Debug)]
pub struct ToolResult {
    /// The tool call ID this result corresponds to.
    pub tool_call_id: String,
    /// The result content (JSON string or text).
    pub content: String,
    /// Whether the tool executed successfully.
    pub success: bool,
}

impl ToolContext {
    pub fn new(bus: Arc<Mutex<Bus>>, cli_agent: Arc<CliAgent>, ccs_vm: Arc<Mutex<CcsVm>>) -> Self {
        ToolContext {
            bus,
            cli_agent,
            ccs_vm,
            conversation: vec![],
            tool_call_count: 0,
        }
    }
}

/// Build a simple Σ∞ packet with an intent and optional plan.
fn make_packet(intent: IntentOp, plan: Option<PlanOp>) -> SigmaPacket {
    let mut p = SigmaPacket::new();
    p.intent.operators.push(intent);
    if let Some(plan) = plan {
        p.plan.operators.push(plan);
    }
    p
}

/// Returns all tool definitions for the A2X coding agent.
pub fn all_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "execute_sigma",
            "Execute a Σ∞ Sigma program and return the result.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "program": {"type": "string", "description": "Σ∞ program source text"}
                },
                "required": ["program"]
            }),
        ),
        ToolDef::new(
            "parse_sigma",
            "Parse a Σ∞ program to validate syntax and show decoded instructions.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "program": {"type": "string", "description": "Σ∞ program source text"}
                },
                "required": ["program"]
            }),
        ),
        ToolDef::new(
            "list_agents",
            "List agents on the A2X bus with capabilities and status.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "filter_type": {"type": "string", "description": "Optional agent type filter"},
                    "filter_capability": {"type": "string", "description": "Optional capability filter"}
                },
                "required": []
            }),
        ),
        ToolDef::new(
            "probe_agent",
            "Probe an agent's state (IP, graph size, trace length, uptime).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string", "description": "Agent ID to probe"}
                },
                "required": ["agent_id"]
            }),
        ),
        ToolDef::new(
            "inspect_graph",
            "Inspect the CCS WorldGraph: node/edge counts, list nodes.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "Max nodes to return (default 20)"}
                },
                "required": []
            }),
        ),
        ToolDef::new(
            "shell_exec",
            "Execute a shell command and return stdout/stderr.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to run"}
                },
                "required": ["command"]
            }),
        ),
        ToolDef::new(
            "fs_read",
            "Read file contents from the filesystem.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "max_lines": {"type": "integer", "description": "Max lines (default 100)"}
                },
                "required": ["path"]
            }),
        ),
        ToolDef::new(
            "fs_write",
            "Write content to a file.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to write"},
                    "content": {"type": "string", "description": "Content to write"}
                },
                "required": ["path", "content"]
            }),
        ),
        ToolDef::new(
            "run_ccs_program",
            "Run a cognitive operation on the CCS VM (bind, ground, reflect, evolve, plan).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "CCS operation: reflect, evolve, plan, ground, bind"}
                },
                "required": ["operation"]
            }),
        ),
        ToolDef::new(
            "compile_omega",
            "Compile a Σ∞ program to the Ω latent representation.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "program": {"type": "string", "description": "Σ∞ program source text"}
                },
                "required": ["program"]
            }),
        ),
        ToolDef::new(
            "vm_status",
            "Get CCS VM status: IP, steps executed, graph nodes/edges, memory trace length, uptime.",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        ToolDef::new(
            "vm_query",
            "Query the CCS WorldGraph by label, neighbors, similarity, or custom query.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query_type": {"type": "string", "description": "Query type: label, neighbors, similarity, nodes, count"},
                    "value": {"type": "string", "description": "Label name, node ID, concept vector [x,y,z], or custom query string"},
                    "threshold": {"type": "number", "description": "Similarity threshold for similarity queries (0.0-1.0, default 0.5)"},
                    "max_hops": {"type": "integer", "description": "Max hops for neighbors query (default 1)"}
                },
                "required": ["query_type", "value"]
            }),
        ),
        ToolDef::new(
            "vm_region",
            "Read a CCS VM StateField region by name (e.g., belief, attention, goal, scratch).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "region": {"type": "string", "description": "Region name: belief, attention, temporal, goal, scratch, observ"},
                    "max_values": {"type": "integer", "description": "Max values to return (default 16)"}
                },
                "required": ["region"]
            }),
        ),
        ToolDef::new(
            "vm_trace",
            "Get the CCS VM memory trace tail (recent instruction history).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "tail": {"type": "integer", "description": "Number of recent trace entries to return (default 10)"}
                },
                "required": []
            }),
        ),
    ]
}

/// Execute a tool by name and return the result.
pub fn execute_tool(
    name: &str,
    args: &Value,
    tool_call_id: &str,
    ctx: &mut ToolContext,
) -> ToolResult {
    ctx.tool_call_count += 1;

    match name {
        "execute_sigma" => execute_sigma_impl(args, tool_call_id, ctx),
        "parse_sigma" => parse_sigma_impl(args, tool_call_id),
        "list_agents" => list_agents_impl(args, tool_call_id, ctx),
        "probe_agent" => probe_agent_impl(args, tool_call_id, ctx),
        "inspect_graph" => inspect_graph_impl(args, tool_call_id, ctx),
        "shell_exec" => shell_exec_impl(args, tool_call_id, ctx),
        "fs_read" => fs_read_impl(args, tool_call_id),
        "fs_write" => fs_write_impl(args, tool_call_id),
        "run_ccs_program" => run_ccs_program_impl(args, tool_call_id, ctx),
        "compile_omega" => compile_omega_impl(args, tool_call_id),
        "vm_status" => vm_status_impl(args, tool_call_id, ctx),
        "vm_query" => vm_query_impl(args, tool_call_id, ctx),
        "vm_region" => vm_region_impl(args, tool_call_id, ctx),
        "vm_trace" => vm_trace_impl(args, tool_call_id, ctx),
        _ => ToolResult {
            tool_call_id: tool_call_id.into(),
            content: format!(r#"{{"error": "unknown tool: {}"}}"#, name),
            success: false,
        },
    }
}

// ── Tool implementations ──────────────────────────────────────────────────

fn execute_sigma_impl(args: &Value, tool_call_id: &str, _ctx: &mut ToolContext) -> ToolResult {
    let program_str = args["program"].as_str().unwrap_or("");
    match a2x_sigma::parse_program(program_str) {
        Ok(mut program) => {
            program.compute_id();
            let inst_count = program.len();
            let orch = crate::Orchestrator::new(AgentId::new("chat-orch"));
            let cid = tool_call_id.to_string();
            match orch.dispatch(program) {
                Ok(result) => {
                    let output = if result.is_empty() {
                        "∅ (empty result — program executed successfully with no output)".into()
                    } else {
                        result
                            .instructions
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    ToolResult {
                        tool_call_id: cid,
                        content: serde_json::json!({
                            "success": true,
                            "instructions_executed": inst_count,
                            "output": output
                        })
                        .to_string(),
                        success: true,
                    }
                }
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("{}", e)})
                        .to_string(),
                    success: false,
                },
            }
        }
        Err(e) => ToolResult {
            tool_call_id: tool_call_id.into(),
            content: serde_json::json!({"success": false, "error": format!("parse error: {}", e)})
                .to_string(),
            success: false,
        },
    }
}

fn parse_sigma_impl(args: &Value, tool_call_id: &str) -> ToolResult {
    let program_str = args["program"].as_str().unwrap_or("");
    let cid = tool_call_id.to_string();
    match a2x_sigma::parse_program(program_str) {
        Ok(mut program) => {
            program.compute_id();
            let instructions: Vec<String> =
                program.instructions.iter().map(|p| p.to_string()).collect();
            ToolResult {
                tool_call_id: cid,
                content: serde_json::json!({
                    "success": true,
                    "program_id": program.id.to_string(),
                    "instruction_count": instructions.len(),
                    "instructions": instructions
                })
                .to_string(),
                success: true,
            }
        }
        Err(e) => ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "error": format!("{}", e)}).to_string(),
            success: false,
        },
    }
}

fn list_agents_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let cid = tool_call_id.to_string();
    let bus = ctx
        .bus
        .lock()
        .unwrap_or_else(|e| panic!("bus lock error: {}", e));

    let filter = if let Some(cap_str) = args["filter_capability"].as_str() {
        AgentFilter::ByCapability(match cap_str.to_lowercase().as_str() {
            "execute" => Capability::Execute,
            "fs" | "filesystem" => Capability::FileSystem,
            "net" | "network" => Capability::Network,
            "shell" => Capability::Shell,
            "probe" => Capability::Probe,
            other => Capability::Custom(other.into()),
        })
    } else if let Some(type_str) = args["filter_type"].as_str() {
        AgentFilter::ByType(match type_str.to_lowercase().as_str() {
            "orchestrator" | "orch" => AgentType::Orchestrator,
            "cli" => AgentType::Cli,
            "llm" => AgentType::Llm,
            "ccs" => AgentType::Ccs,
            "omega" => AgentType::Omega,
            "chat" => AgentType::Chat,
            _ => AgentType::Entity,
        })
    } else {
        AgentFilter::All
    };

    let agents: Vec<Value> = bus
        .discover(&filter)
        .iter()
        .map(|info| {
            serde_json::json!({
                "id": info.id.as_str(), "type": format!("{:?}", info.agent_type),
                "online": info.online,
                "capabilities": info.capabilities.iter().map(|c| c.to_string()).collect::<Vec<_>>()
            })
        })
        .collect();

    ToolResult {
        tool_call_id: cid,
        content: serde_json::json!({"success": true, "count": agents.len(), "agents": agents})
            .to_string(),
        success: true,
    }
}

fn probe_agent_impl(args: &Value, tool_call_id: &str, _ctx: &mut ToolContext) -> ToolResult {
    let agent_id = args["agent_id"].as_str().unwrap_or("orch-1");
    let cid = tool_call_id.to_string();
    let id = AgentId::new(agent_id);
    let agents: Vec<Box<dyn Agent>> = vec![
        Box::new(crate::Orchestrator::new(id.clone())),
        Box::new(crate::CliAgent::new(id.clone())),
        Box::new(crate::LlmAgent::new_stub(id.clone(), "probe")),
        Box::new(crate::CcsAgent::new(id.clone())),
    ];

    for agent in agents {
        if let Some(snapshot) = agent.state_summary() {
            return ToolResult {
                tool_call_id: cid,
                content: serde_json::json!({
                    "success": true, "agent_id": snapshot.agent_id.as_str(),
                    "state": snapshot.state, "ip": snapshot.ip,
                    "world_graph_size": snapshot.world_graph_size,
                    "memory_trace_length": snapshot.memory_trace_length,
                    "uptime_secs": snapshot.uptime.as_secs_f32()
                })
                .to_string(),
                success: true,
            };
        }
    }

    ToolResult {
        tool_call_id: cid,
        content: serde_json::json!({"success": false, "error": format!("agent '{}' not found", agent_id)}).to_string(),
        success: false,
    }
}

fn inspect_graph_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let cid = tool_call_id.to_string();
    let limit = args["limit"].as_u64().unwrap_or(20) as usize;
    let vm = ctx
        .ccs_vm
        .lock()
        .unwrap_or_else(|e| panic!("ccs vm lock error: {}", e));

    let node_count = vm.world_graph.node_count();
    let edge_count = vm.world_graph.edge_count();

    // Collect node info from WorldGraph
    let node_ids = vm.world_graph.node_ids();
    let nodes: Vec<Value> = node_ids.iter().take(limit).filter_map(|nid| {
        vm.world_graph.lookup(*nid).ok().flatten().map(|node| {
            serde_json::json!({
                "id": node.id.as_u64(),
                "label": node.label,
                "concept_preview": node.concept.data.iter().take(4).map(|v| format!("{:.3}", v)).collect::<Vec<_>>(),
                "access_count": node.metadata.access_count,
                "edge_count": node.edges.len()
            })
        })
    }).collect();

    ToolResult {
        tool_call_id: cid,
        content: serde_json::json!({
            "success": true, "node_count": node_count, "edge_count": edge_count,
            "nodes_shown": nodes.len(), "nodes": nodes
        })
        .to_string(),
        success: true,
    }
}

fn shell_exec_impl(args: &Value, tool_call_id: &str, _ctx: &mut ToolContext) -> ToolResult {
    let command = args["command"].as_str().unwrap_or("echo no command");
    let cid = tool_call_id.to_string();

    use std::process::Command;
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{}\n[stderr]\n{}", stdout, stderr)
            };
            ToolResult {
                tool_call_id: cid,
                content: serde_json::json!({
                    "success": out.status.success(), "command": command,
                    "exit_code": out.status.code(), "output": combined
                })
                .to_string(),
                success: out.status.success(),
            }
        }
        Err(e) => ToolResult {
            tool_call_id: cid,
            content:
                serde_json::json!({"success": false, "command": command, "error": format!("{}", e)})
                    .to_string(),
            success: false,
        },
    }
}

fn fs_read_impl(args: &Value, tool_call_id: &str) -> ToolResult {
    let path = args["path"].as_str().unwrap_or("");
    let max_lines = args["max_lines"].as_u64().unwrap_or(100) as usize;
    let cid = tool_call_id.to_string();

    match std::fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().take(max_lines).collect();
            let truncated = lines.len() < content.lines().count();
            ToolResult {
                tool_call_id: cid,
                content: serde_json::json!({
                    "success": true, "path": path, "lines": lines.len(),
                    "truncated": truncated, "content": lines.join("\n")
                })
                .to_string(),
                success: true,
            }
        }
        Err(e) => ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "path": path, "error": format!("{}", e)})
                .to_string(),
            success: false,
        },
    }
}

fn fs_write_impl(args: &Value, tool_call_id: &str) -> ToolResult {
    let path = args["path"].as_str().unwrap_or("");
    let content = args["content"].as_str().unwrap_or("");
    let cid = tool_call_id.to_string();

    match std::fs::write(path, content) {
        Ok(()) => ToolResult {
            tool_call_id: cid,
            content:
                serde_json::json!({"success": true, "path": path, "bytes_written": content.len()})
                    .to_string(),
            success: true,
        },
        Err(e) => ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "path": path, "error": format!("{}", e)})
                .to_string(),
            success: false,
        },
    }
}

fn run_ccs_program_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let operation = args["operation"].as_str().unwrap_or("reflect");
    let cid = tool_call_id.to_string();

    // Build the Sigma program for the operation
    let intent = match operation {
        "reflect" => IntentOp::Contradiction,
        "evolve" => IntentOp::Delay,
        "plan" => IntentOp::Parallel,
        "ground" => IntentOp::Star,
        "bind" => IntentOp::Synthesis,
        _ => IntentOp::Contradiction,
    };
    let mut prog = SigmaProgram::new();
    prog.push(make_packet(intent, None));

    // Clone the Arc so we can run the VM asynchronously without holding the context lock
    let vm_arc = ctx.ccs_vm.clone();
    let op = operation.to_string();

    // Execute the program on the VM using async execution.
    // Uses block_in_place to avoid starving the tokio worker pool while
    // the VM runs (up to 30s with periodic yields).
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // SAFETY: block_in_place runs on a dedicated thread, so holding the
            // MutexGuard across .await is safe here (no worker starvation).
            #[allow(clippy::await_holding_lock)]
            let result = tokio::task::block_in_place(move || {
                handle.block_on(async move {
                    let mut vm = vm_arc
                        .lock()
                        .unwrap_or_else(|e| panic!("ccs vm lock: {}", e));
                    vm.load(prog);
                    use a2x_ccs::async_vm::AsyncRunConfig;
                    let config = AsyncRunConfig {
                        yield_interval: 64,
                        timeout: Some(std::time::Duration::from_secs(30)),
                    };
                    let async_result = vm.run_async(config).await;
                    let steps = vm.steps_executed();
                    let graph_nodes = vm.world_graph.node_count();
                    let graph_edges = vm.world_graph.edge_count();
                    (async_result, steps, graph_nodes, graph_edges)
                })
            });
            match result {
                (Ok(async_result), steps, nodes, edges) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({
                        "success": true, "operation": op,
                        "vm_status": format!("{:?}", async_result.status),
                        "cancelled": async_result.cancelled,
                        "steps": steps,
                        "graph_nodes": nodes,
                        "graph_edges": edges
                    }).to_string(),
                    success: true,
                },
                (Err(e), _, _, _) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "operation": op, "error": format!("{}", e)}).to_string(),
                    success: false,
                },
            }
        }
        Err(_) => {
            // No tokio runtime available — fall back to sync execution
            let mut vm = vm_arc
                .lock()
                .unwrap_or_else(|e| panic!("ccs vm lock: {}", e));
            vm.load(prog);
            match vm.run() {
                Ok(status) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({
                        "success": true, "operation": op,
                        "vm_status": format!("{:?}", status),
                        "steps": vm.steps_executed(),
                        "graph_nodes": vm.world_graph.node_count(),
                        "graph_edges": vm.world_graph.edge_count()
                    }).to_string(),
                    success: true,
                },
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "operation": op, "error": format!("{}", e)}).to_string(),
                    success: false,
                },
            }
        }
    }
}

fn compile_omega_impl(args: &Value, tool_call_id: &str) -> ToolResult {
    let program_str = args["program"].as_str().unwrap_or("");
    let cid = tool_call_id.to_string();
    match a2x_sigma::parse_program(program_str) {
        Ok(program) => {
            use a2x_omega::CompileToOmega;
            match program.compile(a2x_omega::OptimizationLevel::default()) {
                Ok(omega_program) => {
                    ToolResult {
                        tool_call_id: cid,
                        content: serde_json::json!({
                            "success": true, "omega_packets": omega_program.len(),
                            "original_instructions": program.len(),
                            "compression_ratio": if !program.is_empty() {
                                format!("{:.1}%", (omega_program.len() as f64 / program.len() as f64) * 100.0)
                            } else { "N/A".to_string() }
                        }).to_string(),
                        success: true,
                    }
                }
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("compile error: {}", e)}).to_string(),
                    success: false,
                },
            }
        }
        Err(e) => ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "error": format!("parse error: {}", e)})
                .to_string(),
            success: false,
        },
    }
}

// ── CCS VM introspection tools ────────────────────────────────────────────

fn vm_status_impl(_args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let cid = tool_call_id.to_string();
    let vm = ctx
        .ccs_vm
        .lock()
        .unwrap_or_else(|e| panic!("ccs vm lock error: {}", e));

    let node_count = vm.world_graph.node_count();
    let edge_count = vm.world_graph.edge_count();
    let trace_len = vm.memory_trace.len();
    let steps = vm.steps_executed();
    let uptime = vm.uptime();
    let program_id = vm.program().map(|p| p.id.to_string());
    let region_names = vm.region_names();

    ToolResult {
        tool_call_id: cid,
        content: serde_json::json!({
            "success": true,
            "vm_status": {
                "graph_nodes": node_count,
                "graph_edges": edge_count,
                "memory_trace_length": trace_len,
                "steps_executed": steps,
                "uptime_secs": uptime.as_secs_f32(),
                "program_id": program_id,
                "regions": region_names.iter().map(|(n, off, len)| {
                    serde_json::json!({"name": n, "offset": off, "length": len})
                }).collect::<Vec<_>>()
            }
        })
        .to_string(),
        success: true,
    }
}

fn vm_query_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let query_type = args["query_type"].as_str().unwrap_or("nodes");
    let value = args["value"].as_str().unwrap_or("");
    let threshold = args["threshold"].as_f64().unwrap_or(0.5) as f32;
    let max_hops = args["max_hops"].as_u64().unwrap_or(1) as usize;
    let cid = tool_call_id.to_string();

    let vm = ctx
        .ccs_vm
        .lock()
        .unwrap_or_else(|e| panic!("ccs vm lock error: {}", e));

    match query_type {
        "label" => {
            match vm.world_graph.lookup_label(value) {
                Ok(Some(nid)) => {
                    let node = vm.world_graph.lookup(nid).ok().flatten();
                    let concept_preview = node.as_ref().map(|n| n.concept.data.iter().take(8).map(|v| format!("{:.3}", v)).collect::<Vec<_>>());
                    let access_count = node.as_ref().map(|n| n.metadata.access_count);
                    let edge_count = node.as_ref().map(|n| n.edges.len());
                    ToolResult {
                        tool_call_id: cid,
                        content: serde_json::json!({
                            "success": true, "query_type": "label",
                            "node_id": nid.as_u64(),
                            "label": node.as_ref().and_then(|n| n.label.clone()),
                            "concept_preview": concept_preview,
                            "access_count": access_count,
                            "edge_count": edge_count,
                        }).to_string(),
                        success: true,
                    }
                }
                Ok(None) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": true, "query_type": "label", "found": false, "message": format!("label '{}' not found", value)}).to_string(),
                    success: true,
                },
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("{}", e)}).to_string(),
                    success: false,
                },
            }
        }
        "neighbors" => {
            if let Ok(src_id) = value.parse::<u64>() {
                let src_nid = a2x_core::node::NodeId::new(src_id);
                let query = a2x_core::graph::GraphQuery::Neighbors { node: src_nid, max_hops };
                match vm.world_graph.query(&query) {
                    Ok(results) => {
                        let neighbor_info: Vec<Value> = results.iter().map(|nid| {
                            let node = vm.world_graph.lookup(*nid).ok().flatten();
                            serde_json::json!({
                                "id": nid.as_u64(),
                                "label": node.as_ref().and_then(|n| n.label.clone()),
                            })
                        }).collect();
                        ToolResult {
                            tool_call_id: cid,
                            content: serde_json::json!({
                                "success": true, "query_type": "neighbors",
                                "source": src_id, "max_hops": max_hops,
                                "count": neighbor_info.len(), "neighbors": neighbor_info
                            }).to_string(),
                            success: true,
                        }
                    }
                    Err(e) => ToolResult {
                        tool_call_id: cid,
                        content: serde_json::json!({"success": false, "error": format!("{}", e)}).to_string(),
                        success: false,
                    },
                }
            } else {
                ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("invalid node id: '{}'", value)}).to_string(),
                    success: false,
                }
            }
        }
        "similarity" => {
            // Parse value as comma-separated f32 vector
            let concept_vec: Vec<f32> = value
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if concept_vec.is_empty() {
                return ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": "value must be comma-separated floats like '1.0,0.0,0.5'"}).to_string(),
                    success: false,
                };
            }
            let concept = a2x_core::concept::ConceptVector::from_vec(concept_vec);
            let query = a2x_core::graph::GraphQuery::BySimilarity { concept, threshold };
            match vm.world_graph.query(&query) {
                Ok(results) => {
                    let similar: Vec<Value> = results.iter().map(|nid| {
                        let node = vm.world_graph.lookup(*nid).ok().flatten();
                        serde_json::json!({
                            "id": nid.as_u64(),
                            "label": node.as_ref().and_then(|n| n.label.clone()),
                        })
                    }).collect();
                    ToolResult {
                        tool_call_id: cid,
                        content: serde_json::json!({
                            "success": true, "query_type": "similarity",
                            "threshold": threshold, "count": similar.len(),
                            "matches": similar
                        }).to_string(),
                        success: true,
                    }
                }
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("{}", e)}).to_string(),
                    success: false,
                },
            }
        }
        _ => {
            // Fallback: treat as custom query
            let query = a2x_core::graph::GraphQuery::Custom(value.as_bytes().to_vec());
            match vm.world_graph.query(&query) {
                Ok(results) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({
                        "success": true, "query_type": query_type,
                        "count": results.len(),
                        "node_ids": results.iter().map(|n| n.as_u64()).collect::<Vec<_>>()
                    }).to_string(),
                    success: true,
                },
                Err(e) => ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": format!("{}", e)}).to_string(),
                    success: false,
                },
            }
        }
    }
}

fn vm_region_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let region = args["region"].as_str().unwrap_or("belief");
    let max_values = args["max_values"].as_u64().unwrap_or(16) as usize;
    let cid = tool_call_id.to_string();

    let vm = ctx
        .ccs_vm
        .lock()
        .unwrap_or_else(|e| panic!("ccs vm lock error: {}", e));

    match vm.probe_region(region) {
        Some(snapshot) => {
            if let a2x_ccs::ProbeSnapshot::Region { name, offset, len, data } = snapshot {
                let preview: Vec<String> = data.iter().take(max_values).map(|v| format!("{:.4}", v)).collect();
                let stats = if data.len() >= 4 {
                    let mean = data.iter().sum::<f32>() / data.len() as f32;
                    let min = data.iter().fold(f32::MAX, |a, &b| a.min(b));
                    let max = data.iter().fold(f32::MIN, |a, &b| a.max(b));
                    Some(serde_json::json!({"mean": format!("{:.4}", mean), "min": format!("{:.4}", min), "max": format!("{:.4}", max)}))
                } else { None };
                ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({
                        "success": true, "region": name, "offset": offset,
                        "total_length": len, "preview": preview, "stats": stats
                    }).to_string(),
                    success: true,
                }
            } else {
                ToolResult {
                    tool_call_id: cid,
                    content: serde_json::json!({"success": false, "error": "unexpected snapshot type"}).to_string(),
                    success: false,
                }
            }
        }
        None => ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "error": format!("region '{}' not found", region)}).to_string(),
            success: false,
        },
    }
}

fn vm_trace_impl(args: &Value, tool_call_id: &str, ctx: &mut ToolContext) -> ToolResult {
    let tail = args["tail"].as_u64().unwrap_or(10) as usize;
    let cid = tool_call_id.to_string();

    let vm = ctx
        .ccs_vm
        .lock()
        .unwrap_or_else(|e| panic!("ccs vm lock error: {}", e));

    let snapshot = vm.probe_trace_tail(tail);
    if let a2x_ccs::ProbeSnapshot::TraceSegment { entries } = snapshot {
        let entries_json: Vec<Value> = entries.iter().map(|e| {
            serde_json::json!({
                "ip": e.ip,
                "timestamp": e.timestamp,
                "state_preview": e.state_preview.iter().take(4).map(|v| format!("{:.4}", v)).collect::<Vec<_>>(),
            })
        }).collect();
        ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({
                "success": true,
                "trace_length": entries.len(),
                "entries": entries_json
            })
            .to_string(),
            success: true,
        }
    } else {
        ToolResult {
            tool_call_id: cid,
            content: serde_json::json!({"success": false, "error": "unexpected snapshot type"})
                .to_string(),
            success: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tool_defs_count() {
        let tools = all_tool_defs();
        assert!(
            tools.len() >= 8,
            "expected at least 8 tools, got {}",
            tools.len()
        );
    }

    #[test]
    fn test_tool_defs_have_names() {
        for tool in all_tool_defs() {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
        }
    }

    #[test]
    fn test_execute_sigma_valid() {
        let bus = Arc::new(Mutex::new(Bus::new()));
        let cli = Arc::new(CliAgent::new(AgentId::new("tool-cli")));
        let vm = Arc::new(Mutex::new(CcsVm::new()));
        let mut ctx = ToolContext::new(bus, cli, vm);
        let result = execute_sigma_impl(
            &serde_json::json!({"program": "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭"}),
            "call_1",
            &mut ctx,
        );
        assert!(result.success);
        assert_eq!(result.tool_call_id, "call_1");
    }

    #[test]
    fn test_run_ccs_reflect() {
        let bus = Arc::new(Mutex::new(Bus::new()));
        let cli = Arc::new(CliAgent::new(AgentId::new("tool-cli")));
        let vm = Arc::new(Mutex::new(CcsVm::new()));
        let mut ctx = ToolContext::new(bus, cli, vm);
        let result = run_ccs_program_impl(
            &serde_json::json!({"operation": "reflect"}),
            "call_2",
            &mut ctx,
        );
        assert!(result.success);
    }

    #[test]
    fn test_unknown_tool() {
        let bus = Arc::new(Mutex::new(Bus::new()));
        let cli = Arc::new(CliAgent::new(AgentId::new("tool-cli")));
        let vm = Arc::new(Mutex::new(CcsVm::new()));
        let mut ctx = ToolContext::new(bus, cli, vm);
        let result = execute_tool("nonexistent", &serde_json::json!({}), "call_x", &mut ctx);
        assert!(!result.success);
        assert_eq!(result.tool_call_id, "call_x");
    }
}
