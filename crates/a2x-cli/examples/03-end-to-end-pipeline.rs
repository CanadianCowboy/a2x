// examples/03-end-to-end-pipeline.rs
// End-to-end demo: HTTP → Gateway → Bus → Agent → result
//
// Demonstrates:
//   - Creating a Gateway with config and auth
//   - Registering built-in agents on the bus
//   - Registering external entities with permissions
//   - Executing programs through the gateway with permission enforcement
//   - Rate limiting enforcement
//   - Probe permission checks
//   - Agent discovery via the bus
//
// Run: cargo run --example 03-end-to-end-pipeline

use std::sync::Arc;

use a2x_bus::AgentFilter;
use a2x_core::capability::Capability;
use a2x_gateway::entity::EntityInfo;
use a2x_gateway::entity::{EntityId, EntityType, SimpleEntity};
use a2x_gateway::listeners::http::HttpGatewayState;
use a2x_gateway::listeners::http::HttpListener;
use a2x_gateway::Gateway;

fn main() {
    println!("=== A2X End-to-End Pipeline Demo ===\n");

    // ── Step 1: Create the Gateway ────────────────────────────────────────
    println!("Step 1 — Creating the Gateway:");
    let gw = Gateway::new();
    println!("  ✓ Gateway created");

    // ── Step 2: Register built-in agents ──────────────────────────────────
    println!("\nStep 2 — Registering built-in agents:");
    match gw.register_builtin_agents() {
        Ok(()) => {
            let state = gw.state.lock().unwrap();
            let bus = state.bus.lock().unwrap();
            println!("  ✓ {} agents registered on the bus", bus.agent_count());

            // Discover agents by capability
            let exec_agents = bus.discover(&AgentFilter::ByCapability(Capability::Execute));
            println!("  Execute-capable agents:");
            for info in &exec_agents {
                println!(
                    "    - {} ({:?}) [{}]",
                    info.id.as_str(),
                    info.agent_type,
                    if info.online { "online" } else { "offline" }
                );
            }
        }
        Err(e) => println!("  ✗ Failed to register agents: {}", e),
    }

    // ── Step 3: Register an external entity ──────────────────────────────
    println!("\nStep 3 — Registering external entities:");
    {
        let mut state = gw.state.lock().unwrap();
        let entity_id = EntityId::new("app-1");

        // Create entity info (metadata)
        let info = EntityInfo::new(
            entity_id.clone(),
            EntityType::Application,
            "Demo Application",
            vec![
                Capability::Execute,
                Capability::FileSystem,
                Capability::Network,
            ],
        );

        let entity = SimpleEntity::new(info);
        state.register_entity(Box::new(entity));
        println!("  ✓ Entity 'app-1' registered");
        println!("  Total entities: {}", state.list_entities().len());
    }

    // ── Step 4: Execute a program through the gateway ─────────────────────
    println!("\nStep 4 — Executing a program through the gateway:");
    {
        let state = gw.state.lock().unwrap();
        let mut program = a2x_sigma::program::SigmaProgram::new();
        program.push(a2x_sigma::SigmaPacket::default()); // NOP
        program.push(a2x_sigma::SigmaPacket::default()); // NOP
        program.compute_id();

        println!("  Program ID: {}", program.id);
        println!("  Instructions: {}", program.instructions.len());

        match state.execute_program(&program) {
            Ok(result) => {
                println!("  ✓ Program executed successfully");
                println!("  Result instructions: {}", result.instructions.len());
            }
            Err(e) => println!("  ✗ Execution error: {}", e),
        }
    }

    // ── Step 5: Permission enforcement ────────────────────────────────────
    println!("\nStep 5 — Permission enforcement:");
    {
        let mut state = gw.state.lock().unwrap();
        let entity_id = EntityId::new("app-1");

        // Create a program with permission enforcement
        let mut program = a2x_sigma::program::SigmaProgram::new();
        program.push(a2x_sigma::SigmaPacket::default());
        program.compute_id();

        match state.execute_program_for_entity(&program, &entity_id) {
            Ok(result) => {
                println!("  ✓ Program executed with permission check");
                println!("  Result: {} instruction(s)", result.instructions.len());
            }
            Err(e) => println!("  ✗ Permission error: {}", e),
        }
    }

    // ── Step 6: Probe permission check ────────────────────────────────────
    println!("\nStep 6 — Probe permission check:");
    {
        let state = gw.state.lock().unwrap();
        let entity_id = EntityId::new("app-1");

        match state.check_probe_permission(&entity_id) {
            Ok(()) => println!("  ✓ Entity 'app-1' is authorized to probe"),
            Err(e) => println!("  ✗ Probe denied: {}", e),
        }

        // Unknown entity — no permissions configured
        let unknown = EntityId::new("unknown");
        match state.check_probe_permission(&unknown) {
            Ok(()) => {
                println!("  ✓ Entity 'unknown' passes probe check (no permissions = allow)")
            }
            Err(e) => println!("  ✗ Probe denied: {}", e),
        }
    }

    // ── Step 7: Agent state probe ─────────────────────────────────────────
    println!("\nStep 7 — Agent state probe:");
    {
        let state = gw.state.lock().unwrap();

        // Probe the orchestrator agent
        match state.probe_agent("orch-1") {
            Ok(snapshot) => {
                println!("  ✓ Agent '{}' probed:", snapshot.agent_id.as_str());
                println!("    State: {}", snapshot.state);
                println!(
                    "    IP: {}",
                    snapshot
                        .ip
                        .map(|v: usize| v.to_string())
                        .unwrap_or_else(|| "N/A".into())
                );
                println!("    WorldGraph nodes: {}", snapshot.world_graph_size);
                println!("    MemoryTrace length: {}", snapshot.memory_trace_length);
                println!("    Uptime: {:.2}s", snapshot.uptime.as_secs_f32());
            }
            Err(e) => println!("  ✗ Probe error: {}", e),
        }
    }

    // ── Step 8: Gateway lifecycle (start/stop) ────────────────────────────
    println!("\nStep 8 — Gateway lifecycle:");
    {
        // Add an HTTP listener
        {
            let http_state = Arc::new(HttpGatewayState {
                gateway: gw.state_arc(),
            });
            let http_listener = HttpListener::new("127.0.0.1:8080", http_state);

            let mut state = gw.state.lock().unwrap();
            state.add_listener(Box::new(http_listener));
            println!("  ✓ HTTP listener added on 127.0.0.1:8080");
        }

        // Start gateway
        match gw.start() {
            Ok(()) => println!("  ✓ Gateway started"),
            Err(e) => println!("  ✗ Start error: {}", e),
        }

        // Stop gateway
        match gw.stop() {
            Ok(()) => println!("  ✓ Gateway stopped"),
            Err(e) => println!("  ✗ Stop error: {}", e),
        }
    }

    // ── Step 9: Entity management ─────────────────────────────────────────
    println!("\nStep 9 — Entity listing:");
    {
        let state = gw.state.lock().unwrap();
        let entities = state.list_entities();
        println!("  Registered entities: {}", entities.len());
        for entity in &entities {
            println!(
                "    - {} ({:?}): \"{}\"",
                entity.id, entity.entity_type, entity.display_name,
            );
            println!("      Capabilities:");
            for cap in &entity.capabilities {
                println!("        • {:?}", cap);
            }
        }
    }

    println!("\n=== End-to-end pipeline demo complete ===");
}
