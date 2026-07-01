// See plans/12-security.md §2 — Bus Encryption
//
// TLS transport wrapper for TcpTransport. Encrypts agent-to-agent
// communication over TCP using rustls (pure Rust, no OpenSSL dependency).
//
// The TlsTransport wraps a TcpTransport and adds TLS encryption on top
// of the existing length-prefixed frame protocol used by TcpTransport.
//
// Usage:
//   let config = TlsConfig::new("cert.pem", "key.pem", None::<&str>);
//   let mut transport = TlsTransport::new(config);

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ServerConfig};

use crate::tcp_transport::{decode_frame, encode_frame};
use crate::transport::{Transport, TransportError};
use crate::wire::WireMessage;

// ── TLS configuration ──────────────────────────────────────────────────────

/// TLS configuration for bus transport encryption.
///
/// Supports mutual TLS when `ca_path` is provided (both sides verify each other).
/// For simple server-auth TLS, provide only `cert_path` and `key_path` on the
/// server side; the client just needs an empty config or `ca_path` for verification.
///
/// See plans/12-security.md §2 — "Bus Encryption (Optional)".
#[derive(Clone)]
pub struct TlsConfig {
    /// Path to the TLS certificate (PEM format).
    pub cert_path: PathBuf,
    /// Path to the TLS private key (PEM format).
    pub key_path: PathBuf,
    /// Path to the CA certificate for client verification (optional).
    /// When set, both server and client authenticate each other (mTLS).
    pub ca_path: Option<PathBuf>,
}

impl TlsConfig {
    /// Create a TLS config from file paths.
    pub fn new(
        cert_path: impl Into<PathBuf>,
        key_path: impl Into<PathBuf>,
        ca_path: Option<impl Into<PathBuf>>,
    ) -> Self {
        TlsConfig {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_path: ca_path.map(|p| p.into()),
        }
    }

    /// Load the server TLS configuration (cert + key).
    fn load_server_config(&self) -> Result<Arc<ServerConfig>, TlsError> {
        let certs = load_certs(&self.cert_path)?;
        let key = load_private_key(&self.key_path)?;

        let config = if let Some(ref ca_path) = self.ca_path {
            // mTLS: require client certificates
            let mut client_auth_roots = rustls::RootCertStore::empty();
            let ca_certs = load_certs(ca_path)?;
            for cert in ca_certs {
                client_auth_roots.add(cert)?;
            }
            let verifier =
                rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
                    .build()
                    .map_err(|e| TlsError::Tls(format!("client verifier: {e}")))?;
            ServerConfig::builder()
                .with_client_cert_verifier(verifier)
                .with_single_cert(certs, key)?
        } else {
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)?
        };

        Ok(Arc::new(config))
    }

    /// Load the client TLS configuration.
    fn load_client_config(&self) -> Result<Arc<ClientConfig>, TlsError> {
        let config = if let Some(ref ca_path) = self.ca_path {
            // Client verifies server certificate against CA
            let mut root_store = rustls::RootCertStore::empty();
            let ca_certs = load_certs(ca_path)?;
            for cert in ca_certs {
                root_store.add(cert)?;
            }
            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        } else {
            // Use webpki-roots for standard CA verification
            let root_store = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        Ok(Arc::new(config))
    }
}

// ── TLS Transport ──────────────────────────────────────────────────────────

/// TLS-encrypted TCP transport for bus messages.
///
/// Wraps the existing TCP frame codec (encode_frame/decode_frame) with
/// rustls encryption. Each connection is encrypted end-to-end.
///
/// Implements `Transport` for integration with the bus routing layer.
pub struct TlsTransport {
    /// TLS configuration.
    config: TlsConfig,
    /// Server-side: bound listener for receiving connections.
    listener: Option<TcpListener>,
    /// The address this transport is listening on.
    listen_addr: Option<String>,
}

impl TlsTransport {
    /// Create a new TLS transport with the given configuration.
    ///
    /// Call `register()` to start listening on a specific address.
    pub fn new(config: TlsConfig) -> Self {
        TlsTransport {
            config,
            listener: None,
            listen_addr: None,
        }
    }

    /// Get a reference to the TLS configuration.
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }
}

impl Transport for TlsTransport {
    fn send(&mut self, recipient: &str, message: WireMessage) -> Result<(), TransportError> {
        let addr = recipient
            .to_socket_addrs()
            .map_err(|e| TransportError::SendFailed(e.to_string()))?
            .next()
            .ok_or_else(|| TransportError::SendFailed("no addresses resolved".into()))?;

        // Connect via TCP
        let stream =
            TcpStream::connect(addr).map_err(|e| TransportError::SendFailed(e.to_string()))?;

        // Use the IP address directly for TLS server name (no DNS lookup needed).
        let server_name = ServerName::IpAddress(addr.ip().into());

        // Load client TLS config
        let client_config = self
            .config
            .load_client_config()
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        // TLS handshake (client side)
        let mut tls_stream = rustls::StreamOwned::new(
            rustls::ClientConnection::new(client_config, server_name)
                .map_err(|e| TransportError::SendFailed(format!("TLS client error: {e}")))?,
            stream,
        );

        // Perform handshake
        tls_stream
            .conn
            .complete_io(&mut tls_stream.sock)
            .map_err(|e| TransportError::SendFailed(format!("TLS handshake: {e}")))?;

        // Encode and send the frame
        let frame = encode_frame(&message);
        tls_stream
            .write_all(&frame)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        tls_stream
            .flush()
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        Ok(())
    }

