# A2X Startup & Shutdown — Boot Order, Lifecycle & State Persistence

> **How the A2X system starts up, shuts down, and persists state across restarts.**

---

## 1. Overview

A2X is a multi-process, multi-agent system. Startup and shutdown must be **ordered**, **graceful**, and **recoverable**.

---

## 2. Startup Sequence

### Phase 1: Configuration Loading

1. Read `~/.a2x/config.toml` (system-wide defaults)
2. Read `~/.a2x/gateway.toml` (gateway config)
3. Read `~/.a2x/agents/*.toml` (per-agent configs)
4. Merge with CLI arguments (CLI args override config files)
5. Validate config (all required fields present, no contradictions)

```rust
/// Configuration root — loaded at startup.
pub struct A2xConfig {
    pub global: GlobalConfig,
    pub bus: BusConfig,
    pub agents: Vec<AgentConfig>,
    pub gateway: Option<GatewayConfig>,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

impl A2xConfig {
    /// Load from default paths + CLI overrides.
    pub fn load(cli_overrides: &CliOverrides) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // 1. Load config.toml
        if let Some(path) = &cli_overrides.config_path {
            config.merge_file(path)?;
        } else if let Some(home) = dirs::home_dir() {
            let default_path = home.join(".a2x/config.toml");
            if default_path.exists() {
                config.merge_file(&default_path)?;
            }
        }

        // 2. Apply CLI overrides (highest priority)
        config.apply_cli(cli_overrides)?;

        // 3. Validate
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Agents must have unique IDs
        // Gateway requires at least one listener
        // Storage paths must be writable (check at startup)
    }
}
```

### Phase 2: Storage Initialization

1. Create directory structure (`~/.a2x/`, `~/.a2x/data/`, `~/.a2x/logs/`)
2. Load persisted WorldGraph from disk (if exists)
3. Load persisted MemoryTrace from disk (if exists)
4. Verify integrity (checksum validation)

### Phase 3: Bus Startup

1. Start bus with configured transport (in-memory or TCP)
2. Bind to listen address
3. Start discovery service
4. Bus is ready — agents can now connect

### Phase 4: Agent Startup (Ordered)

```
Orchestrator ──► CLI Agent(s) ──► CCS Agent(s) ──► LLM Agent(s)
     │                │                │                │
     ▼                ▼                ▼                ▼
  Announce ──► Announce ──► Announce ──► Announce
     │                │                │                │
     └────────────────┴────────────────┴────────────────┘
                              │
                              ▼
                      Bus registers all
                      agents in routing table
```

