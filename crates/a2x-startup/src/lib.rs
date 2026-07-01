// a2x-startup — System startup, shutdown, configuration, and state persistence
//
// See plans/11-startup-shutdown.md for the full design specification.
//
// Modules:
//   config.rs      — A2xConfig loading from ~/.a2x/, directory creation, first-run setup
//   shutdown.rs    — ShutdownManager with hooks, graceful timeout, PID file
//   persistence.rs — Atomic state save/load with Blake3 checksums

pub mod config;
pub mod key_rotation;
pub mod persistence;
pub mod resilience;
pub mod secure_storage;
pub mod shutdown;

// Re-exports
pub use config::A2xConfig;
pub use persistence::{
    load_state, load_state_atomic, save_state, save_state_atomic, StateSaveError,
};
pub use resilience::{
    AgentSupervisor, DegradationMode, DegradationSummary, FaultAction, InstructionFaultMode,
    MemoryPressureAction, ProgramWatchdog, ResourceMonitor, ResourceStatus, RetryPolicy,
    TimeoutAction, WatchdogError,
};
pub use secure_storage::{
    agent_key_path, delete_key, ensure_keys_dir, keys_dir, load_key, save_key, tls_key_path,
    KeyStorageError,
};
pub use shutdown::{PidFile, ShutdownHook, ShutdownManager};

// Key rotation (requires key-rotation feature for rotate/force_rotate)
pub use key_rotation::{
    KeyRotationPolicy, KeyRotator, KeyRotatorError, RotatedKey, RotationMetadata,
};
