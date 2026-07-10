// See plans/12-security.md §6 — Key Rotation & Secrets Management
//
// Secure key storage with filesystem permission enforcement.
//
// On Unix: uses chmod 0600 for key files and chmod 0700 for the keys directory.
// On Windows: sets the file to readonly and logs an informational message
// (full ACL-based security would require the `windows` crate).
//
// Key files live at: ~/.a2x/keys/<agent-id>.key
// TLS keys live at:  ~/.a2x/keys/tls/<cert-or-key-name>.pem

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Errors during secure key storage operations.
#[derive(Debug)]
pub enum KeyStorageError {
    /// I/O error (read/write/create).
    Io(io::Error),
    /// Home directory not found.
    HomeDirNotFound,
    /// Key file not found.
    NotFound(PathBuf),
    /// Permission setting failed (platform-specific).
    PermissionDenied(PathBuf, String),
}

impl std::fmt::Display for KeyStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyStorageError::Io(e) => write!(f, "I/O error: {e}"),
            KeyStorageError::HomeDirNotFound => {
                write!(f, "home directory not found (set HOME or USERPROFILE)")
            }
            KeyStorageError::NotFound(path) => {
                write!(f, "key file not found: {}", path.display())
            }
            KeyStorageError::PermissionDenied(path, reason) => {
                write!(
                    f,
                    "failed to set permissions on {}: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for KeyStorageError {}

/// Resolve the A2X home directory (~/.a2x).
fn a2x_home_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".a2x"))
}

/// Get the path to the keys directory (~/.a2x/keys/).
pub fn keys_dir() -> Result<PathBuf, KeyStorageError> {
    let home = a2x_home_dir().ok_or(KeyStorageError::HomeDirNotFound)?;
    Ok(home.join("keys"))
}

/// Get the path for a specific agent's key file.
pub fn agent_key_path(agent_id: &str) -> Result<PathBuf, KeyStorageError> {
    if agent_id.is_empty() {
        return Err(KeyStorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "agent_id must not be empty",
        )));
    }
    let dir = keys_dir()?;
    // Sanitize agent_id for use as filename: replace path separators.
    // Joining to an absolute base path (keys_dir) prevents parent traversal
    // even if agent_id contains ".." — the result stays within keys_dir.
    let safe_id = agent_id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    Ok(dir.join(format!("{}.key", safe_id)))
}

/// Get the path for a TLS key file.
pub fn tls_key_path(name: &str) -> Result<PathBuf, KeyStorageError> {
    let dir = keys_dir()?;
    let safe_name = name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    Ok(dir.join("tls").join(safe_name))
}

/// Ensure the keys directory exists with proper permissions.
///
/// Creates `~/.a2x/keys/` and `~/.a2x/keys/tls/` if they don't exist.
/// Always reapplies secure permissions, even if the directories already exist
/// (defense-in-depth against accidental chmod changes).
/// On Unix, sets directory permissions to 0700 (owner-only).
///
/// Returns the path to the keys directory on success.
pub fn ensure_keys_dir() -> Result<PathBuf, KeyStorageError> {
    let dir = keys_dir()?;

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| {
            KeyStorageError::Io(io::Error::new(e.kind(), format!("create keys dir: {e}")))
        })?;
        info!("Created keys directory at {}", dir.display());
    }
    // Always reapply permissions (defense-in-depth)
    set_secure_dir_permissions(&dir)?;

    // Ensure TLS subdirectory
    let tls_dir = dir.join("tls");
    if !tls_dir.exists() {
        fs::create_dir_all(&tls_dir).map_err(|e| {
            KeyStorageError::Io(io::Error::new(e.kind(), format!("create tls dir: {e}")))
        })?;
    }
    set_secure_dir_permissions(&tls_dir)?;

    Ok(dir)
}

