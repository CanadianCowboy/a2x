// examples/03-gateway-entity.rs
// Entity Gateway integration demo — authentication, permission enforcement,
// entity registration, and program execution through the gateway.
//
// Demonstrates:
//   - Gateway creation and configuration
//   - Entity registration with typed metadata
//   - API key authentication
//   - Permission enforcement (max_instructions, rate_limit, can_probe)
//   - Program execution through the gateway
//   - Agent probing through the gateway
//
// Run: cargo run --example 03-gateway-entity

use a2x_bus::AgentFilter;
use a2x_core::agent_id::AgentType;
use a2x_core::capability::Capability;
use a2x_gateway::auth::{AuthMethod, EntityPermissions, InMemoryAuthProvider};
use a2x_gateway::entity::{EntityId, EntityInfo, EntityType, SimpleEntity};
use a2x_gateway::gateway::Gateway;
use a2x_gateway::listeners::http::HttpListener;
use a2x_gateway::listeners::stdio::StdioListener;
use a2x_sigma::program::SigmaProgram;

fn main() {
    println!("=== A2X Entity Gateway Demo ===\n");

    // ── Step 1: Create the gateway ────────────────────────────────────────
    println!("Step 1 — Creating the gateway:");
    let gateway = Gateway::new();
    let state_arc = gateway.state_arc();
    println!("  ✓ Gateway created");

    // ── Step 2: Configure authentication (API keys) ────────────────────────
    println!("\nStep 2 — Setting up authentication:");
    {
        let mut state = state_arc.lock().unwrap();

        // Create an auth provider with API keys
        let mut auth = InMemoryAuthProvider::new();
        let app1_id = EntityId::new("app-1");
        let app2_id = EntityId::new("app-2");
        let admin_id = EntityId::new("admin-1");

        auth.register_key("sk-app-1-abc123".into(), app1_id.clone());
        auth.register_key("sk-app-2-def456".into(), app2_id.clone());
        auth.register_key("sk-admin-ghi789".into(), admin_id.clone());

        // ── Step 3: Set entity permissions ─────────────────────────────────
        println!("\nStep 3 — Setting entity permissions:");

        // Application 1: Limited — 500 instructions, no probe, rate limited to 60/min
        let perms1 = EntityPermissions {
            entity_id: app1_id.clone(),
            max_instructions: 500,
            can_probe: false,
            can_network: false,
            rate_limit: 60,
        };
        auth.set_permissions(perms1);
        println!("  ✓ app-1: max_instructions=500, can_probe=false, rate_limit=60/min");

        // Application 2: More permissive — 5000 instructions, can probe, no rate limit
        let perms2 = EntityPermissions {
            entity_id: app2_id.clone(),
            max_instructions: 5000,
            can_probe: true,
            can_network: true,
            rate_limit: 0, // unlimited
        };
        auth.set_permissions(perms2);
        println!("  ✓ app-2: max_instructions=5000, can_probe=true, no rate limit");

        // Admin: Full access
        let perms_admin = EntityPermissions {
            entity_id: admin_id.clone(),
            max_instructions: u64::MAX,
            can_probe: true,
            can_network: true,
            rate_limit: 0,
        };
        auth.set_permissions(perms_admin);
        println!("  ✓ admin-1: full access");

        state.auth = Box::new(auth);
    }

    // ── Step 4: Authenticate entities ──────────────────────────────────────
    println!("\nStep 4 — Authenticating entities:");
    {
        let state = state_arc.lock().unwrap();

        // Valid API key
        match state.authenticate(&AuthMethod::ApiKey("sk-app-1-abc123".into())) {
            Ok(eid) => println!("  ✓ Authenticated as: {}", eid),
            Err(e) => println!("  ✗ Auth failed: {}", e),
        }

        // Invalid API key
        match state.authenticate(&AuthMethod::ApiKey("bad-key".into())) {
            Ok(eid) => println!("  ? Unexpected success: {}", eid),
            Err(e) => println!("  ✓ Correctly rejected bad key: {}", e),
        }

        // Local connection (no auth)
        match state.authenticate(&AuthMethod::Local) {
            Ok(eid) => println!("  ✓ Local auth as: {}", eid),
            Err(e) => println!("  ✗ Local auth failed: {}", e),
        }
    }

    // ── Step 5: Permission enforcement (BUG-005 fix verification) ──────────
    println!("\nStep 5 — Permission enforcement (BUG-005):");
    {
        let mut state = state_arc.lock().unwrap();

        // Check probe permission for app-1 (should be denied)
        let eid = EntityId::new("app-1");
        match state.check_probe_permission(&eid) {
            Ok(()) => println!("  ? Unexpected: app-1 can probe"),
            Err(e) => println!("  ✓ app-1 denied probe access: {}", e),
        }

        // Check probe permission for app-2 (should be allowed)
        let eid = EntityId::new("app-2");
        match state.check_probe_permission(&eid) {
            Ok(()) => println!("  ✓ app-2 allowed to probe"),
            Err(e) => println!("  ✗ Unexpected denial for app-2: {}", e),
        }

        // Check rate limiting for app-1 (60 requests per window)
        let eid = EntityId::new("app-1");
        let perms = state
            .auth
            .permissions(&eid)
            .expect("app-1 should have permissions");

        // Make 60 requests — should all succeed
        let mut succeeded = 0;
        let program = SigmaProgram::new();
        for _ in 0..70 {
            match state.enforce_permissions(&perms, &program) {
                Ok(()) => succeeded += 1,
                Err(e) => {
                    println!("  ✓ Rate limited after {} requests: {}", succeeded, e);
                    break;
                }
            }
        }
        assert!(succeeded == 60, "expected 60 successes before rate limit");
    }

    // ── Step 6: Register entities (BUG-001 fix verification) ──────────────
    println!("\nStep 6 — Registering entities (BUG-001):");
    {
        let mut state = state_arc.lock().unwrap();

        let entity1 = SimpleEntity::new(EntityInfo::new(
            EntityId::new("my-app"),
            EntityType::Application,
            "My External App",
            vec![Capability::Execute],
        ));
        state.register_entity(Box::new(entity1));
        println!("  ✓ Registered entity: my-app");

        let entity2 = SimpleEntity::new(EntityInfo::new(
            EntityId::new("user-josh"),
            EntityType::HumanCli,
            "Josh (CLI)",
            vec![
                Capability::Execute,
                Capability::FileSystem,
                Capability::Shell,
            ],
        ));
        state.register_entity(Box::new(entity2));
        println!("  ✓ Registered entity: user-josh");

        // List all entities
        let entities = state.list_entities();
        println!("  Total entities: {}", entities.len());
        for entity in &entities {
            println!(
                "    - {} ({:?}): {}",
                entity.id.as_str(),
                entity.entity_type,
                entity.display_name,
            );
        }
    }

    // ── Step 7: Execute a program through the gateway ─────────────────────
    println!("\nStep 7 — Executing a program through the gateway:");
    {
        let state = state_arc.lock().unwrap();
        let program = SigmaProgram::new();
        match state.execute_program(&program) {
            Ok(_) => println!("  ✓ Empty program executed successfully"),
            Err(e) => println!("  ✗ Error: {}", e),
        }
    }

    // ── Step 8: Add listeners (BUG-001 fix verification) ──────────────────
    println!("\nStep 8 — Adding protocol listeners (BUG-001):");
    {
        let mut state = state_arc.lock().unwrap();
        println!("  Listeners before: {}", state.listeners.len());

        // Add stdio listener
        let stdio = StdioListener::new("stdio-entity");
        state.add_listener(Box::new(stdio));
        println!("  ✓ Added stdio listener");

        // Add HTTP listener
        let http_state = std::sync::Arc::new(a2x_gateway::listeners::http::HttpGatewayState {
            gateway: state_arc.clone(),
        });
        let http = HttpListener::new("127.0.0.1:8778", http_state);
        state.add_listener(Box::new(http));
        println!("  ✓ Added HTTP listener");

        println!("  Listeners after: {}", state.listeners.len());
    }

    // ── Step 9: Register built-in agents ──────────────────────────────────
    println!("\nStep 9 — Registering built-in agents on the bus:");
    gateway.register_builtin_agents().unwrap();
    {
        let state = state_arc.lock().unwrap();
        println!(
            "  ✓ {} agents on bus",
            state.bus.lock().unwrap().agent_count()
        );

        // Discover agents via bus
        let orch = state
            .bus
            .lock()
            .unwrap()
            .discover(&AgentFilter::ByType(AgentType::Orchestrator));
        println!("  Orchestrator agents: {}", orch.len());

        let cli = state
            .bus
            .lock()
            .unwrap()
            .discover(&AgentFilter::ByType(AgentType::Cli));
        println!("  CLI agents: {}", cli.len());
    }

    println!("\n=== Gateway entity demo complete ===");
}
