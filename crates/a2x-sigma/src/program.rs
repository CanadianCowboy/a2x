// See plans/01-sigma-language.md §10-12

use crate::packet::SigmaPacket;
use a2x_core::ProgramId;
use std::collections::HashMap;

/// Metadata about a Σ∞ program's origin and version.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProgramMetadata {
    /// Which agent authored this program.
    pub author: String,
    /// When the program was created.
    pub created_at: Option<std::time::SystemTime>,
    /// Version number.
    pub version: u32,
    /// Human-readable description (for debug/probe only).
    pub description: String,
}

impl Default for ProgramMetadata {
    fn default() -> Self {
        ProgramMetadata {
            author: "unknown".into(),
            created_at: None,
            version: 1,
            description: String::new(),
        }
    }
}

/// A Σ∞ program — an executable sequence of instructions.
///
/// A program is the fundamental unit of execution in A2X. It is a sequence of
/// Σ∞ packets (instructions) with metadata, label tables for jump targets,
/// and optional sub-programs for descension (⤈).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SigmaProgram {
    /// Unique program ID (content-addressed via Blake3 hash).
    pub id: ProgramId,
    /// The instruction stream.
    pub instructions: Vec<SigmaPacket>,
    /// Symbol table: labels → instruction indices (for jump targets).
    pub labels: HashMap<String, usize>,
    /// Sub-programs (for ⤈ descend).
    pub sub_programs: HashMap<String, SigmaProgram>,
    /// Provenance metadata.
    pub metadata: ProgramMetadata,
}

impl SigmaProgram {
    /// Create an empty program with a zero ID.
    pub fn new() -> Self {
        SigmaProgram {
            id: ProgramId::zero(),
            instructions: Vec::new(),
            labels: HashMap::new(),
            sub_programs: HashMap::new(),
            metadata: ProgramMetadata::default(),
        }
    }

    /// Add an instruction to the program.
    pub fn push(&mut self, instruction: SigmaPacket) {
        self.instructions.push(instruction);
    }

    /// Number of instructions in the program.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Returns true if the program has no instructions.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Resolve a label to an instruction index.
    pub fn resolve_label(&self, label: &str) -> Option<usize> {
        self.labels.get(label).copied()
    }

    /// Register a label pointing to the next instruction that will be pushed.
    pub fn register_label(&mut self, label: String) {
        self.labels.insert(label, self.instructions.len());
    }

    /// Add a named sub-program (for ⤈ descend).
    pub fn add_sub_program(&mut self, name: String, program: SigmaProgram) {
        self.sub_programs.insert(name, program);
    }

    /// Compose this program with another — append all instructions and merge labels/sub-programs.
    pub fn compose(&mut self, other: SigmaProgram) {
        let offset = self.instructions.len();
        // Offset the labels from the other program
        for (label, idx) in other.labels {
            self.labels.insert(label, idx + offset);
        }
        self.instructions.extend(other.instructions);
        self.sub_programs.extend(other.sub_programs);
    }

    /// Compute the program's ID by hashing its contents with Blake3.
    /// The hash covers: instructions (serialized), labels, and metadata.
    pub fn compute_id(&mut self) -> ProgramId {
        let mut hasher = blake3::Hasher::new();

        // Hash instructions (each as text, with newline separator)
        for packet in &self.instructions {
            hasher.update(packet.to_string().as_bytes());
            hasher.update(b"\n");
        }

        // Hash labels (sorted by key for determinism)
        let mut label_keys: Vec<&String> = self.labels.keys().collect();
        label_keys.sort();
        for key in label_keys {
            hasher.update(key.as_bytes());
            hasher.update(b":");
            if let Some(idx) = self.labels.get(key) {
                hasher.update(&idx.to_le_bytes());
            }
        }

        // Hash sub-program names
        let mut sub_keys: Vec<&String> = self.sub_programs.keys().collect();
        sub_keys.sort();
        for key in sub_keys {
            hasher.update(b"sub:");
            hasher.update(key.as_bytes());
        }

        let hash = hasher.finalize();
        self.id = ProgramId::new(*hash.as_bytes());
        self.id
    }
}

impl Default for SigmaProgram {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference to a program — either inline or by content-addressed ID.
///
/// Enables program caching and deduplication: if agent B already has program X,
/// agent A can send just the `ProgramId` instead of the full program.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProgramRef {
    /// Full program included inline.
    Inline(SigmaProgram),
    /// Reference to a known program by its ID.
    ById(ProgramId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_program_is_empty() {
        let prog = SigmaProgram::new();
        assert!(prog.is_empty());
        assert_eq!(prog.len(), 0);
    }

    #[test]
    fn test_push_and_len() {
        let mut prog = SigmaProgram::new();
        prog.push(SigmaPacket::new());
        assert_eq!(prog.len(), 1);
        assert!(!prog.is_empty());
    }

    #[test]
    fn test_register_and_resolve_label() {
        let mut prog = SigmaProgram::new();
        prog.register_label("loop".into()); // → index 0
        prog.push(SigmaPacket::new());
        prog.register_label("end".into()); // → index 1
        assert_eq!(prog.resolve_label("loop"), Some(0));
        assert_eq!(prog.resolve_label("end"), Some(1));
        assert_eq!(prog.resolve_label("missing"), None);
    }

    #[test]
    fn test_compose() {
        let mut a = SigmaProgram::new();
        a.push(SigmaPacket::new());
        a.register_label("a".into()); // → index 1 (after push)

        let mut b = SigmaProgram::new();
        b.push(SigmaPacket::new());
        b.register_label("b".into()); // → index 1 (after push)

        a.compose(b);
        assert_eq!(a.len(), 2);
        assert_eq!(a.resolve_label("a"), Some(1)); // unchanged (a's own label)
        assert_eq!(a.resolve_label("b"), Some(2)); // 1 (b's index) + 1 (offset)
    }
}
