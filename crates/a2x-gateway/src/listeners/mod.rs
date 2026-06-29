// See plans/06-entity-gateway.md §5
// Protocol listener trait and types.

pub mod http;
pub mod stdio;
pub mod tcp;
pub mod ws;

use crate::entity::EntityId;
use crate::error::GatewayError;

/// Type of protocol listener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolListenerType {
    Http,
    WebSocket,
    Tcp,
    Stdio,
}

impl std::fmt::Display for ProtocolListenerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolListenerType::Http => write!(f, "http"),
            ProtocolListenerType::WebSocket => write!(f, "websocket"),
            ProtocolListenerType::Tcp => write!(f, "tcp"),
            ProtocolListenerType::Stdio => write!(f, "stdio"),
        }
    }
}

/// A message received from an external entity (via any protocol).
#[derive(Clone, Debug)]
pub struct IncomingMessage {
    /// The entity that sent this message.
    pub entity_id: EntityId,
    /// The raw Σ∞ program text (for SigmaProgram) or serialized bytes.
    pub payload: Vec<u8>,
    /// Optional correlation ID for request-response matching.
    pub correlation_id: u64,
    /// Whether the payload is Σ∞ text (true) or binary Ω (false).
    pub is_text: bool,
}

/// A message to send back to an external entity.
#[derive(Clone, Debug)]
pub struct OutgoingMessage {
    /// The entity to send to.
    pub entity_id: EntityId,
    /// The raw payload (Σ∞ text or binary).
    pub payload: Vec<u8>,
    /// Correlation ID matching the request.
    pub correlation_id: u64,
    /// Whether the payload is Σ∞ text.
    pub is_text: bool,
}

/// A protocol listener accepts connections from external entities
/// on a specific protocol (HTTP, WebSocket, TCP, stdio).
///
/// See plans/06-entity-gateway.md §5 for protocol specifications.
pub trait ProtocolListener: Send {
    /// Get the type of this listener.
    fn listener_type(&self) -> ProtocolListenerType;

    /// Start listening (bind to address, begin accepting).
    fn start(&mut self) -> Result<(), GatewayError>;

    /// Stop the listener.
    fn stop(&mut self) -> Result<(), GatewayError>;

    /// Check if the listener is running.
    fn is_running(&self) -> bool;

    /// Get the bound address (if applicable).
    fn bound_address(&self) -> Option<String>;
}

/// Channel-based message bridge between listeners and the gateway.
///
/// Listeners send IncomingMessages on the tx side; the gateway reads from rx.
/// The gateway sends OutgoingMessages on the response_tx side;
/// listeners read from response_rx and deliver to entities.
pub struct MessageBridge {
    /// Incoming messages from entities to the gateway.
    pub incoming_tx: std::sync::mpsc::Sender<IncomingMessage>,
    pub incoming_rx: std::sync::mpsc::Receiver<IncomingMessage>,
    /// Outgoing messages from the gateway to entities.
    pub response_tx: std::sync::mpsc::Sender<OutgoingMessage>,
    pub response_rx: std::sync::mpsc::Receiver<OutgoingMessage>,
}

impl MessageBridge {
    pub fn new(_channel_size: usize) -> Self {
        let (incoming_tx, incoming_rx) = std::sync::mpsc::channel();
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        MessageBridge {
            incoming_tx,
            incoming_rx,
            response_tx,
            response_rx,
        }
    }
}
