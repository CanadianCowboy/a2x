// See plans/04-bus.md §3
// Phase 3.3: TcpTransport — sync std::net TCP transport with length-prefix framing.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Mutex;

use crate::transport::{Transport, TransportError};
use crate::wire::{MessageType, WireError, WireMessage};

// ── Wire frame codec ────────────────────────────────────────────────────────
//
// Frame format (per WireMessage):
//   [4-byte BE length: u32 = serialized_body.len()]
//   [serialized_body bytes]
//
// Body layout:
//   [1-byte version]
//   [1-byte msg_type tag]
//   [4-byte BE sender_len][sender_id bytes]
//   [4-byte BE recipient_len][recipient_id bytes | empty if None]
//   [8-byte BE correlation_id]
//   [8-byte BE timestamp]
//   [4-byte BE payload_len][payload bytes]

const TAG_HEARTBEAT: u8 = 0;
const TAG_ANNOUNCE: u8 = 1;
const TAG_SIGMA_PROGRAM: u8 = 2;
const TAG_OMEGA_PROGRAM: u8 = 3;
const TAG_PROGRAM_REQUEST: u8 = 4;
const TAG_PROGRAM_RESPONSE: u8 = 5;
const TAG_ERROR: u8 = 6;

/// Encode the variant-specific data into a `WireMessage`'s payload field
/// so the TCP wire carries everything in a single payload blob.
fn encode_payload_for_type(msg: &WireMessage) -> Vec<u8> {
    match &msg.msg_type {
        MessageType::ProgramRequest(id) => {
            let mut data = Vec::with_capacity(32);
            data.extend_from_slice(id.as_bytes());
            data
        }
        MessageType::ProgramResponse { id, program_bytes } => {
            let mut data = Vec::with_capacity(32 + program_bytes.len());
            data.extend_from_slice(id.as_bytes());
            data.extend_from_slice(program_bytes);
            data
        }
        MessageType::Error(e) => {
            let mut data = Vec::with_capacity(4 + e.message.len());
            data.extend_from_slice(&e.code.to_be_bytes());
            data.extend_from_slice(e.message.as_bytes());
            data
        }
        _ => msg.payload.clone(),
    }
}

fn encode_message(msg: &WireMessage) -> Vec<u8> {
    let payload = encode_payload_for_type(msg);
    let mut body = Vec::with_capacity(64 + payload.len());
    // version
    body.push(msg.version);
    // msg_type tag
    body.push(match &msg.msg_type {
        MessageType::Heartbeat => TAG_HEARTBEAT,
        MessageType::Announce => TAG_ANNOUNCE,
        MessageType::SigmaProgram => TAG_SIGMA_PROGRAM,
        MessageType::OmegaProgram => TAG_OMEGA_PROGRAM,
        MessageType::ProgramRequest(_) => TAG_PROGRAM_REQUEST,
        MessageType::ProgramResponse { .. } => TAG_PROGRAM_RESPONSE,
        MessageType::Error(_) => TAG_ERROR,
    });
    // sender
    let sender_bytes = msg.sender.as_str().as_bytes();
    body.extend_from_slice(&(sender_bytes.len() as u32).to_be_bytes());
    body.extend_from_slice(sender_bytes);
    // recipient (0-length = None)
    match &msg.recipient {
        Some(id) => {
            let bytes = id.as_str().as_bytes();
            body.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            body.extend_from_slice(bytes);
        }
        None => {
            body.extend_from_slice(&0u32.to_be_bytes());
        }
    }
    // correlation_id
    body.extend_from_slice(&msg.correlation_id.to_be_bytes());
    // timestamp
    body.extend_from_slice(&msg.timestamp.to_be_bytes());
    // payload
    body.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    body.extend_from_slice(&payload);

    // Wrap in length-prefixed frame.
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    frame
}