/// Save binary key data to a file with secure permissions.
///
/// On Unix: file is created with mode 0600 (owner read/write only).
/// Uses atomic write (tmp + rename) for crash safety.
///
/// On Windows: Windows Defender/AV may temporarily lock files after they
/// are read or written. To avoid "Access Denied" on overwrite, we:
///
/// 1. Back up the existing file (copy + retry)
/// 2. Delete the target file (with retry, since AV may hold a handle)
/// 3. Write the new file fresh (no overwrite → no AV conflict)
///
/// This is structurally safe because the backup preserves the old key.
pub fn save_key(path: &Path, data: &[u8]) -> Result<(), KeyStorageError> {
    // If file exists, back it up first
    if path.exists() {
        let bak = path.with_extension("bak");
        if let Err(e) = fs::copy(path, &bak) {
            warn!("Failed to back up key file {}: {e}", path.display());
        } else {
            set_secure_file_permissions(&bak)?;
            info!("Backed up key to {}", bak.display());
        }
    }

    // On Windows: AV holds file handles after copy. Delete the target first
    // (with retry), then write fresh to avoid any overwrite-path lock.
    #[cfg(windows)]
    {
        if path.exists() {
            remove_with_retry(path).map_err(|e| {
                KeyStorageError::Io(io::Error::new(e.kind(), format!("remove key: {e}")))
            })?;
        }
        fs::write(path, data).map_err(|e| {
            KeyStorageError::Io(io::Error::new(e.kind(), format!("write key: {e}")))
        })?;
    }

    // On Unix: atomic write via tmp + rename (crash-safe, no AV issues)
    #[cfg(not(windows))]
    {
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, data).map_err(|e| {
            KeyStorageError::Io(io::Error::new(e.kind(), format!("write key tmp: {e}")))
        })?;
        fs::rename(&tmp_path, path).map_err(|e| {
            KeyStorageError::Io(io::Error::new(e.kind(), format!("rename key: {e}")))
        })?;
    }

    // Set secure permissions (Unix: chmod 600)
    set_secure_file_permissions(path)?;

    info!("Saved key to {}", path.display());
    Ok(())
}

/// Load binary key data from a file.
///
/// Returns `None` if the file doesn't exist (caller decides if that's an error).
pub fn load_key(path: &Path) -> Result<Option<Vec<u8>>, KeyStorageError> {
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read(path)
        .map_err(|e| KeyStorageError::Io(io::Error::new(e.kind(), format!("read key: {e}"))))?;

    Ok(Some(data))
}

/// Delete a key file (with secure overwrite attempt).
///
/// On Unix: attempts a single-pass zero overwrite before unlinking.
/// Key files are expected to be small (< 1 MiB), so allocation is safe.
/// On Windows: deletes directly (no overwrite).
pub fn delete_key(path: &Path) -> Result<(), KeyStorageError> {
    if !path.exists() {
        return Err(KeyStorageError::NotFound(path.to_path_buf()));
    }

    // Attempt secure overwrite (best-effort). Key files are small
    // (Ed25519 keys are 32–64 bytes), so allocation is bounded.
    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(path) {
            let len = metadata.len() as usize;
            if len < 1024 * 1024 {
                let zeros = vec![0u8; len];
                let _ = fs::write(path, &zeros); // best-effort overwrite
            }
        }
    }

    fs::remove_file(path)
        .map_err(|e| KeyStorageError::Io(io::Error::new(e.kind(), format!("delete key: {e}"))))?;

    // Also remove backup and temp files if they exist
    let bak = path.with_extension("bak");
    let _ = fs::remove_file(&bak);
    let tmp = path.with_extension("tmp");
    let _ = fs::remove_file(&tmp);

    info!("Deleted key at {}", path.display());
    Ok(())
}

/// Retry file removal on Windows (up to 20 attempts, exponential backoff).
///
/// Windows Defender/AV may hold handles on recently-read files for scanning.
/// Exponential backoff from 50ms up to ~6.4s gives AV time to release.
#[cfg(windows)]
fn remove_with_retry(path: &Path) -> io::Result<()> {
    use std::{thread, time::Duration};
    let mut delay_ms = 50u64;
    for attempt in 0..20 {
        match fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && attempt < 19 => {
                thread::sleep(Duration::from_millis(delay_ms));
                delay_ms = (delay_ms * 2).min(1000); // cap at 1s per attempt
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

// ── Platform-specific permission helpers ───────────────────────────────────

/// Set secure permissions on a key file.
///
/// Unix: chmod 0600 (owner read+write only).
/// Windows: no-op (the readonly flag would block subsequent writes via
/// `fs::write`, which breaks key rotation. ACL-based security requires
/// the `windows` crate).
fn set_secure_file_permissions(path: &Path) -> Result<(), KeyStorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)
            .map_err(|e| KeyStorageError::Io(io::Error::new(e.kind(), format!("metadata: {e}"))))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)
            .map_err(|e| KeyStorageError::PermissionDenied(path.to_path_buf(), e.to_string()))?;
    }

    #[cfg(windows)]
    {
        // Don't set readonly on Windows — it blocks subsequent writes
        // and is trivially bypassed via Properties > uncheck "Read-only".
        // For production Windows deployments, use EFS or BitLocker.
        info!(
            "Key stored at {}. For production Windows deployments, use EFS or BitLocker for encryption at rest.",
            path.display()
        );
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        warn!("Secure file permissions not supported on this platform");
    }

    Ok(())
}

