// See plans/04-bus.md §1-7 and gap report 2026-07-02-a2x-bus.md
//
// BusBridge — convenience wrapper around Bus for publishing domain events
// as Σ∞ packets and polling the bus for incoming messages.

use crate::bus::{Bus, BusError};
use crate::discovery::{AgentInfo, DiscoveryError};
use crate::transport::{InMemoryTransport, Transport};
use crate::wire::{MessageType, WireMessage};
use a2x_core::{AgentId, AgentType, Capability};
use a2x_sigma::{ContextField, ContextOp, DataField, DataOp, IntentField, IntentOp, SigmaPacket};

/// Convenience bridge that wraps a [`Bus`] and automates:
/// - Correlation ID generation
/// - Agent identity management
/// - Domain event → Σ∞ packet construction (intent/context mapping)
/// - Message polling
///
/// Generic over the [`Transport`] backend with a default of [`InMemoryTransport`].
///
/// # Lifecycle
///
/// 1. **Create** a [`Bus`] and wrap it with `BusBridge::new(bus, agent_id)`
/// 2. **Register** the bridge agent on the bus via [`register`](BusBridge::register)
/// 3. **Publish** messages via [`publish_event`](BusBridge::publish_event),
///    [`publish_sigma`](BusBridge::publish_sigma), or
///    [`publish_raw`](BusBridge::publish_raw)
/// 4. **Poll** for incoming messages via [`poll`](BusBridge::poll)
/// 5. **Deregister** on shutdown via [`deregister`](BusBridge::deregister)
///
/// # Error Handling
///
/// - [`publish_sigma`](BusBridge::publish_sigma) and
///   [`publish_event`](BusBridge::publish_event) return [`BusError`] if no
///   agent matches the routing capability — publish to a registered target first
/// - [`poll`](BusBridge::poll) returns [`BusError`] if the agent was deregistered
///   or never registered — always register before polling
/// - Correlation IDs are sequential and thread-local to this bridge instance;
///   they start at 1
///
/// # Direct Transport (publish_raw with recipient)
///
/// When `publish_raw` is called with `Some(recipient)`, the message bypasses
/// capability routing and goes directly to the transport layer. This is useful
/// for point-to-point messages (heartbeats, acks, RPC responses). When
/// `recipient` is `None`, the payload is wrapped as a Σ∞ packet and routed
/// by capability — suitable for pub/sub and event broadcasting.
///
/// # Example
///
/// ```no_run
/// use a2x_bus::{Bus, BusBridge};
/// use a2x_core::{AgentId, AgentType, Capability};
///
/// let bus = Bus::new();
/// let mut bridge = BusBridge::new(bus, AgentId::new("daemon-1"));
/// bridge.register(AgentType::Orchestrator, vec![Capability::Execute]).unwrap();
///
/// // Publish a domain event as a Σ∞ packet
/// let corr_id = bridge.publish_event("cognition:thought", b"hello").unwrap();
///
/// // Poll for incoming messages
/// let msgs = bridge.poll().unwrap();
/// ```
pub struct BusBridge<T: Transport = InMemoryTransport> {
    bus: Bus<T>,
    agent_id: AgentId,
    correlation_counter: u64,
}

impl<T: Transport> BusBridge<T> {
    /// Create a new bridge wrapping the given bus, identified by `agent_id`.
    pub fn new(bus: Bus<T>, agent_id: AgentId) -> Self {
        BusBridge {
            bus,
            agent_id,
            correlation_counter: 0,
        }
    }

    // ── Bus lifecycle ──────────────────────────────────────────────────

    /// Register this bridge's agent on the bus (transport + discovery).
    ///
    /// Call this once during startup before publishing messages.
    /// # Panics
    ///
    /// Panics if the agent is already registered (duplicate AgentId).
    pub fn register(
        &mut self,
        agent_type: AgentType,
        capabilities: Vec<Capability>,
    ) -> Result<(), DiscoveryError> {
        let info = AgentInfo::new(self.agent_id.clone(), agent_type, capabilities);
        self.bus.register_agent(info)
    }

