// a2x-bus — Message bus, routing, transport, agent discovery
// See plans/04-bus.md

pub mod bus;
pub mod discovery;
pub mod routing;
pub mod tcp_transport;
pub mod transport;
pub mod wire;

// Re-export key types
pub use bus::{Bus, BusError};
pub use discovery::{AgentFilter, AgentInfo, Discovery, DiscoveryError, InMemoryDiscovery};
pub use routing::{Router, RoutingStrategy};
pub use tcp_transport::TcpTransport;
pub use transport::{InMemoryTransport, Transport, TransportError};
pub use wire::{MessageType, WireError, WireMessage, WIRE_VERSION};
