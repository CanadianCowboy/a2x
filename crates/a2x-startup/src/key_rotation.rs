// See plans/12-security.md §6 — Key Rotation & Secrets Management
//
// Key rotation for agent Ed25519 signing keys. Supports time-based and
// usage-based rotation policies. Keys are stored via the secure_storage
// module; metadata (last rotated, signature count) is stored alongside.
//
// The metadata file lives at ~/.a2x/keys/<agent-id>.key.meta and uses a
// compact 24-byte binary format:
//   [8 bytes: last_rotated i64 BE]
//   [8 bytes: signature_count u64 BE]
//   [8 bytes: created_at i64 BE]

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::secure_storage::{self, agent_key_path};

// ── Rotation policy ────────────────────────────────────────────────────────

/// When to rotate an agent's signing key.
///
/// See plans/12-security.md §6 — "Agent Key Rotation".
#[derive(Clone, Debug, PartialEq)]
pub enum KeyRotationPolicy {
    /// Never rotate (static keys, suitable for development).
    Never,
    /// Rotate every N days since the last rotation.
    TimeBased { interval_days: u32 },
    /// Rotate after N signatures have been produced with this key.
    UsageBased { max_signatures: u64 },
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        // Sensible default: rotate every 90 days
        KeyRotationPolicy::TimeBased { interval_days: 90 }
    }
}

// ── Rotation metadata ──────────────────────────────────────────────────────

/// Metadata stored alongside the key file to track rotation state.
///
/// Binary format (24 bytes):
///   - last_rotated: i64 Unix timestamp (seconds), big-endian
///   - signature_count: u64, big-endian
///   - created_at: i64 Unix timestamp (seconds), big-endian
#[derive(Clone, Debug)]
pub struct RotationMetadata {
    /// Unix timestamp (seconds) of the last rotation.
    pub last_rotated: i64,
    /// Number of signatures produced since last rotation.
    pub signature_count: u64,
    /// Unix timestamp (seconds) when the metadata file was first created.
    pub created_at: i64,
}

impl Default for RotationMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl RotationMetadata {
    /// Create a new metadata entry with the current time.
    pub fn new() -> Self {
        let now = current_unix_timestamp();
        RotationMetadata {
            last_rotated: now,
            signature_count: 0,
            created_at: now,
        }
    }

    /// Serialize to 24-byte binary format.
    fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.last_rotated.to_be_bytes());
        buf[8..16].copy_from_slice(&self.signature_count.to_be_bytes());
        buf[16..24].copy_from_slice(&self.created_at.to_be_bytes());
        buf
    }

    /// Deserialize from 24-byte binary format.
    fn from_bytes(bytes: &[u8; 24]) -> Self {
        let last_rotated = i64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let signature_count = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let created_at = i64::from_be_bytes(bytes[16..24].try_into().unwrap());
        RotationMetadata {
            last_rotated,
            signature_count,
            created_at,
        }
    }
}

// ── Key rotator ────────────────────────────────────────────────────────────

/// Manages key rotation for an agent's Ed25519 signing key.
///
/// On construction, loads existing metadata (or creates fresh metadata).
/// Callers should check `should_rotate()` periodically (or before each
/// sign operation) and call `rotate()` when rotation is due.
///
/// # Example
///
/// ```ignore
/// let mut rotator = KeyRotator::new(
///     "my-agent",
///     KeyRotationPolicy::TimeBased { interval_days: 30 },
/// );
/// if rotator.should_rotate() {
///     rotator.rotate().expect("key rotation failed");
/// }
/// ```
pub struct KeyRotator {
    /// Agent ID this rotator manages.
    agent_id: String,
    /// Rotation policy.
    policy: KeyRotationPolicy,
    /// In-memory metadata (synced from/to disk).
    metadata: RotationMetadata,
    /// Path to the key file.
    key_path: PathBuf,
    /// Path to the metadata file.
    meta_path: PathBuf,
}