/// Set secure permissions on a directory.
///
/// Unix: chmod 0700 (owner rwx only).
/// Windows: no-op (ACL-based security would require the `windows` crate).
fn set_secure_dir_permissions(path: &Path) -> Result<(), KeyStorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)
            .map_err(|e| KeyStorageError::Io(io::Error::new(e.kind(), format!("metadata: {e}"))))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(path, perms)
            .map_err(|e| KeyStorageError::PermissionDenied(path.to_path_buf(), e.to_string()))?;
    }

    #[cfg(windows)]
    {
        // Muted to debug level — this runs on every startup and would spam
        // production logs. Users deploying on Windows should use EFS/BitLocker.
        tracing::debug!(
            "Keys directory {} exists. On Windows, restrict access via folder Properties > Security.",
            path.display()
        );
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        warn!("Secure directory permissions not supported on this platform");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("a2x-secure-storage-test-{}", name))
    }

    #[test]
    fn test_save_and_load_key_roundtrip() {
        let path = temp_path("roundtrip.key");
        let data = b"super-secret-ed25519-key-material-32-bytes!!";

        save_key(&path, data).unwrap();
        let loaded = load_key(&path).unwrap().unwrap();
        assert_eq!(loaded, data);

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("bak"));
        let _ = fs::remove_file(path.with_extension("tmp"));
    }

    #[test]
    fn test_load_nonexistent_key_returns_none() {
        let path = temp_path("nonexistent.key");
        let result = load_key(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_overwrites_existing_key() {
        let path = temp_path("overwrite.key");

        save_key(&path, b"version-1").unwrap();
        save_key(&path, b"version-2").unwrap();

        let loaded = load_key(&path).unwrap().unwrap();
        assert_eq!(loaded, b"version-2");

        // Backup should contain version-1
        let bak = path.with_extension("bak");
        let bak_data = fs::read(&bak).unwrap();
        assert_eq!(bak_data, b"version-1");

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_file(path.with_extension("tmp"));
    }

    #[test]
    fn test_delete_key_removes_file() {
        let path = temp_path("delete.key");

        save_key(&path, b"to-be-deleted").unwrap();
        assert!(path.exists());

        delete_key(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_nonexistent_key_errors() {
        let path = temp_path("nonexistent-delete.key");
        let result = delete_key(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_keys_dir_creates_directory() {
        let path = temp_path("keys-dir");
        // Override HOME for this test
        let home = path.parent().unwrap();
        // We can't easily override a2x_home_dir in unit tests since it reads
        // process env vars. Instead, test the public API by calling save_key
        // with an explicit temp path (already tested above). For the dir
        // creation, verify the function works when we control the env.
        let _ = home;
        // This test validates the API shape compiles and basic logic works.
        // Full integration test would set HOME and call ensure_keys_dir().
        let dir_result = ensure_keys_dir();
        // If HOME is set, this should succeed; if not, it returns HomeDirNotFound
        match dir_result {
            Ok(dir) => {
                assert!(dir.exists());
                info!("Keys dir exists at {}", dir.display());
            }
            Err(KeyStorageError::HomeDirNotFound) => {
                // Expected in CI without HOME set
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn test_agent_key_path_sanitizes_id() {
        let result = agent_key_path("evil/../../etc/passwd");
        match result {
            Ok(path) => {
                let filename = path.file_name().unwrap().to_str().unwrap();
                // Slashes should be replaced with underscores
                assert!(!filename.contains('/'));
                assert!(!filename.contains('\\'));
                assert!(filename.ends_with(".key"));
                assert!(filename.contains("evil"));
            }
            Err(KeyStorageError::HomeDirNotFound) => {
                // Expected in CI environments without HOME set
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn test_tls_key_path_creates_path() {
        let result = tls_key_path("gateway-cert.pem");
        match result {
            Ok(path) => {
                assert!(path.ends_with("gateway-cert.pem"));
                assert!(path.to_str().unwrap().contains("tls"));
            }
            Err(KeyStorageError::HomeDirNotFound) => {
                // Expected in CI
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn test_key_file_permissions_are_restrictive_unix() {
        let path = temp_path("perms.key");
        save_key(&path, b"test-key-data-32-bytes-long!!").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path).unwrap();
            let mode = metadata.permissions().mode();
            // Should be exactly 0o600 (or 0o100600 with file type bits)
            assert_eq!(
                mode & 0o777,
                0o600,
                "key file permissions should be 0600, got {:o}",
                mode & 0o777
            );
        }

        // Cleanup
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("bak"));
        let _ = fs::remove_file(path.with_extension("tmp"));
    }
}
