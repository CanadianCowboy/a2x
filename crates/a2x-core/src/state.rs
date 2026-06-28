// See plans/09-core-types.md §2-3

use crate::agent_id::AgentId;
use crate::error::CoreError;
use crate::program_id::ProgramId;

/// A snapshot of an agent's current internal state (for probe/debug).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateSnapshot {
    /// The agent's identifier.
    pub agent_id: AgentId,
    /// Current lifecycle state as a string (e.g., "idle", "running", "error").
    pub state: String,
    /// Program currently being executed, if any.
    pub current_program: Option<ProgramId>,
    /// Current instruction pointer, if executing.
    pub ip: Option<usize>,
    /// Number of nodes in the agent's WorldGraph.
    pub world_graph_size: usize,
    /// Number of entries in the agent's MemoryTrace.
    pub memory_trace_length: usize,
    /// How long the agent has been running.
    pub uptime: std::time::Duration,
}

impl StateSnapshot {
    /// Create a new idle snapshot for the given agent.
    pub fn idle(agent_id: AgentId, uptime: std::time::Duration) -> Self {
        StateSnapshot {
            agent_id,
            state: "idle".to_string(),
            current_program: None,
            ip: None,
            world_graph_size: 0,
            memory_trace_length: 0,
            uptime,
        }
    }
}

/// Trait for the StateField (registers / working memory).
///
/// The StateField is the agent's high-dimensional working memory — analogous
/// to CPU registers + stack. It holds named regions backed by a flat tensor.
///
/// # Zero-dependency note
/// At the core layer, region data is represented as plain `Vec<f32>`. The
/// `ndarray`-backed implementation lives in `a2x-ccs`.
pub trait StateField: Send + Sync {
    /// Define a named region within the StateField.
    ///
    /// # Arguments
    /// * `name` — register-like label (e.g., "goal", "belief", "scratch").
    /// * `offset` — starting position in the flat tensor.
    /// * `len` — number of f32 elements in the region.
    fn define_region(&mut self, name: &str, offset: usize, len: usize) -> Result<(), CoreError>;

    /// Read a region by name. Returns a slice of f32 values.
    fn read_region(&self, name: &str) -> Result<&[f32], CoreError>;

    /// Write data into a named region.
    fn write_region(&mut self, name: &str, data: &[f32]) -> Result<(), CoreError>;

    /// Total number of f32 elements in the StateField.
    fn total_len(&self) -> usize;

    /// Return raw access to the underlying flat data.
    fn raw_data(&self) -> &[f32];
}
