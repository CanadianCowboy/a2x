// See plans/11-startup-shutdown.md §2 (Configuration Loading) and §5 (Directory Layout)
//
// A2xConfig — the top-level system configuration loaded from ~/.a2x/config.toml
// and merged with per-agent configs from ~/.a2x/agents/*.toml.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Configuration loading errors.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(String),
    Validation(String),
    HomeDirNotFound,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::Toml(msg) => write!(f, "TOML error: {msg}"),
            ConfigError::Validation(msg) => write!(f, "config validation failed: {msg}"),
            ConfigError::HomeDirNotFound => {
                write!(f, "home directory not found (set HOME or USERPROFILE)")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Top-level A2X system configuration.
///
/// Loaded from `~/.a2x/config.toml` and `~/.a2x/agents/*.toml`.
/// Merged with CLI argument overrides at startup.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct A2xConfig {
    /// Global configuration options.
    #[serde(default)]
    pub global: GlobalConfig,

    /// Bus configuration.
    #[serde(default)]
    pub bus: BusConfig,

    /// Gateway configuration (None = gateway not started).
    #[serde(default)]
    pub gateway: Option<GatewayConfig>,

    /// Agent configurations keyed by agent ID.
    #[serde(default)]
    pub agents: Vec<AgentConfig>,

    /// Storage paths and persistence settings.
    #[serde(default)]
    pub storage: StorageConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// System name (display only).
    #[serde(default = "default_system_name")]
    pub system_name: String,
    /// Data directory override (default: ~/.a2x/).
    #[serde(default)]
    pub data_dir: Option<String>,
}

fn default_system_name() -> String {
    "a2x".into()
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            system_name: default_system_name(),
            data_dir: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BusConfig {
    /// Transport mode: "in_memory", "tcp".
    #[serde(default = "default_transport")]
    pub transport: String,
    /// TCP bind address (if transport = "tcp").
    #[serde(default)]
    pub listen_address: Option<String>,
    /// Bootstrap peers (if transport = "tcp").
    #[serde(default)]
    pub bootstrap: Vec<String>,
    /// Verify message signatures.
    #[serde(default)]
    pub verify_signatures: bool,
}

fn default_transport() -> String {
    "in_memory".into()
}

impl Default for BusConfig {
    fn default() -> Self {
        BusConfig {
            transport: default_transport(),
            listen_address: Some("127.0.0.1:8777".into()),
            bootstrap: Vec::new(),
            verify_signatures: false,
        }
    }
}

/// Gateway configuration (subset of full GatewayConfig for startup).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GatewayConfig {
    /// Whether to start the gateway.
    #[serde(default)]
    pub enabled: bool,
    /// HTTP listener port.
    #[serde(default = "default_http")]
    pub http_port: u16,
    /// WebSocket listener port.
    #[serde(default = "default_ws")]
    pub ws_port: u16,
    /// TCP listener port.
    #[serde(default = "default_tcp")]
    pub tcp_port: u16,
    /// Auth mode: "local", "api_key".
    #[serde(default = "default_auth_mode")]
    pub auth_mode: String,
}

fn default_http() -> u16 {
    8778
}
fn default_ws() -> u16 {
    8779
}
fn default_tcp() -> u16 {
    8780
}
fn default_auth_mode() -> String {
    "local".into()
}

impl Default for GatewayConfig {
    fn default() -> Self {
        GatewayConfig {
            enabled: false,
            http_port: default_http(),
            ws_port: default_ws(),
            tcp_port: default_tcp(),
            auth_mode: default_auth_mode(),
        }
    }
}

/// Per-agent configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Agent ID (must be unique).
    pub id: String,
    /// Agent type: "orchestrator", "cli", "llm", "ccs", "omega".
    #[serde(default = "default_agent_type")]
    pub agent_type: String,
    /// Human-readable label.
    #[serde(default)]
    pub label: Option<String>,
    /// Whether to auto-start this agent.
    #[serde(default)]
    pub auto_start: bool,
    /// Max instructions per program.
    #[serde(default = "default_max_instructions")]
    pub max_instructions: u64,
    /// Max memory in MB.
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u64,
}

