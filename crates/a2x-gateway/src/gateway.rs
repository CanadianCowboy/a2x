// See plans/06-entity-gateway.md §4
// Gateway service — the core bridge between external entities and the A2X bus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a2x_agents::{
    CcsAgent, ChatAgent, CliAgent, LlmAgent, LlmBackend, NoopBackend, OpenAiBackend, OpenAiConfig,
    Orchestrator, SandboxMode,
};
use a2x_bus::{AgentInfo, Bus};
use a2x_ccs::CcsVm;
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::graph::WorldGraph;

use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

use crate::auth::{AuthMethod, AuthProvider, EntityPermissions, InMemoryAuthProvider};
use crate::config::GatewayConfig;
use crate::entity::{Entity, EntityId, EntityInfo};
use crate::error::GatewayError;
use crate::listeners::ProtocolListener;
use crate::rate_limiter::RateLimiter;
use crate::security_event::SecurityEvent;
use crate::webhook::WebhookManager;

/// Mutable gateway state (behind Arc<Mutex>).
pub struct GatewayState {
    /// Registered entities.
    pub entities: HashMap<EntityId, Box<dyn Entity>>,
    /// The A2X bus for routing programs to agents (shared with ChatAgent).
    pub bus: Arc<Mutex<Bus>>,
    /// Auth provider.
    pub auth: Box<dyn AuthProvider>,
    /// Webhook manager.
    pub webhooks: WebhookManager,
    /// Gateway configuration.
    pub config: GatewayConfig,
    /// Protocol listeners.
    pub listeners: Vec<Box<dyn ProtocolListener>>,
    /// Correlation ID counter.
    pub next_correlation_id: u64,
    /// Rate limiter: token-bucket per entity.
    pub rate_limiter: RateLimiter,
    /// Dashboard: ring buffer of recent bus events (max 200).
    pub bus_log: Vec<DashboardEvent>,
    /// Dashboard: ring buffer of recent program executions (max 50).
    pub program_history: Vec<ProgramHistoryEntry>,
    /// Chat agent for the dashboard chat interface (lazily initialized).
    pub chat_agent: Option<Arc<ChatAgent>>,
    /// CCS VM shared with the chat agent (persistent state).
    pub chat_ccs_vm: Arc<Mutex<CcsVm>>,
}

/// A single event entry for the dashboard bus log.
#[derive(Clone, Debug)]
pub struct DashboardEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub message: String,
}

/// A single program execution entry for the dashboard history.
#[derive(Clone, Debug)]
pub struct ProgramHistoryEntry {
    pub timestamp: u64,
    pub source: String,
    pub result: String,
    pub status: String,
    pub duration_ms: u64,
}

impl GatewayState {
    pub fn new() -> Self {
        GatewayState {
            entities: HashMap::new(),
            bus: Arc::new(Mutex::new(Bus::new())),
            auth: Box::new(InMemoryAuthProvider::new()),
            webhooks: WebhookManager::new(),
            config: GatewayConfig::default(),
            listeners: Vec::new(),
            next_correlation_id: 1,
            rate_limiter: RateLimiter::new(60),
            bus_log: Vec::new(),
            program_history: Vec::new(),
            chat_agent: None,
            chat_ccs_vm: Arc::new(Mutex::new(CcsVm::new())),
        }
    }

    /// Get or initialize the chat agent (lazy init).
    /// Uses the configured backend (ollama/openai/none) from GatewayConfig.
    pub fn get_chat_agent(&mut self) -> Arc<ChatAgent> {
        if self.chat_agent.is_none() {
            let agent = self.build_chat_agent(
                self.config.chat_backend.model.clone(),
                self.config.chat_backend.backend_type.clone(),
            );
            self.chat_agent = Some(agent);

            // Load existing conversation from disk
            if let Some(agent) = self.chat_agent.as_ref() {
                if let Some(path) = Self::conversation_path() {
                    let _ = agent.load_conversation(&path);
                }
            }
        }
        self.chat_agent.as_ref().unwrap().clone()
    }

