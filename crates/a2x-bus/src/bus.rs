// See plans/04-bus.md §1-7

use crate::discovery::{AgentFilter, AgentInfo, Discovery, InMemoryDiscovery};
use crate::routing::{Router, RoutingStrategy};
use crate::transport::{InMemoryTransport, Transport, TransportError};
use crate::wire::{MessageType, WireMessage};
use a2x_core::{AgentId, Capability};
use a2x_sigma::SigmaPacket;

/// The A2X message bus — routes Σ∞/Ω programs between agents.
///
/// Generic over the [`Transport`] backend with a default of [`InMemoryTransport`]
/// for zero-config local usage. For TCP or custom transports, use
/// [`Bus::with_transport`].
///
/// # Examples
///
/// ```
/// use a2x_bus::Bus;
/// let bus = Bus::new(); // defaults to InMemoryTransport
/// ```
pub struct Bus<T: Transport = InMemoryTransport> {
    transport: T,
    discovery: InMemoryDiscovery,
    router: Router,
}

/// Constructors for the default [`InMemoryTransport`] backend.
impl Bus<InMemoryTransport> {
    /// Create a new bus with the default in-memory transport.
    pub fn new() -> Self {
        Bus {
            transport: InMemoryTransport::new(),
            discovery: InMemoryDiscovery::new(),
            router: Router::new(RoutingStrategy::FirstMatch),
        }
    }

    /// Create a new bus with a specific routing strategy (in-memory transport).
    pub fn with_strategy(strategy: RoutingStrategy) -> Self {
        Bus {
            transport: InMemoryTransport::new(),
            discovery: InMemoryDiscovery::new(),
            router: Router::new(strategy),
        }
    }
}

impl<T: Transport> Bus<T> {
    /// Create a bus with a custom transport backend.
    ///
    /// Use this for TCP, TLS, or other transport implementations.
    pub fn with_transport(transport: T) -> Self {
        Bus {
            transport,
            discovery: InMemoryDiscovery::new(),
            router: Router::new(RoutingStrategy::FirstMatch),
        }
    }

    /// Create a bus with a custom transport and routing strategy.
    pub fn with_transport_and_strategy(transport: T, strategy: RoutingStrategy) -> Self {
        Bus {
            transport,
            discovery: InMemoryDiscovery::new(),
            router: Router::new(strategy),
        }
    }

    /// Register an agent on the bus (transport + discovery).
    pub fn register_agent(
        &mut self,
        info: AgentInfo,
    ) -> Result<(), crate::discovery::DiscoveryError> {
        self.transport
            .register(info.id.as_str())
            .map_err(crate::discovery::DiscoveryError::Transport)?;
        self.discovery.register(info)
    }

    /// Deregister an agent from the bus.
    pub fn deregister_agent(&mut self, id: &AgentId) {
        self.discovery.mark_offline(id);
        self.transport.deregister(id.as_str());
    }

    /// Send a Σ∞ program to an agent capable of handling it.
    pub fn send_sigma(
        &mut self,
        sender: &AgentId,
        packet: &SigmaPacket,
        capability: &Capability,
        correlation_id: u64,
    ) -> Result<(), BusError> {
        let target = self
            .router
            .route(&self.discovery, capability)
            .ok_or_else(|| BusError::NoRoute {
                capability: capability.clone(),
            })?;

        let msg = WireMessage::new(
            MessageType::SigmaProgram,
            sender.clone(),
            Some(target.clone()),
            correlation_id,
            packet.to_string().into_bytes(),
        );

        self.transport
            .send(target.as_str(), msg)
            .map_err(BusError::Transport)
    }

    /// Receive messages for an agent.
    pub fn receive(&mut self, agent_id: &AgentId) -> Result<Vec<WireMessage>, BusError> {
        self.transport
            .recv(agent_id.as_str())
            .map_err(BusError::Transport)
    }

    pub fn agent_count(&self) -> usize {
        self.discovery
            .discover(&crate::discovery::AgentFilter::All)
            .len()
    }

    pub fn has_agent(&self, id: &AgentId) -> bool {
        !self
            .discovery
            .discover(&crate::discovery::AgentFilter::ById(id.clone()))
            .is_empty()
    }

    /// Discover agents matching a filter.
    pub fn discover(&self, filter: &AgentFilter) -> Vec<AgentInfo> {
        self.discovery.discover(filter)
    }

    /// Mutable reference to the transport layer (for advanced/raw sends).
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl Default for Bus<InMemoryTransport> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BusError {
    NoRoute { capability: Capability },
    Transport(TransportError),
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BusError::NoRoute { capability } => {
                write!(f, "no route for capability: {}", capability)
            }
            BusError::Transport(err) => write!(f, "transport error: {}", err),
        }
    }
}

impl std::error::Error for BusError {}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::AgentType;

    #[test]
    fn test_register_and_count() {
        let mut bus = Bus::new();
        bus.register_agent(AgentInfo::new(
            AgentId::new("a1"),
            AgentType::Cli,
            vec![Capability::Execute],
        ))
        .unwrap();
        assert_eq!(bus.agent_count(), 1);
    }

    #[test]
    fn test_send_and_receive_sigma() {
        let mut bus = Bus::new();
        let sender = AgentId::new("orch-1");
        let receiver = AgentId::new("cli-1");

        // Register the receiver
        bus.register_agent(AgentInfo::new(
            receiver.clone(),
            AgentType::Cli,
            vec![Capability::Execute],
        ))
        .unwrap();

        // Create a packet and send it
        let packet = SigmaPacket::new();
        let result = bus.send_sigma(&sender, &packet, &Capability::Execute, 42);
        assert!(result.is_ok());

        // Receive the message
        let msgs = bus.receive(&receiver).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, sender);
        assert_eq!(msgs[0].msg_type, MessageType::SigmaProgram);
    }

    #[test]
    fn test_with_transport_equivalent_to_new() {
        // A bus created with with_transport should behave identically to Bus::new().
        let mut bus = Bus::with_transport(InMemoryTransport::new());
        let receiver = AgentId::new("cli-2");

        bus.register_agent(AgentInfo::new(
            receiver.clone(),
            AgentType::Cli,
            vec![Capability::Execute],
        ))
        .unwrap();

        let packet = SigmaPacket::new();
        bus.send_sigma(&AgentId::new("orch-2"), &packet, &Capability::Execute, 1)
            .unwrap();

        let msgs = bus.receive(&receiver).unwrap();
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn test_with_transport_and_strategy() {
        let mut bus =
            Bus::with_transport_and_strategy(InMemoryTransport::new(), RoutingStrategy::FirstMatch);
        bus.register_agent(AgentInfo::new(
            AgentId::new("a3"),
            AgentType::Cli,
            vec![Capability::Execute],
        ))
        .unwrap();
        assert_eq!(bus.agent_count(), 1);
    }
}
