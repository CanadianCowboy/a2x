// See plans/06-entity-gateway.md §6

use std::fmt;

/// Error type for gateway operations.
#[derive(Debug)]
pub enum GatewayError {
    /// Entity authentication failed.
    AuthFailed(String),
    /// Entity not found.
    EntityNotFound(String),
    /// Agent not found on the bus.
    AgentNotFound(String),
    /// Transport-level error.
    Transport(String),
    /// Configuration error.
    Config(String),
    /// Program parsing or execution error.
    ProgramError(String),
    /// Webhook delivery failed.
    WebhookFailed(String),
    /// Listener startup error.
    ListenerError(String),
    /// Rate limit exceeded.
    RateLimited { entity_id: String, limit: u32 },
    /// Permission denied for the requested operation.
    PermissionDenied(String),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayError::AuthFailed(msg) => write!(f, "auth failed: {}", msg),
            GatewayError::EntityNotFound(id) => write!(f, "entity not found: {}", id),
            GatewayError::AgentNotFound(id) => write!(f, "agent not found: {}", id),
            GatewayError::Transport(msg) => write!(f, "transport error: {}", msg),
            GatewayError::Config(msg) => write!(f, "config error: {}", msg),
            GatewayError::ProgramError(msg) => write!(f, "program error: {}", msg),
            GatewayError::WebhookFailed(msg) => write!(f, "webhook failed: {}", msg),
            GatewayError::ListenerError(msg) => write!(f, "listener error: {}", msg),
            GatewayError::RateLimited { entity_id, limit } => {
                write!(
                    f,
                    "rate limited: entity '{}' exceeded {} req/min",
                    entity_id, limit
                )
            }
            GatewayError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
        }
    }
}

impl std::error::Error for GatewayError {}

impl From<GatewayError> for String {
    fn from(err: GatewayError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_variants() {
        let err = GatewayError::AuthFailed("bad key".into());
        assert!(format!("{}", err).contains("auth failed"));

        let err = GatewayError::EntityNotFound("e-1".into());
        assert!(format!("{}", err).contains("entity not found"));

        let err = GatewayError::RateLimited {
            entity_id: "e-1".into(),
            limit: 100,
        };
        assert!(format!("{}", err).contains("rate limited"));
    }
}