    /// Build a new chat agent with the given model.
    fn build_chat_agent(&self, model: String, backend_type: String) -> Arc<ChatAgent> {
        let backend: Arc<dyn LlmBackend> = match backend_type.as_str() {
            "ollama" => {
                tracing::info!(
                    model = %model,
                    url = %self.config.chat_backend.api_url,
                    "ChatAgent: using Ollama backend"
                );
                Arc::new(OpenAiBackend::new(OpenAiConfig {
                    api_url: self.config.chat_backend.api_url.clone(),
                    api_key: String::new(),
                    model,
                    max_tokens: self.config.chat_backend.max_tokens,
                    temperature: self.config.chat_backend.temperature,
                }))
            }
            "openai" => {
                tracing::info!(model = %model, "ChatAgent: using OpenAI backend");
                Arc::new(OpenAiBackend::new(OpenAiConfig {
                    api_url: self.config.chat_backend.api_url.clone(),
                    api_key: self.config.chat_backend.api_key.clone(),
                    model,
                    max_tokens: self.config.chat_backend.max_tokens,
                    temperature: self.config.chat_backend.temperature,
                }))
            }
            _ => {
                tracing::info!("ChatAgent: no backend configured (noop)");
                Arc::new(NoopBackend)
            }
        };
        let bus = self.bus.clone();
        let cli = Arc::new(CliAgent::with_sandbox(
            AgentId::new("chat-cli"),
            SandboxMode::None,
        ));
        let mut agent = ChatAgent::new(
            AgentId::new("chat-1"),
            backend,
            bus,
            cli,
            self.chat_ccs_vm.clone(),
        );
        agent.set_max_context_tokens(self.config.chat_backend.max_context_tokens);
        Arc::new(agent)
    }

    /// Switch to a different model (hot-swap the chat agent backend).
    pub fn switch_chat_model(&mut self, model: String) -> Arc<ChatAgent> {
        let agent = self.build_chat_agent(model, self.config.chat_backend.backend_type.clone());
        self.chat_agent = Some(agent.clone());
        agent
    }