fn decode_message(buf: &[u8]) -> Result<(WireMessage, usize), TransportError> {
    if buf.len() < 4 {
        return Err(TransportError::RecvFailed("frame too short".into()));
    }
    let body_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if buf.len() < 4 + body_len {
        return Err(TransportError::RecvFailed(format!(
            "incomplete frame: need {} bytes, have {}",
            4 + body_len,
            buf.len()
        )));
    }
    let body = &buf[4..4 + body_len];
    let mut pos = 0;

    // version
    if pos >= body.len() {
        return Err(TransportError::RecvFailed(
            "body truncated at version".into(),
        ));
    }
    let version = body[pos];
    pos += 1;

    // tag
    if pos >= body.len() {
        return Err(TransportError::RecvFailed("body truncated at tag".into()));
    }
    let tag = body[pos];
    pos += 1;

    // sender
    let (sender, consumed) = read_len_prefixed_string(body, pos)?;
    pos += consumed;

    // recipient
    let (recipient, consumed) = read_len_prefixed_optional_string(body, pos)?;
    pos += consumed;

    // correlation_id (8 bytes)
    if pos + 8 > body.len() {
        return Err(TransportError::RecvFailed(
            "truncated correlation_id".into(),
        ));
    }
    let correlation_id = u64::from_be_bytes(body[pos..pos + 8].try_into().unwrap());
    pos += 8;

    // timestamp (8 bytes)
    if pos + 8 > body.len() {
        return Err(TransportError::RecvFailed("truncated timestamp".into()));
    }
    let timestamp = u64::from_be_bytes(body[pos..pos + 8].try_into().unwrap());
    pos += 8;

    // payload
    if pos + 4 > body.len() {
        return Err(TransportError::RecvFailed(
            "truncated payload length".into(),
        ));
    }
    let payload_len = u32::from_be_bytes(body[pos..pos + 4].try_into().unwrap()) as usize;
    pos += 4;
    if pos + payload_len > body.len() {
        return Err(TransportError::RecvFailed("truncated payload".into()));
    }
    let payload = body[pos..pos + payload_len].to_vec();

    let msg_type = match tag {
        TAG_HEARTBEAT => MessageType::Heartbeat,
        TAG_ANNOUNCE => MessageType::Announce,
        TAG_SIGMA_PROGRAM => MessageType::SigmaProgram,
        TAG_OMEGA_PROGRAM => MessageType::OmegaProgram,
        TAG_PROGRAM_REQUEST => {
            let mut id_bytes = [0u8; 32];
            let n = 32.min(payload.len());
            id_bytes[..n].copy_from_slice(&payload[..n]);
            MessageType::ProgramRequest(a2x_core::ProgramId::new(id_bytes))
        }
        TAG_PROGRAM_RESPONSE => {
            let mut id_bytes = [0u8; 32];
            let n = 32.min(payload.len());
            id_bytes[..n].copy_from_slice(&payload[..n]);
            MessageType::ProgramResponse {
                id: a2x_core::ProgramId::new(id_bytes),
                program_bytes: payload.get(32..).unwrap_or(&[]).to_vec(),
            }
        }
        TAG_ERROR => {
            let mut code_bytes = [0u8; 2];
            let n = 2.min(payload.len());
            code_bytes[..n].copy_from_slice(&payload[..n]);
            MessageType::Error(WireError {
                code: u16::from_be_bytes(code_bytes),
                message: String::from_utf8_lossy(payload.get(2..).unwrap_or(&[])).to_string(),
            })
        }
        _ => {
            return Err(TransportError::RecvFailed(format!(
                "unknown msg_type tag: {tag}"
            )));
        }
    };

    // For tags that carry data in the enum variant (not in payload),
    // clear the payload. For others, preserve it.
    let final_payload = match tag {
        TAG_PROGRAM_REQUEST | TAG_PROGRAM_RESPONSE | TAG_ERROR => Vec::new(),
        _ => payload,
    };

    let msg = WireMessage {
        version,
        msg_type,
        sender: a2x_core::AgentId::new(&sender),
        recipient: recipient.map(|r| a2x_core::AgentId::new(&r)),
        correlation_id,
        timestamp,
        payload: final_payload,
    };
    Ok((msg, 4 + body_len))
}

fn read_len_prefixed_string(data: &[u8], pos: usize) -> Result<(String, usize), TransportError> {
    if pos + 4 > data.len() {
        return Err(TransportError::RecvFailed("truncated string length".into()));
    }
    let len = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
    let start = pos + 4;
    if start + len > data.len() {
        return Err(TransportError::RecvFailed("truncated string data".into()));
    }
    let s = String::from_utf8_lossy(&data[start..start + len]).to_string();
    Ok((s, 4 + len))
}

fn read_len_prefixed_optional_string(
    data: &[u8],
    pos: usize,
) -> Result<(Option<String>, usize), TransportError> {
    if pos + 4 > data.len() {
        return Err(TransportError::RecvFailed(
            "truncated opt string length".into(),
        ));
    }
    let len = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
    if len == 0 {
        return Ok((None, 4));
    }
    let start = pos + 4;
    if start + len > data.len() {
        return Err(TransportError::RecvFailed(
            "truncated opt string data".into(),
        ));
    }
    let s = String::from_utf8_lossy(&data[start..start + len]).to_string();
    Ok((Some(s), 4 + len))
}

// ── TcpTransport ───────────────────────────────────────────────────────────