    /// Deregister this bridge's agent from the bus.
    pub fn deregister(&mut self) {
        self.bus.deregister_agent(&self.agent_id);
    }

    // ── Publishing ─────────────────────────────────────────────────────

    /// Publish a pre-built [`SigmaPacket`] to the bus, routing by capability.
    ///
    /// Returns the correlation ID assigned to this message.
    pub fn publish_sigma(
        &mut self,
        packet: &SigmaPacket,
        capability: &Capability,
    ) -> Result<u64, BusError> {
        let corr_id = self.next_correlation();
        self.bus
            .send_sigma(&self.agent_id, packet, capability, corr_id)?;
        Ok(corr_id)
    }

    /// Build a Σ∞ packet from a domain event and publish it.
    ///
    /// The `event_type` string (e.g. `"cognition:thought"`, `"system:alert"`)
    /// is parsed to determine:
    /// - **Intent operators** — inferred from keywords in the event type
    /// - **Context labels** — each colon-separated segment becomes a label
    ///
    /// The raw `payload` bytes are placed in the `D:` field as `Binary` data.
    ///
    /// Returns the correlation ID assigned to this message.
    pub fn publish_event(&mut self, event_type: &str, payload: &[u8]) -> Result<u64, BusError> {
        let packet = event_to_sigma(event_type, payload);
        let corr_id = self.next_correlation();
        // Route by Execute capability by default; caller can use
        // `publish_sigma` directly if a different capability is needed.
        self.bus
            .send_sigma(&self.agent_id, &packet, &Capability::Execute, corr_id)?;
        Ok(corr_id)
    }

    /// Publish raw bytes as a [`WireMessage`] with the given type.
    ///
    /// If `recipient` is `Some`, the message is sent directly to that agent
    /// via the transport layer. If `None`, the raw payload is wrapped in a
    /// Σ∞ packet (Binary data, Lightning intent) and routed by capability
    /// via `send_sigma`.
    ///
    /// Returns the correlation ID assigned to this message.
    pub fn publish_raw(
        &mut self,
        payload: &[u8],
        msg_type: MessageType,
        capability: &Capability,
        recipient: Option<AgentId>,
    ) -> Result<u64, BusError> {
        let corr_id = self.next_correlation();
        if let Some(ref target) = recipient {
            let msg = WireMessage::new(
                msg_type,
                self.agent_id.clone(),
                Some(target.clone()),
                corr_id,
                payload.to_vec(),
            );
            self.bus_mut()
                .transport_mut()
                .send(target.as_str(), msg)
                .map_err(BusError::Transport)?;
        } else {
            // Wrap raw payload as Σ∞ packet data and route by capability.
            // The msg_type is embedded as a context label for consumer dispatch.
            let label = format!("raw:{}", msg_type.as_str());
            let packet = event_to_sigma(&label, payload);
            self.bus
                .send_sigma(&self.agent_id, &packet, capability, corr_id)?;
        }
        Ok(corr_id)
    }

    // ── Receiving ──────────────────────────────────────────────────────

    /// Poll the bus for messages addressed to this bridge's agent.
    ///
    /// Returns all pending messages. Call this in a loop to receive messages.
    pub fn poll(&mut self) -> Result<Vec<WireMessage>, BusError> {
        self.bus.receive(&self.agent_id)
    }

    // ── Accessors ──────────────────────────────────────────────────────

    /// The AgentId this bridge publishes as.
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Number of agents currently registered on the bus.
    pub fn agent_count(&self) -> usize {
        self.bus.agent_count()
    }

    /// Whether a specific agent is registered on the bus.
    pub fn has_agent(&self, id: &AgentId) -> bool {
        self.bus.has_agent(id)
    }

    /// Immutable reference to the underlying [`Bus`].
    pub fn bus(&self) -> &Bus<T> {
        &self.bus
    }

    /// Mutable reference to the underlying [`Bus`] (for advanced use).
    pub fn bus_mut(&mut self) -> &mut Bus<T> {
        &mut self.bus
    }

