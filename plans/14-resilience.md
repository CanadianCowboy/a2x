# A2X Resilience — Graceful Degradation, Fault Tolerance & Crash Recovery

> **What happens when things go wrong: bus failures, agent crashes, lost connections, and corrupted state.**

---

## 1. Overview

A2X is a distributed system. Components fail. Networks partition. State gets corrupted. This plan defines how the system behaves under failure.

**Design philosophy:** Fail _safe_, not _hard_. Degrade gracefully. Recover automatically when possible.

---

## 2. Failure Modes

| Mode | What Fails | Impact | Recovery |
|------|-----------|--------|----------|
| **Agent crash** | Individual agent process stops | Programs on that agent are lost | Restart agent, reload from checkpoint |
| **Bus failure** | Message routing stops | All inter-agent communication drops | Reconnect with backoff |
| **Gateway failure** | Entity connections drop | External systems can't reach A2X | Restart gateway, re-auth entities |
| **Network partition** | Agents can't reach each other | Agents become isolated | Continue in offline mode, reconcile later |
| **Storage corruption** | WorldGraph/memory files corrupt | Agent can't restore state | Fall back to empty state, log corruption |
| **Resource exhaustion** | Memory/disk full | System-wide degradation | Throttle, evict, alert |
| **Deadlock / infinite loop** | Program doesn't complete | Agent stuck on one program | Watchdog timeout, kill program |
| **Malicious input** | Bad Σ∞ packet | Parser crash or undefined behavior | Input validation, fuzzing, sandbox |

---

## 3. Agent Crash Recovery

### Detection

```rust
pub enum CrashDetection {
    /// Watchdog thread monitors agent heartbeat.
    Watchdog {
        interval: Duration,    // Check every N seconds
        threshold: u32,        // N missed heartbeats = dead
    },
    /// Parent process monitors child exit status.
    ProcessExit {
        restart_on: Vec<i32>,  // Exit codes that trigger restart
    },
}
```

### Crash Recovery Sequence

```
1. DETECT: Heartbeat timeout or process exit
2. ISOLATE: Bus marks agent as offline, routes around it
3. ALERT: Log crash event, emit SecurityEvent::AgentLeft
4. RESTART: Start new agent process (with backoff)
5. RECOVER: Load latest checkpoint from disk
6. REJOIN: Agent re-announces on bus
7. VERIFY: Run health check program to confirm state
```

```rust
/// Agent supervisor — monitors and restarts crashed agents.
pub struct AgentSupervisor {
    agents: HashMap<AgentId, AgentProcess>,
    max_restarts_per_minute: u32,
    restart_delay: Duration,         // Base delay (exponential backoff)
}

impl AgentSupervisor {
    async fn watch_agent(&mut self, id: AgentId) {
        loop {
            tokio::time::sleep(self.check_interval).await;
            if !self.is_alive(&id) {
                // Exponential backoff
                let delay = self.restart_delay * 2u32.pow(self.agents[&id].restart_count);
                tracing::warn!("Agent {id} unresponsive, restarting in {delay:?}");

                tokio::time::sleep(delay).await;
                match self.restart_agent(&id).await {
                    Ok(()) => {
                        tracing::info!("Agent {id} restarted successfully");
                        self.agents.get_mut(&id).unwrap().restart_count = 0;
                    },
                    Err(e) => {
                        tracing::error!("Failed to restart agent {id}: {e}");
                        if self.agents[&id].restart_count > self.max_restarts_per_minute {
                            tracing::error!("Agent {id} exceeded restart limit, giving up");
                            return;
                        }
                    }
                }
            }
        }
    }
}
```

---

## 4. Bus Failure Handling

### In-Memory Bus

The in-memory bus uses `mpsc` channels. If a receiver drops (agent crash), the send gets a `SendError`:

```rust
// Sender side: handle disconnected agent
match bus.send(&agent_id, message) {
    Ok(()) => {},
    Err(SendError::ReceiverDropped) => {
        // Agent is gone, remove from routing table
        routing_table.remove(&agent_id);
        // Log event
        events.emit(DiscoveryEvent::AgentLeft { id: agent_id });
    }
}
```

### Network Bus (TCP / gRPC)

```
1. TCP connection drops
2. Transport detects via read timeout or broken pipe
3. Router marks agent as offline
4. In-flight programs: return error program "agent disconnected"
5. Agent reconnects (with exponential backoff: 1s, 2s, 4s, 8s, max 60s)
6. On reconnect: re-announce, resume heartbeats
```

### Network Partition

When agents can't reach each other but aren't crashed:

```
1. Each partition continues independently (offline mode)
2. Agents queue outbound messages for delivery when partition heals
3. On reconnect: reconcile queued messages
4. If conflicts detected: escalate to human (or use last-writer-wins)
```

---

## 5. Gateway Resilience

### Connection Retry

```rust
pub struct GatewayRetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,        // 1 second
    pub max_delay: Duration,         // 60 seconds
    pub jitter: f32,                 // 0.0 – 1.0 random jitter factor
}
```