impl KeyRotator {
    /// Create a new key rotator for the given agent.
    ///
    /// Loads existing metadata from disk if present; otherwise creates
    /// fresh metadata. If the key file doesn't exist yet, `rotate()` will
    /// generate it on first call.
    pub fn new(agent_id: &str, policy: KeyRotationPolicy) -> Result<Self, KeyRotatorError> {
        let key_path = agent_key_path(agent_id)
            .map_err(|e| KeyRotatorError::Storage(format!("failed to resolve key path: {e}")))?;
        let meta_path = metadata_path(&key_path);
        let metadata = load_metadata(&meta_path).unwrap_or_else(|| {
            info!(
                "No rotation metadata found for agent '{}', creating fresh metadata",
                agent_id
            );
            RotationMetadata::new()
        });

        Ok(KeyRotator {
            agent_id: agent_id.to_string(),
            policy,
            metadata,
            key_path,
            meta_path,
        })
    }

    /// Check whether the key should be rotated according to the policy.
    ///
    /// Returns `true` if rotation is due.
    pub fn should_rotate(&self) -> bool {
        match self.policy {
            KeyRotationPolicy::Never => false,
            KeyRotationPolicy::TimeBased { interval_days } => {
                let now = current_unix_timestamp();
                let elapsed_seconds = now - self.metadata.last_rotated;
                let interval_seconds = interval_days as i64 * 86_400;
                elapsed_seconds >= interval_seconds
            }
            KeyRotationPolicy::UsageBased { max_signatures } => {
                self.metadata.signature_count >= max_signatures
            }
        }
    }

    /// Rotate the key: generate a new Ed25519 key pair, save it, and update
    /// metadata. Requires the `key-rotation` feature.
    #[cfg(feature = "key-rotation")]
    pub fn rotate(&mut self) -> Result<RotatedKey, KeyRotatorError> {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;
        use rand::RngCore;

        // Ensure the keys directory exists
        secure_storage::ensure_keys_dir()
            .map_err(|e| KeyRotatorError::Storage(format!("keys dir: {e}")))?;

        // Generate a new Ed25519 key pair
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        // Save the new signing key (32-byte seed)
        secure_storage::save_key(&self.key_path, &seed)
            .map_err(|e| KeyRotatorError::Storage(format!("save key: {e}")))?;

        // Update metadata
        let now = current_unix_timestamp();
        self.metadata.last_rotated = now;
        self.metadata.signature_count = 0;
        save_metadata(&self.meta_path, &self.metadata)?;

        info!(
            "Rotated signing key for agent '{}' (last rotated: {})",
            self.agent_id, now
        );

        Ok(RotatedKey {
            agent_id: self.agent_id.clone(),
            signing_key_seed: seed,
            verifying_key_bytes: verifying_key.to_bytes(),
            rotated_at: now,
        })
    }

    /// Stub: rotate the key without actually generating a new one.
    /// Used when the `key-rotation` feature is not enabled.
    #[cfg(not(feature = "key-rotation"))]
    pub fn rotate(&mut self) -> Result<RotatedKey, KeyRotatorError> {
        Err(KeyRotatorError::FeatureDisabled(
            "key-rotation feature is not enabled; rebuild with --features key-rotation".into(),
        ))
    }

    /// Record that a signature was produced with the current key.
    ///
    /// Increments the in-memory counter and persists to disk. Callers
    /// should invoke this after each `sign()` operation.
    ///
    /// Note: uses `saturating_add` to prevent overflow. If the counter
    /// reaches `u64::MAX`, further calls are no-ops. For `UsageBased`
    /// rotation with `max_signatures = u64::MAX`, the counter can never
    /// reach the threshold — use a lower limit or `TimeBased` instead.
    pub fn record_signature(&mut self) -> Result<(), KeyRotatorError> {
        self.metadata.signature_count = self.metadata.signature_count.saturating_add(1);
        save_metadata(&self.meta_path, &self.metadata)?;
        Ok(())
    }

    /// Get the Unix timestamp of the last rotation.
    pub fn last_rotated(&self) -> i64 {
        self.metadata.last_rotated
    }

    /// Get the current signature count.
    pub fn signature_count(&self) -> u64 {
        self.metadata.signature_count
    }

