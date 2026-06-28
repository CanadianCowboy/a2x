// See plans/09-core-types.md §2-3 (Agent trait)

use crate::agent_id::{AgentId, AgentType};
use crate::capability::Capability;
use crate::packet::Packet;
use crate::state::StateSnapshot;

/// An execution context for A2X programs.
///
/// Each agent has a CCS runtime that executes programs (compiled Σ∞ programs
/// or raw byte packets). The Agent trait defines the interface that agents
/// present to the bus and other agents.
///
/// # Zero-dependency note
/// At the core layer, programs are exchanged as raw `Packet` values.
/// Typed `SigmaProgram` and `OmegaProgram` wrappers are defined in higher
/// crates (`a2x-sigma` and `a2x-omega`).
/// The execute method is synchronous in core; async execution is layered
/// on top by `a2x-agents` using the `tokio` runtime.
pub trait Agent: Send + Sync {
    /// Unique agent identifier (like a process ID).
    fn id(&self) -> AgentId;

    /// Agent type tag.
    fn agent_type(&self) -> AgentType;

    /// Execute a program (raw packet) on this agent's CCS runtime.
    /// Returns the result as a raw packet.
    fn execute(&self, program: Packet) -> Result<Packet, crate::error::AgentError>;

    /// Current internal state (for probing / debug).
    fn state_summary(&self) -> Option<StateSnapshot>;

    /// List of capabilities this agent advertises.
    fn capabilities(&self) -> Vec<Capability>;
}