    /// Get the path to the default conversation file (~/.a2x/conversations/chat-1.json).
    /// Returns None if no home directory can be determined.
    pub fn conversation_path() -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        if home.is_empty() {
            return None;
        }
        Some(
            std::path::PathBuf::from(home)
                .join(".a2x")
                .join("conversations")
                .join("chat-1.json"),
        )
    }

    /// List available Ollama models (empty if not connected, models fetched async via WS).
    pub fn list_ollama_models(&self) -> Vec<String> {
        // Models are fetched asynchronously through the dashboard WebSocket handler.
        // This sync method returns an empty list; the actual fetch happens in handle_models_command.
        vec![]
    }

    /// Register a new entity.
    pub fn register_entity(&mut self, entity: Box<dyn Entity>) {
        let id = entity.entity_id();
        tracing::info!("Entity registered: {} ({:?})", id, entity.entity_type());
        SecurityEvent::emit(SecurityEvent::EntityAuthenticated {
            entity_id: id.clone(),
            method: "registration".into(),
        });
        self.entities.insert(id, entity);
    }

    /// Add a protocol listener (fixes BUG-001).
    pub fn add_listener(&mut self, listener: Box<dyn ProtocolListener>) {
        let ltype = format!("{:?}", listener.listener_type());
        tracing::info!("Added listener: {}", ltype);
        SecurityEvent::emit(SecurityEvent::ListenerAdded {
            listener_type: ltype,
            address: listener.bound_address(),
        });
        self.listeners.push(listener);
    }

    /// Remove an entity by ID.
    pub fn unregister_entity(&mut self, entity_id: &EntityId) -> bool {
        self.webhooks.unregister_entity(entity_id);
        self.entities.remove(entity_id).is_some()
    }

    /// List all registered entities.
    pub fn list_entities(&self) -> Vec<EntityInfo> {
        self.entities
            .values()
            .map(|e| {
                EntityInfo::new(
                    e.entity_id(),
                    e.entity_type(),
                    e.display_name(),
                    e.capabilities(),
                )
            })
            .collect()
    }

    /// Get entity info by ID.
    pub fn get_entity(&self, entity_id: &EntityId) -> Option<EntityInfo> {
        self.entities.get(entity_id).map(|e| {
            EntityInfo::new(
                e.entity_id(),
                e.entity_type(),
                e.display_name(),
                e.capabilities(),
            )
        })
    }

    /// Execute a Σ∞ program on the default orchestrator agent.
    pub fn execute_program(&self, program: &SigmaProgram) -> Result<SigmaProgram, GatewayError> {
        let orchestrator = Orchestrator::new(AgentId::new("gateway-orch"));
        orchestrator
            .dispatch(program.clone())
            .map_err(|e| GatewayError::ProgramError(e.to_string()))
    }

    /// Execute a program with permission enforcement for the given entity.
    pub fn execute_program_for_entity(
        &mut self,
        program: &SigmaProgram,
        entity_id: &EntityId,
    ) -> Result<SigmaProgram, GatewayError> {
        let permissions_checked = self.auth.permissions(entity_id).is_some();
        SecurityEvent::emit(SecurityEvent::ProgramSubmitted {
            entity_id: entity_id.clone(),
            instruction_count: program.instructions.len(),
            permissions_checked,
        });

        if let Some(perms) = self.auth.permissions(entity_id) {
            self.enforce_permissions(&perms, program)?;
        }
        let result = self.execute_program(program);
        SecurityEvent::emit(SecurityEvent::ProgramCompleted {
            entity_id: entity_id.clone(),
            status: if result.is_ok() { "completed" } else { "error" }.into(),
        });
        result
    }

    /// Enforce entity permissions against a program request.
    pub fn enforce_permissions(
        &mut self,
        perms: &EntityPermissions,
        program: &SigmaProgram,
    ) -> Result<(), GatewayError> {
        let inst_count = program.instructions.len() as u64;
        if inst_count > perms.max_instructions {
            SecurityEvent::emit(SecurityEvent::PermissionDenied {
                entity_id: perms.entity_id.clone(),
                action: format!(
                    "execute ({} > {} instructions)",
                    inst_count, perms.max_instructions
                ),
            });
            return Err(GatewayError::PermissionDenied(format!(
                "program has {} instructions, max allowed is {}",
                inst_count, perms.max_instructions
            )));
        }

        if perms.rate_limit > 0 {
            self.enforce_rate_limit(&perms.entity_id, perms.rate_limit)?;
        }

        Ok(())
    }

    fn enforce_rate_limit(&mut self, entity_id: &EntityId, limit: u32) -> Result<(), GatewayError> {
        if !self.rate_limiter.check(entity_id, limit) {
            SecurityEvent::emit(SecurityEvent::RateLimited {
                entity_id: entity_id.clone(),
                count: limit.saturating_add(1),
                limit,
            });
            return Err(GatewayError::RateLimited {
                entity_id: entity_id.to_string(),
                limit,
            });
        }
        Ok(())
    }

    /// Check if an entity is allowed to probe agent state.
    pub fn check_probe_permission(&self, entity_id: &EntityId) -> Result<(), GatewayError> {
        if let Some(perms) = self.auth.permissions(entity_id) {
            if !perms.can_probe {
                SecurityEvent::emit(SecurityEvent::PermissionDenied {
                    entity_id: entity_id.clone(),
                    action: "probe".into(),
                });
                return Err(GatewayError::PermissionDenied(format!(
                    "entity '{}' is not authorized to probe agent state",
                    entity_id
                )));
            }
        }
        Ok(())
    }

    /// Probe an agent's state by type and ID.
    pub fn probe_agent(&self, agent_id: &str) -> Result<StateSnapshot, GatewayError> {
        let id = AgentId::new(agent_id);
        let agents: Vec<Box<dyn Agent>> = vec![
            Box::new(Orchestrator::new(id.clone())),
            Box::new(CliAgent::new(id.clone())),
            Box::new(LlmAgent::new_stub(id.clone(), "probe")),
            Box::new(CcsAgent::new(id.clone())),
        ];

        for agent in agents {
            if let Some(snapshot) = agent.state_summary() {
                return Ok(snapshot);
            }
        }

        Err(GatewayError::AgentNotFound(agent_id.into()))
    }

    /// Authenticate an incoming request.
    pub fn authenticate(&self, method: &AuthMethod) -> Result<EntityId, GatewayError> {
        self.auth.authenticate(method)
    }

    /// Get the next correlation ID.
    pub fn next_correlation_id(&mut self) -> u64 {
        let id = self.next_correlation_id;
        self.next_correlation_id = self.next_correlation_id.wrapping_add(1);
        id
    }

    /// Record a bus event for the dashboard log.
    pub fn record_bus_event(&mut self, event_type: &str, message: &str) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.bus_log.push(DashboardEvent {
            timestamp: ts,
            event_type: event_type.to_string(),
            message: message.to_string(),
        });
        if self.bus_log.len() > 200 {
            self.bus_log.remove(0);
        }
    }

    /// Record a program execution for the dashboard history.
    pub fn record_execution(&mut self, source: &str, result: &str, status: &str, duration_ms: u64) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.program_history.push(ProgramHistoryEntry {
            timestamp: ts,
            source: source.to_string(),
            result: result.to_string(),
            status: status.to_string(),
            duration_ms,
        });
        if self.program_history.len() > 50 {
            self.program_history.remove(0);
        }
    }

    /// Clone the bus log (non-draining — safe for multiple dashboard clients).
    pub fn clone_bus_log(&self) -> Vec<DashboardEvent> {
        self.bus_log.clone()
    }

    /// Clone the program history (non-draining — safe for multiple dashboard clients).
    pub fn clone_program_history(&self) -> Vec<ProgramHistoryEntry> {
        self.program_history.clone()
    }

    /// Bootstrap the CCS VM WorldGraph with system concepts.
    ///
    /// Directly allocates concept nodes and creates relation edges in the
    /// WorldGraph so the dashboard has meaningful data on startup.
    pub fn bootstrap_world_graph(&self) -> Result<(), String> {
        let mut vm = self
            .chat_ccs_vm
            .lock()
            .map_err(|e| format!("vm lock: {}", e))?;

        use a2x_core::concept::ConceptVector;
        use a2x_core::relation::{RelationEdge, RelationType};

        // Step 1: Allocate concept nodes directly in the WorldGraph.
        let concepts: &[(&str, &[f32])] = &[
            ("sys", &[1.0, 0.1, 0.0, 0.0]),
            ("orch", &[0.1, 1.0, 0.0, 0.0]),
            ("cli", &[0.0, 0.1, 1.0, 0.0]),
            ("llm", &[0.0, 0.0, 0.1, 1.0]),
            ("ccs", &[0.5, 0.5, 0.0, 0.0]),
            ("bus", &[0.3, 0.3, 0.3, 0.1]),
            ("gw", &[0.2, 0.2, 0.2, 0.4]),
            ("exec", &[0.8, 0.0, 0.2, 0.0]),
            ("plan", &[0.0, 0.8, 0.0, 0.2]),
            ("probe", &[0.0, 0.0, 0.0, 1.0]),
            ("goal", &[0.1, 0.1, 0.1, 0.7]),
            ("task", &[0.6, 0.4, 0.0, 0.0]),
        ];

        let mut label_ids: std::collections::HashMap<String, a2x_core::node::NodeId> =
            std::collections::HashMap::new();

        for (label, floats) in concepts {
            let cv = ConceptVector::from_vec(floats.to_vec());
            let id = vm
                .world_graph
                .allocate(cv)
                .map_err(|e| format!("bootstrap: allocate '{}': {}", label, e))?;
            vm.world_graph
                .set_label(id, label)
                .map_err(|e| format!("bootstrap: set_label '{}': {}", label, e))?;
            label_ids.insert(label.to_string(), id);
        }

        // Step 2: Create Hierarchical relation edges between related concepts.
        let relations: &[(&[&str], &str)] = &[
            (&["sys", "orch"], "sys-orch"),
            (&["sys", "cli"], "sys-cli"),
            (&["sys", "llm"], "sys-llm"),
            (&["sys", "ccs"], "sys-ccs"),
            (&["sys", "bus"], "sys-bus"),
            (&["sys", "gw"], "sys-gw"),
            (&["orch", "plan"], "orch-plan"),
            (&["cli", "exec"], "cli-exec"),
            (&["ccs", "probe"], "ccs-probe"),
            (&["goal", "task"], "goal-task"),
        ];

        for (sources, label) in relations {
            // Create a composite BIND node by running through the VM's
            // dispatch_bind which handles auto-labels and provenance
            let operand_labels: Vec<String> = sources.iter().map(|s| s.to_string()).collect();

            // Verify all source labels exist
            for src in *sources {
                if !label_ids.contains_key(*src) {
                    tracing::warn!(src, "bootstrap: BIND source label not found, skipping");
                    continue;
                }
            }

            // Allocate bind node and create edges manually for reliability
            let concept_vecs: Vec<ConceptVector> = operand_labels
                .iter()
                .filter_map(|l| {
                    let id = label_ids.get(l)?;
                    let node = vm.world_graph.lookup(*id).ok()??;
                    Some(node.concept.clone())
                })
                .collect();

            if concept_vecs.len() < 2 {
                continue;
            }

            // Create composite concept (simple average)
            let dim = concept_vecs[0].data.len();
            let mut composite = vec![0.0f32; dim];
            for cv in &concept_vecs {
                for (i, v) in cv.data.iter().enumerate() {
                    composite[i] += v;
                }
            }
            let n = concept_vecs.len() as f32;
            for v in &mut composite {
                *v /= n;
            }

            let bind_id = vm
                .world_graph
                .allocate(ConceptVector::from_vec(composite))
                .map_err(|e| format!("bootstrap: BIND allocate: {}", e))?;

            vm.world_graph.set_label(bind_id, label).ok();

            // Create Hierarchical edges from each source to the BIND node
            for src in *sources {
                if let Some(&src_id) = label_ids.get(*src) {
                    let edge = RelationEdge::new(src_id, bind_id, RelationType::Hierarchical, 1.0);
                    let _ = vm.world_graph.add_edge(src_id, bind_id, edge);
                }
            }
        }

        // Step 3: Run EVOLVE to update access counts and attention.
        let mut pkt = a2x_sigma::SigmaPacket::new();
        pkt.intent
            .operators
            .push(a2x_sigma::intent::IntentOp::Delay); // EVOLVE
        let mut prog = a2x_sigma::program::SigmaProgram::new();
        prog.push(pkt);
        vm.load(prog);
        if let Err(e) = vm.run() {
            tracing::warn!(error = %e, "bootstrap: EVOLVE failed");
        }

        tracing::info!(
            nodes = vm.world_graph.node_count(),
            edges = vm.world_graph.edge_count(),
            "WorldGraph bootstrapped with system concepts"
        );
        Ok(())
    }
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