### Webhook Redelivery

If A2X can't deliver a webhook callback:

```rust
pub struct WebhookRetry {
    pub max_retries: u32,            // Default: 3
    pub retry_delays: Vec<Duration>, // [1s, 10s, 60s]
    pub on_failure: WebhookFailureMode,
}

pub enum WebhookFailureMode {
    /// Drop the result (best-effort).
    Drop,
    /// Save for manual retrieval (ephemeral storage).
    Store(u32),  // Max TTL in seconds
    /// Escalate to human operator.
    Escalate,
}
```

---

## 6. Program-Level Resilience

### Watchdog Timer

Every program execution has a watchdog:

```rust
pub struct ProgramWatchdog {
    /// Max wall-clock time per program.
    pub max_execution_time: Duration,
    /// Max VM steps per program.
    pub max_instructions: u64,
    /// Action on timeout.
    pub on_timeout: TimeoutAction,
}

pub enum TimeoutAction {
    /// Kill the program, return error.
    Kill,
    /// Yield the program (save state, move to back of queue).
    Yield,
    /// Escalate to supervisor agent.
    Escalate,
}
```

### Retry Policies

```rust
pub enum RetryPolicy {
    /// No retry.
    None,
    /// Fixed number of retries with constant delay.
    Fixed { max_retries: u32, delay: Duration },
    /// Exponential backoff with jitter.
    Exponential { max_retries: u32, base_delay: Duration, max_delay: Duration },
}
```

### Instruction-Level Fault Tolerance

```rust
pub enum InstructionFaultMode {
    /// On fault: stop program, return error (default).
    FailFast,
    /// On fault: skip instruction, continue.
    SkipAndContinue,
    /// On fault: retry instruction N times.
    Retry(RetryPolicy),
    /// On fault: execute fallback instruction.
    Fallback(SigmaInstruction),
}
```

---

## 7. Storage Corruption

### Atomic Writes

All state files are written atomically:

```rust
pub fn save_atomic(path: &Path, data: &[u8]) -> Result<(), StorageError> {
    // 1. Write to temp file
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, data)?;

    // 2. Verify integrity (checksum)
    let checksum = blake3::hash(data);
    std::fs::write(tmp_path.with_extension("checksum"), checksum.as_bytes())?;

    // 3. Rename (atomic on same filesystem)
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}
```

### Integrity Checks

```rust
pub fn load_with_integrity<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }

    // 1. Read data
    let data = std::fs::read(path)?;

    // 2. Verify checksum
    let checksum_path = path.with_extension("checksum");
    if checksum_path.exists() {
        let expected = std::fs::read(&checksum_path)?;
        let actual = blake3::hash(&data);
        if expected != actual.as_bytes() {
            // Corruption detected!
            // Try loading from backup (.bak)
            let bak_path = path.with_extension("bak");
            if bak_path.exists() {
                tracing::warn!("Corruption detected in {path:?}, loading backup");
                return load_with_integrity(&bak_path);
            }
            return Err(StorageError::Corruption { path: path.into() });
        }
    }

    // 3. Deserialize
    let value: T = bincode::deserialize(&data)?;
    Ok(Some(value))
}
```

### Corruption Recovery

```
Corruption detected
    │
    ├── Try backup (.bak) ──OK──→ Load backup, log warning
    │
    ├── Try checkpoint (.tmp) ──OK──→ Load checkpoint, log warning
    │
    └── All failed ──→ Fall back to empty state
                       Log error
                       Alert operator
```

---

## 8. Resource Exhaustion

### Memory Pressure

```rust
pub enum MemoryPressureAction {
    /// Log warning, continue.
    Warn(f32), // Threshold (e.g., 0.8 = 80% of limit)
    /// Throttle new program submissions.
    Throttle(f32), // Threshold
    /// Evict least-recently-used WorldGraph nodes.
    Evict(f32), // Threshold (saves to disk first)
    /// Kill lowest-priority programs.
    Kill(f32), // Threshold
}
```

### Disk Space

- Periodic check: is `~/.a2x/` disk usage > 90%?
- If yes: compact logs, drop old checkpoints, alert operator
- If critical (< 5% free): stop accepting new programs, flush logs

---

## 9. Graceful Degradation Summary

| Failure | Degraded Behavior | Full Recovery |
|---------|-------------------|---------------|
| Agent crash | Work redistributed to peers | Agent restarts, reloads checkpoint |
| Bus down | Agents run in isolation, queue messages | Bus restarts, queued messages delivered |
| Gateway down | External systems can't connect | Gateway restarts, entities re-auth |
| Network partition | Each partition continues independently | Partition heals, messages reconciled |
| Storage corrupt | Fall back to empty state, log error | Restore from backup if available |
| Memory full | Throttle + evict LRU nodes | Free memory by eviction |
| Program timeout (⚡) | Kill program, return error | None |
| Program timeout (normal) | Yield program, move to back of queue | Program resumes when scheduler runs it |

---

*This sub-plan maps to phases 2–6 (incremental hardening).*
