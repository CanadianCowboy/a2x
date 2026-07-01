// See plans/12-security.md §7
//
// SecurityEvent — structured audit logging for all security-relevant events.
// Every authentication, authorization, rate-limit, and configuration action
// produces a SecurityEvent that can be written to structured logs.

use crate::entity::EntityId;
use a2x_core::agent_id::AgentId;

/// A security-relevant event in the A2X gateway.
///
/// All authentication attempts, permission checks, rate limit violations,
/// configuration changes, and agent lifecycle events are recorded through
/// this enum. Events are designed to be serializable for structured logging
/// (JSON output to file or stdout via tracing).
#[derive(Clone, Debug)]
pub enum SecurityEvent {
    /// An agent joined the bus.
    AgentJoined {
        agent_id: AgentId,
        /// Address the agent connected from (string representation).
        addr: String,
    },
    /// An agent disconnected from the bus.
    AgentLeft { agent_id: AgentId },
    /// An entity successfully authenticated.
    EntityAuthenticated {
        entity_id: EntityId,
        /// Auth method used.
        method: String,
    },
    /// An entity failed to authenticate.
    AuthenticationFailure {
        /// Source IP or identifier of the failed attempt.
        source: String,
        /// Why authentication failed (sanitized — no secrets).
        reason: String,
    },
    /// A permission check was denied.
    PermissionDenied {
        entity_id: EntityId,
        /// What action was attempted.
        action: String,
    },
    /// A rate limit was exceeded.
    RateLimited {
        entity_id: EntityId,
        /// Current request count in this window.
        count: u32,
        /// The rate limit that was exceeded.
        limit: u32,
    },
    /// A safety violation occurred during program execution.
    SafetyViolation {
        agent_id: AgentId,
        /// The instruction that triggered the violation.
        instruction: String,
        /// Why the violation was raised.
        reason: String,
    },
    /// Gateway configuration was changed.
    ConfigChange {
        /// Who initiated the change.
        source: String,
        /// Human-readable list of what changed.
        changes: Vec<String>,
    },
    /// A new listener was added to the gateway.
    ListenerAdded {
        /// Type of listener (http, ws, tcp, stdio).
        listener_type: String,
        /// Bound address, if applicable.
        address: Option<String>,
    },
    /// A listener was removed from the gateway.
    ListenerRemoved { listener_type: String },
    /// Gateway started.
    GatewayStarted { listener_count: usize },
    /// Gateway stopped.
    GatewayStopped,
    /// A program was submitted for execution.
    ProgramSubmitted {
        entity_id: EntityId,
        /// Number of instructions.
        instruction_count: usize,
        /// Whether permissions were checked.
        permissions_checked: bool,
    },
    /// A program completed execution.
    ProgramCompleted {
        entity_id: EntityId,
        /// Execution result status.
        status: String,
    },
}

