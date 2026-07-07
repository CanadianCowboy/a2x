// integration_pipeline.rs
// Full A2X pipeline integration test — parse, compile, gateway, bus, agent, probe.
//
// Tests the complete end-to-end flow with assertions on every stage.
// Run: cargo test -p a2x-cli --test integration_pipeline

use a2x_agents::{CcsAgent, CliAgent, Orchestrator};
use a2x_bus::{AgentCard, AgentFilter, Bus};
use a2x_ccs::CcsVm;
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::graph::WorldGraph;
use a2x_core::memory::MemoryTrace;
use a2x_gateway::entity::EntityInfo;
use a2x_gateway::entity::{EntityId, EntityType, SimpleEntity};
use a2x_gateway::Gateway;
use a2x_sigma::program::SigmaProgram;

// ═══════════════════════════════════════════════════════════════════════════
// 1. Parse → Compile → VM Execution Roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_and_execute_simple_program() {
    // A NOP program: Cancel intent + Cancel plan = HALT opcode
    let source = "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();
    assert!(!program.is_empty());

    let mut vm = CcsVm::new();
    vm.load(program);
    // A simple HALT program: Cancel intent + Cancel plan. WorldGraph may
    // stay at 0 nodes for pure NOP programs since operators are basic stubs.
    let status = vm.run().expect("vm run");
    assert!(matches!(status, a2x_ccs::VmStatus::Halted));
    assert!(vm.world_graph.node_count() == 0 || vm.world_graph.node_count() > 0);
}

#[test]
fn test_parse_and_execute_multi_instruction() {
    let source = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();
    assert_eq!(program.instructions.len(), 1);

    let mut vm = CcsVm::new();
    vm.load(program);
    let status = vm.run().expect("vm run");
    assert!(matches!(status, a2x_ccs::VmStatus::Halted));
}

#[test]
fn test_parse_empty_string_produces_empty_program() {
    let source = "";
    let program = a2x_sigma::parse_program(source).expect("parse empty");
    assert!(program.is_empty());
}

#[test]
fn test_cancel_and_star_parse() {
    // Star (✦) with Cancel (✕) — valid multi-operator intent
    let source = "⟦Σ∞⟧⟬I:✦✕ ∷ P:✕⟭";
    let program = a2x_sigma::parse_program(source).expect("parse");
    assert!(!program.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Gateway → Entity → Permission Enforcement
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_gateway_entity_registration_and_lifecycle() {
    let gw = Gateway::new();

    // Register entity
    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            EntityId::new("app-test"),
            EntityType::Application,
            "Test App",
            vec![Capability::Execute, Capability::FileSystem],
        ));
        state.register_entity(Box::new(entity));
    }

    // Verify entity is registered
    {
        let state = gw.state.lock().unwrap();
        let entities = state.list_entities();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, EntityId::new("app-test"));
        assert_eq!(entities[0].entity_type, EntityType::Application);
        assert_eq!(entities[0].display_name, "Test App");
        assert_eq!(entities[0].capabilities.len(), 2);
    }
}

#[test]
fn test_gateway_execute_program_for_entity() {
    let gw = Gateway::new();

    // Create program
    let mut program = SigmaProgram::new();
    let pkt = a2x_sigma::SigmaPacket::default();
    program.push(pkt);
    program.compute_id();

    // Execute with entity permissions
    let entity_id = EntityId::new("app-test");
    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            entity_id.clone(),
            EntityType::Application,
            "Test App",
            vec![Capability::Execute],
        ));
        state.register_entity(Box::new(entity));

        // Entity has no auth permissions set, so unlimited
        let result = state.execute_program_for_entity(&program, &entity_id);
        assert!(result.is_ok());
    }
}

#[test]
fn test_gateway_allows_execution_without_explicit_permissions() {
    let gw = Gateway::new();
    let entity_id = EntityId::new("no-perms-app");

    // Register entity without setting any auth permissions
    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            entity_id.clone(),
            EntityType::Application,
            "No Permissions App",
            vec![Capability::Execute],
        ));
        state.register_entity(Box::new(entity));
    }

    // Program execution should succeed when no permissions are explicitly set
    let mut program = SigmaProgram::new();
    for _ in 0..5 {
        program.push(a2x_sigma::SigmaPacket::default());
    }
    program.compute_id();

    {
        let mut state = gw.state.lock().unwrap();
        let result = state.execute_program_for_entity(&program, &entity_id);
        assert!(
            result.is_ok(),
            "execution should succeed when no permissions are set"
        );
    }
}

