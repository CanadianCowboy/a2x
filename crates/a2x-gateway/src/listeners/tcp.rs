// See plans/06-entity-gateway.md §5 (TCP Listener)
//
// Raw TCP socket listener. Each message is a length-prefixed serialized packet:
//   [4-byte BE length][serialized packet bytes]
//
// Uses the same framing format as a2x-bus::tcp_transport.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::listeners::{IncomingMessage, OutgoingMessage, ProtocolListener, ProtocolListenerType};

/// TCP protocol listener.
///
/// Accepts raw TCP connections with length-prefixed binary framing.
/// Each connection is handled on its own OS thread (blocking I/O).
/// Incoming frames are pushed to the channel bridge; outgoing messages
/// are broadcast to all connected clients via a shared writer list.
pub struct TcpListener {
    bind_address: String,
    running: bool,
    incoming_tx: Option<std::sync::mpsc::Sender<IncomingMessage>>,
    response_rx: Option<std::sync::mpsc::Receiver<OutgoingMessage>>,
    server_thread: Option<std::thread::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
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
            server_thread: None,
            shutdown_tx: None,
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
            is_text: false,
        })
    }

    pub fn incoming_sender(&self) -> Option<&std::sync::mpsc::Sender<IncomingMessage>> {
        self.incoming_tx.as_ref()
    }

    pub fn response_receiver(&self) -> Option<&std::sync::mpsc::Receiver<OutgoingMessage>> {
        self.response_rx.as_ref()
    }

    /// Format an outgoing message as a length-prefixed TCP frame.
    pub fn format_frame(msg: &OutgoingMessage) -> Vec<u8> {
        let mut frame = Vec::with_capacity(4 + msg.payload.len());
        frame.extend_from_slice(&(msg.payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&msg.payload);
        frame
    }

    /// Read exactly one length-prefixed frame from a TCP stream.
    fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let body_len = u32::from_be_bytes(len_buf) as usize;
        if body_len > 16 * 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("frame too large: {} bytes", body_len),
            ));
        }
        let mut body = vec![0u8; body_len];
        stream.read_exact(&mut body)?;
        let mut frame = Vec::with_capacity(4 + body_len);
        frame.extend_from_slice(&len_buf);
        frame.extend_from_slice(&body);
        Ok(frame)
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.running = false;
    }
}