    // ── Internal ───────────────────────────────────────────────────────

    fn next_correlation(&mut self) -> u64 {
        self.correlation_counter += 1;
        self.correlation_counter
    }
}

// ── Domain event → Σ∞ packet construction ──────────────────────────────

/// Build a [`SigmaPacket`] from a domain event type string and payload.
///
/// # Intent mapping
///
/// Keywords in `event_type` (case-insensitive) map to intent operators:
///
/// | Keywords                          | IntentOp     |
/// |-----------------------------------|--------------|
/// | alert, error, critical, warning   | Warning      |
/// | discover, explore, search         | Star         |
/// | merge, combine, synthesize, fuse  | Synthesis    |
/// | cancel, stop, halt                | Cancel       |
/// | parallel, fork                    | Parallel     |
/// | split, divide                     | Split        |
/// | delay, pause, wait                | Delay        |
/// | accelerate, speed, fast           | Accelerate   |
/// | (anything else)                   | Lightning    |
///
/// # Context labels
///
/// Each colon-separated segment of `event_type` becomes a label in the
/// context field. E.g. `"cognition:thought"` → labels `["cognition", "thought"]`.
///
/// # Data
///
/// The payload is placed in the `D:` field with the `Binary` data operator.
pub fn event_to_sigma(event_type: &str, payload: &[u8]) -> SigmaPacket {
    let lower = event_type.to_lowercase();

    let intent_op = if lower.contains("alert")
        || lower.contains("error")
        || lower.contains("critical")
        || lower.contains("warning")
    {
        IntentOp::Warning
    } else if lower.contains("discover") || lower.contains("explore") || lower.contains("search") {
        IntentOp::Star
    } else if lower.contains("merge")
        || lower.contains("combine")
        || lower.contains("synthesize")
        || lower.contains("fuse")
    {
        IntentOp::Synthesis
    } else if lower.contains("cancel") || lower.contains("stop") || lower.contains("halt") {
        IntentOp::Cancel
    } else if lower.contains("parallel") || lower.contains("fork") {
        IntentOp::Parallel
    } else if lower.contains("split") || lower.contains("divide") {
        IntentOp::Split
    } else if lower.contains("delay") || lower.contains("pause") || lower.contains("wait") {
        IntentOp::Delay
    } else if lower.contains("accelerate") || lower.contains("speed") || lower.contains("fast") {
        IntentOp::Accelerate
    } else {
        IntentOp::Lightning
    };

    // Parse context labels from colon-separated event type segments
    let labels: Vec<String> = event_type
        .split(':')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let chosen_context_op = if labels.iter().any(|l| l.contains("cognition")) {
        ContextOp::CausalChain
    } else if labels.iter().any(|l| l.contains("system")) {
        ContextOp::Universal
    } else if labels
        .iter()
        .any(|l| l.contains("detect") || l.contains("anomaly"))
    {
        ContextOp::Uncertainty
    } else {
        ContextOp::Compression
    };

    let mut intent = IntentField::new();
    intent.operators.push(intent_op);

    let mut context = ContextField::new();
    context.operators.push(chosen_context_op);
    context.labels = labels;

    let mut data = DataField::new();
    data.operators.push(DataOp::Binary);
    data.payload = payload.to_vec();

    SigmaPacket {
        protocol: a2x_core::ProtocolId::Sigma,
        intent,
        context,
        plan: a2x_sigma::PlanField::new(),
        data,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::{AgentId, Capability};

    fn test_bus_with_agent() -> Bus {
        let mut bus = Bus::new();
        bus.register_agent(AgentInfo::new(
            AgentId::new("target-1"),
            AgentType::Orchestrator,
            vec![Capability::Execute],
        ))
        .unwrap();
        bus
    }

    // ── BusBridge tests ────────────────────────────────────────────────

    #[test]
    fn test_bridge_register_and_count() {
        let bus = Bus::new();
        let mut bridge = BusBridge::new(bus, AgentId::new("b1"));
        bridge
            .register(AgentType::Orchestrator, vec![Capability::Execute])
            .unwrap();
        assert_eq!(bridge.agent_count(), 1);
    }

    #[test]
    fn test_bridge_deregister() {
        let bus = Bus::new();
        let mut bridge = BusBridge::new(bus, AgentId::new("b1"));
        bridge
            .register(AgentType::Orchestrator, vec![Capability::Execute])
            .unwrap();
        assert_eq!(bridge.agent_count(), 1);
        bridge.deregister();
        // After deregister, the agent is marked offline in discovery
        // and removed from the transport.
        // agent_count returns all agents including offline ones from discovery,
        // but the transport no longer has the agent registered.
        // The key assertion: deregister shouldn't panic, and the agent
        // can no longer receive messages.
        let count = bridge.agent_count();
        assert!(count <= 1, "deregister should not increase count");

        // Verify the agent can't receive messages after deregister —
        // transport removes the mailbox, so recv returns an error.
        let recv_result = bridge.poll();
        assert!(
            recv_result.is_err(),
            "poll() after deregister should fail: transport mailbox removed"
        );
    }

    #[test]
    fn test_publish_sigma_and_poll() {
        let bus = test_bus_with_agent();
        let mut sender = BusBridge::new(bus, AgentId::new("sender-1"));
        // Don't register sender — just send to the target that IS registered

        let packet = {
            let mut p = SigmaPacket::new();
            p.data.operators.push(DataOp::Binary);
            p.data.payload = b"hello world".to_vec();
            p
        };

        let corr = sender.publish_sigma(&packet, &Capability::Execute).unwrap();
        assert_eq!(corr, 1); // first message, correlation ID should be 1

        // Poll as the target agent
        let msgs = sender.bus_mut().receive(&AgentId::new("target-1")).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, AgentId::new("sender-1"));
        assert_eq!(msgs[0].msg_type, MessageType::SigmaProgram);
        assert_eq!(msgs[0].correlation_id, 1);
    }

    #[test]
    fn test_correlation_counter_increments() {
        let bus = test_bus_with_agent();
        let mut sender = BusBridge::new(bus, AgentId::new("s1"));

        let packet = SigmaPacket::new();
        let c1 = sender.publish_sigma(&packet, &Capability::Execute).unwrap();
        let c2 = sender.publish_sigma(&packet, &Capability::Execute).unwrap();
        let c3 = sender.publish_sigma(&packet, &Capability::Execute).unwrap();

        assert_eq!(c1, 1);
        assert_eq!(c2, 2);
        assert_eq!(c3, 3);
    }

    #[test]
    fn test_poll_empty_when_no_messages() {
        let bus = test_bus_with_agent();
        let mut bridge = BusBridge::new(bus, AgentId::new("lonely"));
        bridge
            .register(AgentType::Orchestrator, vec![Capability::Execute])
            .unwrap();

        let msgs = bridge.poll().unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_agent_id_accessor() {
        let bus = Bus::new();
        let bridge = BusBridge::new(bus, AgentId::new("my-agent"));
        assert_eq!(bridge.agent_id().as_str(), "my-agent");
    }

    #[test]
    fn test_has_agent() {
        let mut bus = Bus::new();
        bus.register_agent(AgentInfo::new(
            AgentId::new("alice"),
            AgentType::Cli,
            vec![Capability::Execute],
        ))
        .unwrap();

        let bridge = BusBridge::new(bus, AgentId::new("bob"));
        assert!(bridge.has_agent(&AgentId::new("alice")));
        assert!(!bridge.has_agent(&AgentId::new("charlie")));
    }

    // ── event_to_sigma tests ───────────────────────────────────────────

    #[test]
    fn test_event_to_sigma_alert() {
        let pkt = event_to_sigma("system:alert", b"disk full");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Warning]);
        assert_eq!(pkt.data.payload, b"disk full");
        assert_eq!(pkt.data.operators, vec![DataOp::Binary]);
    }

    #[test]
    fn test_event_to_sigma_discover() {
        let pkt = event_to_sigma("explore:search", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Star]);
        assert_eq!(
            pkt.context.labels,
            vec!["explore".to_string(), "search".to_string()]
        );
    }

    #[test]
    fn test_event_to_sigma_synthesis() {
        let pkt = event_to_sigma("merge:synthesize", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Synthesis]);
    }

    #[test]
    fn test_event_to_sigma_cancel() {
        let pkt = event_to_sigma("cancel:halt", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Cancel]);
    }

    #[test]
    fn test_event_to_sigma_parallel() {
        let pkt = event_to_sigma("parallel:fork", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Parallel]);
    }

    #[test]
    fn test_event_to_sigma_split() {
        let pkt = event_to_sigma("split:divide", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Split]);
    }

    #[test]
    fn test_event_to_sigma_delay() {
        let pkt = event_to_sigma("pause:wait", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Delay]);
    }

    #[test]
    fn test_event_to_sigma_accelerate() {
        let pkt = event_to_sigma("accelerate:fast", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Accelerate]);
    }

    #[test]
    fn test_event_to_sigma_default_lightning() {
        let pkt = event_to_sigma("something:unknown", b"");
        assert_eq!(pkt.intent.operators, vec![IntentOp::Lightning]);
    }

    #[test]
    fn test_event_to_sigma_context_cognition() {
        let pkt = event_to_sigma("cognition:thought", b"");
        assert_eq!(pkt.context.operators, vec![ContextOp::CausalChain]);
        assert_eq!(
            pkt.context.labels,
            vec!["cognition".to_string(), "thought".to_string()]
        );
    }

    #[test]
    fn test_event_to_sigma_context_system() {
        let pkt = event_to_sigma("system:status", b"");
        assert_eq!(pkt.context.operators, vec![ContextOp::Universal]);
    }

    #[test]
    fn test_event_to_sigma_context_anomaly() {
        let pkt = event_to_sigma("detect:anomaly", b"");
        assert_eq!(pkt.context.operators, vec![ContextOp::Uncertainty]);
    }

    #[test]
    fn test_event_to_sigma_context_default() {
        let pkt = event_to_sigma("foo:bar", b"");
        assert_eq!(pkt.context.operators, vec![ContextOp::Compression]);
    }

    #[test]
    fn test_event_to_sigma_empty_labels() {
        let pkt = event_to_sigma("", b"");
        assert!(pkt.context.labels.is_empty());
        assert_eq!(pkt.intent.operators, vec![IntentOp::Lightning]);
    }

    #[test]
    fn test_event_to_sigma_payload_preserved() {
        let payload = vec![0u8, 1, 2, 3, 255];
        let pkt = event_to_sigma("event:test", &payload);
        assert_eq!(pkt.data.payload, payload);
    }

    #[test]
    fn test_event_to_sigma_has_protocol() {
        let pkt = event_to_sigma("test", b"");
        assert_eq!(pkt.protocol, a2x_core::ProtocolId::Sigma);
    }

    // ── publish_event tests ────────────────────────────────────────────

    #[test]
    fn test_publish_event_routes_correctly() {
        let bus = test_bus_with_agent();
        let mut bridge = BusBridge::new(bus, AgentId::new("publisher"));

        let corr = bridge.publish_event("system:alert", b"critical").unwrap();
        assert_eq!(corr, 1);

        // Target agent should receive it
        let msgs = bridge.bus_mut().receive(&AgentId::new("target-1")).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, MessageType::SigmaProgram);
        assert_eq!(msgs[0].sender, AgentId::new("publisher"));

        // Payload should contain the event as a Σ∞ packet serialization
        let payload_str = String::from_utf8_lossy(&msgs[0].payload);
        assert!(
            payload_str.contains("⚠"),
            "expected Warning intent (⚠) in packet, got: {}",
            payload_str
        );
        assert!(
            payload_str.contains("system"),
            "expected 'system' label, got: {}",
            payload_str
        );
        assert!(
            payload_str.contains("alert"),
            "expected 'alert' label, got: {}",
            payload_str
        );
    }
}