    /// Get the path to the key file.
    pub fn key_path(&self) -> &PathBuf {
        &self.key_path
    }

    /// Get the rotation policy.
    pub fn policy(&self) -> &KeyRotationPolicy {
        &self.policy
    }

    /// Update the rotation policy.
    pub fn set_policy(&mut self, policy: KeyRotationPolicy) {
        self.policy = policy;
    }

    /// Force a rotation regardless of policy (useful for key compromise).
    #[cfg(feature = "key-rotation")]
    pub fn force_rotate(&mut self) -> Result<RotatedKey, KeyRotatorError> {
        warn!(
            "Forced key rotation for agent '{}' — this is typically used after a key compromise",
            self.agent_id
        );
        self.rotate()
    }

    /// Force a rotation regardless of policy (stub without feature).
    #[cfg(not(feature = "key-rotation"))]
    pub fn force_rotate(&mut self) -> Result<RotatedKey, KeyRotatorError> {
        self.rotate()
    }

    /// Delete the key and metadata files.
    pub fn delete(self) -> Result<(), KeyRotatorError> {
        if self.key_path.exists() {
            secure_storage::delete_key(&self.key_path)
                .map_err(|e| KeyRotatorError::Storage(format!("delete key: {e}")))?;
        }
        if self.meta_path.exists() {
            fs::remove_file(&self.meta_path).map_err(KeyRotatorError::Io)?;
        }
        info!("Deleted key and metadata for agent '{}'", self.agent_id);
        Ok(())
    }
}

// ── Rotated key output ─────────────────────────────────────────────────────

/// The result of a successful key rotation.
#[derive(Clone)]
pub struct RotatedKey {
    /// Agent ID.
    pub agent_id: String,
    /// New signing key seed (32 bytes — keep secret!).
    pub signing_key_seed: [u8; 32],
    /// New verifying key bytes (32 bytes — public, can be shared).
    pub verifying_key_bytes: [u8; 32],
    /// Unix timestamp of rotation.
    pub rotated_at: i64,
}

// Manual Debug impl — redacts the secret seed to prevent accidental logging.
impl std::fmt::Debug for RotatedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RotatedKey")
            .field("agent_id", &self.agent_id)
            .field("signing_key_seed", &"<redacted>")
            .field(
                "verifying_key_bytes",
                &hex::encode(&self.verifying_key_bytes),
            )
            .field("rotated_at", &self.rotated_at)
            .finish()
    }
}

// Simple hex encoding for Debug (avoid pulling in a hex crate for one use).
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// Errors during key rotation operations.
#[derive(Debug)]
pub enum KeyRotatorError {
    /// I/O error.
    Io(io::Error),
    /// Key storage error.
    Storage(String),
    /// A required feature is not enabled.
    FeatureDisabled(String),
}

impl std::fmt::Display for KeyRotatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyRotatorError::Io(e) => write!(f, "I/O error: {e}"),
            KeyRotatorError::Storage(s) => write!(f, "storage error: {s}"),
            KeyRotatorError::FeatureDisabled(s) => write!(f, "feature disabled: {s}"),
        }
    }
}

impl std::error::Error for KeyRotatorError {}

// ── Metadata persistence helpers ───────────────────────────────────────────

/// Resolve the metadata path for a key file.
/// For `~/.a2x/keys/agent.key`, metadata is at `~/.a2x/keys/agent.key.meta`.
fn metadata_path(key_path: &std::path::Path) -> PathBuf {
    let mut meta = key_path.as_os_str().to_os_string();
    meta.push(".meta");
    PathBuf::from(meta)
}

/// Load rotation metadata from disk.
fn load_metadata(path: &std::path::Path) -> Option<RotationMetadata> {
    let data = fs::read(path).ok()?;
    if data.len() != 24 {
        warn!(
            "Rotation metadata at {} has unexpected length {} (expected 24), ignoring",
            path.display(),
            data.len()
        );
        return None;
    }
    let mut bytes = [0u8; 24];
    bytes.copy_from_slice(&data);
    Some(RotationMetadata::from_bytes(&bytes))
}

