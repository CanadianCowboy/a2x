// examples/02-multi-agent.rs
// Multi-agent communication demo using the A2X bus.
//
// Demonstrates:
//   - Creating agents (Orchestrator, CLI, CCS)
//   - Registering agents on the bus with AgentCard metadata
//   - Agent discovery by capability and type
//   - Program dispatch from Orchestrator to CLI agent
//   - Result collection
//
// Run: cargo run --example 02-multi-agent

use a2x_agents::{CcsAgent, CliAgent, Orchestrator};
use a2x_bus::{AgentCard, AgentFilter, Bus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_sigma::program::SigmaProgram;

fn main() {
    println!("=== A2X Multi-Agent Demo ===\n");

    // ── Step 1: Create the bus with discovery ─────────────────────────────
    println!("Step 1 — Starting the A2X bus:");
    let mut bus = Bus::new();
    println!("  ✓ Bus created");

    // ── Step 2: Create agent cards (A2A pattern) ───────────────────────────
    println!("\nStep 2 — Creating agent cards:");

    let orch_card = AgentCard::new(
        AgentId::new("orch-1"),
        "Primary Orchestrator",
        "0.6.0",
        AgentType::Orchestrator,
        vec![Capability::Execute, Capability::Custom("schedule".into())],
        vec!["a2x://localhost:8777".into()],
        vec!["api_key".into()],
        vec!["sigma".into(), "omega".into()],
        "Top-level planner and program dispatcher",
    );
    println!(
        "  ✓ Orchestrator card: {} v{}",
        orch_card.name, orch_card.version
    );

    let cli_card = AgentCard::new(
        AgentId::new("cli-1"),
        "CLI Executor",
        "0.6.0",
        AgentType::Cli,
        vec![
            Capability::Execute,
            Capability::FileSystem,
            Capability::Network,
            Capability::Shell,
        ],
        vec!["a2x://localhost:8778".into()],
        vec!["api_key".into()],
        vec!["sigma".into()],
        "Command-line agent for system operations",
    );
    println!(
        "  ✓ CLI agent card: {} v{}",
        cli_card.name, cli_card.version
    );

    let ccs_card = AgentCard::new(
        AgentId::new("ccs-1"),
        "Cognitive Substrate Agent",
        "0.6.0",
        AgentType::Ccs,
        vec![
            Capability::Execute,
            Capability::Custom("plan".into()),
            Capability::Custom("cognitive".into()),
        ],
        vec!["a2x://localhost:8779".into()],
        vec!["api_key".into()],
        vec!["omega".into()],
        "Long-running cognitive agent with persistent WorldGraph",
    );
    println!(
        "  ✓ CCS agent card: {} v{}",
        ccs_card.name, ccs_card.version
    );

    // ── Step 3: Register agents on the bus ────────────────────────────────
    println!("\nStep 3 — Registering agents on the bus:");
    let _ = bus.register_agent(orch_card.to_agent_info());
    let _ = bus.register_agent(cli_card.to_agent_info());
    let _ = bus.register_agent(ccs_card.to_agent_info());
    println!("  ✓ {} agents registered", bus.agent_count());

    // ── Step 4: Agent discovery ───────────────────────────────────────────
    println!("\nStep 4 — Agent discovery:");

    // Discover by type
    let cli_agents = bus.discover(&AgentFilter::ByType(AgentType::Cli));
    println!(
        "  CLI agents found: {} ({})",
        cli_agents.len(),
        cli_agents
            .iter()
            .map(|a| a.id.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Discover by capability
    let exec_agents = bus.discover(&AgentFilter::ByCapability(Capability::Execute));
    println!(
        "  Execute-capable agents found: {} ({})",
        exec_agents.len(),
        exec_agents
            .iter()
            .map(|a| a.id.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Discover all
    let all = bus.discover(&AgentFilter::All);
    println!("  All agents: {}", all.len());

    // ── Step 5: Create agents and test basic operations ───────────────────
    println!("\nStep 5 — Creating agent instances:");

    let orchestrator = Orchestrator::new(AgentId::new("orch-1"));
    let cli_agent = CliAgent::new(AgentId::new("cli-1"));
    let ccs_agent = CcsAgent::new(AgentId::new("ccs-1"));

    println!("  ✓ Orchestrator: {}", orchestrator.id().as_str());
    println!("  ✓ CLI agent: {}", cli_agent.id().as_str());
    println!("  ✓ CCS agent: {}", ccs_agent.id().as_str());

    // Show capabilities
    println!("\n  Orchestrator capabilities:");
    for cap in orchestrator.capabilities() {
        println!("    - {:?}", cap);
    }
    println!("\n  CLI agent capabilities:");
    for cap in cli_agent.capabilities() {
        println!("    - {:?}", cap);
    }

    // ── Step 6: Dispatch a program ────────────────────────────────────────
    println!("\nStep 6 — Dispatching a program from Orchestrator:");

    let mut program = SigmaProgram::new();
    let pkt = a2x_sigma::SigmaPacket::default();
    program.push(pkt);

    match orchestrator.dispatch(program.clone()) {
        Ok(result) => {
            println!("  ✓ Program dispatched successfully");
            println!("  Result instructions: {}", result.instructions.len());
        }
        Err(e) => println!("  ✗ Dispatch error: {}", e),
    }

    // ── Step 7: State summaries ───────────────────────────────────────────
    println!("\nStep 7 — Agent state summaries:");

    for agent in [&orchestrator as &dyn Agent, &cli_agent as &dyn Agent] {
        if let Some(snapshot) = agent.state_summary() {
            println!(
                "  {}: state={}, ip={:?}, graph_nodes={}, trace_len={}",
                snapshot.agent_id.as_str(),
                snapshot.state,
                snapshot.ip,
                snapshot.world_graph_size,
                snapshot.memory_trace_length,
            );
        }
    }

    println!("\n=== Multi-agent demo complete ===");
}
