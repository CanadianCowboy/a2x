// Phase 7.3: Async TCP transport — bridges tokio TCP I/O into the async bus.
//
// See plans/10-concurrency.md §5 (async bus) and plans/04-bus.md §3 (transport).
//
// Uses the existing `tcp_transport` codec (encode_frame / decode_frame) for
// wire format compatibility. Provides async TCP listeners and connection
// handling via tokio, bridging to channels for async bus integration.

use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::tcp_transport::{decode_frame, encode_frame};
use crate::wire::WireMessage;
use a2x_core::AgentId;

/// Error type for async TCP transport operations.
#[derive(Clone, Debug)]
pub enum TcpAsyncError {
    BindFailed(String),
    ConnectFailed(String),
    SendFailed(String),
    RecvFailed(String),
    ChannelClosed,
    FrameTooLarge(usize),
}

impl std::fmt::Display for TcpAsyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpAsyncError::BindFailed(s) => write!(f, "bind failed: {}", s),
            TcpAsyncError::ConnectFailed(s) => write!(f, "connect failed: {}", s),
            TcpAsyncError::SendFailed(s) => write!(f, "send failed: {}", s),
            TcpAsyncError::RecvFailed(s) => write!(f, "recv failed: {}", s),
            TcpAsyncError::ChannelClosed => write!(f, "channel closed"),
            TcpAsyncError::FrameTooLarge(n) => write!(f, "frame too large: {} bytes", n),
        }
    }
}

impl std::error::Error for TcpAsyncError {}

/// Maximum frame size (16 MiB) and maximum receive buffer before dropping
/// a connection.
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Manages TCP listeners and bridges them to tokio channels.
///
/// Each bound address spawns an accept loop. Accepted connections are
/// handled by per-connection read tasks that push `WireMessage`s into
/// a channel. The caller integrates this channel with the async bus.
pub struct TcpAsyncBridge {
    /// Active bindings: address → shutdown signal sender.
    bindings: HashMap<String, BindingHandle>,
}

struct BindingHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl TcpAsyncBridge {
    pub fn new() -> Self {
        TcpAsyncBridge {
            bindings: HashMap::new(),
        }
    }

    /// Bind a TCP listener and start accepting connections.
    ///
    /// Returns a bounded receiver (capacity 128) for incoming messages
    /// from all accepted connections. Each message is paired with the
    /// connection's `AgentId` for routing.
    pub async fn bind(
        &mut self,
        addr: &str,
    ) -> Result<mpsc::Receiver<(AgentId, WireMessage)>, TcpAsyncError> {
        if self.bindings.contains_key(addr) {
            return Err(TcpAsyncError::BindFailed(format!(
                "already bound to {}",
                addr
            )));
        }

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| TcpAsyncError::BindFailed(e.to_string()))?;

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let (msg_tx, msg_rx) = mpsc::channel::<(AgentId, WireMessage)>(128);

        let bound_addr = addr.to_string();
        tokio::spawn(async move {
            let mut conn_id: u64 = 0;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer_addr)) => {
                                conn_id = conn_id.wrapping_add(1);
                                let tx = msg_tx.clone();
                                let conn_agent = AgentId::new(
                                    format!("tcp-{}", peer_addr),
                                );
                                tokio::spawn(handle_connection(
                                    stream, conn_agent, tx,
                                ));
                            }
                            Err(e) => {
                                eprintln!(
                                    "TCP async accept error on {}: {}",
                                    bound_addr, e
                                );
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
            eprintln!("TCP async bridge shut down for {}", bound_addr);
        });

        self.bindings
            .insert(addr.to_string(), BindingHandle { shutdown_tx });

        Ok(msg_rx)
    }

    /// Unbind and stop accepting connections.
    pub fn unbind(&mut self, addr: &str) {
        if let Some(handle) = self.bindings.remove(addr) {
            let _ = handle.shutdown_tx.send(());
        }
    }

    /// Open a TCP connection and send a single frame.
    pub async fn send_to(addr: &str, message: WireMessage) -> Result<(), TcpAsyncError> {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|e| TcpAsyncError::ConnectFailed(e.to_string()))?;

        let frame = encode_frame(&message);
        tokio::io::AsyncWriteExt::write_all(&mut stream, &frame)
            .await
            .map_err(|e| TcpAsyncError::SendFailed(e.to_string()))?;
        tokio::io::AsyncWriteExt::flush(&mut stream)
            .await
            .map_err(|e| TcpAsyncError::SendFailed(e.to_string()))?;

        Ok(())
    }
}

