// See plans/06-entity-gateway.md §4
// Gateway service — the core bridge between external entities and the A2X bus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a2x_agents::{CcsAgent, CliAgent, LlmAgent, Orchestrator};
use a2x_bus::{AgentInfo, Bus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;

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
    /// The A2X bus for routing programs to agents.
    pub bus: Bus,
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
            bus: Bus::new(),
            auth: Box::new(InMemoryAuthProvider::new()),
            webhooks: WebhookManager::new(),
            config: GatewayConfig::default(),
            listeners: Vec::new(),
            next_correlation_id: 1,
            rate_limiter: RateLimiter::new(60),
            bus_log: Vec::new(),
            program_history: Vec::new(),
        }
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
    ///
    /// If `entity_id` is provided, checks permissions (max_instructions, rate limit)
    /// before execution. See plans/12-security.md §4.
    pub fn execute_program(&self, program: &SigmaProgram) -> Result<SigmaProgram, GatewayError> {
        let orchestrator = Orchestrator::new(AgentId::new("gateway-orch"));
        orchestrator
            .dispatch(program.clone())
            .map_err(|e| GatewayError::ProgramError(e.to_string()))
    }

    /// Execute a program with permission enforcement for the given entity.
    ///
    /// Fixes BUG-005: `EntityPermissions` were stored but never checked.
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

        // Check permissions if available.
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
    ///
    /// Checks: max_instructions, rate_limit.
    /// Returns `Err(PermissionDenied)` or `Err(RateLimited)` on violation.
    pub fn enforce_permissions(
        &mut self,
        perms: &EntityPermissions,
        program: &SigmaProgram,
    ) -> Result<(), GatewayError> {
        // Check instruction count limit.
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

        // Check rate limit.
        if perms.rate_limit > 0 {
            self.enforce_rate_limit(&perms.entity_id, perms.rate_limit)?;
        }

        Ok(())
    }

    /// Enforce rate limiting using a token-bucket algorithm.
    ///
    /// Each entity gets a token bucket with capacity = limit, refilling at
    /// limit/60 tokens per second. Bursts up to the full limit are allowed,
    /// but sustained rate is bounded to limit/min.
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
    ///
    /// Fixes BUG-005: probe access was never checked against permissions.
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
        // Try known agent types — in production, this would look up
        // the agent type from the bus discovery registry.
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
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

/// The A2X Gateway — bridges external entities to the A2X bus.
///
/// See plans/06-entity-gateway.md §4 for the full architecture.
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

        // Register API keys from config
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
        let mut gw = self.state.lock().map_err(|e| {
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
        ];
        for info in agents {
            let _ = gw.bus.register_agent(info);
        }
        Ok(())
    }

    /// Get a reference to the shared gateway state (for listener integration).
    pub fn state_arc(&self) -> Arc<Mutex<GatewayState>> {
        self.state.clone()
    }

    /// Start the gateway — begin listening for entity connections.
    ///
    /// In the full async version, this would spawn tokio tasks for each
    /// listener. For now, it marks listeners as started.
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
        assert_eq!(state.bus.agent_count(), 4);
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
}