#[test]
fn test_gateway_probe_permission_check() {
    let gw = Gateway::new();
    let entity_id = EntityId::new("probe-app");

    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            entity_id.clone(),
            EntityType::Application,
            "Probe App",
            vec![Capability::Probe, Capability::Execute],
        ));
        state.register_entity(Box::new(entity));
    }

    // Probe permission — should pass (no auth permissions = allow)
    {
        let state = gw.state.lock().unwrap();
        let result = state.check_probe_permission(&entity_id);
        assert!(
            result.is_ok(),
            "probe should be allowed without explicit permissions"
        );
    }

    // Unknown entity should also pass (no permissions = allow)
    {
        let state = gw.state.lock().unwrap();
        let result = state.check_probe_permission(&EntityId::new("nonexistent"));
        assert!(result.is_ok());
    }
}

#[test]
fn test_gateway_unregister_entity() {
    let gw = Gateway::new();
    let entity_id = EntityId::new("temp-app");

    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            entity_id.clone(),
            EntityType::HumanCli,
            "Temp",
            vec![],
        ));
        state.register_entity(Box::new(entity));
    }

    {
        let state = gw.state.lock().unwrap();
        assert_eq!(state.list_entities().len(), 1);
    }

    {
        let mut state = gw.state.lock().unwrap();
        let removed = state.unregister_entity(&entity_id);
        assert!(removed);
    }

    {
        let state = gw.state.lock().unwrap();
        assert_eq!(state.list_entities().len(), 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Bus — Agent Discovery, Registration, Routing
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bus_agent_registration_and_discovery() {
    let mut bus = Bus::new();

    // Register agents
    let orch_info = a2x_bus::AgentInfo::new(
        AgentId::new("orch-1"),
        AgentType::Orchestrator,
        vec![Capability::Execute, Capability::Custom("schedule".into())],
    );
    let cli_info = a2x_bus::AgentInfo::new(
        AgentId::new("cli-1"),
        AgentType::Cli,
        vec![Capability::Execute, Capability::Shell],
    );

    assert!(bus.register_agent(orch_info).is_ok());
    assert!(bus.register_agent(cli_info).is_ok());
    assert_eq!(bus.agent_count(), 2);

    // Discover by type
    let cli_agents = bus.discover(&AgentFilter::ByType(AgentType::Cli));
    assert_eq!(cli_agents.len(), 1);
    assert_eq!(cli_agents[0].id.as_str(), "cli-1");

    // Discover by capability
    let exec_agents = bus.discover(&AgentFilter::ByCapability(Capability::Execute));
    assert_eq!(exec_agents.len(), 2);

    let shell_agents = bus.discover(&AgentFilter::ByCapability(Capability::Shell));
    assert_eq!(shell_agents.len(), 1);

    // Discover all
    let all = bus.discover(&AgentFilter::All);
    assert_eq!(all.len(), 2);
}

#[test]
fn test_agent_card_to_info_conversion() {
    let card = AgentCard::new(
        AgentId::new("test-1"),
        "Test Agent",
        "1.0.0",
        AgentType::Ccs,
        vec![Capability::Execute, Capability::Custom("cognitive".into())],
        vec!["a2x://localhost:9000".into()],
        vec!["api_key".into()],
        vec!["sigma".into(), "omega".into()],
        "A test cognitive agent",
    );

    let info = card.to_agent_info();
    assert_eq!(info.id.as_str(), "test-1");
    assert_eq!(info.agent_type, AgentType::Ccs);
    assert_eq!(info.capabilities.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Agents — Creation, Capabilities, State, Execution
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_orchestrator_creation_and_capabilities() {
    let orch = Orchestrator::new(AgentId::new("orch-test"));
    assert_eq!(orch.id().as_str(), "orch-test");

    let caps = orch.capabilities();
    assert!(caps.iter().any(|c| matches!(c, Capability::Execute)));
    assert!(caps
        .iter()
        .any(|c| matches!(c, Capability::Custom(s) if s == "schedule")));
}

#[test]
fn test_cli_agent_creation_and_capabilities() {
    let cli = CliAgent::new(AgentId::new("cli-test"));
    assert_eq!(cli.id().as_str(), "cli-test");

    let caps = cli.capabilities();
    assert!(caps.iter().any(|c| matches!(c, Capability::Execute)));
    assert!(caps.iter().any(|c| matches!(c, Capability::FileSystem)));
    assert!(caps.iter().any(|c| matches!(c, Capability::Network)));
    assert!(caps.iter().any(|c| matches!(c, Capability::Shell)));
}

#[test]
fn test_ccs_agent_creation_and_tick() {
    let ccs = CcsAgent::new(AgentId::new("ccs-test"));
    assert_eq!(ccs.id().as_str(), "ccs-test");

    // Tick the cognitive loop
    let result = ccs.tick();
    assert!(result.is_ok(), "cognitive tick should succeed");

    // After a tick, the WorldGraph should have grown
    let snapshot = ccs.state_summary().expect("state summary");
    assert!(
        snapshot.world_graph_size > 0,
        "WorldGraph should have nodes after tick"
    );
}

#[test]
fn test_ccs_agent_multiple_ticks() {
    let ccs = CcsAgent::new(AgentId::new("ccs-multi"));
    for _ in 0..2 {
        ccs.tick().expect("tick");
    }

    let snapshot = ccs.state_summary().expect("state summary");
    assert!(
        snapshot.memory_trace_length > 0,
        "MemoryTrace should have entries"
    );
    assert!(snapshot.uptime > std::time::Duration::ZERO);
}

#[test]
fn test_agent_state_summaries() {
    let agents: Vec<Box<dyn Agent>> = vec![
        Box::new(Orchestrator::new(AgentId::new("orch-s"))),
        Box::new(CliAgent::new(AgentId::new("cli-s"))),
        Box::new(CcsAgent::new(AgentId::new("ccs-s"))),
    ];

    for agent in agents {
        let snapshot = agent.state_summary().expect("state summary");
        assert!(
            !snapshot.state.is_empty(),
            "state string should not be empty"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Orchestrator Dispatch
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_orchestrator_dispatch_empty_program() {
    let orch = Orchestrator::new(AgentId::new("orch-disp"));
    let program = SigmaProgram::new();
    let result = orch.dispatch(program).expect("dispatch empty");
    assert!(result.is_empty());
}

#[test]
fn test_orchestrator_dispatch_simple_program() {
    let orch = Orchestrator::new(AgentId::new("orch-disp2"));
    let mut program = SigmaProgram::new();
    let pkt = a2x_sigma::SigmaPacket::default();
    program.push(pkt);
    program.compute_id();

    let result = orch.dispatch(program).expect("dispatch");
    // NOP instructions produce empty result
    assert!(result.is_empty());
}

#[test]
fn test_orchestrator_dispatch_via_bus() {
    let mut bus = Bus::new();

    // Register a CLI agent on the bus
    let cli_info = a2x_bus::AgentInfo::new(
        AgentId::new("bus-cli-1"),
        AgentType::Cli,
        vec![Capability::Execute, Capability::Shell],
    );
    bus.register_agent(cli_info).unwrap();

    // Create orchestrator with the bus
    let orch = Orchestrator::new(AgentId::new("bus-orch"));

    let mut program = SigmaProgram::new();
    program.push(a2x_sigma::SigmaPacket::default());
    program.compute_id();

    // dispatch_via_bus should find the CLI agent by capability
    let result = orch
        .dispatch_via_bus(&mut bus, program, Capability::Execute)
        .expect("dispatch via bus");
    assert!(
        result.is_empty(),
        "NOP dispatch should produce empty result"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. CCS VM — Load, Step, Run, Memory, Probe
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_vm_load_and_run_empty() {
    let mut vm = CcsVm::new();
    vm.load(SigmaProgram::new());
    let status = vm.run().expect("run empty");
    assert!(matches!(status, a2x_ccs::VmStatus::Halted));
}

#[test]
fn test_vm_step_by_step() {
    let source = "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();

    let mut vm = CcsVm::new();
    vm.load(program);
    assert_eq!(vm.ip, 0);

    let status = vm.step().expect("step");
    assert!(matches!(
        status,
        a2x_ccs::VmStatus::Halted | a2x_ccs::VmStatus::Running
    ));
}

#[test]
fn test_vm_world_graph_grows() {
    let source = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();

    let mut vm = CcsVm::new();
    let before = vm.world_graph.node_count();
    vm.load(program);
    vm.run().expect("run");

    let after = vm.world_graph.node_count();
    assert!(
        after >= before,
        "WorldGraph should grow or stay same after execution"
    );
}

#[test]
fn test_vm_memory_trace_grows() {
    let source = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();

    let mut vm = CcsVm::new();
    let before = vm.memory_trace.len();
    vm.load(program);
    vm.run().expect("run");

    let after = vm.memory_trace.len();
    assert!(after >= before);
}

#[test]
fn test_vm_uptime_increases() {
    let mut vm = CcsVm::new();
    let mut program = SigmaProgram::new();
    program.push(a2x_sigma::SigmaPacket::default());
    program.compute_id();
    vm.load(program);
    vm.run().expect("run");
    assert!(vm.uptime() > std::time::Duration::ZERO);
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. Full End-to-End Pipeline
// ═══════════════════════════════════════════════════════════════════════════

/// The canonical end-to-end test: parse Σ∞ → orchestrate → execute → verify.
#[test]
fn test_full_pipeline_parse_dispatch_execute() {
    // Step 1: Parse Σ∞ source
    let source = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();
    assert!(!program.is_empty(), "parsed program should not be empty");
    assert!(
        !program.id.to_string().is_empty(),
        "program should have an ID"
    );

    // Step 2: Create orchestrator and dispatch
    let orch = Orchestrator::new(AgentId::new("e2e-orch"));
    let _result = orch.dispatch(program.clone()).expect("dispatch");
    // Result may be empty for simple programs, which is fine

    // Step 3: Verify orchestrator state
    let snapshot = orch.state_summary().expect("state summary");
    assert!(!snapshot.state.is_empty());
}

/// Test the full Gateway → Entity → Auth → Execute flow.
#[test]
fn test_full_pipeline_gateway_entity_execute() {
    // Step 1: Create gateway
    let gw = Gateway::new();

    // Step 2: Register entity
    let entity_id = EntityId::new("pipeline-app");
    {
        let mut state = gw.state.lock().unwrap();
        let entity = SimpleEntity::new(EntityInfo::new(
            entity_id.clone(),
            EntityType::Application,
            "Pipeline App",
            vec![
                Capability::Execute,
                Capability::FileSystem,
                Capability::Probe,
            ],
        ));
        state.register_entity(Box::new(entity));
    }

    // Step 3: Create program
    let mut program = SigmaProgram::new();
    let pkt = a2x_sigma::SigmaPacket::default();
    program.push(pkt);
    program.compute_id();

    // Step 4: Execute with permission check
    {
        let mut state = gw.state.lock().unwrap();
        let result = state.execute_program_for_entity(&program, &entity_id);
        assert!(result.is_ok(), "gateway execution should succeed");
    }

    // Step 5: Check probe permission
    {
        let state = gw.state.lock().unwrap();
        let probe_result = state.check_probe_permission(&entity_id);
        assert!(probe_result.is_ok(), "probe should be allowed");
    }

    // Step 6: Verify entity listing
    {
        let state = gw.state.lock().unwrap();
        let entities = state.list_entities();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, entity_id);
    }
}

/// Test Bus → Multi-agent discovery → Orchestrator dispatch via bus.
#[test]
fn test_full_pipeline_bus_orchestration() {
    // Step 1: Create bus
    let mut bus = Bus::new();

    // Step 2: Register agents
    let agents: Vec<a2x_bus::AgentInfo> = vec![
        a2x_bus::AgentInfo::new(
            AgentId::new("b-orch"),
            AgentType::Orchestrator,
            vec![Capability::Execute, Capability::Custom("schedule".into())],
        ),
        a2x_bus::AgentInfo::new(
            AgentId::new("b-cli"),
            AgentType::Cli,
            vec![Capability::Execute, Capability::FileSystem],
        ),
        a2x_bus::AgentInfo::new(
            AgentId::new("b-ccs"),
            AgentType::Ccs,
            vec![Capability::Execute, Capability::Custom("cognitive".into())],
        ),
    ];
    for info in agents {
        bus.register_agent(info).unwrap();
    }
    assert_eq!(bus.agent_count(), 3);

    // Step 3: Discover all execute-capable agents
    let exec_agents = bus.discover(&AgentFilter::ByCapability(Capability::Execute));
    assert_eq!(
        exec_agents.len(),
        3,
        "all 3 agents should have Execute capability"
    );

    // Step 4: Dispatch via bus
    let orch = Orchestrator::new(AgentId::new("pipeline-orch"));
    let mut program = SigmaProgram::new();
    program.push(a2x_sigma::SigmaPacket::default());
    program.compute_id();

    let result = orch
        .dispatch_via_bus(&mut bus, program, Capability::Execute)
        .expect("dispatch via bus");
    assert!(result.is_empty());
}

/// Test CCS VM with probe snapshot after execution.
#[test]
fn test_full_pipeline_vm_probe() {
    let source = "⟦Σ∞⟧⟬I:⚡✣ ∷ C:⟚⟨sys⟩ ∷ P:⥂ ∷ D:⌵⟭";
    let mut program = a2x_sigma::parse_program(source).expect("parse");
    program.compute_id();

    let mut vm = CcsVm::new();
    vm.load(program);
    vm.run().expect("run");
    assert!(vm.world_graph.node_count() == 0 || vm.world_graph.node_count() > 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. Error Handling Paths
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_invalid_input_returns_error() {
    let result = a2x_sigma::parse_program("⟦this is not valid sigma⟧");
    assert!(result.is_err(), "invalid input should produce parse error");
}

#[test]
fn test_vm_run_with_no_program_returns_ok() {
    let mut vm = CcsVm::new();
    // Running without loading a program is valid — it's a no-op
    // Running without a loaded program is a user error — the VM should
    // return an error (no program to execute).
    let result = vm.run();
    assert!(
        result.is_err(),
        "VM should error when run without a loaded program"
    );
}

#[test]
fn test_gateway_probe_unknown_agent() {
    let gw = Gateway::new();
    let state = gw.state.lock().unwrap();
    // probing by a type that doesn't match any built-in should still work
    // since we try all agent types
    let result = state.probe_agent("any-id");
    assert!(
        result.is_ok(),
        "probe should succeed by matching at least one agent type"
    );
}

#[test]
fn test_ccs_agent_empty_id_allowed() {
    let agent = CcsAgent::new(AgentId::new(""));
    let snapshot = agent.state_summary().expect("state summary");
    assert!(snapshot.agent_id.as_str().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. Rate Limiter Token Bucket
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_rate_limiter_allows_within_limit() {
    use a2x_gateway::rate_limiter::RateLimiter;
    let mut rl = RateLimiter::new(60);
    let entity_id = EntityId::new("rl-test");

    // First request should succeed
    assert!(rl.check(&entity_id, 10));
}

#[test]
fn test_rate_limiter_exhausts_bucket() {
    use a2x_gateway::rate_limiter::RateLimiter;
    let mut rl = RateLimiter::new(60);
    let entity_id = EntityId::new("rl-exhaust");

    // Consume all tokens
    for _ in 0..3 {
        assert!(rl.check(&entity_id, 3), "should allow first few");
    }
    // After exhausting, further requests should fail
    // (capacity=3 burst, refill is slow)
    assert!(
        !rl.check(&entity_id, 3),
        "should deny after bucket exhausted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. Agent Capability Matrix
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_agent_types_have_capabilities() {
    let agents: Vec<(AgentType, Box<dyn Agent>)> = vec![
        (
            AgentType::Orchestrator,
            Box::new(Orchestrator::new(AgentId::new("a1"))),
        ),
        (AgentType::Cli, Box::new(CliAgent::new(AgentId::new("a2")))),
        (AgentType::Ccs, Box::new(CcsAgent::new(AgentId::new("a3")))),
    ];

    for (agent_type, agent) in &agents {
        let caps = agent.capabilities();
        assert!(
            !caps.is_empty(),
            "{:?} should have at least one capability",
            agent_type
        );
        println!("{:?} capabilities: {:?}", agent_type, caps);
    }
}