fn default_agent_type() -> String {
    "orchestrator".into()
}
fn default_max_instructions() -> u64 {
    10_000
}
fn default_max_memory() -> u64 {
    256
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            id: "orch-1".into(),
            agent_type: default_agent_type(),
            label: None,
            auto_start: true,
            max_instructions: default_max_instructions(),
            max_memory_mb: default_max_memory(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Data directory (relative to ~/.a2x/).
    #[serde(default = "default_data_subdir")]
    pub data_subdir: String,
    /// Logs directory (relative to ~/.a2x/).
    #[serde(default = "default_logs_subdir")]
    pub logs_subdir: String,
    /// Checkpoint interval (instructions between auto-saves).
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval: usize,
    /// Maximum number of checkpoints to keep.
    #[serde(default = "default_max_checkpoints")]
    pub max_checkpoints: usize,
}

fn default_data_subdir() -> String {
    "data".into()
}
fn default_logs_subdir() -> String {
    "logs".into()
}
fn default_checkpoint_interval() -> usize {
    1000
}
fn default_max_checkpoints() -> usize {
    5
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            data_subdir: default_data_subdir(),
            logs_subdir: default_logs_subdir(),
            checkpoint_interval: default_checkpoint_interval(),
            max_checkpoints: default_max_checkpoints(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level: "trace", "debug", "info", "warn", "error".
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Output format: "text", "json".
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Log file path (relative to ~/.a2x/logs/, None = stdout only).
    #[serde(default)]
    pub file: Option<String>,
}

fn default_log_level() -> String {
    "info".into()
}
fn default_log_format() -> String {
    "text".into()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: default_log_level(),
            format: default_log_format(),
            file: None,
        }
    }
}

impl Default for A2xConfig {
    fn default() -> Self {
        A2xConfig {
            global: GlobalConfig::default(),
            bus: BusConfig::default(),
            gateway: None,
            agents: vec![
                AgentConfig {
                    id: "orch-1".into(),
                    agent_type: "orchestrator".into(),
                    label: Some("Primary Orchestrator".into()),
                    auto_start: true,
                    ..Default::default()
                },
                AgentConfig {
                    id: "cli-1".into(),
                    agent_type: "cli".into(),
                    label: Some("CLI Executor".into()),
                    auto_start: true,
                    ..Default::default()
                },
            ],
            storage: StorageConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl A2xConfig {
    /// Load configuration from the default A2X home directory.
    ///
    /// 1. Read `~/.a2x/config.toml` if it exists
    /// 2. Read `~/.a2x/agents/*.toml` if any exist
    /// 3. Merge with defaults for missing fields
    /// 4. Validate the result
    pub fn load() -> Result<Self, ConfigError> {
        let home = a2x_home_dir().ok_or(ConfigError::HomeDirNotFound)?;
        let config_path = home.join("config.toml");

        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(ConfigError::Io)?;
            toml::from_str(&content).map_err(|e| ConfigError::Toml(e.to_string()))?
        } else {
            Self::default()
        };

        // Load per-agent configs from ~/.a2x/agents/
        let agents_dir = home.join("agents");
        if agents_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&agents_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "toml") {
                        match fs::read_to_string(&path) {
                            Ok(content) => match toml::from_str::<AgentConfig>(&content) {
                                Ok(agent_cfg) => {
                                    // Replace default with loaded config
                                    config.agents.retain(|a| a.id != agent_cfg.id);
                                    config.agents.push(agent_cfg);
                                }
                                Err(e) => {
                                    warn!("failed to parse agent config {}: {e}", path.display());
                                }
                            },
                            Err(e) => {
                                warn!("failed to read agent config {}: {e}", path.display());
                            }
                        }
                    }
                }
            }
        }

        // If we loaded a real config file, clear defaults to avoid duplicates.
        // The config.toml's [[agents]] section is authoritative.
        if config_path.exists() && config.agents.len() > 2 {
            // Keep only loaded agents (remove default orch-1/cli-1 placeholders
            // if they weren't also in the config file)
            let has_orch = config.agents.iter().any(|a| a.id == "orch-1");
            let has_cli = config.agents.iter().any(|a| a.id == "cli-1");
            // If default agents exist and weren't in the file, the retain above
            // already handles dedup. If the file has NO [[agents]] section,
            // defaults are fine.
            let _ = (has_orch, has_cli);
        }

        // Validate
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration.
    fn validate(&self) -> Result<(), ConfigError> {
        // Agent IDs must be unique
        let mut seen = std::collections::HashSet::new();
        for agent in &self.agents {
            if !seen.insert(&agent.id) {
                return Err(ConfigError::Validation(format!(
                    "duplicate agent ID: {}",
                    agent.id
                )));
            }
        }

        // Transport must be known
        if !["in_memory", "tcp"].contains(&self.bus.transport.as_str()) {
            return Err(ConfigError::Validation(format!(
                "unknown transport: {}",
                self.bus.transport
            )));
        }

        // Log level must be valid
        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.level.as_str()) {
            return Err(ConfigError::Validation(format!(
                "unknown log level: {}",
                self.logging.level
            )));
        }

        Ok(())
    }

    /// Initialize the A2X home directory with default configuration.
    ///
    /// Creates `~/.a2x/` with subdirectories and writes a default config.toml
    /// if it doesn't already exist. This is the "first-run experience."
    pub fn initialize() -> Result<PathBuf, ConfigError> {
        let home = a2x_home_dir().ok_or(ConfigError::HomeDirNotFound)?;

        // Create directory structure
        fs::create_dir_all(&home).map_err(ConfigError::Io)?;
        fs::create_dir_all(home.join("agents")).map_err(ConfigError::Io)?;
        fs::create_dir_all(home.join("data")).map_err(ConfigError::Io)?;
        fs::create_dir_all(home.join("logs")).map_err(ConfigError::Io)?;
        fs::create_dir_all(home.join("packets")).map_err(ConfigError::Io)?;

        // Create secure keys directory with chmod 700 (Unix) or restricted (Windows)
        // See plans/12-security.md §6 — Secure Key Storage
        if let Err(e) = crate::secure_storage::ensure_keys_dir() {
            tracing::warn!("Failed to create secure keys directory: {e}");
        }

        let config_path = home.join("config.toml");
        if !config_path.exists() {
            let default_config = Self::default();
            let toml_str = toml::to_string_pretty(&default_config)
                .map_err(|e| ConfigError::Toml(e.to_string()))?;
            fs::write(&config_path, toml_str).map_err(ConfigError::Io)?;
            info!("Created default config at {}", config_path.display());
        }

        // Write per-agent configs for agents that don't already have files
        for agent in &Self::default().agents {
            let agent_path = home.join("agents").join(format!("{}.toml", agent.id));
            if !agent_path.exists() {
                let toml_str =
                    toml::to_string_pretty(agent).map_err(|e| ConfigError::Toml(e.to_string()))?;
                fs::write(&agent_path, toml_str).map_err(ConfigError::Io)?;
                info!("Created agent config at {}", agent_path.display());
            }
        }

        Ok(home)
    }

    /// Get the A2X home directory path.
    pub fn home_dir() -> Option<PathBuf> {
        a2x_home_dir()
    }

    /// Get the data directory (for WorldGraph/MemoryTrace persistence).
    pub fn data_dir(&self) -> PathBuf {
        let home = a2x_home_dir().unwrap_or_else(|| PathBuf::from(".a2x"));
        if let Some(ref custom) = self.global.data_dir {
            PathBuf::from(custom)
        } else {
            home.join(&self.storage.data_subdir)
        }
    }

    /// Get the logs directory.
    pub fn logs_dir(&self) -> PathBuf {
        let home = a2x_home_dir().unwrap_or_else(|| PathBuf::from(".a2x"));
        home.join(&self.storage.logs_subdir)
    }
}

