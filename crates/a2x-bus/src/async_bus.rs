// Phase 7.1: Async bus — InMemoryAsyncBus with tokio mpsc/rwlock
//
// See plans/10-concurrency.md §5 — "The Bus is Async-Native"
//
// The sync InMemoryTransport uses HashMap<String, Vec<WireMessage>> with
// &mut self. The async version uses tokio::sync::mpsc for lock-free
// message passing between agents, with RwLock for discovery.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use crate::discovery::{AgentFilter, AgentInfo, Discovery, DiscoveryError};
use crate::routing::{Router, RoutingStrategy};
use crate::wire::{MessageType, WireMessage};
use a2x_core::{AgentId, Capability};
use a2x_sigma::SigmaPacket;

/// Async error type for bus operations.
#[derive(Clone, Debug, PartialEq)]
pub enum AsyncBusError {
    NoRoute { capability: Capability },
    AgentNotFound(AgentId),
    ChannelClosed(AgentId),
    Discovery(DiscoveryError),
}

impl std::fmt::Display for AsyncBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncBusError::NoRoute { capability } => {
                write!(f, "no route for capability: {}", capability)
            }
            AsyncBusError::AgentNotFound(id) => write!(f, "agent not found: {}", id),
            AsyncBusError::ChannelClosed(id) => write!(f, "channel closed for agent: {}", id),
            AsyncBusError::Discovery(e) => write!(f, "discovery error: {}", e),
        }
    }
}

impl std::error::Error for AsyncBusError {}

/// Per-agent mailbox — an mpsc channel for incoming messages.
struct AgentMailbox {
    sender: mpsc::UnboundedSender<WireMessage>,
}

/// Async in-memory message bus using tokio channels.
///
/// Each registered agent gets an `mpsc::UnboundedSender<WireMessage>`.
/// Messages are delivered by sending to the recipient's channel.
/// No mutex contention on the hot path — only discovery uses RwLock.
pub struct InMemoryAsyncBus {
    /// Per-agent send channels (agent_id → channel sender).
    mailboxes: Arc<RwLock<HashMap<String, AgentMailbox>>>,
    /// Agent registry (for discovery + routing).
    discovery: Arc<RwLock<HashMap<AgentId, AgentInfo>>>,
    /// Router for capability-based dispatch.
    router: tokio::sync::Mutex<Router>,
}

impl InMemoryAsyncBus {
    /// Create a new async bus with default (FirstMatch) routing.
    pub fn new() -> Self {
        Self::with_strategy(RoutingStrategy::FirstMatch)
    }

    /// Create a new async bus with the given routing strategy.
    pub fn with_strategy(strategy: RoutingStrategy) -> Self {
        InMemoryAsyncBus {
            mailboxes: Arc::new(RwLock::new(HashMap::new())),
            discovery: Arc::new(RwLock::new(HashMap::new())),
            router: tokio::sync::Mutex::new(Router::new(strategy)),
        }
    }

    /// Register an agent. Returns a receiver half for incoming messages.
    pub async fn register_agent(
        &self,
        info: AgentInfo,
    ) -> Result<mpsc::UnboundedReceiver<WireMessage>, AsyncBusError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let key = info.id.as_str().to_string();

        self.mailboxes
            .write()
            .await
            .insert(key, AgentMailbox { sender: tx });

        self.discovery.write().await.insert(info.id.clone(), info);

        Ok(rx)
    }

    /// Deregister an agent and drop its mailbox.
    pub async fn deregister_agent(&self, id: &AgentId) {
        self.mailboxes.write().await.remove(id.as_str());
        self.discovery.write().await.remove(id);
    }

    /// Send a message to a specific agent.
    pub async fn send_to(
        &self,
        recipient: &AgentId,
        message: WireMessage,
    ) -> Result<(), AsyncBusError> {
        let mailboxes = self.mailboxes.read().await;
        let mailbox = mailboxes
            .get(recipient.as_str())
            .ok_or_else(|| AsyncBusError::AgentNotFound(recipient.clone()))?;
        mailbox
            .sender
            .send(message)
            .map_err(|_| AsyncBusError::ChannelClosed(recipient.clone()))
    }

    /// Route a Σ∞ program to an agent with the required capability.
    pub async fn send_sigma(
        &self,
        sender: &AgentId,
        packet: &SigmaPacket,
        capability: &Capability,
        correlation_id: u64,
    ) -> Result<(), AsyncBusError> {
        // Route: find target agent
        let target = {
            let mut router = self.router.lock().await;
            let disc = self.discovery.read().await;
            // Build a Discovery impl from the RwLock for routing
            let disc_read = DiscView(&disc);
            router
                .route(&disc_read, capability)
                .ok_or_else(|| AsyncBusError::NoRoute {
                    capability: capability.clone(),
                })?
        };

        let msg = WireMessage::new(
            MessageType::SigmaProgram,
            sender.clone(),
            Some(target.clone()),
            correlation_id,
            packet.to_string().into_bytes(),
        );

        self.send_to(&target, msg).await
    }

    /// Get the number of registered agents.
    pub async fn agent_count(&self) -> usize {
        self.discovery.read().await.len()
    }

    /// Check if an agent is registered.
    pub async fn has_agent(&self, id: &AgentId) -> bool {
        self.discovery.read().await.contains_key(id)
    }

    /// Discover agents matching a filter.
    pub async fn discover(&self, filter: &AgentFilter) -> Vec<AgentInfo> {
        let disc = self.discovery.read().await;
        let mut results: Vec<_> = disc
            .values()
            .filter(|info| match filter {
                AgentFilter::ByCapability(cap) => info.capabilities.contains(cap),
                AgentFilter::ByType(t) => info.agent_type == *t,
                AgentFilter::ById(id) => info.id == *id,
                AgentFilter::All => true,
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.id.cmp(&b.id));
        results
    }

    /// Get a cloned reference to the discovery store (for external Router use).
    pub async fn discovery_snapshot(&self) -> HashMap<AgentId, AgentInfo> {
        self.discovery.read().await.clone()
    }
}