/// Save rotation metadata to disk.
fn save_metadata(
    path: &std::path::Path,
    metadata: &RotationMetadata,
) -> Result<(), KeyRotatorError> {
    let bytes = metadata.to_bytes();
    fs::write(path, bytes).map_err(|e| {
        KeyRotatorError::Io(io::Error::new(e.kind(), format!("write metadata: {e}")))
    })?;
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Get the current Unix timestamp in seconds.
fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_meta_path(name: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir();
        let key_path = dir.join(format!("a2x-kr-{}.key", name));
        let meta_path = metadata_path(&key_path);
        (key_path, meta_path)
    }

    #[test]
    fn test_metadata_roundtrip() {
        let meta = RotationMetadata {
            last_rotated: 1_750_000_000,
            signature_count: 42,
            created_at: 1_749_000_000,
        };
        let bytes = meta.to_bytes();
        let restored = RotationMetadata::from_bytes(&bytes);
        assert_eq!(restored.last_rotated, 1_750_000_000);
        assert_eq!(restored.signature_count, 42);
        assert_eq!(restored.created_at, 1_749_000_000);
    }

    #[test]
    fn test_metadata_new_uses_current_time() {
        let meta = RotationMetadata::new();
        let now = current_unix_timestamp();
        // Within 2 seconds (allow for test execution time)
        assert!((meta.last_rotated - now).abs() <= 2);
        assert_eq!(meta.signature_count, 0);
    }

    #[test]
    fn test_save_and_load_metadata() {
        let (_, meta_path) = temp_meta_path("save-load");
        let meta = RotationMetadata {
            last_rotated: 1_700_000_000,
            signature_count: 99,
            created_at: 1_600_000_000,
        };
        save_metadata(&meta_path, &meta).unwrap();
        let loaded = load_metadata(&meta_path).unwrap();
        assert_eq!(loaded.last_rotated, meta.last_rotated);
        assert_eq!(loaded.signature_count, meta.signature_count);
        assert_eq!(loaded.created_at, meta.created_at);

        // Cleanup
        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_load_nonexistent_metadata_returns_none() {
        let path = std::env::temp_dir().join("a2x-kr-nonexistent.key.meta");
        assert!(load_metadata(&path).is_none());
    }

    #[test]
    fn test_load_corrupt_metadata_returns_none() {
        let (_, meta_path) = temp_meta_path("corrupt");
        fs::write(&meta_path, b"not-24-bytes").unwrap();
        assert!(load_metadata(&meta_path).is_none());

        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_metadata_path_resolution() {
        let key = std::path::PathBuf::from("/home/user/.a2x/keys/agent-1.key");
        let meta = metadata_path(&key);
        assert_eq!(
            meta,
            std::path::PathBuf::from("/home/user/.a2x/keys/agent-1.key.meta")
        );
    }

    #[test]
    fn test_should_rotate_never() {
        let policy = KeyRotationPolicy::Never;
        // We need to construct a KeyRotator without actually needing a key path
        // Let's test the policy logic directly by using a temporary directory
        let dir = std::env::temp_dir();
        let key_path = dir.join("a2x-kr-never-test.key");
        let meta_path = metadata_path(&key_path);

        let meta = RotationMetadata {
            last_rotated: 0, // very old
            signature_count: 1_000_000,
            created_at: 0,
        };
        save_metadata(&meta_path, &meta).unwrap();

        let rotator = KeyRotator {
            agent_id: "never-agent".into(),
            policy,
            metadata: meta,
            key_path: key_path.clone(),
            meta_path: meta_path.clone(),
        };

        assert!(!rotator.should_rotate());

        // Cleanup
        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_should_rotate_time_based() {
        let policy = KeyRotationPolicy::TimeBased { interval_days: 30 };

        // Key rotated 31 days ago → should rotate
        let old_meta = RotationMetadata {
            last_rotated: current_unix_timestamp() - 31 * 86_400,
            signature_count: 0,
            created_at: 0,
        };
        let dir = std::env::temp_dir();
        let key_path = dir.join("a2x-kr-time-test.key");
        let meta_path = metadata_path(&key_path);
        save_metadata(&meta_path, &old_meta).unwrap();

        let rotator = KeyRotator {
            agent_id: "time-agent".into(),
            policy: policy.clone(),
            metadata: old_meta,
            key_path: key_path.clone(),
            meta_path: meta_path.clone(),
        };
        assert!(rotator.should_rotate());

        // Key rotated 1 day ago → should NOT rotate
        let fresh_meta = RotationMetadata {
            last_rotated: current_unix_timestamp() - 86_400,
            signature_count: 0,
            created_at: 0,
        };
        let rotator2 = KeyRotator {
            agent_id: "time-agent".into(),
            policy,
            metadata: fresh_meta,
            key_path,
            meta_path: meta_path.clone(),
        };
        assert!(!rotator2.should_rotate());

        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_should_rotate_usage_based() {
        let policy = KeyRotationPolicy::UsageBased {
            max_signatures: 1000,
        };

        let below_meta = RotationMetadata {
            last_rotated: 0,
            signature_count: 500,
            created_at: 0,
        };
        let dir = std::env::temp_dir();
        let key_path = dir.join("a2x-kr-usage-test.key");
        let meta_path = metadata_path(&key_path);
        save_metadata(&meta_path, &below_meta).unwrap();

        let rotator = KeyRotator {
            agent_id: "usage-agent".into(),
            policy: policy.clone(),
            metadata: below_meta,
            key_path: key_path.clone(),
            meta_path: meta_path.clone(),
        };
        assert!(!rotator.should_rotate(), "500 < 1000 should not rotate");

        let at_limit_meta = RotationMetadata {
            last_rotated: 0,
            signature_count: 1000,
            created_at: 0,
        };
        let rotator2 = KeyRotator {
            agent_id: "usage-agent".into(),
            policy,
            metadata: at_limit_meta,
            key_path,
            meta_path: meta_path.clone(),
        };
        assert!(rotator2.should_rotate(), "1000 >= 1000 should rotate");

        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_record_signature_increments_counter() {
        let dir = std::env::temp_dir();
        let key_path = dir.join("a2x-kr-sig-test.key");
        let meta_path = metadata_path(&key_path);

        let meta = RotationMetadata::new();
        save_metadata(&meta_path, &meta).unwrap();

        let mut rotator = KeyRotator {
            agent_id: "sig-agent".into(),
            policy: KeyRotationPolicy::Never,
            metadata: meta,
            key_path: key_path.clone(),
            meta_path: meta_path.clone(),
        };

        assert_eq!(rotator.signature_count(), 0);
        rotator.record_signature().unwrap();
        assert_eq!(rotator.signature_count(), 1);
        rotator.record_signature().unwrap();
        assert_eq!(rotator.signature_count(), 2);

        // Verify persistence
        let loaded = load_metadata(&meta_path).unwrap();
        assert_eq!(loaded.signature_count, 2);

        // Cleanup
        let _ = fs::remove_file(&meta_path);
    }

    #[test]
    fn test_policy_default_is_90_days() {
        let policy = KeyRotationPolicy::default();
        assert_eq!(policy, KeyRotationPolicy::TimeBased { interval_days: 90 });
    }

    #[test]
    fn test_set_policy_updates() {
        let dir = std::env::temp_dir();
        let key_path = dir.join("a2x-kr-policy-test.key");
        let meta_path = metadata_path(&key_path);

        let meta = RotationMetadata::new();
        save_metadata(&meta_path, &meta).unwrap();

        let mut rotator = KeyRotator {
            agent_id: "policy-agent".into(),
            policy: KeyRotationPolicy::Never,
            metadata: meta,
            key_path: key_path.clone(),
            meta_path: meta_path.clone(),
        };

        assert_eq!(rotator.policy(), &KeyRotationPolicy::Never);
        rotator.set_policy(KeyRotationPolicy::TimeBased { interval_days: 7 });
        assert_eq!(
            rotator.policy(),
            &KeyRotationPolicy::TimeBased { interval_days: 7 }
        );

        let _ = fs::remove_file(&meta_path);
    }
}