/// The A2X Gateway — bridges external entities to the A2X bus.
pub struct Gateway {
    /// Shared gateway state.
    pub state: Arc<Mutex<GatewayState>>,
}

impl Gateway {
    /// Create a new gateway with default configuration.
    pub fn new() -> Self {
        Gateway {
            state: Arc::new(Mutex::new(GatewayState::new())),
        }
    }

    /// Create a gateway from a TOML config string.
    pub fn from_config(config: GatewayConfig) -> Result<Self, GatewayError> {
        let mut gw_state = GatewayState::new();
        gw_state.config = config.clone();

        let mut auth = InMemoryAuthProvider::new();
        for entry in &config.auth.api_keys {
            auth.register_key(entry.key.clone(), EntityId::new(&entry.entity_id));
        }
        gw_state.auth = Box::new(auth);

        Ok(Gateway {
            state: Arc::new(Mutex::new(gw_state)),
        })
    }

    /// Register a built-in agent on the bus (for demo/testing).
    pub fn register_builtin_agents(&self) -> Result<(), GatewayError> {
        let gw = self.state.lock().map_err(|e| {
            GatewayError::ListenerError(format!("failed to lock gateway state: {}", e))
        })?;
        let agents = vec![
            AgentInfo::new(
                AgentId::new("orch-1"),
                AgentType::Orchestrator,
                vec![Capability::Execute, Capability::Custom("schedule".into())],
            ),
            AgentInfo::new(
                AgentId::new("cli-1"),
                AgentType::Cli,
                vec![
                    Capability::Execute,
                    Capability::FileSystem,
                    Capability::Network,
                    Capability::Shell,
                ],
            ),
            AgentInfo::new(
                AgentId::new("llm-1"),
                AgentType::Llm,
                vec![Capability::Execute, Capability::Custom("plan".into())],
            ),
            AgentInfo::new(
                AgentId::new("ccs-1"),
                AgentType::Ccs,
                vec![
                    Capability::Execute,
                    Capability::Custom("plan".into()),
                    Capability::Custom("cognitive".into()),
                ],
            ),
            AgentInfo::new(
                AgentId::new("chat-1"),
                AgentType::Chat,
                vec![
                    Capability::Chat,
                    Capability::Execute,
                    Capability::Generate,
                    Capability::Probe,
                    Capability::Reflect,
                    Capability::Shell,
                    Capability::FileSystem,
                ],
            ),
        ];
        for info in agents {
            let _ = gw.bus.lock().unwrap().register_agent(info);
        }
        Ok(())
    }