impl Default for InMemoryAsyncBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only view of the discovery map, implementing the `Discovery` trait
/// for the Router (which expects `&dyn Discovery`).
struct DiscView<'a>(&'a HashMap<AgentId, AgentInfo>);

impl<'a> Discovery for DiscView<'a> {
    fn register(&mut self, _agent: AgentInfo) -> Result<(), DiscoveryError> {
        // Read-only view — registration goes through the bus.
        Err(DiscoveryError::AlreadyRegistered)
    }

    fn discover(&self, filter: &AgentFilter) -> Vec<AgentInfo> {
        let mut results: Vec<_> = self
            .0
            .values()
            .filter(|info| match filter {
                AgentFilter::ByCapability(cap) => info.capabilities.contains(cap),
                AgentFilter::ByType(t) => info.agent_type == *t,
                AgentFilter::ById(id) => info.id == *id,
                AgentFilter::All => true,
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.id.cmp(&b.id));
        results
    }

    fn mark_offline(&mut self, _id: &AgentId) {
        // Read-only view.
    }

    fn mark_online(&mut self, _id: &AgentId) {
        // Read-only view.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::{AgentType, Capability};
    use tokio::runtime::Runtime;

    fn rt() -> Runtime {
        Runtime::new().unwrap()
    }

    #[test]
    fn test_async_bus_register_and_count() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let _rx = bus
                .register_agent(AgentInfo::new(
                    AgentId::new("a1"),
                    AgentType::Cli,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();
            assert_eq!(bus.agent_count().await, 1);
            assert!(bus.has_agent(&AgentId::new("a1")).await);
        });
    }

    #[test]
    fn test_async_bus_send_and_receive() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let mut rx = bus
                .register_agent(AgentInfo::new(
                    AgentId::new("recv"),
                    AgentType::Cli,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();

            let sender = AgentId::new("sender");
            let msg = WireMessage::new(
                MessageType::Heartbeat,
                sender.clone(),
                Some(AgentId::new("recv")),
                1,
                vec![],
            );
            bus.send_to(&AgentId::new("recv"), msg).await.unwrap();

            let received = rx.recv().await.unwrap();
            assert_eq!(received.sender, sender);
            assert_eq!(received.msg_type, MessageType::Heartbeat);
        });
    }

    #[test]
    fn test_async_bus_send_sigma_routes() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let mut _rx = bus
                .register_agent(AgentInfo::new(
                    AgentId::new("cli-1"),
                    AgentType::Cli,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();

            let sender = AgentId::new("orch-1");
            let packet = SigmaPacket::new();
            bus.send_sigma(&sender, &packet, &Capability::Execute, 42)
                .await
                .unwrap();
        });
    }

    #[test]
    fn test_async_bus_no_route() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let sender = AgentId::new("orch-1");
            let packet = SigmaPacket::new();
            let result = bus
                .send_sigma(&sender, &packet, &Capability::Shell, 1)
                .await;
            assert!(matches!(result, Err(AsyncBusError::NoRoute { .. })));
        });
    }

    #[test]
    fn test_async_bus_deregister() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let id = AgentId::new("a1");
            let _rx = bus
                .register_agent(AgentInfo::new(
                    id.clone(),
                    AgentType::Cli,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();
            assert!(bus.has_agent(&id).await);
            bus.deregister_agent(&id).await;
            assert!(!bus.has_agent(&id).await);
        });
    }

    #[test]
    fn test_async_bus_discover() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let _rx1 = bus
                .register_agent(AgentInfo::new(
                    AgentId::new("cli-1"),
                    AgentType::Cli,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();
            let _rx2 = bus
                .register_agent(AgentInfo::new(
                    AgentId::new("orch-1"),
                    AgentType::Orchestrator,
                    vec![Capability::Execute],
                ))
                .await
                .unwrap();

            let all = bus.discover(&AgentFilter::All).await;
            assert_eq!(all.len(), 2);

            let cli_only = bus.discover(&AgentFilter::ByType(AgentType::Cli)).await;
            assert_eq!(cli_only.len(), 1);
            assert_eq!(cli_only[0].id.as_str(), "cli-1");
        });
    }

    #[test]
    fn test_async_bus_send_to_nonexistent() {
        rt().block_on(async {
            let bus = InMemoryAsyncBus::new();
            let msg = WireMessage::new(
                MessageType::Heartbeat,
                AgentId::new("sender"),
                Some(AgentId::new("ghost")),
                0,
                vec![],
            );
            let result = bus.send_to(&AgentId::new("ghost"), msg).await;
            assert!(matches!(result, Err(AsyncBusError::AgentNotFound(_))));
        });
    }
}