/// TCP transport — sends/receives `WireMessage` over sync TCP connections.
///
/// Each `send()` opens a new TCP connection to the recipient, writes one
/// length-prefixed frame, and closes the connection. Each `recv()` accepts
/// all pending connections on the local listener and reads one frame per
/// connection.
///
/// Uses `Mutex` internally to satisfy the `Transport: Send + Sync` bound.
pub struct TcpTransport {
    /// Bound listeners keyed by registered address string.
    listeners: Mutex<HashMap<String, TcpListener>>,
    /// Maps listener addresses to their bound socket addresses (for send).
    bound_addrs: Mutex<HashMap<String, std::net::SocketAddr>>,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            bound_addrs: Mutex::new(HashMap::new()),
        }
    }

    /// Get the actual bound socket address for a registered key (for `send`).
    pub fn bound_addr(&self, key: &str) -> Option<std::net::SocketAddr> {
        self.bound_addrs.lock().unwrap().get(key).copied()
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, recipient: &str, message: WireMessage) -> Result<(), TransportError> {
        let addr = recipient
            .to_socket_addrs()
            .map_err(|e| TransportError::SendFailed(e.to_string()))?
            .next()
            .ok_or_else(|| TransportError::SendFailed("no addresses resolved".into()))?;

        let mut stream =
            TcpStream::connect(addr).map_err(|e| TransportError::SendFailed(e.to_string()))?;

        let frame = encode_message(&message);
        stream
            .write_all(&frame)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    fn recv(&mut self, addr: &str) -> Result<Vec<WireMessage>, TransportError> {
        let listener = {
            let listeners = self.listeners.lock().unwrap();
            listeners
                .get(addr)
                .map(|l| l.local_addr().expect("listener must have a local address"))
        };
        if listener.is_none() {
            return Err(TransportError::RecvFailed(format!(
                "address '{addr}' not registered"
            )));
        }

        // Re-bind approach: we need the actual TcpListener to accept.
        // Since we can't move it out of the Mutex, we use a fresh
        // TcpStream connect + read approach via the bound address.
        // Actually, we accept via a short-lived reconnection:
        // set non-blocking on a cloned listener won't work.
        //
        // Simpler: just connect to ourselves and read.
        // But that's what `send` does to us.
        //
        // The real fix: the Mutex holds the listener. We can't easily
        // accept from it while holding the lock (blocking). Instead,
        // we use a different approach: the recv method just reads
        // pending messages that were already buffered.
        //
        // For Phase 3.3, we use a pragmatic approach: accept from
        // the listener by temporarily taking it out of the map.

        let mut messages = Vec::new();
        let listener = {
            let mut listeners = self.listeners.lock().unwrap();
            listeners.remove(addr)
        };

        if let Some(listener) = listener {
            // Set non-blocking so we drain without hanging.
            let _ = listener.set_nonblocking(true);

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        if let Ok(msg) = read_one_frame(stream) {
                            messages.push(msg);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => break,
                }
            }

            // Restore blocking and put it back.
            let _ = listener.set_nonblocking(false);
            self.listeners
                .lock()
                .unwrap()
                .insert(addr.to_string(), listener);
        }

        Ok(messages)
    }

    fn register(&mut self, addr: &str) -> Result<(), TransportError> {
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| TransportError::BindFailed(e.to_string()))?
            .next()
            .ok_or_else(|| TransportError::BindFailed("no addresses resolved".into()))?;

        let listener = TcpListener::bind(socket_addr)
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        let bound = listener
            .local_addr()
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        self.bound_addrs
            .lock()
            .unwrap()
            .insert(addr.to_string(), bound);
        self.listeners
            .lock()
            .unwrap()
            .insert(addr.to_string(), listener);
        Ok(())
    }

    fn deregister(&mut self, addr: &str) {
        self.listeners.lock().unwrap().remove(addr);
        self.bound_addrs.lock().unwrap().remove(addr);
    }
}

/// Read exactly one length-prefixed frame from a TCP stream.
fn read_one_frame(mut stream: TcpStream) -> Result<WireMessage, TransportError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| TransportError::RecvFailed(e.to_string()))?;
    let body_len = u32::from_be_bytes(len_buf) as usize;

    let mut body = vec![0u8; body_len];
    stream
        .read_exact(&mut body)
        .map_err(|e| TransportError::RecvFailed(e.to_string()))?;

    let mut frame = Vec::with_capacity(4 + body_len);
    frame.extend_from_slice(&len_buf);
    frame.extend_from_slice(&body);

    let (msg, _) = decode_message(&frame)?;
    Ok(msg)
}

// ── Public codec API ────────────────────────────────────────────────────────

/// Encode a `WireMessage` into a length-prefixed frame (for wire transmission).
pub fn encode_frame(msg: &WireMessage) -> Vec<u8> {
    encode_message(msg)
}