    /// Get a reference to the shared gateway state (for listener integration).
    pub fn state_arc(&self) -> Arc<Mutex<GatewayState>> {
        self.state.clone()
    }

    /// Start the gateway — begin listening for entity connections.
    pub fn start(&self) -> Result<(), GatewayError> {
        let mut gw = self.state.lock().map_err(|e| {
            GatewayError::ListenerError(format!("failed to lock gateway state: {}", e))
        })?;

        for listener in gw.listeners.iter_mut() {
            listener.start()?;
        }

        tracing::info!("Gateway started with {} listener(s)", gw.listeners.len());
        SecurityEvent::emit(SecurityEvent::GatewayStarted {
            listener_count: gw.listeners.len(),
        });
        Ok(())
    }

    /// Stop all listeners.
    pub fn stop(&self) -> Result<(), GatewayError> {
        let mut gw = self.state.lock().map_err(|e| {
            GatewayError::ListenerError(format!("failed to lock gateway state: {}", e))
        })?;

        for listener in gw.listeners.iter_mut() {
            let _ = listener.stop();
        }

        tracing::info!("Gateway stopped");
        SecurityEvent::emit(SecurityEvent::GatewayStopped);
        Ok(())
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityType, SimpleEntity};

    #[test]
    fn test_gateway_new() {
        let gw = Gateway::new();
        let state = gw.state.lock().unwrap();
        assert!(state.entities.is_empty());
    }

