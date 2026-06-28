// See plans/04-bus.md §3

use crate::wire::WireMessage;

/// Error from transport operations.
#[derive(Clone, Debug, PartialEq)]
pub enum TransportError {
    ConnectionLost,
    SendFailed(String),
    RecvFailed(String),
    BindFailed(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::ConnectionLost => write!(f, "connection lost"),
            TransportError::SendFailed(msg) => write!(f, "send failed: {}", msg),
            TransportError::RecvFailed(msg) => write!(f, "recv failed: {}", msg),
            TransportError::BindFailed(msg) => write!(f, "bind failed: {}", msg),
        }
    }
}

impl std::error::Error for TransportError {}

/// Transport abstraction — how messages move between agents.
///
/// Phase 0: synchronous in-memory transport. Future phases add async, TCP, etc.
pub trait Transport: Send + Sync {
    /// Send a message to the given recipient address.
    fn send(&mut self, recipient: &str, message: WireMessage) -> Result<(), TransportError>;

    /// Receive and drain all pending messages for the given address.
    fn recv(&mut self, addr: &str) -> Result<Vec<WireMessage>, TransportError>;

    /// Register an address on the transport (creates a mailbox).
    fn register(&mut self, addr: &str) -> Result<(), TransportError>;

    /// Deregister an address and clear its mailbox.
    fn deregister(&mut self, addr: &str);
}

/// In-memory transport — uses HashMap-based mailboxes for local agent communication.
#[derive(Default)]
pub struct InMemoryTransport {
    mailboxes: std::collections::HashMap<String, Vec<WireMessage>>,
}

impl InMemoryTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for InMemoryTransport {
    fn send(&mut self, recipient: &str, message: WireMessage) -> Result<(), TransportError> {
        let mailbox = self
            .mailboxes
            .get_mut(recipient)
            .ok_or(TransportError::SendFailed(format!(
                "recipient '{}' not registered",
                recipient
            )))?;
        mailbox.push(message);
        Ok(())
    }

    fn recv(&mut self, addr: &str) -> Result<Vec<WireMessage>, TransportError> {
        let mailbox = self
            .mailboxes
            .get_mut(addr)
            .ok_or(TransportError::RecvFailed(format!(
                "address '{}' not registered",
                addr
            )))?;
        Ok(std::mem::take(mailbox))
    }

    fn register(&mut self, addr: &str) -> Result<(), TransportError> {
        self.mailboxes.entry(addr.to_string()).or_default();
        Ok(())
    }

    fn deregister(&mut self, addr: &str) {
        self.mailboxes.remove(addr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{MessageType, WIRE_VERSION};
    use a2x_core::AgentId;

    #[test]
    fn test_send_and_recv() {
        let mut transport = InMemoryTransport::new();
        transport.register("agent-1").unwrap();
        transport.register("agent-2").unwrap();

        let msg = WireMessage {
            version: WIRE_VERSION,
            msg_type: MessageType::Heartbeat,
            sender: AgentId::new("agent-1"),
            recipient: Some(AgentId::new("agent-2")),
            correlation_id: 1,
            timestamp: 0,
            payload: vec![],
        };
        transport.send("agent-2", msg).unwrap();

        let received = transport.recv("agent-2").unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].msg_type, MessageType::Heartbeat);

        // Mailbox should be empty after recv
        let received2 = transport.recv("agent-2").unwrap();
        assert!(received2.is_empty());
    }
}