impl ProtocolListener for TcpListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::Tcp
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        if self.running {
            return Err(GatewayError::ListenerError(
                "TCP listener is already running".into(),
            ));
        }

        if let Some(handle) = self.server_thread.take() {
            let _ = handle.join();
        }

        let incoming_tx = self
            .incoming_tx
            .as_ref()
            .ok_or_else(|| GatewayError::ListenerError("no incoming channel".into()))?
            .clone();

        let response_rx = self
            .response_rx
            .take()
            .ok_or_else(|| GatewayError::ListenerError("no response channel".into()))?;
        let response_rx = Arc::new(Mutex::new(response_rx));

        let addr = self.bind_address.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (ready_tx, ready_rx) =
            std::sync::mpsc::sync_channel::<Result<Option<std::net::SocketAddr>, String>>(1);

        let writers: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
        let writers_bg = writers.clone();
        let response_rx_bg = response_rx.clone();
        let broadcast_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let broadcast_done_bg = broadcast_done.clone();

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for TCP listener");

            rt.block_on(async move {
                // ── Bind ──────────────────────────────────────────
                let listener: tokio::net::TcpListener =
                    match tokio::net::TcpListener::bind(&addr).await {
                        Ok(l) => {
                            let local = l.local_addr().ok();
                            let _ = ready_tx.send(Ok(local));
                            l
                        }
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("bind failed: {}", e)));
                            tracing::error!("TCP listener failed to bind {}: {}", addr, e);
                            return;
                        }
                    };

                let local_addr: String = listener
                    .local_addr()
                    .map(|a: std::net::SocketAddr| a.to_string())
                    .unwrap_or_else(|_| addr.clone());
                tracing::info!("TCP listener serving on {}", local_addr);

                // ── Outgoing broadcast background thread ──────────
                let bg_handle = std::thread::spawn(move || loop {
                    let msg = response_rx_bg
                        .lock()
                        .unwrap()
                        .recv_timeout(std::time::Duration::from_millis(500));
                    match msg {
                        Ok(outgoing) => {
                            let frame = TcpListener::format_frame(&outgoing);
                            if let Ok(mut guard) = writers_bg.lock() {
                                guard.retain_mut(|stream: &mut TcpStream| {
                                    stream.write_all(&frame).is_ok() && stream.flush().is_ok()
                                });
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if broadcast_done_bg.load(std::sync::atomic::Ordering::Relaxed) {
                                break;
                            }
                        }
                    }
                });

                // ── Accept loop ───────────────────────────────────
                let mut conn_id: u64 = 0;
                loop {
                    let stream: tokio::net::TcpStream = tokio::select! {
                        result = listener.accept() => {
                            match result {
                                Ok((s, _)) => s,
                                Err(e) => {
                                    tracing::error!("TCP accept error: {}", e);
                                    continue;
                                }
                            }
                        }
                        _ = &mut shutdown_rx => {
                            break;
                        }
                    };

                    let std_stream: TcpStream = match stream.into_std() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!("TCP: failed to convert stream: {}", e);
                            continue;
                        }
                    };

                    conn_id = conn_id.wrapping_add(1);
                    let tx = incoming_tx.clone();
                    let w = writers.clone();
                    let peer_str: String = std_stream
                        .peer_addr()
                        .map(|a: std::net::SocketAddr| a.to_string())
                        .unwrap_or_else(|_| format!("conn-{}", conn_id));

                    match std_stream.try_clone() {
                        Ok(clone) => {
                            if let Ok(mut guard) = w.lock() {
                                guard.push(clone);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("TCP: failed to clone stream for broadcast: {}", e);
                        }
                    }

                    std::thread::spawn(move || {
                        let mut stream = std_stream;
                        let _ = stream.set_nonblocking(false);
                        let conn_entity = EntityId::new(format!("tcp-{}", peer_str));
                        loop {
                            match TcpListener::read_frame(&mut stream) {
                                Ok(frame) => {
                                    let msg =
                                        TcpListener::parse_frame(&conn_entity, &frame, conn_id)
                                            .unwrap_or_else(|_| IncomingMessage {
                                                entity_id: conn_entity.clone(),
                                                payload: frame,
                                                correlation_id: conn_id,
                                                is_text: false,
                                            });
                                    if tx.send(msg).is_err() {
                                        break;
                                    }
                                }
                                Err(ref e)
                                    if e.kind() == std::io::ErrorKind::UnexpectedEof
                                        || e.kind() == std::io::ErrorKind::ConnectionReset =>
                                {
                                    break;
                                }
                                Err(e) => {
                                    tracing::warn!("TCP read error from {}: {}", peer_str, e);
                                    break;
                                }
                            }
                        }
                    });
                }

                broadcast_done.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = bg_handle.join();
                tracing::info!("TCP listener shut down");
            });
        });

        // ── Wait for bind confirmation ───────────────────────────
        match ready_rx.recv() {
            Ok(Ok(_)) => {
                self.server_thread = Some(handle);
                self.shutdown_tx = Some(shutdown_tx);
                self.running = true;
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(GatewayError::ListenerError(e))
            }
            Err(_) => {
                let _ = handle.join();
                Err(GatewayError::ListenerError(
                    "server thread panicked during bind".into(),
                ))
            }
        }
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        if !self.running {
            return Ok(());
        }
        self.running = false;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_thread.take() {
            let backoff = std::time::Duration::from_millis(100);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                std::thread::sleep(backoff);
            }
        }
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
        let mut frame = vec![0x00, 0x00, 0x00, 0x64];
        frame.extend_from_slice(&[0x01, 0x02]);
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
        let mut listener = TcpListener::new("127.0.0.1:0", tx, rr);
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