**Rules:**
- Orchestrator starts first (no dependencies)
- CLI agents start second (need bus, don't need other agents)
- CCS agents start third (may depend on CLI for persistence)
- LLM agents start last (need bus + orchestration)

### Phase 5: Gateway Startup (Optional)

1. Start HTTP listener (port 8778)
2. Start WebSocket listener (port 8779)
3. Start TCP listener (port 8780)
4. Enable stdin/stdout listener
5. Announce gateway on bus: `"I am gateway, entity bridge ready"`
6. Accept incoming entity connections

### Phase 6: Ready Signal

When all configured components are running:
- Write PID file: `~/.a2x/a2x.pid`
- Emit `Ready` event on bus
- Begin accepting programs

---

## 3. Shutdown Sequence

### Initiation

Shutdown can be triggered by:
- **SIGTERM** / `Ctrl+C` (graceful, default)
- **SIGQUIT** (forceful, no persistence)
- **API call** (`POST /a2x/shutdown`)
- **Internal fault** (out of memory, unrecoverable error)

### Graceful Shutdown (SIGTERM)

```
1. STOP accepting new programs/connections
2. SIGTERM to all child processes
3. Drain in-flight programs (wait up to timeout)
4. Persist state to disk
5. Disconnect from bus
6. Close transport connections
7. Flush logs
8. Exit(0)
```

```rust
pub struct ShutdownManager {
    /// Delay before forcing shutdown.
    graceful_timeout: Duration,
    /// Callbacks to run during shutdown (in order).
    shutdown_hooks: Vec<Box<dyn FnOnce() -> Result<(), ShutdownError>>>,
}

impl ShutdownManager {
    /// Register a shutdown hook (e.g., "save WorldGraph").
    pub fn add_hook<F>(&mut self, hook: F)
    where F: FnOnce() -> Result<(), ShutdownError> + 'static { /* ... */ }

    /// Handle SIGTERM gracefully.
    pub async fn shutdown(&mut self) {
        // Log shutdown start
        tracing::info!("Shutdown initiated");

        for hook in self.shutdown_hooks.drain(..) {
            match tokio::time::timeout(self.graceful_timeout, async { hook() }).await {
                Ok(Ok(())) => {},
                Ok(Err(e)) => tracing::error!("Shutdown hook failed: {e}"),
                Err(_) => tracing::warn!("Shutdown hook timed out"),
            }
        }

        tracing::info!("Shutdown complete");
    }
}
```

### Agent Shutdown Order

```
Orchestrator ◄── CLI Agent(s) ◄── CCS Agent(s) ◄── LLM Agent(s)
     │                │                │                │
     ▼                ▼                ▼                ▼
  Halt + Save ──► Halt + Save ──► Halt + Save ──► Halt + Save
```

- **LLM agents** shut down first (they generate programs, not critical state)
- **CCS agents** shut down second (save WorldGraph + MemoryTrace)
- **CLI agents** shut down third (complete in-flight shell commands)
- **Orchestrator** shuts down last (drains all pending dispatches)

### Forceful Shutdown (SIGQUIT)

```
1. Kill all child processes immediately
2. Skip state persistence
3. Log warning
4. Exit(1)
```

---

## 4. State Persistence

### What Gets Persisted

| Data | Format | When Saved | When Loaded |
|------|--------|:----------:|:-----------:|
| WorldGraph (graph + nodes) | bincode (`worldgraph.bin`) | Shutdown + periodic checkpoint | Startup |
| MemoryTrace (execution history) | bincode (`memory.bin`) | Shutdown + periodic checkpoint | Startup |
| Agent config | TOML (`agents/*.toml`) | Created by user | Startup |
| Gateway config | TOML (`gateway.toml`) | Created by user | Startup (if gateway) |
| Log files | Plain text / JSON | Continuous | N/A |
| PID file | Plain text (`a2x.pid`) | Startup | Startup (check for existing) |
| Packet files | `.sigma` / `.omega` | On user request | On user request |

### Periodic Checkpointing

```rust
impl CcsVm {
    /// Periodically save state to disk (every N instructions).
    fn maybe_checkpoint(&self) -> Result<(), VmError> {
        if self.instruction_pointer % CHECKPOINT_INTERVAL == 0 {
            let checkpoint = Checkpoint {
                world_graph: &self.world_graph,
                state_field: &self.state_field,
                memory_trace: &self.memory_trace,
                ip: self.instruction_pointer,
                program_id: self.program.id,
            };
            // Write atomically (write to temp file, then rename)
            self.storage.save_checkpoint(&checkpoint)?;
        }
        Ok(())
    }
}
```

### Crash Recovery

On startup after an unclean shutdown:
1. Detect stale PID file OR corrupted state files
2. Load latest checkpoint (`checkpoint.tmp` → `worldgraph.bin`)
3. Replay MemoryTrace from checkpoint to last committed instruction
4. If no checkpoint exists, start with empty WorldGraph
5. Log recovery summary: `"Recovered from crash at instruction 1234"`

---

## 5. Directory Layout

```
~/.a2x/
├── config.toml            # Global config
├── gateway.toml           # Gateway config
├── a2x.pid                # PID file (runtime)
├── agents/                # Per-agent configs
│   ├── orchestrator-1.toml
│   ├── cli-1.toml
│   └── ccs-1.toml
├── data/                  # Persisted state
│   ├── orchestrator-1/
│   │   ├── worldgraph.bin
│   │   ├── worldgraph.bin.tmp  # Atomic write target
│   │   └── memory.bin
│   ├── cli-1/
│   │   └── worldgraph.bin
│   └── ccs-1/
│       ├── worldgraph.bin
│       └── memory.bin
├── packets/               # Saved packet files
│   └── 2026-06-28/
│       ├── *.sigma
│       └── *.omega
└── logs/                  # Log files
    ├── orchestrator-1.log
    ├── cli-1.log
    └── gateway.log
```

---

## 6. First Run Experience

When A2X runs for the first time:

```
$ a2x start
No config found at ~/.a2x/config.toml
Creating default configuration...
  ✓ ~/.a2x/ created
  ✓ ~/.a2x/config.toml created
  ✓ ~/.a2x/agents/orchestrator-1.toml created
  ✓ ~/.a2x/data/ created
  ✓ ~/.a2x/logs/ created

A2X is ready. Start agents with:
  a2x agent start cli-1
  a2x agent start ccs-1

Connect via gateway:
  a2x gateway start
  curl -X POST http://localhost:8778/a2x/execute ...
```

---

*This sub-plan maps to phases 0–1 of the implementation roadmap.*
