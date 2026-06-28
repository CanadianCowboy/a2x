// See plans/09-core-types.md §2-3

use crate::error::CoreError;
use crate::program_id::ProgramId;

/// A single entry in the MemoryTrace (execution history).
///
/// Each time the CCS VM executes an instruction, it records a MemoryEntry
/// capturing the instruction, current IP, and a state snapshot.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryEntry {
    /// When this instruction was executed.
    pub timestamp: Option<std::time::SystemTime>,
    /// The instruction that was executed (as raw bytes in core layer;
    /// typed SigmaPacket lives in a2x-sigma).
    pub instruction_bytes: Vec<u8>,
    /// Instruction pointer at the time of execution.
    pub ip: usize,
    /// Associated program identifier, if known.
    pub program_id: Option<ProgramId>,
    /// State snapshot data (raw bytes at core layer).
    pub state_snapshot_bytes: Vec<u8>,
}

/// Trait for the MemoryTrace (execution history).
///
/// The MemoryTrace records a time-indexed sequence of state transitions
/// as the CCS VM executes. It supports tail queries, compression, and
/// replay for debugging and meta-learning.
pub trait MemoryTrace: Send + Sync {
    /// Append a new entry to the trace.
    fn push(&mut self, entry: MemoryEntry) -> Result<(), CoreError>;

    /// Get the `n` most recent entries.
    fn tail(&self, n: usize) -> Vec<MemoryEntry>;

    /// Total number of entries in the trace.
    fn len(&self) -> usize;

    /// Returns true if the trace has no entries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Compress the trace (e.g., merge similar entries, drop old history).
    fn compress(&mut self) -> Result<(), CoreError>;
}
