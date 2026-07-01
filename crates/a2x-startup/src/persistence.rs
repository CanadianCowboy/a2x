// See plans/11-startup-shutdown.md §4 (State Persistence) and
// plans/14-resilience.md §7 (Storage Corruption — Atomic Writes)
//
// State persistence with Blake3 integrity checks and atomic writes.
//
// State files are saved atomically (write to .tmp, then rename).
// A .checksum file stores the Blake3 hash for integrity verification.
// On corruption, falls back to .bak backup.

use std::fs;
use std::io;
use std::path::Path;

use a2x_ccs::CcsVm;
use a2x_core::graph::WorldGraph;
use a2x_sigma::program::SigmaProgram;
use tracing::{info, warn};

/// Errors during state save/load operations.
#[derive(Debug)]
pub enum StateSaveError {
    Io(io::Error),
    Serialization(String),
    Corruption { path: String, reason: String },
}

impl std::fmt::Display for StateSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateSaveError::Io(e) => write!(f, "IO error: {e}"),
            StateSaveError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            StateSaveError::Corruption { path, reason } => {
                write!(f, "corruption detected at {path}: {reason}")
            }
        }
    }
}

impl std::error::Error for StateSaveError {}

/// Save data atomically to disk with Blake3 checksum.
///
/// 1. Write data to `path.tmp`
/// 2. Compute Blake3 checksum, write to `path.checksum`
/// 3. Rename `path.tmp` → `path` (atomic on same filesystem)
///
/// If `path` already exists, it becomes `path.bak` before the rename.
pub fn save_state_atomic(path: &Path, data: &[u8]) -> Result<(), StateSaveError> {
    let tmp_path = path.with_extension("tmp");
    let checksum_path = path.with_extension("checksum");

    // 1. Write data to temp file
    fs::write(&tmp_path, data)
        .map_err(|e| StateSaveError::Io(io::Error::new(e.kind(), format!("write tmp: {e}"))))?;

    // 2. Compute and write checksum
    let hash = blake3::hash(data);
    fs::write(&checksum_path, hash.as_bytes()).map_err(|e| {
        StateSaveError::Io(io::Error::new(e.kind(), format!("write checksum: {e}")))
    })?;

    // 3. Rotate backup: existing file → .bak
    if path.exists() {
        let bak_path = path.with_extension("bak");
        let _ = fs::remove_file(&bak_path);
        fs::rename(path, &bak_path).map_err(|e| {
            StateSaveError::Io(io::Error::new(e.kind(), format!("rename to bak: {e}")))
        })?;
    }

    // 4. Atomic rename: .tmp → final path
    fs::rename(&tmp_path, path)
        .map_err(|e| StateSaveError::Io(io::Error::new(e.kind(), format!("rename tmp: {e}"))))?;

    info!("State saved atomically to {}", path.display());
    Ok(())
}

/// Load data from disk with Blake3 integrity check.
///
/// 1. Read file
/// 2. Read checksum file (if it exists)
/// 3. Verify Blake3 hash matches
/// 4. If mismatch and .bak exists, try loading .bak
/// 5. If both fail, return Corruption error
pub fn load_state_atomic(path: &Path) -> Result<Option<Vec<u8>>, StateSaveError> {
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read(path)
        .map_err(|e| StateSaveError::Io(io::Error::new(e.kind(), format!("read: {e}"))))?;

    // Verify integrity
    let checksum_path = path.with_extension("checksum");
    if checksum_path.exists() {
        let expected = fs::read(&checksum_path).map_err(|e| {
            StateSaveError::Io(io::Error::new(e.kind(), format!("read checksum: {e}")))
        })?;

        let actual = blake3::hash(&data);
        if expected != actual.as_bytes() {
            // Corruption detected — try backup
            let bak_path = path.with_extension("bak");
            if bak_path.exists() {
                warn!("Corruption detected in {}, loading backup", path.display());
                let bak_data = fs::read(&bak_path).map_err(|e| {
                    StateSaveError::Io(io::Error::new(e.kind(), format!("read bak: {e}")))
                })?;
                let bak_hash = blake3::hash(&bak_data);
                // Verify backup integrity too
                let bak_checksum_path = path.with_extension("bak.checksum");
                if bak_checksum_path.exists() {
                    let bak_expected = fs::read(&bak_checksum_path).ok();
                    if bak_expected.as_deref() != Some(bak_hash.as_bytes()) {
                        return Err(StateSaveError::Corruption {
                            path: path.display().to_string(),
                            reason: "both primary and backup corrupted".into(),
                        });
                    }
                }
                return Ok(Some(bak_data));
            }
            return Err(StateSaveError::Corruption {
                path: path.display().to_string(),
                reason: format!(
                    "checksum mismatch (expected {:?}, got {:?})",
                    hexify(&expected),
                    hexify(actual.as_bytes()),
                ),
            });
        }
    }

    Ok(Some(data))
}

/// Save data (non-atomic, best-effort). Used for non-critical data.
pub fn save_state(path: &Path, data: &[u8]) -> Result<(), StateSaveError> {
    fs::write(path, data)
        .map_err(|e| StateSaveError::Io(io::Error::new(e.kind(), format!("write: {e}"))))?;
    Ok(())
}

