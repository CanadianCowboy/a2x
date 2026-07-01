// See plans/09-core-types.md §7

use crate::agent_id::AgentId;
use crate::program_id::ProgramId;

/// Core error type for the A2X language.
///
/// This is the foundational error type in `a2x-core`. Each higher crate
/// defines its own error enum that wraps or converts from `CoreError`.
#[derive(Debug)]
pub enum CoreError {
    /// Dimensionality mismatch between two ConceptVectors.
    DimensionMismatch { expected: usize, actual: usize },
    /// Referenced a NodeId that doesn't exist.
    InvalidNodeId(u64),
    /// A label was reused when it must be unique.
    LabelConflict(String),
    /// Memory allocation limit exceeded in the WorldGraph.
    OutOfMemory,
    /// A catch-all for errors from external or opaque sources.
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "dimensionality mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            CoreError::InvalidNodeId(id) => write!(f, "invalid node ID: {}", id),
            CoreError::LabelConflict(label) => write!(f, "label already exists: {}", label),
            CoreError::OutOfMemory => write!(f, "out of memory: cannot allocate node"),
            CoreError::Other(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Other(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<String> for CoreError {
    fn from(s: String) -> Self {
        CoreError::Other(s.into())
    }
}

impl From<&str> for CoreError {
    fn from(s: &str) -> Self {
        CoreError::Other(s.to_string().into())
    }
}

/// Error type for agent operations.
///
/// # Zero-dependency note
/// At the core layer, VM errors are carried as strings since `a2x-core`
/// cannot depend on `a2x-ccs`. Typed `VmError` is defined in `a2x-ccs`
/// and converted to `AgentError::VmError` in `a2x-agents`.
#[derive(Debug)]
pub enum AgentError {
    /// Referenced agent does not exist.
    NotFound { id: AgentId },
    /// Agent is at capacity and cannot accept more programs.
    AtCapacity { max: usize },
    /// A program crashed during execution.
    ProgramCrash {
        program_id: ProgramId,
        reason: String,
    },
    /// Program exceeded its time limit.
    Timeout { timeout: std::time::Duration },
    /// Program exceeded a resource limit (memory, output size, etc.).
    ResourceLimitExceeded {
        program_id: ProgramId,
        limit: String,
        used: u64,
        max: u64,
    },
    /// Output program is too large.
    OutputTooLarge {
        program_id: ProgramId,
        size_bytes: u64,
        max_bytes: u64,
    },
    /// VM-level error (string-only at core layer).
    VmError(String),
    /// Transport-level error.
    TransportError(String),
    /// Safety constraint violated.
    SafetyViolation(String),
    /// A core-level error occurred.
    Core(CoreError),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::NotFound { id } => write!(f, "agent {} not found", id),
            AgentError::AtCapacity { max } => {
                write!(f, "agent is at capacity (max {} concurrent programs)", max)
            }
            AgentError::ProgramCrash { program_id, reason } => {
                write!(f, "program {} crashed: {}", program_id, reason)
            }
            AgentError::Timeout { timeout } => {
                write!(f, "program exceeded time limit of {:?}", timeout)
            }
            AgentError::ResourceLimitExceeded {
                program_id,
                limit,
                used,
                max,
            } => {
                write!(
                    f,
                    "program {} exceeded {} limit: {} used (max {})",
                    program_id, limit, used, max
                )
            }
            AgentError::OutputTooLarge {
                program_id,
                size_bytes,
                max_bytes,
            } => {
                write!(
                    f,
                    "program {} output too large: {} bytes (max {})",
                    program_id, size_bytes, max_bytes
                )
            }
            AgentError::VmError(msg) => write!(f, "VM error: {}", msg),
            AgentError::TransportError(msg) => write!(f, "transport error: {}", msg),
            AgentError::SafetyViolation(msg) => write!(f, "safety violation: {}", msg),
            AgentError::Core(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AgentError::Core(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CoreError> for AgentError {
    fn from(err: CoreError) -> Self {
        AgentError::Core(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_error_display() {
        let err = CoreError::DimensionMismatch {
            expected: 3,
            actual: 2,
        };
        assert!(format!("{}", err).contains("dimensionality mismatch"));
    }

    #[test]
    fn test_agent_error_from_core() {
        let core_err = CoreError::OutOfMemory;
        let agent_err: AgentError = core_err.into();
        assert!(matches!(agent_err, AgentError::Core(_)));
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::NotFound {
            id: AgentId::new("test"),
        };
        assert!(format!("{}", err).contains("not found"));
    }
}