    #[test]
    fn test_gateway_register_entity() {
        let gw = Gateway::new();
        let entity = SimpleEntity::new(EntityInfo::new(
            EntityId::new("e-1"),
            EntityType::Application,
            "Test App",
            vec![Capability::Execute],
        ));
        {
            let mut state = gw.state.lock().unwrap();
            state.register_entity(Box::new(entity));
        }
        let state = gw.state.lock().unwrap();
        assert_eq!(state.entities.len(), 1);
        assert!(state.get_entity(&EntityId::new("e-1")).is_some());
    }

    #[test]
    fn test_gateway_unregister_entity() {
        let gw = Gateway::new();
        let entity = SimpleEntity::new(EntityInfo::new(
            EntityId::new("e-1"),
            EntityType::Application,
            "App",
            vec![],
        ));
        {
            let mut state = gw.state.lock().unwrap();
            state.register_entity(Box::new(entity));
        }
        let removed = {
            let mut state = gw.state.lock().unwrap();
            state.unregister_entity(&EntityId::new("e-1"))
        };
        assert!(removed);
        let state = gw.state.lock().unwrap();
        assert!(state.entities.is_empty());
    }

    #[test]
    fn test_gateway_list_entities() {
        let gw = Gateway::new();
        let e1 = SimpleEntity::new(EntityInfo::new(
            EntityId::new("e-1"),
            EntityType::HumanCli,
            "Human",
            vec![],
        ));
        let e2 = SimpleEntity::new(EntityInfo::new(
            EntityId::new("e-2"),
            EntityType::Application,
            "App",
            vec![],
        ));
        {
            let mut state = gw.state.lock().unwrap();
            state.register_entity(Box::new(e1));
            state.register_entity(Box::new(e2));
        }
        let state = gw.state.lock().unwrap();
        let entities = state.list_entities();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_gateway_probe_agent() {
        let gw = Gateway::new();
        let state = gw.state.lock().unwrap();
        let snapshot = state.probe_agent("orch-1").unwrap();
        assert_eq!(snapshot.agent_id.as_str(), "orch-1");
    }

    #[test]
    fn test_gateway_from_config() {
        let toml_str = r#"
[http]
port = 9000

[auth]
mode = "api_key"

[[auth.api_keys]]
key = "sk-test"
entity_id = "app-1"
"#;
        let config = GatewayConfig::from_toml(toml_str).unwrap();
        let gw = Gateway::from_config(config).unwrap();
        let state = gw.state.lock().unwrap();
        let eid = state
            .authenticate(&crate::auth::AuthMethod::ApiKey("sk-test".into()))
            .unwrap();
        assert_eq!(eid, EntityId::new("app-1"));
    }

    #[test]
    fn test_gateway_register_builtin_agents() {
        let gw = Gateway::new();
        gw.register_builtin_agents().unwrap();
        let state = gw.state.lock().unwrap();
        assert_eq!(state.bus.lock().unwrap().agent_count(), 5);
    }

    #[test]
    fn test_gateway_correlation_id_increments() {
        let gw = Gateway::new();
        let id1 = {
            let mut state = gw.state.lock().unwrap();
            state.next_correlation_id()
        };
        let id2 = {
            let mut state = gw.state.lock().unwrap();
            state.next_correlation_id()
        };
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_execute_program_empty() {
        let gw = Gateway::new();
        let state = gw.state.lock().unwrap();
        let program = SigmaProgram::new();
        let result = state.execute_program(&program).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_chat_agent_lazy_init() {
        let gw = Gateway::new();
        let agent = {
            let mut state = gw.state.lock().unwrap();
            state.get_chat_agent()
        };
        let stats = agent.stats();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_tool_calls, 0);
    }
}