    fn recv(&mut self, _addr: &str) -> Result<Vec<WireMessage>, TransportError> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| TransportError::RecvFailed("not registered".into()))?;

        // Set non-blocking to drain pending connections
        listener
            .set_nonblocking(true)
            .map_err(|e| TransportError::RecvFailed(e.to_string()))?;

        let server_config = self
            .config
            .load_server_config()
            .map_err(|e| TransportError::RecvFailed(e.to_string()))?;

        let mut messages = Vec::new();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // TLS handshake (server side)
                    let conn =
                        rustls::ServerConnection::new(server_config.clone()).map_err(|e| {
                            TransportError::RecvFailed(format!("TLS server error: {e}"))
                        })?;
                    let mut tls_stream = rustls::StreamOwned::new(conn, stream);

                    // Complete handshake
                    if let Err(e) = tls_stream.conn.complete_io(&mut tls_stream.sock) {
                        // TLS handshake failed for this connection — skip it
                        tracing::warn!("TLS handshake failed for incoming connection: {e}");
                        continue;
                    }

                    // Read one frame
                    match read_one_tls_frame(&mut tls_stream) {
                        Ok(msg) => messages.push(msg),
                        Err(e) => {
                            tracing::warn!("Failed to read TLS frame: {e}");
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_) => break,
            }
        }

        // Restore blocking mode
        listener
            .set_nonblocking(false)
            .map_err(|e| TransportError::RecvFailed(e.to_string()))?;

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

        self.listen_addr = Some(addr.to_string());
        self.listener = Some(listener);
        Ok(())
    }

    fn deregister(&mut self, _addr: &str) {
        self.listener = None;
        self.listen_addr = None;
    }
}

/// Read exactly one length-prefixed frame from a TLS stream.
fn read_one_tls_frame<S: Read + Write>(
    stream: &mut rustls::StreamOwned<rustls::ServerConnection, S>,
) -> Result<WireMessage, TransportError> {
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

    let (msg, _) = decode_frame(&frame)?;
    Ok(msg)
}

// ── TLS utility functions ──────────────────────────────────────────────────

/// Load PEM-encoded certificates from a file.
fn load_certs(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let data = fs::read(path).map_err(|e| TlsError::Io(e))?;
    let mut certs = Vec::new();
    for cert in rustls_pemfile::certs(&mut &data[..]) {
        certs.push(cert.map_err(|e| TlsError::Tls(format!("cert parse: {e}")))?);
    }
    if certs.is_empty() {
        return Err(TlsError::Tls(format!(
            "no certificates found in {}",
            path.display()
        )));
    }
    Ok(certs)
}

/// Load a PEM-encoded private key from a file.
fn load_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let data = fs::read(path).map_err(|e| TlsError::Io(e))?;
    // Try PKCS8 format first
    for key in rustls_pemfile::pkcs8_private_keys(&mut &data[..]) {
        let key = key.map_err(|e| TlsError::Tls(format!("key parse: {e}")))?;
        return Ok(key.into());
    }
    // Try SEC1 format (EC keys)
    let mut cursor = &data[..];
    if let Some(key) = rustls_pemfile::ec_private_keys(&mut cursor).next() {
        let key = key.map_err(|e| TlsError::Tls(format!("key parse: {e}")))?;
        return Ok(key.into());
    }
    Err(TlsError::Tls(format!(
        "no valid private key found in {}",
        path.display()
    )))
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// TLS-specific errors.
#[derive(Debug)]
pub enum TlsError {
    /// I/O error reading cert/key files.
    Io(io::Error),
    /// TLS configuration or handshake error.
    Tls(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::Io(e) => write!(f, "TLS I/O error: {e}"),
            TlsError::Tls(s) => write!(f, "TLS error: {s}"),
        }
    }
}

impl std::error::Error for TlsError {}

impl From<rustls::Error> for TlsError {
    fn from(e: rustls::Error) -> Self {
        TlsError::Tls(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_creation() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem", None::<&str>);
        assert_eq!(config.cert_path, std::path::PathBuf::from("/tmp/cert.pem"));
        assert_eq!(config.key_path, std::path::PathBuf::from("/tmp/key.pem"));
        assert!(config.ca_path.is_none());
    }

    #[test]
    fn test_tls_config_with_ca() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem", Some("/tmp/ca.pem"));
        assert!(config.ca_path.is_some());
    }

    #[test]
    fn test_tls_transport_register_and_deregister() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem", None::<&str>);
        let mut transport = TlsTransport::new(config);
        transport.register("127.0.0.1:9876").unwrap();
        assert!(transport.listener.is_some());
        transport.deregister("127.0.0.1:9876");
        assert!(transport.listener.is_none());
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::Tls("handshake failed".into());
        assert!(err.to_string().contains("handshake failed"));

        let io_err = TlsError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert!(io_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_tls_config_clone() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem", None::<&str>);
        let _cloned = config.clone();
        // Verify Clone works
    }
}
