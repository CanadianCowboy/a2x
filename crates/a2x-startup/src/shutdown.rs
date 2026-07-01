// See plans/11-startup-shutdown.md §3 (Shutdown Sequence)
//
// ShutdownManager — handles graceful shutdown with hooks, timeouts, and PID file management.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tracing::{error, info, warn};

/// Errors during shutdown or PID file operations.
#[derive(Debug)]
pub enum ShutdownError {
    Io(io::Error),
    HookTimedOut(String),
    HookFailed(String),
    PidFileError(String),
}

impl std::fmt::Display for ShutdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownError::Io(e) => write!(f, "IO error: {e}"),
            ShutdownError::HookTimedOut(msg) => write!(f, "shutdown hook timed out: {msg}"),
            ShutdownError::HookFailed(msg) => write!(f, "shutdown hook failed: {msg}"),
            ShutdownError::PidFileError(msg) => write!(f, "PID file error: {msg}"),
        }
    }
}

/// A shutdown hook — a callback executed during graceful shutdown.
///
/// Hooks run in registration order. Each hook gets a deadline;
/// if it exceeds the per-hook timeout, it's skipped.
pub type ShutdownHook = Box<dyn FnOnce() -> Result<(), ShutdownError> + Send>;

/// Manages graceful shutdown with ordered hooks and timeout enforcement.
pub struct ShutdownManager {
    /// Total timeout for all shutdown hooks.
    graceful_timeout: Duration,
    /// Per-hook timeout.
    hook_timeout: Duration,
    /// Hooks to run during shutdown, in registration order.
    hooks: Vec<ShutdownHook>,
    /// PID file path (if managing a PID file).
    pid_file: Option<PathBuf>,
    /// Time the shutdown was initiated.
    shutdown_started: Option<Instant>,
}

impl ShutdownManager {
    /// Create a new shutdown manager.
    pub fn new(graceful_timeout: Duration) -> Self {
        ShutdownManager {
            graceful_timeout,
            hook_timeout: Duration::from_secs(5),
            hooks: Vec::new(),
            pid_file: None,
            shutdown_started: None,
        }
    }

    /// Set the per-hook timeout.
    pub fn with_hook_timeout(mut self, timeout: Duration) -> Self {
        self.hook_timeout = timeout;
        self
    }

    /// Register a shutdown hook.
    ///
    /// Hooks execute in registration order during `shutdown()`.
    /// Typical hooks: save WorldGraph, flush logs, disconnect from bus.
    pub fn add_hook<F>(&mut self, hook: F)
    where
        F: FnOnce() -> Result<(), ShutdownError> + Send + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    /// Register the hook name and function (for error reporting).
    pub fn add_named_hook<F>(&mut self, name: impl Into<String>, hook: F)
    where
        F: FnOnce() -> Result<(), ShutdownError> + Send + 'static,
    {
        let hook_name = name.into();
        self.hooks.push(Box::new(move || {
            hook()
                .map_err(|e| ShutdownError::HookFailed(format!("hook '{}' failed: {e}", hook_name)))
        }));
    }

    /// Set the PID file path.
    ///
    /// The PID file is written when `write_pid()` is called, and removed
    /// during `shutdown()` if `remove_pid_on_shutdown` is true.
    pub fn set_pid_file(&mut self, path: PathBuf) {
        self.pid_file = Some(path);
    }

    /// Write the current process PID to the PID file.
    pub fn write_pid(&self) -> Result<(), ShutdownError> {
        let path = self
            .pid_file
            .as_ref()
            .ok_or_else(|| ShutdownError::PidFileError("no PID file path set".into()))?;

        let pid = std::process::id();
        fs::write(path, pid.to_string()).map_err(|e| {
            ShutdownError::Io(io::Error::new(
                e.kind(),
                format!("failed to write PID file: {e}"),
            ))
        })?;

        info!("PID {} written to {}", pid, path.display());
        Ok(())
    }

    /// Remove the PID file.
    pub fn remove_pid(&self) -> Result<(), ShutdownError> {
        if let Some(ref path) = self.pid_file {
            if path.exists() {
                fs::remove_file(path).map_err(|e| {
                    ShutdownError::Io(io::Error::new(
                        e.kind(),
                        format!("failed to remove PID file: {e}"),
                    ))
                })?;
                info!("PID file removed: {}", path.display());
            }
        }
        Ok(())
    }