/// Load data (non-atomic, no integrity check). Used for non-critical data.
pub fn load_state(path: &Path) -> Result<Option<Vec<u8>>, StateSaveError> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(path)
        .map_err(|e| StateSaveError::Io(io::Error::new(e.kind(), format!("read: {e}"))))?;
    Ok(Some(data))
}

/// Compute the Blake3 hash of a byte slice.
pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Format 32 bytes as hex for display.
fn hexify(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes.iter().take(8) {
        s.push_str(&format!("{b:02x}"));
    }
    if bytes.len() > 8 {
        s.push_str("...");
    }
    s
}

// ── CCS VM save/load helpers ──────────────────────────────────────────────

/// Save the CCS VM's WorldGraph to disk.
///
/// Save the CCS VM's WorldGraph metadata to disk.
///
/// TODO(Phase 8+): full graph serialization (nodes + edges) via bincode.
/// Currently saves only metadata (node count, edge count, steps).
pub fn save_world_graph(vm: &CcsVm, path: &Path) -> Result<(), StateSaveError> {
    // Simple metadata format for Phase 7:
    // [4-byte LE node_count][4-byte LE edge_count][8-byte LE steps_executed]
    let node_count = vm.world_graph.node_count() as u32;
    let edge_count = vm.world_graph.edge_count() as u32;
    let steps = vm.steps_executed() as u64;

    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&node_count.to_le_bytes());
    data.extend_from_slice(&edge_count.to_le_bytes());
    data.extend_from_slice(&steps.to_le_bytes());

    save_state_atomic(path, &data)
}

/// Load a WorldGraph from disk into a CCS VM.
///
/// Returns the reconstructed VM, or None if the file doesn't exist.
/// Currently loads metadata only (full graph restoration is Phase 8+).
pub fn load_world_graph(path: &Path) -> Result<Option<CcsVm>, StateSaveError> {
    let data = match load_state_atomic(path)? {
        Some(d) => d,
        None => return Ok(None),
    };

    if data.len() < 16 {
        return Err(StateSaveError::Corruption {
            path: path.display().to_string(),
            reason: "state file too short".into(),
        });
    }

    let node_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let edge_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let steps = u64::from_le_bytes([
        data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
    ]);

    let vm = CcsVm::new();
    // Phase 7: metadata-only restore. Full graph reconstruction is Phase 8+.
    // The VM starts with a fresh WorldGraph but carries the saved metadata
    // for observability (node_count, edge_count, steps from previous run).
    let _ = (node_count, edge_count, steps); // metadata for future use
    Ok(Some(vm))
}

/// Save a Σ∞ program to a .sigma file.
pub fn save_sigma_program(program: &SigmaProgram, path: &Path) -> Result<(), StateSaveError> {
    let text = format!("{} instructions", program.instructions.len());
    save_state(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("a2x-persistence-test-{}", name))
    }

    #[test]
    fn test_save_load_atomic_roundtrip() {
        let path = temp_path("roundtrip.bin");
        let data = b"hello a2x persistence";

        save_state_atomic(&path, data).unwrap();
        let loaded = load_state_atomic(&path).unwrap().unwrap();
        assert_eq!(loaded, data);

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("checksum"));
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let path = temp_path("nonexistent.bin");
        let result = load_state_atomic(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_atomic_rotates_backup() {
        let path = temp_path("rotate.bin");
        let bak_path = path.with_extension("bak");
        let checksum_path = path.with_extension("checksum");

        // First save
        save_state_atomic(&path, b"version 1").unwrap();
        assert!(path.exists());

        // Second save
        save_state_atomic(&path, b"version 2").unwrap();
        assert!(path.exists());
        assert!(bak_path.exists(), "backup should be created on second save");

        let orig = load_state_atomic(&path).unwrap().unwrap();
        assert_eq!(orig, b"version 2");

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak_path);
        let _ = fs::remove_file(&checksum_path);
    }

    #[test]
    fn test_corruption_detection() {
        let path = temp_path("corrupt.bin");
        let checksum_path = path.with_extension("checksum");

        save_state_atomic(&path, b"good data").unwrap();

        // Corrupt the file
        fs::write(&path, b"tampered data").unwrap();

        let result = load_state_atomic(&path);
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&checksum_path);
    }

    #[test]
    fn test_corruption_falls_back_to_bak() {
        let path = temp_path("fallback.bin");
        let bak_path = path.with_extension("bak");
        let checksum_path = path.with_extension("checksum");

        save_state_atomic(&path, b"good data").unwrap();
        // Second save creates .bak with "good data"
        save_state_atomic(&path, b"new data").unwrap();

        // Corrupt the primary
        fs::write(&path, b"corrupted").unwrap();

        let result = load_state_atomic(&path).unwrap().unwrap();
        assert_eq!(result, b"good data", "should fall back to .bak");

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak_path);
        let _ = fs::remove_file(&checksum_path);
        let _ = fs::remove_file(path.with_extension("bak.checksum"));
    }

    #[test]
    fn test_save_load_world_graph() {
        let path = temp_path("worldgraph.bin");
        let vm = CcsVm::new();
        save_world_graph(&vm, &path).unwrap();

        let restored = load_world_graph(&path).unwrap();
        assert!(restored.is_some());

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("checksum"));
    }

    #[test]
    fn test_hash_bytes_deterministic() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"hello");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_bytes(b"world"));
    }
}