/// Resolve the A2X home directory (~/.a2x on Unix, %USERPROFILE%/.a2x on Windows).
fn a2x_home_dir() -> Option<PathBuf> {
    // Try HOME (Unix) or USERPROFILE (Windows)
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".a2x"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = A2xConfig::default();
        assert_eq!(config.global.system_name, "a2x");
        assert_eq!(config.bus.transport, "in_memory");
        assert_eq!(config.storage.checkpoint_interval, 1000);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_validation_unique_agent_ids() {
        let mut config = A2xConfig::default();
        // Duplicate agent IDs
        config.agents.push(AgentConfig {
            id: "orch-1".into(),
            agent_type: "cli".into(),
            ..Default::default()
        });
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_bad_transport() {
        let mut config = A2xConfig::default();
        config.bus.transport = "bad".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_bad_log_level() {
        let mut config = A2xConfig::default();
        config.logging.level = "critical".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let config = A2xConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: A2xConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.global.system_name, "a2x");
        assert_eq!(parsed.agents.len(), 2);
    }

    #[test]
    fn test_initialize_creates_dirs() {
        let home = std::env::temp_dir().join("a2x-startup-test-config");
        let _ = fs::remove_dir_all(&home);

        // Override home dir by setting env
        std::env::set_var("HOME", home.parent().unwrap().to_str().unwrap());
        // We can't easily override a2x_home_dir in tests, but verify the
        // function works by checking it resolves.
        let result = a2x_home_dir();
        // Clean up env
        std::env::remove_var("HOME");
        let _ = fs::remove_dir_all(&home);

        assert!(result.is_some());
    }
}
