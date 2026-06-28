// See plans/03-ccs-vm.md §5 (MemoryTrace)

use a2x_core::error::CoreError;
use a2x_core::memory::{MemoryEntry, MemoryTrace};

/// Vec-backed implementation of the MemoryTrace trait.
///
/// The MemoryTrace records a time-indexed sequence of state transitions
/// as the CCS VM executes instructions. Supports tail queries and basic
/// truncation-based compression.
pub struct VecMemoryTrace {
    entries: Vec<MemoryEntry>,
    /// Optional: maximum capacity before auto-compression triggers.
    max_capacity: usize,
}

impl VecMemoryTrace {
    /// Create a new empty MemoryTrace with the given max capacity.
    pub fn new(max_capacity: usize) -> Self {
        VecMemoryTrace {
            entries: Vec::new(),
            max_capacity,
        }
    }

    /// Create with a default capacity of 10,000 entries.
    pub fn default_capacity() -> Self {
        Self::new(10_000)
    }

    /// Get all entries (for iteration).
    pub fn all_entries(&self) -> &[MemoryEntry] {
        &self.entries
    }
}

impl Default for VecMemoryTrace {
    fn default() -> Self {
        Self::default_capacity()
    }
}

impl MemoryTrace for VecMemoryTrace {
    fn push(&mut self, entry: MemoryEntry) -> Result<(), CoreError> {
        if self.entries.len() >= self.max_capacity {
            // Auto-compress: drop oldest half to make room
            let keep = self.max_capacity / 2;
            let drain_end = self.entries.len() - keep;
            self.entries.drain(0..drain_end);
        }
        self.entries.push(entry);
        Ok(())
    }

    fn tail(&self, n: usize) -> Vec<MemoryEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].to_vec()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn compress(&mut self) -> Result<(), CoreError> {
        // Simple compression: keep every other entry, plus the most recent N
        const KEEP_RECENT: usize = 100;
        let total = self.entries.len();
        if total <= KEEP_RECENT {
            return Ok(());
        }
        let recent = self.entries.split_off(total - KEEP_RECENT);
        let sampled: Vec<MemoryEntry> = recent
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, e)| e)
            .collect();
        // Add back the kept recent entries
        self.entries.extend(sampled);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::memory::MemoryEntry;

    fn make_entry() -> MemoryEntry {
        MemoryEntry {
            timestamp: None,
            instruction_bytes: vec![1u8, 2, 3],
            ip: 0,
            program_id: None,
            state_snapshot_bytes: vec![0u8; 64],
        }
    }

    #[test]
    fn test_push_and_len() {
        let mut trace = VecMemoryTrace::new(100);
        trace.push(make_entry()).unwrap();
        trace.push(make_entry()).unwrap();
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn test_tail() {
        let mut trace = VecMemoryTrace::new(100);
        for _ in 0..5 {
            trace.push(make_entry()).unwrap();
        }
        let tail = trace.tail(3);
        assert_eq!(tail.len(), 3);
    }

    #[test]
    fn test_tail_more_than_len() {
        let mut trace = VecMemoryTrace::new(100);
        trace.push(make_entry()).unwrap();
        let tail = trace.tail(10);
        assert_eq!(tail.len(), 1);
    }

    #[test]
    fn test_auto_compress_on_overflow() {
        let mut trace = VecMemoryTrace::new(10);
        for _ in 0..15 {
            trace.push(make_entry()).unwrap();
        }
        // After overflow, should have dropped oldest, keep at most capacity
        assert!(trace.len() <= 10);
    }

    #[test]
    fn test_compress() {
        let mut trace = VecMemoryTrace::new(1000);
        for _ in 0..200 {
            trace.push(make_entry()).unwrap();
        }
        let before = trace.len();
        trace.compress().unwrap();
        let after = trace.len();
        assert!(after < before);
    }
}