impl SecurityEvent {
    /// Human-readable event category for filtering/logging.
    pub fn category(&self) -> &'static str {
        match self {
            SecurityEvent::AgentJoined { .. } => "agent_lifecycle",
            SecurityEvent::AgentLeft { .. } => "agent_lifecycle",
            SecurityEvent::EntityAuthenticated { .. } => "authentication",
            SecurityEvent::AuthenticationFailure { .. } => "authentication",
            SecurityEvent::PermissionDenied { .. } => "authorization",
            SecurityEvent::RateLimited { .. } => "rate_limiting",
            SecurityEvent::SafetyViolation { .. } => "safety",
            SecurityEvent::ConfigChange { .. } => "configuration",
            SecurityEvent::ListenerAdded { .. } => "gateway_lifecycle",
            SecurityEvent::ListenerRemoved { .. } => "gateway_lifecycle",
            SecurityEvent::GatewayStarted { .. } => "gateway_lifecycle",
            SecurityEvent::GatewayStopped => "gateway_lifecycle",
            SecurityEvent::ProgramSubmitted { .. } => "execution",
            SecurityEvent::ProgramCompleted { .. } => "execution",
        }
    }

    /// Severity level for this event.
    pub fn severity(&self) -> &'static str {
        match self {
            SecurityEvent::AuthenticationFailure { .. } => "warn",
            SecurityEvent::PermissionDenied { .. } => "warn",
            SecurityEvent::RateLimited { .. } => "warn",
            SecurityEvent::SafetyViolation { .. } => "error",
            SecurityEvent::AgentJoined { .. } => "info",
            SecurityEvent::AgentLeft { .. } => "info",
            SecurityEvent::EntityAuthenticated { .. } => "info",
            SecurityEvent::ConfigChange { .. } => "info",
            SecurityEvent::ListenerAdded { .. } => "info",
            SecurityEvent::ListenerRemoved { .. } => "info",
            SecurityEvent::GatewayStarted { .. } => "info",
            SecurityEvent::GatewayStopped => "info",
            SecurityEvent::ProgramSubmitted { .. } => "debug",
            SecurityEvent::ProgramCompleted { .. } => "debug",
        }
    }

    /// Emit this event via the tracing infrastructure.
    ///
    /// Uses structured fields so downstream collectors (e.g. JSON logger,
    /// OpenTelemetry) can index on individual event properties.
    pub fn log(&self) {
        match self {
            SecurityEvent::AgentJoined { agent_id, addr } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    agent_id = %agent_id.as_str(),
                    addr = %addr,
                    "agent joined bus"
                );
            }
            SecurityEvent::AgentLeft { agent_id } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    agent_id = %agent_id.as_str(),
                    "agent left bus"
                );
            }
            SecurityEvent::EntityAuthenticated { entity_id, method } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    entity_id = %entity_id,
                    method = %method,
                    "entity authenticated"
                );
            }
            SecurityEvent::AuthenticationFailure { source, reason } => {
                tracing::warn!(
                    category = self.category(),
                    severity = self.severity(),
                    source = %source,
                    reason = %reason,
                    "authentication failure"
                );
            }
            SecurityEvent::PermissionDenied { entity_id, action } => {
                tracing::warn!(
                    category = self.category(),
                    severity = self.severity(),
                    entity_id = %entity_id,
                    action = %action,
                    "permission denied"
                );
            }
            SecurityEvent::RateLimited {
                entity_id,
                count,
                limit,
            } => {
                tracing::warn!(
                    category = self.category(),
                    severity = self.severity(),
                    entity_id = %entity_id,
                    count = count,
                    limit = limit,
                    "rate limited"
                );
            }
            SecurityEvent::SafetyViolation {
                agent_id,
                instruction,
                reason,
            } => {
                tracing::error!(
                    category = self.category(),
                    severity = self.severity(),
                    agent_id = %agent_id.as_str(),
                    instruction = %instruction,
                    reason = %reason,
                    "safety violation"
                );
            }
            SecurityEvent::ConfigChange { source, changes } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    source = %source,
                    changes = ?changes,
                    "configuration changed"
                );
            }
            SecurityEvent::ListenerAdded {
                listener_type,
                address,
            } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    listener_type = %listener_type,
                    address = ?address,
                    "listener added"
                );
            }
            SecurityEvent::ListenerRemoved { listener_type } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    listener_type = %listener_type,
                    "listener removed"
                );
            }
            SecurityEvent::GatewayStarted { listener_count } => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    listener_count = listener_count,
                    "gateway started"
                );
            }
            SecurityEvent::GatewayStopped => {
                tracing::info!(
                    category = self.category(),
                    severity = self.severity(),
                    "gateway stopped"
                );
            }
            SecurityEvent::ProgramSubmitted {
                entity_id,
                instruction_count,
                permissions_checked,
            } => {
                tracing::debug!(
                    category = self.category(),
                    severity = self.severity(),
                    entity_id = %entity_id,
                    instruction_count = instruction_count,
                    permissions_checked = permissions_checked,
                    "program submitted"
                );
            }
            SecurityEvent::ProgramCompleted { entity_id, status } => {
                tracing::debug!(
                    category = self.category(),
                    severity = self.severity(),
                    entity_id = %entity_id,
                    status = %status,
                    "program completed"
                );
            }
        }
    }

    /// Create and log the event in one call.
    pub fn emit(event: SecurityEvent) {
        event.log();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_categories_are_defined() {
        // Every variant should have a non-empty category.
        let events = vec![
            SecurityEvent::AgentJoined {
                agent_id: AgentId::new("a1"),
                addr: "127.0.0.1:8080".into(),
            },
            SecurityEvent::AgentLeft {
                agent_id: AgentId::new("a1"),
            },
            SecurityEvent::EntityAuthenticated {
                entity_id: EntityId::new("e1"),
                method: "api_key".into(),
            },
            SecurityEvent::AuthenticationFailure {
                source: "127.0.0.1".into(),
                reason: "invalid key".into(),
            },
            SecurityEvent::PermissionDenied {
                entity_id: EntityId::new("e1"),
                action: "probe".into(),
            },
            SecurityEvent::RateLimited {
                entity_id: EntityId::new("e1"),
                count: 61,
                limit: 60,
            },
            SecurityEvent::SafetyViolation {
                agent_id: AgentId::new("a1"),
                instruction: "FORBIDDEN".into(),
                reason: "command not allowed".into(),
            },
            SecurityEvent::ConfigChange {
                source: "admin".into(),
                changes: vec!["max_instructions: 1000→2000".into()],
            },
            SecurityEvent::ListenerAdded {
                listener_type: "http".into(),
                address: Some("0.0.0.0:8778".into()),
            },
            SecurityEvent::ListenerRemoved {
                listener_type: "ws".into(),
            },
            SecurityEvent::GatewayStarted { listener_count: 3 },
            SecurityEvent::GatewayStopped,
            SecurityEvent::ProgramSubmitted {
                entity_id: EntityId::new("e1"),
                instruction_count: 5,
                permissions_checked: true,
            },
            SecurityEvent::ProgramCompleted {
                entity_id: EntityId::new("e1"),
                status: "completed".into(),
            },
        ];

        for event in &events {
            assert!(!event.category().is_empty(), "category missing for event");
            assert!(!event.severity().is_empty(), "severity missing for event");
        }
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(
            SecurityEvent::AuthenticationFailure {
                source: "x".into(),
                reason: "y".into(),
            }
            .severity(),
            "warn"
        );
        assert_eq!(
            SecurityEvent::PermissionDenied {
                entity_id: EntityId::new("x"),
                action: "y".into(),
            }
            .severity(),
            "warn"
        );
        assert_eq!(
            SecurityEvent::SafetyViolation {
                agent_id: AgentId::new("x"),
                instruction: "y".into(),
                reason: "z".into(),
            }
            .severity(),
            "error"
        );
        assert_eq!(
            SecurityEvent::GatewayStarted { listener_count: 0 }.severity(),
            "info"
        );
    }

    #[test]
    fn test_emit_does_not_panic() {
        // All variants should be loggable without panicking.
        SecurityEvent::emit(SecurityEvent::GatewayStarted { listener_count: 1 });
        SecurityEvent::emit(SecurityEvent::AuthenticationFailure {
            source: "test".into(),
            reason: "bad token".into(),
        });
        SecurityEvent::emit(SecurityEvent::AgentJoined {
            agent_id: AgentId::new("test"),
            addr: "127.0.0.1".into(),
        });
    }
}