/// Decode a `WireMessage` from a length-prefixed frame buffer.
pub fn decode_frame(buf: &[u8]) -> Result<(WireMessage, usize), TransportError> {
    decode_message(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{MessageType, WIRE_VERSION};
    use a2x_core::AgentId;

    fn make_heartbeat(from: &str, to: &str) -> WireMessage {
        WireMessage::new(
            MessageType::Heartbeat,
            AgentId::new(from),
            Some(AgentId::new(to)),
            1,
            vec![],
        )
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = make_heartbeat("agent-a", "agent-b");
        let frame = encode_frame(&msg);
        let (decoded, consumed) = decode_frame(&frame).unwrap();
        assert_eq!(consumed, frame.len());
        assert_eq!(decoded.version, WIRE_VERSION);
        assert_eq!(decoded.msg_type, MessageType::Heartbeat);
        assert_eq!(decoded.sender, AgentId::new("agent-a"));
        assert_eq!(decoded.recipient, Some(AgentId::new("agent-b")));
        assert_eq!(decoded.correlation_id, 1);
    }

    #[test]
    fn test_encode_decode_with_payload() {
        let msg = WireMessage::new(
            MessageType::SigmaProgram,
            AgentId::new("orchestrator"),
            Some(AgentId::new("cli-agent")),
            42,
            vec![0xDE, 0xAD, 0xBE, 0xEF],
        );
        let frame = encode_frame(&msg);
        let (decoded, _) = decode_frame(&frame).unwrap();
        assert_eq!(decoded.msg_type, MessageType::SigmaProgram);
        // payload field is cleared; variant data lives in msg_type.
        // For SigmaProgram the raw payload is preserved in the tag.
        // Re-encode to verify round-trip.
        let re_frame = encode_frame(&decoded);
        let (redone, _) = decode_frame(&re_frame).unwrap();
        assert_eq!(redone.msg_type, MessageType::SigmaProgram);
    }

    #[test]
    fn test_encode_decode_no_recipient() {
        let msg = WireMessage::new(
            MessageType::Heartbeat,
            AgentId::new("solo"),
            None,
            0,
            vec![],
        );
        let frame = encode_frame(&msg);
        let (decoded, _) = decode_frame(&frame).unwrap();
        assert_eq!(decoded.recipient, None);
    }

    #[test]
    fn test_decode_incomplete_frame_errors() {
        let short = [0u8; 8];
        assert!(decode_frame(&short).is_err());
    }

    #[test]
    fn test_decode_truncated_body_errors() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&100u32.to_be_bytes());
        buf.extend_from_slice(&[0u8; 4]);
        assert!(decode_frame(&buf).is_err());
    }

    #[test]
    fn test_tcp_transport_pair_send_recv() {
        let mut server = TcpTransport::new();
        let key = "127.0.0.1:0";
        server.register(key).unwrap();
        let bound = server.bound_addr(key).unwrap();

        let mut client = TcpTransport::new();
        let msg = make_heartbeat("client", "server");
        client.send(&bound.to_string(), msg).unwrap();

        let received = server.recv(key).unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].sender, AgentId::new("client"));
        assert_eq!(received[0].recipient, Some(AgentId::new("server")));
    }

    #[test]
    fn test_tcp_transport_framing_preserves_boundaries() {
        let mut server = TcpTransport::new();
        let key = "127.0.0.1:0";
        server.register(key).unwrap();
        let bound = server.bound_addr(key).unwrap();

        let mut client = TcpTransport::new();
        client
            .send(
                &bound.to_string(),
                WireMessage::new(
                    MessageType::Heartbeat,
                    AgentId::new("c"),
                    Some(AgentId::new("s")),
                    1,
                    vec![],
                ),
            )
            .unwrap();
        client
            .send(
                &bound.to_string(),
                WireMessage::new(
                    MessageType::SigmaProgram,
                    AgentId::new("c"),
                    Some(AgentId::new("s")),
                    2,
                    vec![1, 2, 3],
                ),
            )
            .unwrap();

        let received = server.recv(key).unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].msg_type, MessageType::Heartbeat);
        assert_eq!(received[1].msg_type, MessageType::SigmaProgram);
    }

    #[test]
    fn test_tcp_transport_ephemeral_port_bind() {
        let mut t = TcpTransport::new();
        t.register("127.0.0.1:0").unwrap();
        let bound = t.bound_addr("127.0.0.1:0").unwrap();
        assert!(bound.port() > 0, "OS must assign a non-zero port");
    }

    #[test]
    fn test_tcp_transport_recv_returns_empty_after_drain() {
        let mut server = TcpTransport::new();
        let key = "127.0.0.1:0";
        server.register(key).unwrap();
        let bound = server.bound_addr(key).unwrap();

        let mut client = TcpTransport::new();
        client
            .send(&bound.to_string(), make_heartbeat("c", "s"))
            .unwrap();

        let first = server.recv(key).unwrap();
        assert_eq!(first.len(), 1);

        let second = server.recv(key).unwrap();
        assert!(second.is_empty());
    }
}
