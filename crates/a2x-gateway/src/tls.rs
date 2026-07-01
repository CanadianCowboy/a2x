// See plans/12-security.md §3 — Entity-to-Gateway Security
//
// TLS configuration for the gateway HTTP listener.
//
// In production, TLS termination is typically handled by a reverse proxy
// (nginx, caddy, etc.). This module provides the configuration types for
// native TLS support when a reverse proxy is not available.
//
// See also: a2x-bus/src/tls.rs for bus-level TLS transport.

use std::path::PathBuf;

/// TLS configuration for the gateway HTTP/HTTPS listener.
///
/// Supports standard TLS (server certificate + private key) and
/// mutual TLS with an optional CA certificate for client verification.
#[derive(Clone, Debug)]
pub struct GatewayTlsConfig {
    /// Path to the TLS certificate (PEM format).
    pub cert_path: PathBuf,
    /// Path to the TLS private key (PEM format).
    pub key_path: PathBuf,
    /// Optional CA certificate path for mutual TLS (mTLS).
    /// When set, clients must present a valid certificate signed by this CA.
    pub ca_path: Option<PathBuf>,
}

impl GatewayTlsConfig {
    /// Create a new gateway TLS configuration.
    pub fn new(cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        GatewayTlsConfig {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_path: None,
        }
    }

    /// Enable mutual TLS by specifying a CA certificate path.
    pub fn with_mutual_tls(mut self, ca_path: impl Into<PathBuf>) -> Self {
        self.ca_path = Some(ca_path.into());
        self
    }

    /// Check whether TLS is configured for mutual authentication.
    pub fn is_mutual_tls(&self) -> bool {
        self.ca_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_tls_config_creation() {
        let config = GatewayTlsConfig::new("/etc/a2x/cert.pem", "/etc/a2x/key.pem");
        assert_eq!(config.cert_path, PathBuf::from("/etc/a2x/cert.pem"));
        assert_eq!(config.key_path, PathBuf::from("/etc/a2x/key.pem"));
        assert!(!config.is_mutual_tls());
    }

    #[test]
    fn test_gateway_tls_config_mutual() {
        let config = GatewayTlsConfig::new("/etc/a2x/cert.pem", "/etc/a2x/key.pem")
            .with_mutual_tls("/etc/a2x/ca.pem");
        assert!(config.is_mutual_tls());
        assert_eq!(config.ca_path, Some(PathBuf::from("/etc/a2x/ca.pem")));
    }

    #[test]
    fn test_gateway_tls_config_clone() {
        let config =
            GatewayTlsConfig::new("/tmp/cert.pem", "/tmp/key.pem").with_mutual_tls("/tmp/ca.pem");
        let cloned = config.clone();
        assert_eq!(cloned.cert_path, config.cert_path);
        assert_eq!(cloned.key_path, config.key_path);
        assert_eq!(cloned.ca_path, config.ca_path);
    }
}
