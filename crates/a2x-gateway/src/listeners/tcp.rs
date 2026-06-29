// See plans/06-entity-gateway.md §5 (TCP Listener)
//
// Raw TCP socket listener. Each message is a length-prefixed serialized packet:
//   [4-byte BE length][serialized packet bytes]
//
// Uses the same framing format as a2x-bus::tcp_transport.

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::listeners::{IncomingMessage, OutgoingMessage, ProtocolListener, ProtocolListenerType};

/// TCP protocol listener.
///
/// Accepts raw TCP connections with length-prefixed binary framing.
pub struct TcpListener {
    bind_address: String,
    running: bool,
    #[allow(dead_code)]
    incoming_tx: Option<std::sync::mpsc::Sender<IncomingMessage>>,
    #[allow(dead_code)]
    response_rx: Option<std::sync::mpsc::Receiver<OutgoingMessage>>,
}

impl TcpListener {
    pub fn new(
        bind_address: impl Into<String>,
        incoming_tx: std::sync::mpsc::Sender<IncomingMessage>,
        response_rx: std::sync::mpsc::Receiver<OutgoingMessage>,
    ) -> Self {
        TcpListener {
            bind_address: bind_address.into(),
            running: false,
            incoming_tx: Some(incoming_tx),
            response_rx: Some(response_rx),
        }
    }

    /// Parse a length-prefixed TCP frame.
    ///
    /// Frame format: [4-byte BE length][payload bytes]
    pub fn parse_frame(
        entity_id: &EntityId,
        data: &[u8],
        correlation_id: u64,
    ) -> Result<IncomingMessage, GatewayError> {
        if data.len() < 4 {
            return Err(GatewayError::Transport("frame too short".into()));
        }
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len {
            return Err(GatewayError::Transport(format!(
                "incomplete frame: need {} bytes, have {}",
                4 + len,
                data.len()
            )));
        }
        Ok(IncomingMessage {
            entity_id: entity_id.clone(),
            payload: data[4..4 + len].to_vec(),
            correlation_id,
            is_text: false, // TCP is binary by default
        })
    }

    /// Format an outgoing message as a length-prefixed TCP frame.
    pub fn format_frame(msg: &OutgoingMessage) -> Vec<u8> {
        let mut frame = Vec::with_capacity(4 + msg.payload.len());
        frame.extend_from_slice(&(msg.payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&msg.payload);
        frame
    }
}

impl ProtocolListener for TcpListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::Tcp
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        self.running = true;
        tracing::info!("TCP listener started on {}", self.bind_address);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        self.running = false;
        tracing::info!("TCP listener stopped");
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
    fn test_parse_frame_valid() {
        let eid = EntityId::new("tcp-1");
        let payload = vec![0x01, 0x02, 0x03];
        let mut frame = Vec::new();
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&payload);

        let msg = TcpListener::parse_frame(&eid, &frame, 99).unwrap();
        assert_eq!(msg.entity_id, eid);
        assert_eq!(msg.payload, payload);
        assert_eq!(msg.correlation_id, 99);
    }

    #[test]
    fn test_parse_frame_too_short() {
        let eid = EntityId::new("tcp-1");
        let result = TcpListener::parse_frame(&eid, &[0x00], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frame_incomplete() {
        let eid = EntityId::new("tcp-1");
        let mut frame = vec![0x00, 0x00, 0x00, 0x64]; // len=100
        frame.extend_from_slice(&[0x01, 0x02]); // only 2 bytes
        let result = TcpListener::parse_frame(&eid, &frame, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_frame() {
        let msg = OutgoingMessage {
            entity_id: EntityId::new("tcp-1"),
            payload: vec![0xAA, 0xBB],
            correlation_id: 42,
            is_text: false,
        };
        let frame = TcpListener::format_frame(&msg);
        assert_eq!(&frame[0..4], &[0x00, 0x00, 0x00, 0x02]);
        assert_eq!(&frame[4..], &[0xAA, 0xBB]);
    }

    #[test]
    fn test_roundtrip() {
        let eid = EntityId::new("tcp-1");
        let payload = b"hello A2X";
        let mut frame = Vec::new();
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);

        let msg = TcpListener::parse_frame(&eid, &frame, 1).unwrap();
        let outgoing = OutgoingMessage {
            entity_id: msg.entity_id.clone(),
            payload: msg.payload.clone(),
            correlation_id: msg.correlation_id,
            is_text: false,
        };
        let re_frame = TcpListener::format_frame(&outgoing);
        assert_eq!(re_frame, frame);
    }

    #[test]
    fn test_listener_lifecycle() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_rtx, rr) = std::sync::mpsc::channel();
        let mut listener = TcpListener::new("0.0.0.0:8780", tx, rr);
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