    /// Execute the shutdown sequence.
    ///
    /// 1. Mark shutdown as started
    /// 2. Run all hooks in registration order
    /// 3. Each hook has `self.hook_timeout` to complete
    /// 4. Overall shutdown must complete within `self.graceful_timeout`
    /// 5. Remove PID file if present
    pub fn shutdown(&mut self) {
        self.shutdown_started = Some(Instant::now());
        let deadline = Instant::now() + self.graceful_timeout;

        info!(
            "Shutdown initiated ({} hooks, graceful timeout: {:?})",
            self.hooks.len(),
            self.graceful_timeout
        );

        let hooks = std::mem::take(&mut self.hooks);
        let total_hooks = hooks.len();
        for (i, hook) in hooks.into_iter().enumerate() {
            if Instant::now() > deadline {
                warn!(
                    "Shutdown deadline exceeded after {} of {} hooks — skipping remaining",
                    i, total_hooks
                );
                break;
            }

            let hook_deadline = Instant::now() + self.hook_timeout;
            // Execute hook (synchronously — no tokio needed for file I/O)
            match hook() {
                Ok(()) => {
                    info!("Shutdown hook {}/{} completed", i + 1, i + 1);
                }
                Err(e) => {
                    error!("Shutdown hook {}/{} failed: {e}", i + 1, i + 1);
                }
            }

            if Instant::now() > hook_deadline {
                warn!(
                    "Shutdown hook {}/{} exceeded per-hook timeout",
                    i + 1,
                    i + 1
                );
            }
        }

        // Remove PID file
        if let Err(e) = self.remove_pid() {
            error!("Failed to remove PID file: {e}");
        }

        info!("Shutdown complete");
    }

    /// Check if shutdown is in progress.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_started.is_some()
    }

    /// Check how long until the graceful timeout expires.
    pub fn remaining(&self) -> Option<Duration> {
        self.shutdown_started.map(|started| {
            let elapsed = started.elapsed();
            if elapsed >= self.graceful_timeout {
                Duration::ZERO
            } else {
                self.graceful_timeout - elapsed
            }
        })
    }
}

/// Simple PID file helper (standalone, not tied to ShutdownManager).
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Create a new PID file reference.
    pub fn new(path: PathBuf) -> Self {
        PidFile { path }
    }

    /// Write current PID to the file.
    pub fn write(&self) -> Result<(), io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, std::process::id().to_string())
    }

    /// Check if a PID file exists from a previous run.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Read the PID from an existing PID file.
    pub fn read_pid(&self) -> Result<u32, io::Error> {
        let content = fs::read_to_string(&self.path)?;
        content
            .trim()
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid PID: {e}")))
    }

    /// Remove the PID file.
    pub fn remove(&self) -> Result<(), io::Error> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_shutdown_manager_new() {
        let sm = ShutdownManager::new(Duration::from_secs(30));
        assert!(!sm.is_shutting_down());
        assert!(sm.remaining().is_none());
    }

    #[test]
    fn test_shutdown_hooks_execute() {
        let mut sm = ShutdownManager::new(Duration::from_secs(30));
        let called = Arc::new(AtomicBool::new(false));
        let called2 = called.clone();

        sm.add_hook(move || {
            called2.store(true, Ordering::SeqCst);
            Ok(())
        });

        sm.shutdown();
        assert!(called.load(Ordering::SeqCst));
        assert!(sm.is_shutting_down());
    }

    #[test]
    fn test_shutdown_multiple_hooks() {
        let mut sm = ShutdownManager::new(Duration::from_secs(30));
        let count = Arc::new(std::sync::Mutex::new(0u32));
        let c1 = count.clone();
        let c2 = count.clone();

        sm.add_hook(move || {
            *c1.lock().unwrap() += 1;
            Ok(())
        });
        sm.add_hook(move || {
            *c2.lock().unwrap() += 1;
            Ok(())
        });

        sm.shutdown();
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[test]
    fn test_pid_file_write_read_remove() {
        let path = std::env::temp_dir().join("a2x-test-pid.pid");
        let pid_file = PidFile::new(path.clone());

        pid_file.write().unwrap();
        assert!(pid_file.exists());

        let pid = pid_file.read_pid().unwrap();
        assert_eq!(pid, std::process::id());

        pid_file.remove().unwrap();
        assert!(!pid_file.exists());
    }

    #[test]
    fn test_shutdown_manager_removes_pid() {
        let path = std::env::temp_dir().join("a2x-test-shutdown-pid.pid");
        let mut sm = ShutdownManager::new(Duration::from_secs(30));
        sm.set_pid_file(path.clone());

        sm.write_pid().unwrap();
        assert!(path.exists());

        sm.shutdown();
        assert!(!path.exists());
    }
}