impl Default for TcpAsyncBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a single TCP connection: read bytes, drain complete frames,
/// decode them, and push to the channel.
async fn handle_connection(
    mut stream: TcpStream,
    conn_agent: AgentId,
    tx: mpsc::Sender<(AgentId, WireMessage)>,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut read_buf = [0u8; 8192];

    loop {
        match stream.read(&mut read_buf).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                buf.extend_from_slice(&read_buf[..n]);

                // Guard against unbounded buffer growth.
                if buf.len() > MAX_FRAME_SIZE {
                    eprintln!(
                        "TCP async: buffer exceeded {} bytes from {}, dropping connection",
                        MAX_FRAME_SIZE, conn_agent
                    );
                    break;
                }

                // Drain complete frames from the buffer.
                while let Ok((msg, consumed)) = decode_frame(&buf) {
                    if tx.send((conn_agent.clone(), msg)).await.is_err() {
                        // Channel closed — stop reading.
                        return;
                    }
                    buf.drain(..consumed);
                }

                // Compact if buffer grew large and was mostly consumed.
                if buf.capacity() > MAX_FRAME_SIZE && buf.len() < buf.capacity() / 4 {
                    buf.shrink_to(MAX_FRAME_SIZE);
                }
            }
            Err(_) => {
                // Connection closed or error — stop reading.
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{MessageType, WIRE_VERSION};
    use tokio::runtime::Runtime;

    fn rt() -> Runtime {
        Runtime::new().unwrap()
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = WireMessage::new(
            MessageType::Heartbeat,
            AgentId::new("agent-a"),
            Some(AgentId::new("agent-b")),
            1,
            vec![],
        );
        let frame = encode_frame(&msg);
        let (decoded, consumed) = decode_frame(&frame).unwrap();
        assert_eq!(consumed, frame.len());
        assert_eq!(decoded.version, WIRE_VERSION);
        assert_eq!(decoded.msg_type, MessageType::Heartbeat);
        assert_eq!(decoded.sender, AgentId::new("agent-a"));
        assert_eq!(decoded.recipient, Some(AgentId::new("agent-b")));
    }

    #[test]
    fn test_encode_decode_with_payload() {
        let msg = WireMessage::new(
            MessageType::SigmaProgram,
            AgentId::new("orch"),
            Some(AgentId::new("cli")),
            42,
            vec![0xDE, 0xAD, 0xBE, 0xEF],
        );
        let frame = encode_frame(&msg);
        let (decoded, _) = decode_frame(&frame).unwrap();
        assert_eq!(decoded.msg_type, MessageType::SigmaProgram);
    }

    #[test]
    fn test_decode_incomplete_frame() {
        let short = [0u8; 8];
        assert!(decode_frame(&short).is_err());
    }

    #[test]
    fn test_decode_truncated_body() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&100u32.to_be_bytes());
        buf.extend_from_slice(&[0u8; 4]);
        assert!(decode_frame(&buf).is_err());
    }

    #[test]
    fn test_bridge_bind_and_unbind() {
        rt().block_on(async {
            let mut bridge = TcpAsyncBridge::new();
            let rx = bridge.bind("127.0.0.1:0").await.unwrap();
            assert!(!rx.is_closed());
            bridge.unbind("127.0.0.1:0");
        });
    }

    #[test]
    fn test_send_to_unreachable_fails() {
        rt().block_on(async {
            let msg = WireMessage::new(
                MessageType::Heartbeat,
                AgentId::new("test"),
                None,
                0,
                vec![],
            );
            let result = TcpAsyncBridge::send_to("127.0.0.1:1", msg).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_double_bind_fails() {
        rt().block_on(async {
            let mut bridge = TcpAsyncBridge::new();
            let _rx = bridge.bind("127.0.0.1:0").await.unwrap();
            let result = bridge.bind("127.0.0.1:0").await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_send_and_receive_over_localhost() {
        rt().block_on(async {
            let mut bridge = TcpAsyncBridge::new();
            // Bind to port 0 to get an OS-assigned port, then connect send_to
            // via a second TcpAsyncBridge. We use two bridges: one listens,
            // the other sends. This verifies real TCP send/receive roundtrip.

            // Find an available port by binding a standalone listener first
            let scout = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = scout.local_addr().unwrap();
            let addr_str = addr.to_string();
            drop(scout); // release the port so our bridge can grab it

            let mut rx = bridge.bind(&addr_str).await.unwrap();

            // Send a message from a separate bridge to the listener
            let msg = WireMessage::new(
                MessageType::Heartbeat,
                AgentId::new("sender"),
                Some(AgentId::new("tcp-receiver")),
                42,
                vec![0xAA, 0xBB],
            );
            TcpAsyncBridge::send_to(&addr_str, msg.clone())
                .await
                .unwrap();

            // Receive the message on the listening side
            let (recv_agent, recv_msg) =
                tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                    .await
                    .expect("timeout waiting for message")
                    .expect("channel closed before message received");

            assert_eq!(recv_msg.sender, AgentId::new("sender"));
            assert_eq!(recv_msg.msg_type, MessageType::Heartbeat);
            assert_eq!(recv_msg.correlation_id, 42);
            assert_eq!(recv_msg.payload, vec![0xAA, 0xBB]);
            assert!(recv_agent.as_str().starts_with("tcp-"));

            bridge.unbind(&addr_str);
        });
    }

    #[test]
    fn test_send_multiple_messages_over_localhost() {
        rt().block_on(async {
            let mut bridge = TcpAsyncBridge::new();

            // Scout pattern: bind to port 0 to discover a free port, then
            // release it so our real bridge can grab it. There is a TOCTOU
            // race here (another process could grab the port between drop
            // and bind), but on localhost with a small delay window this
            // is negligible for tests.
            let scout = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = scout.local_addr().unwrap().to_string();
            drop(scout);

            let mut rx = bridge.bind(&addr).await.unwrap();

            // Send three messages — each send_to creates a separate TCP
            // connection, so ordering across connections is NOT guaranteed.
            let count = 3u64;
            for i in 0..count {
                let msg = WireMessage::new(
                    MessageType::Heartbeat,
                    AgentId::new("sender"),
                    None,
                    i,
                    vec![i as u8],
                );
                TcpAsyncBridge::send_to(&addr, msg).await.unwrap();
            }

            // Collect all messages and sort by correlation_id (order is
            // not guaranteed across separate TCP connections).
            let mut received: Vec<WireMessage> = Vec::new();
            for _ in 0..count {
                let (_agent, msg) =
                    tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                        .await
                        .expect("timeout")
                        .expect("channel closed");
                received.push(msg);
            }
            received.sort_by_key(|m| m.correlation_id);

            for (i, msg) in received.iter().enumerate() {
                assert_eq!(msg.correlation_id, i as u64);
                assert_eq!(msg.payload, vec![i as u8]);
            }

            bridge.unbind(&addr);
        });
    }

    #[test]
    fn test_many_messages_over_localhost() {
        rt().block_on(async {
            let mut bridge = TcpAsyncBridge::new();

            // Scout pattern for port discovery (see comment above).
            let scout = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = scout.local_addr().unwrap().to_string();
            drop(scout);

            let mut rx = bridge.bind(&addr).await.unwrap();

            // Send 96 messages — well within channel capacity (128) but
            // enough to stress the accept/spawn/drain pipeline.
            let count = 96u64;
            for i in 0..count {
                let msg = WireMessage::new(
                    MessageType::Heartbeat,
                    AgentId::new("sender"),
                    None,
                    i,
                    vec![],
                );
                TcpAsyncBridge::send_to(&addr, msg).await.unwrap();
            }

            // Drain all messages with timeout and explicit error on failure.
            let mut received = 0u64;
            for _ in 0..count {
                match tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv()).await {
                    Ok(Some((_, msg))) => {
                        assert!(
                            msg.correlation_id < count,
                            "unexpected correlation_id {} outside 0..{}",
                            msg.correlation_id,
                            count
                        );
                        received += 1;
                    }
                    Ok(None) => {
                        panic!(
                            "channel closed after receiving {}/{} messages",
                            received, count
                        );
                    }
                    Err(_elapsed) => {
                        panic!("timeout after receiving {}/{} messages", received, count);
                    }
                }
            }
            assert_eq!(received, count, "all messages should be received");

            bridge.unbind(&addr);
        });
    }
}
