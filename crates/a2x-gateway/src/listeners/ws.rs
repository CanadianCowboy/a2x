// See plans/06-entity-gateway.md §5 (WebSocket Listener)
//
// WebSocket listener for streaming bidirectional Σ∞/Ω communication.
//
// Protocol:
//   Client → Server: Raw Σ∞ text frames or binary Ω frames
//   Server → Client: Result packets, streaming data, events
//
// Frame format:
//   Text frame: Σ∞ packet source text (one packet per frame)
//   Binary frame: length-prefixed Ω packet (4-byte BE length + payload)

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::listeners::{IncomingMessage, OutgoingMessage, ProtocolListener, ProtocolListenerType};

/// WebSocket protocol listener.
///
/// Accepts WebSocket connections and bridges them to the gateway
/// for streaming bidirectional Σ∞/Ω communication.
pub struct WebSocketListener {
    bind_address: String,
    running: bool,
    incoming_tx: Option<std::sync::mpsc::Sender<IncomingMessage>>,
    response_rx: Option<std::sync::mpsc::Receiver<OutgoingMessage>>,
}

impl WebSocketListener {
    pub fn new(
        bind_address: impl Into<String>,
        incoming_tx: std::sync::mpsc::Sender<IncomingMessage>,
        response_rx: std::sync::mpsc::Receiver<OutgoingMessage>,
    ) -> Self {
        WebSocketListener {
            bind_address: bind_address.into(),
            running: false,
            incoming_tx: Some(incoming_tx),
            response_rx: Some(response_rx),
        }
    }

    /// Parse a text frame as a Σ∞ packet.
    pub fn parse_text_frame(
        entity_id: &EntityId,
        text: &str,
        correlation_id: u64,
    ) -> IncomingMessage {
        IncomingMessage {
            entity_id: entity_id.clone(),
            payload: text.as_bytes().to_vec(),
            correlation_id,
            is_text: true,
        }
    }

    /// Parse a binary frame as an Ω packet.
    pub fn parse_binary_frame(
        entity_id: &EntityId,
        data: &[u8],
        correlation_id: u64,
    ) -> Result<IncomingMessage, GatewayError> {
        if data.len() < 4 {
            return Err(GatewayError::Transport("frame too short".into()));
        }
        // First 4 bytes: big-endian length prefix
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len {
            return Err(GatewayError::Transport(format!(
                "incomplete frame: expected {} bytes, got {}",
                4 + len,
                data.len()
            )));
        }
        Ok(IncomingMessage {
            entity_id: entity_id.clone(),
            payload: data[4..4 + len].to_vec(),
            correlation_id,
            is_text: false,
        })
    }

    /// Format an outgoing message as a text frame.
    pub fn format_text_frame(msg: &OutgoingMessage) -> Option<String> {
        if msg.is_text {
            String::from_utf8(msg.payload.clone()).ok()
        } else {
            None
        }
    }

    /// Get a reference to the incoming message channel.
    ///
    /// Used by the async event loop to drain messages from WebSocket clients.
    pub fn incoming_sender(&self) -> Option<&std::sync::mpsc::Sender<IncomingMessage>> {
        self.incoming_tx.as_ref()
    }

    /// Get a reference to the outgoing message channel.
    ///
    /// Used by the async event loop to push responses to WebSocket clients.
    pub fn response_receiver(&self) -> Option<&std::sync::mpsc::Receiver<OutgoingMessage>> {
        self.response_rx.as_ref()
    }

    /// Format an outgoing message as a binary frame (with length prefix).
    pub fn format_binary_frame(msg: &OutgoingMessage) -> Vec<u8> {
        let mut frame = Vec::with_capacity(4 + msg.payload.len());
        frame.extend_from_slice(&(msg.payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&msg.payload);
        frame
    }
}

impl ProtocolListener for WebSocketListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::WebSocket
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        self.running = true;
        tracing::info!("WebSocket listener started on {}", self.bind_address);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        self.running = false;
        tracing::info!("WebSocket listener stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn bound_address(&self) -> Option<String> {
        if self.running {
            Some(self.bind_address.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_frame() {
        let eid = EntityId::new("ws-1");
        let msg = WebSocketListener::parse_text_frame(&eid, "⟦Σ∞⟧⟬I:✕⟭", 42);
        assert_eq!(msg.entity_id, eid);
        assert!(msg.is_text);
        assert_eq!(msg.correlation_id, 42);
        let input = "⟦Σ∞⟧⟬I:✕⟭";
        assert_eq!(msg.payload, input.as_bytes());
    }

    #[test]
    fn test_parse_binary_frame() {
        let eid = EntityId::new("ws-1");
        let payload = vec![0x00, 0x01, 0x02, 0x03];
        let mut frame = Vec::new();
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&payload);

        let msg = WebSocketListener::parse_binary_frame(&eid, &frame, 1).unwrap();
        assert!(!msg.is_text);
        assert_eq!(msg.payload, payload);
    }

    #[test]
    fn test_parse_binary_frame_too_short() {
        let eid = EntityId::new("ws-1");
        let result = WebSocketListener::parse_binary_frame(&eid, &[0x00, 0x01], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_binary_frame_incomplete() {
        let eid = EntityId::new("ws-1");
        // Length says 10 bytes but only 2 follow
        let mut frame = vec![0x00, 0x00, 0x00, 0x0A];
        frame.extend_from_slice(&[0x01, 0x02]);
        let result = WebSocketListener::parse_binary_frame(&eid, &frame, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_text_frame() {
        let msg = OutgoingMessage {
            entity_id: EntityId::new("ws-1"),
            payload: "⟦Σ∞⟧⟬I:✕⟭".as_bytes().to_vec(),
            correlation_id: 1,
            is_text: true,
        };
        let frame = WebSocketListener::format_text_frame(&msg);
        let expected = "⟦Σ∞⟧⟬I:✕⟭";
        assert_eq!(frame, Some(expected.into()));
    }

    #[test]
    fn test_format_binary_frame() {
        let msg = OutgoingMessage {
            entity_id: EntityId::new("ws-1"),
            payload: vec![0xDE, 0xAD],
            correlation_id: 1,
            is_text: false,
        };
        let frame = WebSocketListener::format_binary_frame(&msg);
        assert_eq!(&frame[0..4], &[0x00, 0x00, 0x00, 0x02]);
        assert_eq!(&frame[4..], &[0xDE, 0xAD]);
    }

    #[test]
    fn test_listener_lifecycle() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_rtx, rr) = std::sync::mpsc::channel();
        let mut listener = WebSocketListener::new("0.0.0.0:8779", tx, rr);
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        assert_eq!(listener.bound_address(), Some("0.0.0.0:8779".into()));
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
