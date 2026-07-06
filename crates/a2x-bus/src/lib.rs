// a2x-bus — Message bus, routing, transport, agent discovery
// See plans/04-bus.md

pub mod bridge;
pub mod bus;
pub mod discovery;
pub mod routing;
pub mod tcp_transport;
pub mod transport;
pub mod wire;

// Phase 7.1: Async bus
#[cfg(feature = "tokio")]
pub mod async_bus;

// TLS transport (behind tls feature)
#[cfg(feature = "tls")]
pub mod tls;

// Phase 7.3: Async TCP transport
#[cfg(feature = "tokio")]
pub mod async_tcp;

// Phase 8: Ed25519 agent identity (T4-1)
#[cfg(feature = "ed25519")]
pub mod identity;

// Re-export key types
pub use bridge::{event_to_sigma, BusBridge};
pub use bus::{Bus, BusError};
pub use discovery::{
    AgentCard, AgentFilter, AgentHandshake, AgentInfo, Discovery, DiscoveryError, InMemoryDiscovery,
};
pub use routing::{Router, RoutingStrategy};
pub use tcp_transport::TcpTransport;
pub use transport::{InMemoryTransport, Transport, TransportError};
pub use wire::{MessageType, WireError, WireMessage, WIRE_VERSION};

// Phase 7.1: Async re-exports
#[cfg(feature = "tokio")]
pub use async_bus::{AsyncBusError, InMemoryAsyncBus};

// Phase 7.3: Async TCP re-exports
#[cfg(feature = "tokio")]
pub use async_tcp::{TcpAsyncBridge, TcpAsyncError};
#[cfg(feature = "ed25519")]
pub use identity::{verify_signed_message, AgentIdentity, SignedWireMessage};

// TLS re-exports
#[cfg(feature = "tls")]
pub use tls::{TlsConfig, TlsError, TlsTransport};
